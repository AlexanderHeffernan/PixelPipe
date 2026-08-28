use std::{collections::BTreeMap, fs, path::PathBuf};

use pixelate_core::{
    ComponentRule, IndexedRaster, Operation, Recipe, ValidationCheck, render, sha256_hex,
};
use pixelate_project::{AssetStyle, ProjectStore, RevisionFiles, StoredRevision};
use serde::Serialize;

use crate::AppError;

pub(crate) struct CommitRaster {
    pub(crate) asset: String,
    pub(crate) raster: IndexedRaster,
    pub(crate) recipe: Recipe,
    pub(crate) brief: String,
    pub(crate) actor: String,
    pub(crate) input_hashes: BTreeMap<String, String>,
    pub(crate) additional_checks: Vec<ValidationCheck>,
    pub(crate) parent: Option<String>,
    pub(crate) style: Option<AssetStyle>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RevisionResult {
    pub project_root: PathBuf,
    pub asset: String,
    pub revision: String,
    pub parent: Option<String>,
    pub revision_path: PathBuf,
    pub native_sha256: String,
}

pub(crate) fn resolve_revision(
    store: &ProjectStore,
    asset: &str,
    revision: Option<String>,
) -> Result<String, AppError> {
    match revision {
        Some(revision) => Ok(revision),
        None => store
            .asset(asset)?
            .head
            .ok_or_else(|| AppError::NoHead(asset.to_owned())),
    }
}

pub(crate) fn component_rule(recipe: &Recipe) -> Option<ComponentRule> {
    recipe
        .operations
        .iter()
        .rev()
        .find_map(|operation| match operation {
            Operation::ConvertReference { settings } => Some(ComponentRule::Raster {
                expectation: settings.components,
            }),
            Operation::ImportPixelArt | Operation::ComposeCanvas { .. } => None,
            Operation::PatchPixels { patch } => patch.structure,
            Operation::RemapPalette { remap } => remap.structure,
        })
}

pub(crate) fn inherit_structure(
    operation: &mut Option<ComponentRule>,
    inherited: Option<ComponentRule>,
) -> Result<(), AppError> {
    match (*operation, inherited) {
        (Some(operation), Some(inherited)) if operation != inherited => {
            Err(AppError::StructureRuleConflict)
        }
        (None, inherited) => {
            *operation = inherited;
            Ok(())
        }
        _ => Ok(()),
    }
}

pub(crate) fn read_optional_brief(path: Option<PathBuf>) -> Result<Option<String>, AppError> {
    match path {
        Some(path) => String::from_utf8(read(&path)?)
            .map(Some)
            .map_err(|_| AppError::BriefUtf8 { path }),
        None => Ok(None),
    }
}

pub(crate) fn commit_raster(
    store: &ProjectStore,
    commit: CommitRaster,
) -> Result<RevisionResult, AppError> {
    let mut rendered = render(&commit.raster, 1)?;
    rendered.validation.checks.extend(commit.additional_checks);
    let native_sha256 = sha256_hex(&rendered.native_png);
    let output_hashes = BTreeMap::from([("native.png".to_owned(), native_sha256.clone())]);
    let files = RevisionFiles {
        raster: commit.raster,
        recipe: commit.recipe,
        validation: rendered.validation,
        native_png: rendered.native_png,
        brief: commit.brief,
        actor: commit.actor,
        input_hashes: commit.input_hashes,
        output_hashes,
        style: commit.style,
    };
    let stored = match commit.parent {
        Some(parent) => store.create_revision_from(&commit.asset, &parent, files)?,
        None => store.create_revision(&commit.asset, files)?,
    };

    Ok(result(stored, native_sha256))
}

pub(crate) fn read(path: &PathBuf) -> Result<Vec<u8>, AppError> {
    fs::read(path).map_err(|source| AppError::Read {
        path: path.clone(),
        source,
    })
}

fn result(stored: StoredRevision, native_sha256: String) -> RevisionResult {
    RevisionResult {
        project_root: stored.project_root,
        asset: stored.asset,
        revision: stored.revision,
        parent: stored.parent,
        revision_path: stored.revision_path,
        native_sha256,
    }
}
