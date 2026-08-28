use std::{fs, io::Cursor, path::PathBuf};

use image::ImageFormat;
use pixelate_core::decode_rgba_png;
use pixelate_project::{ProjectStore, ReferenceSelection};
use serde::{Deserialize, Serialize};

use crate::{AppError, ConvertSelectedReference, RevisionResult, convert_selected_reference};

#[derive(Debug, Deserialize)]
pub struct ImportReference {
    pub start: PathBuf,
    pub asset: String,
    pub file: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAssetSource {
    pub start: PathBuf,
    pub asset: String,
    pub file: PathBuf,
    pub actor: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateAssetSourceResult {
    pub reference: ReferenceSelection,
    pub revision: RevisionResult,
}

/// Normalizes and content-addresses a user-selected image as the asset reference.
///
/// # Errors
///
/// Returns an error when the file, image, project, or asset transition is invalid.
pub fn import_reference(request: ImportReference) -> Result<ReferenceSelection, AppError> {
    let source_file = request.file.clone();
    let bytes = fs::read(&request.file).map_err(|source| AppError::Read {
        path: request.file,
        source,
    })?;
    let image =
        image::load_from_memory(&bytes).map_err(|error| AppError::Image(error.to_string()))?;
    let mut normalized = Cursor::new(Vec::new());
    image
        .write_to(&mut normalized, ImageFormat::Png)
        .map_err(|error| AppError::Image(error.to_string()))?;
    let normalized = normalized.into_inner();
    decode_rgba_png(&normalized)?;
    let store = ProjectStore::discover(&request.start)?;
    let selection = store
        .select_imported_reference(&request.asset, &normalized)
        .map_err(AppError::from)?;
    if let Some(path) = project_relative_path(&store, &source_file)
        && store
            .project_images()?
            .iter()
            .any(|image| image.path == path)
    {
        store.ignore_project_image(&path)?;
    }
    Ok(selection)
}

fn project_relative_path(store: &ProjectStore, file: &std::path::Path) -> Option<String> {
    let root = fs::canonicalize(store.root()).ok()?;
    let file = fs::canonicalize(file).ok()?;
    let relative = file.strip_prefix(root).ok()?;
    Some(
        relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

/// Replaces an existing asset source and reconverts it with its current style.
///
/// # Errors
///
/// Returns an error when the asset has no established conversion style, or when
/// source import, conversion, validation, or storage fails.
pub fn update_asset_source(
    request: UpdateAssetSource,
) -> Result<UpdateAssetSourceResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let asset = store.asset(&request.asset)?;
    let style = asset.style.ok_or_else(|| {
        AppError::UnsupportedConversion(
            "update-source requires an existing converted sprite; import the reference and run convert-selected first"
                .to_owned(),
        )
    })?;
    let reference = import_reference(ImportReference {
        start: request.start.clone(),
        asset: request.asset.clone(),
        file: request.file,
    })?;
    let revision = convert_selected_reference(ConvertSelectedReference {
        start: request.start,
        asset: request.asset,
        color_count: Some(style.color_count),
        palette_overrides: Vec::new(),
        settings: Some(style.settings),
        auto_background: true,
        actor: request.actor,
    })?;
    Ok(UpdateAssetSourceResult {
        reference,
        revision,
    })
}
