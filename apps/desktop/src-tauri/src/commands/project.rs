use base64::{Engine as _, engine::general_purpose::STANDARD};
use pixelate_app::{
    BrowseProject, CommitComposition, ConvertSelectedReference, DeleteAsset, ExportAsset,
    ExportAssetFile, ImportReference, InitializeAsset, OpenProject, PreviewComposition,
    PreviewSelectedReference, ProjectBrowser, RasterInspection, RenameAsset, RevisionResult,
    StoreProjectPalette, StoreProjectRecipe, UpdateAssetBrief,
};
use serde::Serialize;

use super::{CommandResult, blocking, command_error};

#[tauri::command]
pub(crate) async fn browse_project(request: BrowseProject) -> CommandResult<ProjectBrowser> {
    blocking(move || pixelate_app::browse_project(&request)).await
}

#[tauri::command]
pub(crate) async fn open_project(request: OpenProject) -> CommandResult<ProjectBrowser> {
    blocking(move || pixelate_app::open_project(request)).await
}

#[tauri::command]
pub(crate) fn initialize_asset(
    request: InitializeAsset,
) -> CommandResult<pixelate_app::AssetManifest> {
    pixelate_app::initialize_asset(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn delete_asset(request: DeleteAsset) -> CommandResult<()> {
    pixelate_app::delete_asset(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn update_asset_brief(
    request: UpdateAssetBrief,
) -> CommandResult<pixelate_app::AssetManifest> {
    pixelate_app::update_asset_brief(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn rename_asset(request: RenameAsset) -> CommandResult<pixelate_app::AssetManifest> {
    pixelate_app::rename_asset(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) async fn import_reference(
    request: ImportReference,
) -> CommandResult<pixelate_app::ReferenceSelection> {
    blocking(move || pixelate_app::import_reference(request)).await
}

#[tauri::command]
pub(crate) async fn export_asset(
    request: ExportAsset,
) -> CommandResult<pixelate_app::ExportResult> {
    blocking(move || pixelate_app::export_asset(request)).await
}

#[tauri::command]
pub(crate) async fn export_asset_file(
    request: ExportAssetFile,
) -> CommandResult<pixelate_app::ExportFileResult> {
    blocking(move || pixelate_app::export_asset_file(request)).await
}

#[tauri::command]
pub(crate) async fn convert_selected_reference(
    request: ConvertSelectedReference,
) -> CommandResult<RevisionResult> {
    blocking(move || pixelate_app::convert_selected_reference(request)).await
}

#[derive(Debug, Serialize)]
pub(crate) struct ConversionPreviewResponse {
    inspection: RasterInspection,
    palette_name: String,
    native_png_base64: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    background_removed: Option<bool>,
}

#[tauri::command]
pub(crate) async fn preview_selected_reference(
    request: PreviewSelectedReference,
) -> CommandResult<ConversionPreviewResponse> {
    blocking(move || {
        let preview = pixelate_app::preview_selected_reference(request)?;
        Ok(ConversionPreviewResponse {
            inspection: preview.inspection,
            palette_name: preview.palette_name,
            native_png_base64: STANDARD.encode(preview.native_png),
            background_removed: Some(preview.background_removed),
        })
    })
    .await
}

#[tauri::command]
pub(crate) async fn preview_composition(
    request: PreviewComposition,
) -> CommandResult<ConversionPreviewResponse> {
    blocking(move || {
        let preview = pixelate_app::preview_composition(request)?;
        Ok(ConversionPreviewResponse {
            inspection: preview.inspection,
            palette_name: "Current sprite".to_owned(),
            native_png_base64: STANDARD.encode(preview.native_png),
            background_removed: None,
        })
    })
    .await
}

#[tauri::command]
pub(crate) async fn commit_composition(
    request: CommitComposition,
) -> CommandResult<RevisionResult> {
    blocking(move || pixelate_app::commit_composition(request)).await
}

#[tauri::command]
pub(crate) fn store_project_palette(
    request: StoreProjectPalette,
) -> CommandResult<pixelate_app::Palette> {
    pixelate_app::store_project_palette(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn store_project_recipe(
    request: StoreProjectRecipe,
) -> CommandResult<pixelate_app::ConversionRecipeDocument> {
    pixelate_app::store_project_recipe(request).map_err(|error| command_error(&error))
}
