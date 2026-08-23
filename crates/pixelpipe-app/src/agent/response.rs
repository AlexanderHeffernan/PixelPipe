use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use pixelpipe_core::{apply_palette_remap, apply_pixel_patch, decode_rgba_png, sha256_hex};
use pixelpipe_project::{
    AgentCandidate, AgentIdentity, AgentOperation, AgentProposal, ProjectStore,
};
use serde::Deserialize;

use super::attachments::validate_amp_attachment_url;
use super::{RESPONSE_SCHEMA, redact, required_capability, validate_local_id};
use crate::{AppError, component_rule, inherit_structure};

pub(super) type AttachmentFetcher<'a> = dyn Fn(&str, &AtomicBool) -> Result<Vec<u8>, AppError> + 'a;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtocolResponse {
    schema: String,
    adapter: AgentIdentity,
    result: ProtocolResult,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum ProtocolResult {
    GeneratedReferences { candidates: Vec<ProtocolCandidate> },
    Critique { text: String },
    Proposal { proposal: AgentProposal },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtocolCandidate {
    id: String,
    #[serde(default)]
    path: Option<PathBuf>,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    attachment_url: Option<String>,
}

#[derive(Debug)]
pub(super) struct ProcessedResponse {
    pub adapter: AgentIdentity,
    pub candidates: Vec<AgentCandidate>,
    pub candidate_bytes: BTreeMap<String, Vec<u8>>,
    pub critique: Option<String>,
    pub proposal: Option<AgentProposal>,
}

pub(super) struct ResponseContext<'a> {
    pub operation: AgentOperation,
    pub asset: &'a str,
    pub revision: Option<&'a str>,
    pub store: &'a ProjectStore,
    pub output_directory: &'a Path,
    pub secrets: &'a [String],
    pub cancel: &'a AtomicBool,
    pub attachment_fetcher: Option<&'a AttachmentFetcher<'a>>,
    pub progress: &'a dyn Fn(String),
}

pub(super) fn process_response(
    bytes: &[u8],
    context: &ResponseContext<'_>,
) -> Result<ProcessedResponse, AppError> {
    let mut response: ProtocolResponse = serde_json::from_slice(bytes)
        .map_err(|source| AppError::AgentProtocol(format!("invalid JSON response: {source}")))?;
    validate_response(&response, context.operation)?;
    redact_identity(&mut response.adapter, context.secrets);
    let mut processed = ProcessedResponse {
        adapter: response.adapter,
        candidates: Vec::new(),
        candidate_bytes: BTreeMap::new(),
        critique: None,
        proposal: None,
    };
    match (context.operation, response.result) {
        (
            AgentOperation::GenerateReferences,
            ProtocolResult::GeneratedReferences { candidates },
        ) => {
            process_candidates(candidates, context, &mut processed)?;
        }
        (AgentOperation::CritiqueAsset, ProtocolResult::Critique { text }) => {
            if text.trim().is_empty() {
                return Err(AppError::AgentProtocol("critique is empty".to_owned()));
            }
            processed.critique = Some(redact(&text, context.secrets));
        }
        (AgentOperation::ProposeRefinement, ProtocolResult::Proposal { proposal }) => {
            validate_proposal(&proposal, context)?;
            processed.proposal = Some(proposal);
        }
        _ => {
            return Err(AppError::AgentProtocol(
                "response result does not match requested operation".to_owned(),
            ));
        }
    }
    Ok(processed)
}

fn validate_response(
    response: &ProtocolResponse,
    operation: AgentOperation,
) -> Result<(), AppError> {
    if response.schema != RESPONSE_SCHEMA {
        return Err(AppError::AgentProtocol(format!(
            "unsupported response schema '{}'",
            response.schema
        )));
    }
    if !response
        .adapter
        .capabilities
        .contains(&required_capability(operation))
    {
        return Err(AppError::AgentProtocol(
            "adapter did not report the requested capability".to_owned(),
        ));
    }
    Ok(())
}

fn redact_identity(identity: &mut AgentIdentity, secrets: &[String]) {
    identity.adapter = redact(&identity.adapter, secrets);
    identity.provider = identity
        .provider
        .take()
        .map(|value| redact(&value, secrets));
    identity.model = identity.model.take().map(|value| redact(&value, secrets));
}

fn process_candidates(
    candidates: Vec<ProtocolCandidate>,
    context: &ResponseContext<'_>,
    processed: &mut ProcessedResponse,
) -> Result<(), AppError> {
    if candidates.is_empty() {
        return Err(AppError::AgentProtocol(
            "generation returned no candidates".to_owned(),
        ));
    }
    for candidate in candidates {
        let id = candidate.id.clone();
        (context.progress)(format!("Validating reference '{id}'."));
        let (metadata, bytes) = validate_candidate(candidate, context)?;
        if processed.candidate_bytes.contains_key(&metadata.id)
            || processed
                .candidates
                .iter()
                .any(|candidate| candidate.sha256 == metadata.sha256)
        {
            return Err(AppError::AgentProtocol(format!(
                "duplicate candidate id or content hash '{}'",
                metadata.id
            )));
        }
        processed.candidate_bytes.insert(metadata.id.clone(), bytes);
        processed.candidates.push(metadata);
    }
    Ok(())
}

fn validate_candidate(
    candidate: ProtocolCandidate,
    context: &ResponseContext<'_>,
) -> Result<(AgentCandidate, Vec<u8>), AppError> {
    validate_local_id(&candidate.id)?;
    let bytes = match (candidate.path, candidate.sha256, candidate.attachment_url) {
        (Some(path), Some(expected_hash), None) => read_local_candidate(
            context.output_directory,
            &candidate.id,
            &path,
            &expected_hash,
        )?,
        (None, None, Some(url)) => {
            validate_amp_attachment_url(&url)?;
            let fetch = context.attachment_fetcher.ok_or_else(|| {
                AppError::AgentProtocol(
                    "remote Amp attachments require the approved Amp connector".to_owned(),
                )
            })?;
            fetch(&url, context.cancel)?
        }
        _ => {
            return Err(AppError::AgentProtocol(format!(
                "candidate '{}' must provide either path and sha256 or one attachment_url",
                candidate.id
            )));
        }
    };
    context
        .cancel
        .load(Ordering::Relaxed)
        .then_some(())
        .map_or(Ok(()), |()| {
            Err(AppError::AgentProcess("task cancelled by user".to_owned()))
        })?;
    let actual_hash = sha256_hex(&bytes);
    let image = decode_rgba_png(&bytes)?;
    Ok((
        AgentCandidate {
            id: candidate.id,
            sha256: actual_hash.clone(),
            width: image.width,
            height: image.height,
            png: format!("candidates/{actual_hash}.png"),
        },
        bytes,
    ))
}

fn read_local_candidate(
    output_directory: &Path,
    id: &str,
    relative_path: &Path,
    expected_hash: &str,
) -> Result<Vec<u8>, AppError> {
    if relative_path.is_absolute() {
        return Err(AppError::AgentCandidatePath(
            "absolute paths are not accepted".to_owned(),
        ));
    }
    let root = output_directory.canonicalize().map_err(|source| {
        AppError::AgentCandidatePath(format!("cannot verify output directory: {source}"))
    })?;
    let canonical = output_directory
        .join(relative_path)
        .canonicalize()
        .map_err(|source| {
            AppError::AgentCandidatePath(format!("cannot verify '{id}': {source}"))
        })?;
    if !canonical.starts_with(&root) || !canonical.is_file() {
        return Err(AppError::AgentCandidatePath(format!(
            "candidate '{id}' escapes its assigned output directory"
        )));
    }
    let bytes = fs::read(&canonical)
        .map_err(|source| AppError::AgentCandidatePath(format!("cannot read '{id}': {source}")))?;
    if sha256_hex(&bytes) != expected_hash {
        return Err(AppError::AgentProtocol(format!(
            "candidate '{id}' hash does not match its bytes"
        )));
    }
    Ok(bytes)
}

fn validate_proposal(
    proposal: &AgentProposal,
    context: &ResponseContext<'_>,
) -> Result<(), AppError> {
    let revision = context
        .revision
        .ok_or_else(|| AppError::AgentProtocol("proposal requires a revision".to_owned()))?;
    let snapshot = context.store.revision(context.asset, revision)?;
    match proposal {
        AgentProposal::PixelPatch { patch } => {
            let mut patch = patch.clone();
            inherit_structure(&mut patch.structure, component_rule(&snapshot.recipe))?;
            apply_pixel_patch(&snapshot.raster, &patch)?;
        }
        AgentProposal::PaletteRemap { remap } => {
            let mut remap = remap.clone();
            inherit_structure(&mut remap.structure, component_rule(&snapshot.recipe))?;
            apply_palette_remap(&snapshot.raster, &remap)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
