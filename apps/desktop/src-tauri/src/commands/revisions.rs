use base64::{Engine as _, engine::general_purpose::STANDARD};
use pixelate_app::{
    FillRevisionDocument, InspectRevision, PatchRevisionDocument, RemapRevisionDocument,
    RevisionResult, RevisionViewMetadata, SetAssetHead,
};
use serde::Serialize;

use super::{CommandResult, blocking, command_error};

#[derive(Debug, Serialize)]
pub(crate) struct RevisionViewResponse {
    metadata: RevisionViewMetadata,
    native_png_base64: String,
}

#[tauri::command]
pub(crate) async fn load_revision(request: InspectRevision) -> CommandResult<RevisionViewResponse> {
    blocking(move || {
        let view = pixelate_app::load_revision_view(request)?;
        Ok(RevisionViewResponse {
            metadata: view.metadata,
            native_png_base64: STANDARD.encode(view.native_png),
        })
    })
    .await
}

#[tauri::command]
pub(crate) async fn patch_revision(
    request: PatchRevisionDocument,
) -> CommandResult<RevisionResult> {
    blocking(move || pixelate_app::patch_revision_document(request)).await
}

#[tauri::command]
pub(crate) async fn fill_revision(request: FillRevisionDocument) -> CommandResult<RevisionResult> {
    blocking(move || pixelate_app::fill_revision_document(request)).await
}

#[tauri::command]
pub(crate) fn set_asset_head(request: SetAssetHead) -> CommandResult<pixelate_app::AssetManifest> {
    pixelate_app::set_asset_head(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) async fn remap_revision(
    request: RemapRevisionDocument,
) -> CommandResult<RevisionResult> {
    blocking(move || pixelate_app::remap_revision_document(request)).await
}
