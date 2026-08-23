use base64::{Engine as _, engine::general_purpose::STANDARD};
use pixelpipe_app::{
    BrowseProject, CompareRevisions, InspectRevision, PatchRevisionDocument, ProjectBrowser,
    RecordReview, RemapRevisionDocument, ReviewRecord, RevisionComparisonMetadata, RevisionResult,
    RevisionViewMetadata,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct RevisionViewResponse {
    metadata: RevisionViewMetadata,
    native_png_base64: String,
    preview_png_base64: String,
}

#[derive(Debug, Serialize)]
struct RevisionComparisonResponse {
    metadata: RevisionComparisonMetadata,
    visual_native_png_base64: String,
    visual_preview_png_base64: String,
}

type CommandResult<T> = Result<T, String>;

#[tauri::command]
fn browse_project(request: BrowseProject) -> CommandResult<ProjectBrowser> {
    let BrowseProject { start } = request;
    pixelpipe_app::browse_project(&BrowseProject { start }).map_err(|error| command_error(&error))
}

#[tauri::command]
fn load_revision(request: InspectRevision) -> CommandResult<RevisionViewResponse> {
    let view = pixelpipe_app::load_revision_view(request).map_err(|error| command_error(&error))?;
    Ok(RevisionViewResponse {
        metadata: view.metadata,
        native_png_base64: STANDARD.encode(view.native_png),
        preview_png_base64: STANDARD.encode(view.preview_png),
    })
}

#[tauri::command]
fn compare_revisions(request: CompareRevisions) -> CommandResult<RevisionComparisonResponse> {
    let comparison =
        pixelpipe_app::compare_revisions(request).map_err(|error| command_error(&error))?;
    Ok(RevisionComparisonResponse {
        metadata: comparison.metadata(),
        visual_native_png_base64: STANDARD.encode(comparison.visual_native_png),
        visual_preview_png_base64: STANDARD.encode(comparison.visual_preview_png),
    })
}

#[tauri::command]
fn record_review(request: RecordReview) -> CommandResult<ReviewRecord> {
    pixelpipe_app::record_review(request).map_err(|error| command_error(&error))
}

#[tauri::command]
fn patch_revision(request: PatchRevisionDocument) -> CommandResult<RevisionResult> {
    pixelpipe_app::patch_revision_document(request).map_err(|error| command_error(&error))
}

#[tauri::command]
fn remap_revision(request: RemapRevisionDocument) -> CommandResult<RevisionResult> {
    pixelpipe_app::remap_revision_document(request).map_err(|error| command_error(&error))
}

fn command_error(error: &pixelpipe_app::AppError) -> String {
    error.to_string()
}

/// Starts the desktop runtime and typed command adapter.
///
/// # Panics
///
/// Panics when Tauri cannot initialize or run the application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            browse_project,
            load_revision,
            compare_revisions,
            record_review,
            patch_revision,
            remap_revision
        ])
        .run(tauri::generate_context!())
        .expect("PixelPipe desktop runtime failed");
}
