use std::{collections::BTreeMap, path::PathBuf};

use pixelpipe_core::{
    ConversionSettings, IndexedRaster, Operation, Palette, RECIPE_SCHEMA, Recipe, SheetSettings,
    convert_reference, convert_sheet, decode_rgba_png, sha256_hex, stable_json,
};
use pixelpipe_project::{AssetKind, ProjectError, ProjectStore};

use crate::{
    AppError,
    revision_commit::{CommitRaster, RevisionResult, commit_raster, read},
};

#[derive(Debug)]
pub struct CreateRevision {
    pub start: PathBuf,
    pub asset: String,
    pub kind: AssetKind,
    pub raster_path: PathBuf,
    pub brief_path: Option<PathBuf>,
    pub preview_scale: Option<u16>,
    pub actor: String,
}

#[derive(Debug, Clone)]
pub enum ConversionMode {
    Reference(ConversionSettings),
    Sheet(SheetSettings),
}

#[derive(Debug)]
pub struct ConvertRevision {
    pub start: PathBuf,
    pub asset: String,
    pub kind: AssetKind,
    pub source_path: PathBuf,
    pub palette_path: PathBuf,
    pub mode: ConversionMode,
    pub brief_path: Option<PathBuf>,
    pub preview_scale: Option<u16>,
    pub actor: String,
}

/// Validates a structured raster, renders it, and commits an immutable revision.
///
/// # Errors
///
/// Returns an [`AppError`] when project discovery, input decoding, deterministic
/// rendering, or revision storage fails.
pub fn create_revision(request: CreateRevision) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    reject_pre_revision_asset(&store, &request.asset, "create revision")?;
    let raster_bytes = read(&request.raster_path)?;
    let raster: IndexedRaster =
        serde_json::from_slice(&raster_bytes).map_err(|source| AppError::RasterJson {
            path: request.raster_path.clone(),
            source,
        })?;
    let manifest = store.manifest()?;
    let preview_scale = request.preview_scale.unwrap_or(manifest.preview_scale);
    let canonical_raster = stable_json(&raster)?;
    let palette_bytes = stable_json(&raster.palette)?;
    let recipe = Recipe {
        schema: RECIPE_SCHEMA.to_owned(),
        input_sha256: sha256_hex(&canonical_raster),
        palette_sha256: sha256_hex(&palette_bytes),
        operations: vec![Operation::RenderIndexed { preview_scale }],
    };
    let brief = read_brief(request.brief_path)?;
    let input_hashes = BTreeMap::from([
        ("palette".to_owned(), recipe.palette_sha256.clone()),
        ("pixels".to_owned(), recipe.input_sha256.clone()),
    ]);
    commit_raster(
        &store,
        CommitRaster {
            asset: request.asset,
            kind: request.kind,
            raster,
            recipe,
            preview_scale,
            brief,
            actor: request.actor,
            input_hashes,
            additional_checks: Vec::new(),
            parent: None,
            style: None,
        },
    )
}

/// Converts a smooth RGBA reference and commits the result as an immutable revision.
///
/// # Errors
///
/// Returns an [`AppError`] when project discovery, input decoding, deterministic
/// conversion/rendering, or revision storage fails.
pub fn convert_revision(request: ConvertRevision) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    reject_pre_revision_asset(&store, &request.asset, "convert unselected reference")?;
    let source_bytes = read(&request.source_path)?;
    let source = decode_rgba_png(&source_bytes)?;
    let palette_bytes = read(&request.palette_path)?;
    let palette: Palette =
        serde_json::from_slice(&palette_bytes).map_err(|source| AppError::PaletteJson {
            path: request.palette_path.clone(),
            source,
        })?;
    let canonical_palette = stable_json(&palette)?;
    let (converted, operation) = match request.mode {
        ConversionMode::Reference(settings) => {
            let converted = convert_reference(&source, &palette, &settings)?;
            (converted, Operation::ConvertReference { settings })
        }
        ConversionMode::Sheet(settings) => {
            let converted = convert_sheet(&source, &palette, &settings)?;
            (converted, Operation::ConvertSheet { settings })
        }
    };
    let manifest = store.manifest()?;
    let preview_scale = request.preview_scale.unwrap_or(manifest.preview_scale);
    let stored_reference = store.import_reference(&request.asset, &source_bytes)?;
    let recipe = Recipe {
        schema: RECIPE_SCHEMA.to_owned(),
        input_sha256: stored_reference.sha256,
        palette_sha256: sha256_hex(&canonical_palette),
        operations: vec![operation, Operation::RenderIndexed { preview_scale }],
    };
    let brief = read_brief(request.brief_path)?;
    let input_hashes = BTreeMap::from([
        ("palette".to_owned(), recipe.palette_sha256.clone()),
        ("reference".to_owned(), recipe.input_sha256.clone()),
    ]);
    commit_raster(
        &store,
        CommitRaster {
            asset: request.asset,
            kind: request.kind,
            raster: converted.raster,
            recipe,
            preview_scale,
            brief,
            actor: request.actor,
            input_hashes,
            additional_checks: converted.checks,
            parent: None,
            style: None,
        },
    )
}

fn reject_pre_revision_asset(
    store: &ProjectStore,
    asset: &str,
    operation: &'static str,
) -> Result<(), AppError> {
    if store
        .optional_asset(asset)?
        .is_some_and(|manifest| manifest.head.is_none())
    {
        return Err(ProjectError::AssetNotReady {
            asset: asset.to_owned(),
            operation,
            reason: "selected-reference conversion is the only first-revision operation",
        }
        .into());
    }
    Ok(())
}

fn read_brief(path: Option<PathBuf>) -> Result<String, AppError> {
    match path {
        Some(path) => String::from_utf8(read(&path)?).map_err(|_| AppError::BriefUtf8 { path }),
        None => Ok(String::new()),
    }
}
