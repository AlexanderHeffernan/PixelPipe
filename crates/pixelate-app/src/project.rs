use std::path::PathBuf;

use pixelate_project::{AssetManifest, ProjectStore};
use serde::Deserialize;

use crate::AppError;

#[derive(Debug, Deserialize)]
pub struct InitializeAsset {
    pub start: PathBuf,
    pub asset: String,
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

/// Creates a stable project asset before any revision exists.
///
/// # Errors
///
/// Returns an [`AppError`] when discovery, validation, or atomic creation fails.
pub fn initialize_asset(request: InitializeAsset) -> Result<AssetManifest, AppError> {
    let InitializeAsset {
        start,
        asset,
        brief,
    } = request;
    let store = ProjectStore::discover(&start)?;
    Ok(store.create_asset(&asset, &brief)?)
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
