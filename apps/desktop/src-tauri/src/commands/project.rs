use base64::{Engine as _, engine::general_purpose::STANDARD};
use pixelpipe_app::{
    BrowseProject, ConvertSelectedReference, ExportAsset, ImportReference, InitializeAsset,
    OpenProject, PreviewSelectedReference, ProjectBrowser, RasterInspection, RevisionResult,
    StoreProjectPalette, StoreProjectRecipe, UpdateAssetBrief,
};
use serde::Serialize;

use super::{CommandResult, command_error};

#[tauri::command]
pub(crate) fn browse_project(request: BrowseProject) -> CommandResult<ProjectBrowser> {
    let BrowseProject { start } = request;
    pixelpipe_app::browse_project(&BrowseProject { start }).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn open_project(request: OpenProject) -> CommandResult<ProjectBrowser> {
    pixelpipe_app::open_project(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn initialize_asset(
    request: InitializeAsset,
) -> CommandResult<pixelpipe_app::AssetManifest> {
    pixelpipe_app::initialize_asset(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn update_asset_brief(
    request: UpdateAssetBrief,
) -> CommandResult<pixelpipe_app::AssetManifest> {
    pixelpipe_app::update_asset_brief(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn import_reference(
    request: ImportReference,
) -> CommandResult<pixelpipe_app::ReferenceSelection> {
    pixelpipe_app::import_reference(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn export_asset(request: ExportAsset) -> CommandResult<pixelpipe_app::ExportResult> {
    pixelpipe_app::export_asset(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn convert_selected_reference(
    request: ConvertSelectedReference,
) -> CommandResult<RevisionResult> {
    pixelpipe_app::convert_selected_reference(request).map_err(|error| command_error(&error))
}

#[derive(Debug, Serialize)]
pub(crate) struct ConversionPreviewResponse {
    inspection: RasterInspection,
    palette_name: String,
    native_png_base64: String,
}

#[tauri::command]
pub(crate) fn preview_selected_reference(
    request: PreviewSelectedReference,
) -> CommandResult<ConversionPreviewResponse> {
    let preview = pixelpipe_app::preview_selected_reference(request)
        .map_err(|error| command_error(&error))?;
    Ok(ConversionPreviewResponse {
        inspection: preview.inspection,
        palette_name: preview.palette_name,
        native_png_base64: STANDARD.encode(preview.native_png),
    })
}

#[tauri::command]
pub(crate) fn store_project_palette(
    request: StoreProjectPalette,
) -> CommandResult<pixelpipe_app::Palette> {
    pixelpipe_app::store_project_palette(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn store_project_recipe(
    request: StoreProjectRecipe,
) -> CommandResult<pixelpipe_app::ConversionRecipeDocument> {
    pixelpipe_app::store_project_recipe(request).map_err(|error| command_error(&error))
}
