use base64::{Engine as _, engine::general_purpose::STANDARD};
use pixelpipe_app::{
    CompareRevisions, FillRevisionDocument, InspectRevision, PatchRevisionDocument, RecordReview,
    RemapRevisionDocument, ReviewRecord, RevisionComparisonMetadata, RevisionResult,
    RevisionViewMetadata, SetAssetHead,
};
use serde::Serialize;

use super::{CommandResult, blocking, command_error};

#[derive(Debug, Serialize)]
pub(crate) struct RevisionViewResponse {
    metadata: RevisionViewMetadata,
    native_png_base64: String,
    preview_png_base64: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct RevisionComparisonResponse {
    metadata: RevisionComparisonMetadata,
    visual_native_png_base64: String,
    visual_preview_png_base64: String,
}

#[tauri::command]
pub(crate) async fn load_revision(request: InspectRevision) -> CommandResult<RevisionViewResponse> {
    blocking(move || {
        let view = pixelpipe_app::load_revision_view(request)?;
        Ok(RevisionViewResponse {
            metadata: view.metadata,
            native_png_base64: STANDARD.encode(view.native_png),
            preview_png_base64: STANDARD.encode(view.preview_png),
        })
    })
    .await
}

#[tauri::command]
pub(crate) fn compare_revisions(
    request: CompareRevisions,
) -> CommandResult<RevisionComparisonResponse> {
    let comparison =
        pixelpipe_app::compare_revisions(request).map_err(|error| command_error(&error))?;
    Ok(RevisionComparisonResponse {
        metadata: comparison.metadata(),
        visual_native_png_base64: STANDARD.encode(comparison.visual_native_png),
        visual_preview_png_base64: STANDARD.encode(comparison.visual_preview_png),
    })
}

#[tauri::command]
pub(crate) fn record_review(request: RecordReview) -> CommandResult<ReviewRecord> {
    pixelpipe_app::record_review(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) async fn patch_revision(
    request: PatchRevisionDocument,
) -> CommandResult<RevisionResult> {
    blocking(move || pixelpipe_app::patch_revision_document(request)).await
}

#[tauri::command]
pub(crate) async fn fill_revision(request: FillRevisionDocument) -> CommandResult<RevisionResult> {
    blocking(move || pixelpipe_app::fill_revision_document(request)).await
}

#[tauri::command]
pub(crate) fn set_asset_head(request: SetAssetHead) -> CommandResult<pixelpipe_app::AssetManifest> {
    pixelpipe_app::set_asset_head(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) async fn remap_revision(
    request: RemapRevisionDocument,
) -> CommandResult<RevisionResult> {
    blocking(move || pixelpipe_app::remap_revision_document(request)).await
}
