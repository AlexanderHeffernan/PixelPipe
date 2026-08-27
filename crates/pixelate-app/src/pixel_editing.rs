use std::{collections::BTreeMap, path::PathBuf};

use pixelate_core::{
    Operation, PixelPatchSet, RECIPE_SCHEMA, Recipe, ValidationCheck, apply_pixel_patch,
    flood_fill_patch, sha256_hex, stable_json,
};
use pixelate_project::ProjectStore;
use serde::Deserialize;

use crate::{
    AppError,
    revision_commit::{
        CommitRaster, RevisionResult, commit_raster, component_rule, inherit_structure, read,
        read_optional_brief,
    },
};

#[derive(Debug)]
pub struct PatchRevision {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    pub patch_path: PathBuf,
    pub brief_path: Option<PathBuf>,
    pub preview_scale: Option<u16>,
    pub actor: String,
}

#[derive(Debug, Deserialize)]
pub struct PatchRevisionDocument {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    pub patch: PixelPatchSet,
    pub brief: Option<String>,
    pub preview_scale: Option<u16>,
    pub actor: String,
}

#[derive(Debug, Deserialize)]
pub struct FillRevisionDocument {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    pub x: u32,
    pub y: u32,
    pub index: u8,
    pub actor: String,
}

/// Applies a validated pixel patch to an explicit parent and creates a new revision.
///
/// # Errors
///
/// Returns an [`AppError`] when loading, patch validation, rendering, or atomic
/// revision storage fails.
pub fn patch_revision(request: PatchRevision) -> Result<RevisionResult, AppError> {
    let operation_path = request.patch_path;
    let patch: PixelPatchSet =
        serde_json::from_slice(&read(&operation_path)?).map_err(|source| {
            AppError::OperationJson {
                path: operation_path.clone(),
                source,
            }
        })?;
    let brief = read_optional_brief(request.brief_path)?;
    patch_revision_document(PatchRevisionDocument {
        start: request.start,
        asset: request.asset,
        parent: request.parent,
        patch,
        brief,
        preview_scale: request.preview_scale,
        actor: request.actor,
    })
}

/// Applies a typed patch document through the same application use case as the CLI.
///
/// # Errors
///
/// Returns an [`AppError`] when loading, validation, rendering, or storage fails.
pub fn patch_revision_document(request: PatchRevisionDocument) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let parent = store.revision(&request.asset, &request.parent)?;
    let mut patch = request.patch;
    inherit_structure(&mut patch.structure, component_rule(&parent.recipe))?;
    let raster = apply_pixel_patch(&parent.raster, &patch)?;
    let preview_scale = request
        .preview_scale
        .unwrap_or(store.manifest()?.preview_scale);
    let recipe = Recipe {
        schema: RECIPE_SCHEMA.to_owned(),
        input_sha256: sha256_hex(&stable_json(&parent.raster)?),
        palette_sha256: sha256_hex(&stable_json(&raster.palette)?),
        operations: vec![
            Operation::PatchPixels {
                patch: patch.clone(),
            },
            Operation::RenderIndexed { preview_scale },
        ],
    };
    let brief = request.brief.unwrap_or(parent.brief);
    let input_hashes = BTreeMap::from([
        ("palette".to_owned(), recipe.palette_sha256.clone()),
        ("parent_pixels".to_owned(), recipe.input_sha256.clone()),
    ]);
    let kind = store.asset(&request.asset)?.kind;
    commit_raster(
        &store,
        CommitRaster {
            asset: request.asset,
            kind,
            raster,
            recipe,
            preview_scale,
            brief,
            actor: request.actor,
            input_hashes,
            additional_checks: vec![ValidationCheck {
                name: "pixel_patch".to_owned(),
                passed: true,
                detail: patch.edits.len().to_string(),
            }],
            parent: Some(request.parent),
            style: None,
        },
    )
}

/// Resolves and commits one deterministic four-connected fill as one revision.
///
/// # Errors
///
/// Returns an [`AppError`] when loading, fill resolution, validation, or storage fails.
pub fn fill_revision_document(request: FillRevisionDocument) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let parent = store.revision(&request.asset, &request.parent)?;
    let patch = flood_fill_patch(&parent.raster, request.x, request.y, request.index)?;
    patch_revision_document(PatchRevisionDocument {
        start: request.start,
        asset: request.asset,
        parent: request.parent,
        patch,
        brief: None,
        preview_scale: None,
        actor: request.actor,
    })
}
