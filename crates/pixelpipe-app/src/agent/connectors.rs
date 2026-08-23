use std::{
    env, fs,
    path::{Path, PathBuf},
};

use pixelpipe_core::stable_json;
use pixelpipe_project::AgentCapability;
use serde::{Deserialize, Serialize};

use super::{AgentProfile, PROFILE_SCHEMA, user_profile_directory};
use crate::AppError;

const AGENT_PROMPT: &str = concat!(
    "You are PixelPipe's explicitly approved local art agent. Read pixelpipe-request.json in the ",
    "current working directory. It contains a pixelpipe.agent-request/v1 task and approved ",
    "workspace paths. For generate_references, create three distinct smooth high-resolution RGBA ",
    "PNG reference images (not pixel art), faithfully following the brief, with one centered subject ",
    "on transparency or pure white. Prefer your image-generation tools. If none are available, ",
    "create clean vector-like art with a dependency-free supersampled rasterizer and PNG encoder. ",
    "If an image tool returns a remote attachment, try the agent's supported authenticated file ",
    "download command once per image. If transfer fails, do not retry with curl, browsers, login ",
    "flows, or repeated diagnostics; immediately use the local rendering fallback. Never report a ",
    "remote URL as a candidate. A candidate exists only after its PNG is inside the output directory. ",
    "Write PNGs directly inside workspace.output_directory. For critique_asset, inspect the supplied ",
    "native and preview PNGs. For propose_refinement, inspect the supplied pixels JSON and propose ",
    "one focused valid change. Your final response must be only strict pixelpipe.agent-response/v1 ",
    "JSON with adapter identity/capabilities and the matching result. Candidate paths must be ",
    "relative to output_directory and include lowercase-hyphen IDs and exact SHA-256 hashes. Never ",
    "select, convert, apply, review, approve, export, or modify the project.",
);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentConnector {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub approved: bool,
}

#[derive(Debug, Deserialize)]
pub struct ApproveAgentConnector {
    pub id: String,
}

/// Finds supported agent CLIs and reports whether the user approved each connector.
///
/// # Errors
///
/// Returns an error when `PixelPipe` cannot resolve the user-local profile directory.
pub fn detect_agent_connectors() -> Result<Vec<AgentConnector>, AppError> {
    let profiles = user_profile_directory()?;
    Ok([("amp", "Amp"), ("codex", "Codex")]
        .into_iter()
        .map(|(id, name)| AgentConnector {
            id: id.to_owned(),
            name: name.to_owned(),
            installed: find_executable(id).is_some(),
            approved: profiles.join(format!("{id}.json")).is_file(),
        })
        .collect())
}

/// Creates a user-local approved profile for a supported installed connector.
///
/// # Errors
///
/// Returns an error for unsupported or missing executables and profile storage failures.
pub fn approve_agent_connector(request: ApproveAgentConnector) -> Result<AgentConnector, AppError> {
    let (name, executable, args) = connector_command(&request.id)?;
    let secret_environment = connector_secrets(&request.id)
        .iter()
        .filter(|name| env::var_os(name).is_some())
        .map(|name| (*name).to_owned())
        .collect();
    let profile = AgentProfile {
        schema: PROFILE_SCHEMA.to_owned(),
        id: request.id.clone(),
        approved: true,
        executable,
        args,
        capabilities: vec![
            AgentCapability::GenerateReferences,
            AgentCapability::CritiqueAsset,
            AgentCapability::ProposeRefinement,
        ],
        environment: vec!["HOME".to_owned(), "PATH".to_owned()],
        secret_environment,
        timeout_seconds: 600,
    };
    let directory = user_profile_directory()?;
    fs::create_dir_all(&directory).map_err(|source| {
        AppError::AgentProfile(format!("cannot create agent settings: {source}"))
    })?;
    fs::write(
        directory.join(format!("{}.json", request.id)),
        stable_json(&profile)?,
    )
    .map_err(|source| AppError::AgentProfile(format!("cannot approve connector: {source}")))?;
    Ok(AgentConnector {
        id: request.id,
        name: name.to_owned(),
        installed: true,
        approved: true,
    })
}

fn connector_secrets(id: &str) -> &'static [&'static str] {
    match id {
        "amp" => &["AMP_API_KEY"],
        "codex" => &["OPENAI_API_KEY"],
        _ => &[],
    }
}

fn connector_command(id: &str) -> Result<(&'static str, PathBuf, Vec<String>), AppError> {
    let executable = find_executable(id).ok_or_else(|| {
        AppError::AgentProfile(format!("{id} is not installed or visible on PATH"))
    })?;
    match id {
        "amp" => Ok((
            "Amp",
            executable,
            vec!["--execute".to_owned(), AGENT_PROMPT.to_owned()],
        )),
        "codex" => Ok((
            "Codex",
            executable,
            vec![
                "exec".to_owned(),
                "--ephemeral".to_owned(),
                "--sandbox".to_owned(),
                "workspace-write".to_owned(),
                "--skip-git-repo-check".to_owned(),
                AGENT_PROMPT.to_owned(),
            ],
        )),
        _ => Err(AppError::AgentProfile(format!(
            "unsupported built-in connector '{id}'"
        ))),
    }
}

fn find_executable(name: &str) -> Option<PathBuf> {
    let mut directories = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        directories.extend([
            home.join(".amp/bin"),
            home.join(".local/bin"),
            home.join(".npm-global/bin"),
        ]);
    }
    directories.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ]);
    directories.into_iter().find_map(|directory| {
        let candidate = directory.join(name);
        executable_file(&candidate).then(|| candidate.canonicalize().unwrap_or(candidate))
    })
}

fn executable_file(path: &Path) -> bool {
    executable_metadata(path)
}

#[cfg(unix)]
fn executable_metadata(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn executable_metadata(path: &Path) -> bool {
    path.metadata().is_ok_and(|metadata| metadata.is_file())
}
