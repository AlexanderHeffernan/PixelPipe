use std::path::PathBuf;

use pixelpipe_core::Palette;
use pixelpipe_project::{AssetKind, AssetManifest, ConversionRecipeDocument, ProjectStore};
use serde::Deserialize;

use crate::{AppError, revision_commit::read};

#[derive(Debug, Deserialize)]
pub struct InitializeAsset {
    pub start: PathBuf,
    pub asset: String,
    pub kind: AssetKind,
    #[serde(default)]
    pub brief: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteAsset {
    pub start: PathBuf,
    pub asset: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAssetBrief {
    pub start: PathBuf,
    pub asset: String,
    pub brief: String,
}

#[derive(Debug, Deserialize)]
pub struct RenameAsset {
    pub start: PathBuf,
    pub asset: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct StoreProjectPalette {
    pub start: PathBuf,
    pub id: String,
    pub file: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct StoreProjectRecipe {
    pub start: PathBuf,
    pub file: PathBuf,
}

/// Creates a stable project asset before any revision exists.
///
/// # Errors
///
/// Returns an [`AppError`] when discovery, validation, or atomic creation fails.
pub fn initialize_asset(request: InitializeAsset) -> Result<AssetManifest, AppError> {
    let InitializeAsset {
        start,
        asset,
        kind,
        brief,
    } = request;
    let store = ProjectStore::discover(&start)?;
    Ok(store.create_asset(&asset, kind, &brief)?)
}

/// Permanently removes one project asset through the project store.
///
/// # Errors
///
/// Returns an [`AppError`] when discovery, validation, or deletion fails.
pub fn delete_asset(request: DeleteAsset) -> Result<(), AppError> {
    let DeleteAsset { start, asset } = request;
    let store = ProjectStore::discover(&start)?;
    Ok(store.delete_asset(&asset)?)
}

/// Updates the project-owned brief without changing revision history.
///
/// # Errors
///
/// Returns an [`AppError`] when discovery, asset validation, or storage fails.
pub fn update_asset_brief(request: UpdateAssetBrief) -> Result<AssetManifest, AppError> {
    let UpdateAssetBrief {
        start,
        asset,
        brief,
    } = request;
    let store = ProjectStore::discover(&start)?;
    Ok(store.set_asset_brief(&asset, &brief)?)
}

/// Updates an asset's user-facing name while preserving its stable ID and revisions.
///
/// # Errors
///
/// Returns an [`AppError`] when discovery, validation, or storage fails.
pub fn rename_asset(request: RenameAsset) -> Result<AssetManifest, AppError> {
    let RenameAsset {
        start,
        asset,
        display_name,
    } = request;
    let store = ProjectStore::discover(&start)?;
    Ok(store.set_asset_display_name(&asset, &display_name)?)
}

/// Imports a validated palette into project-owned resources.
///
/// # Errors
///
/// Returns an [`AppError`] when the file, JSON, palette, or storage is invalid.
pub fn store_project_palette(request: StoreProjectPalette) -> Result<Palette, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let palette: Palette = serde_json::from_slice(&read(&request.file)?).map_err(|source| {
        AppError::ProjectResourceJson {
            path: request.file,
            source,
        }
    })?;
    store.store_palette(&request.id, &palette)?;
    Ok(palette)
}

/// Imports a complete validated conversion recipe into project-owned resources.
///
/// # Errors
///
/// Returns an [`AppError`] when the file, JSON, recipe, or storage is invalid.
pub fn store_project_recipe(
    request: StoreProjectRecipe,
) -> Result<ConversionRecipeDocument, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let recipe: ConversionRecipeDocument =
        serde_json::from_slice(&read(&request.file)?).map_err(|source| {
            AppError::ProjectResourceJson {
                path: request.file,
                source,
            }
        })?;
    store.store_conversion_recipe(&recipe)?;
    Ok(recipe)
}
