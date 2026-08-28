use base64::{Engine as _, engine::general_purpose::STANDARD};
use pixelate_app::{
    AdoptProjectImage, BrowseProject, CommitComposition, ConvertSelectedReference, CreateFolder,
    DeleteAsset, DeleteFolder, ExportAsset, ExportAssetFile, ImportReference, InitializeAsset,
    LoadProjectImage, MoveAsset, MoveFolder, OpenProject, PreviewComposition,
    PreviewSelectedReference, ProjectBrowser, RasterInspection, RelinkAsset, RenameAsset,
    RevisionResult, UpdateAssetBrief, UpdateLinkedSource,
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
pub(crate) async fn adopt_project_image(
    request: AdoptProjectImage,
) -> CommandResult<pixelate_app::AssetManifest> {
    blocking(move || pixelate_app::adopt_project_image(request)).await
}

#[tauri::command]
pub(crate) fn relink_asset(request: RelinkAsset) -> CommandResult<pixelate_app::AssetManifest> {
    pixelate_app::relink_asset(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) async fn update_linked_source(
    request: UpdateLinkedSource,
) -> CommandResult<pixelate_app::AssetManifest> {
    blocking(move || pixelate_app::update_linked_source(request)).await
}

#[tauri::command]
pub(crate) fn create_folder(request: CreateFolder) -> CommandResult<()> {
    pixelate_app::create_folder(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn move_folder(request: MoveFolder) -> CommandResult<Vec<pixelate_app::AssetManifest>> {
    pixelate_app::move_folder(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn delete_folder(request: DeleteFolder) -> CommandResult<()> {
    pixelate_app::delete_folder(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn move_asset(request: MoveAsset) -> CommandResult<pixelate_app::AssetManifest> {
    pixelate_app::move_asset(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) async fn load_project_image(request: LoadProjectImage) -> CommandResult<String> {
    blocking(move || {
        let extension = request
            .path
            .rsplit('.')
            .next()
            .unwrap_or("png")
            .to_ascii_lowercase();
        let mime = match extension.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            _ => "image/png",
        };
        let bytes = pixelate_app::load_project_image(request)?;
        Ok(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
    })
    .await
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
