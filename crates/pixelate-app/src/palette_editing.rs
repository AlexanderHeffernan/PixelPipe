use std::{collections::BTreeMap, path::PathBuf};

use pixelate_core::{
    Operation, PaletteRemap, RECIPE_SCHEMA, Recipe, ValidationCheck, apply_palette_remap,
    sha256_hex, stable_json,
};
use pixelate_project::{AssetManifest, ProjectStore};
use serde::Deserialize;

use crate::{
    AppError,
    revision_commit::{
        CommitRaster, RevisionResult, commit_raster, component_rule, inherit_structure, read,
        read_optional_brief,
    },
};

#[derive(Debug)]
pub struct RemapRevision {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    pub remap_path: PathBuf,
    pub brief_path: Option<PathBuf>,
    pub preview_scale: Option<u16>,
    pub actor: String,
}

#[derive(Debug, Deserialize)]
pub struct SetAssetHead {
    pub start: PathBuf,
    pub asset: String,
    pub revision: String,
}

#[derive(Debug, Deserialize)]
pub struct RemapRevisionDocument {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    pub remap: PaletteRemap,
    pub brief: Option<String>,
    pub preview_scale: Option<u16>,
    pub actor: String,
}

/// Explicitly moves an asset head to an existing immutable revision.
///
/// # Errors
///
/// Returns an [`AppError`] when discovery, revision verification, or storage fails.
pub fn set_asset_head(request: SetAssetHead) -> Result<AssetManifest, AppError> {
    let SetAssetHead {
        start,
        asset,
        revision,
    } = request;
    let store = ProjectStore::discover(&start)?;
    Ok(store.set_asset_head(&asset, &revision)?)
}

/// Applies an explicit palette index map to a parent and creates a new revision.
///
/// # Errors
///
/// Returns an [`AppError`] when loading, remap validation, rendering, or atomic
/// revision storage fails.
pub fn remap_revision(request: RemapRevision) -> Result<RevisionResult, AppError> {
    let path = request.remap_path;
    let remap: PaletteRemap =
        serde_json::from_slice(&read(&path)?).map_err(|source| AppError::OperationJson {
            path: path.clone(),
            source,
        })?;
    let brief = read_optional_brief(request.brief_path)?;
    remap_revision_document(RemapRevisionDocument {
        start: request.start,
        asset: request.asset,
        parent: request.parent,
        remap,
        brief,
        preview_scale: request.preview_scale,
        actor: request.actor,
    })
}

/// Applies a typed palette remap through the same application use case as the CLI.
///
/// # Errors
///
/// Returns an [`AppError`] when loading, validation, rendering, or storage fails.
pub fn remap_revision_document(request: RemapRevisionDocument) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let parent = store.revision(&request.asset, &request.parent)?;
    let mut remap = request.remap;
    inherit_structure(&mut remap.structure, component_rule(&parent.recipe))?;
    let raster = apply_palette_remap(&parent.raster, &remap)?;
    let preview_scale = request
        .preview_scale
        .unwrap_or(store.manifest()?.preview_scale);
    let recipe = Recipe {
        schema: RECIPE_SCHEMA.to_owned(),
        input_sha256: sha256_hex(&stable_json(&parent.raster)?),
        palette_sha256: sha256_hex(&stable_json(&raster.palette)?),
        operations: vec![
            Operation::RemapPalette {
                remap: remap.clone(),
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
                name: "palette_remap".to_owned(),
                passed: true,
                detail: remap.index_map.len().to_string(),
            }],
            parent: Some(request.parent),
            style: None,
        },
    )
}
