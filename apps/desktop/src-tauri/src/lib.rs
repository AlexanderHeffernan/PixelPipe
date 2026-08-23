use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use pixelpipe_app::{
    AgentRuntime, AgentTaskEvent, AgentTaskEventKind, AgentTaskRequest, BrowseAgentRuns,
    BrowseProject, CompareRevisions, ConvertSelectedReference, InitializeAsset, InspectRevision,
    LoadAgentCandidate, PatchRevisionDocument, ProjectBrowser, RecordReview, RemapRevisionDocument,
    ReviewRecord, RevisionComparisonMetadata, RevisionResult, RevisionViewMetadata,
    SelectAgentCandidate, StoreProjectPalette, StoreProjectRecipe, UpdateAssetBrief,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Default)]
struct AgentTasks {
    cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

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

#[derive(Debug, Serialize)]
struct AgentCandidateResponse {
    png_base64: String,
}

type CommandResult<T> = Result<T, String>;

#[tauri::command]
fn browse_project(request: BrowseProject) -> CommandResult<ProjectBrowser> {
    let BrowseProject { start } = request;
    pixelpipe_app::browse_project(&BrowseProject { start }).map_err(|error| command_error(&error))
}

#[tauri::command]
fn initialize_asset(request: InitializeAsset) -> CommandResult<pixelpipe_app::AssetManifest> {
    pixelpipe_app::initialize_asset(request).map_err(|error| command_error(&error))
}

#[tauri::command]
fn update_asset_brief(request: UpdateAssetBrief) -> CommandResult<pixelpipe_app::AssetManifest> {
    pixelpipe_app::update_asset_brief(request).map_err(|error| command_error(&error))
}

#[tauri::command]
fn convert_selected_reference(request: ConvertSelectedReference) -> CommandResult<RevisionResult> {
    pixelpipe_app::convert_selected_reference(request).map_err(|error| command_error(&error))
}

#[tauri::command]
fn store_project_palette(request: StoreProjectPalette) -> CommandResult<pixelpipe_app::Palette> {
    pixelpipe_app::store_project_palette(request).map_err(|error| command_error(&error))
}

#[tauri::command]
fn store_project_recipe(
    request: StoreProjectRecipe,
) -> CommandResult<pixelpipe_app::ConversionRecipeDocument> {
    pixelpipe_app::store_project_recipe(request).map_err(|error| command_error(&error))
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

#[tauri::command]
fn browse_agent_runs(
    request: BrowseAgentRuns,
) -> CommandResult<Vec<pixelpipe_app::AgentRunRecord>> {
    pixelpipe_app::browse_agent_runs(request).map_err(|error| command_error(&error))
}

#[tauri::command]
fn load_agent_candidate(request: LoadAgentCandidate) -> CommandResult<AgentCandidateResponse> {
    let bytes =
        pixelpipe_app::load_agent_candidate(request).map_err(|error| command_error(&error))?;
    Ok(AgentCandidateResponse {
        png_base64: STANDARD.encode(bytes),
    })
}

#[tauri::command]
fn select_agent_candidate(
    request: SelectAgentCandidate,
) -> CommandResult<pixelpipe_app::ReferenceSelection> {
    pixelpipe_app::select_agent_candidate(request).map_err(|error| command_error(&error))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri state extractors are passed by value.
fn start_agent_task(
    app: AppHandle,
    tasks: State<'_, AgentTasks>,
    request: AgentTaskRequest,
) -> CommandResult<String> {
    let runtime = AgentRuntime::user_local().map_err(|error| command_error(&error))?;
    let task = AgentRuntime::allocate_task_id().map_err(|error| command_error(&error))?;
    let cancel = Arc::new(AtomicBool::new(false));
    let task_registry = Arc::clone(&tasks.cancellations);
    task_registry
        .lock()
        .map_err(|_| "agent task registry is unavailable".to_owned())?
        .insert(task.clone(), Arc::clone(&cancel));
    let cancellations = Arc::clone(&task_registry);
    let spawned_task = task.clone();
    std::thread::spawn(move || {
        let event_app = app.clone();
        let sink = Arc::new(move |event: AgentTaskEvent| {
            let _ = event_app.emit("pixelpipe://agent-task", event);
        });
        if let Err(error) =
            runtime.run_with_task(request, spawned_task.clone(), cancel.as_ref(), sink)
        {
            let _ = app.emit(
                "pixelpipe://agent-task",
                AgentTaskEvent {
                    schema: "pixelpipe.agent-task-event/v1".to_owned(),
                    task: spawned_task.clone(),
                    sequence: 1,
                    event: AgentTaskEventKind::Failed {
                        run: None,
                        error: command_error(&error),
                    },
                },
            );
        }
        if let Ok(mut tasks) = cancellations.lock() {
            tasks.remove(&spawned_task);
        }
    });
    Ok(task)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri state extractors are passed by value.
fn cancel_agent_task(tasks: State<'_, AgentTasks>, task: String) -> CommandResult<()> {
    let task_registry = Arc::clone(&tasks.cancellations);
    let tasks = task_registry
        .lock()
        .map_err(|_| "agent task registry is unavailable".to_owned())?;
    let cancel = tasks
        .get(&task)
        .ok_or_else(|| format!("agent task '{task}' is not running"))?;
    cancel.store(true, Ordering::Relaxed);
    drop(task);
    Ok(())
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
        .manage(AgentTasks::default())
        .invoke_handler(tauri::generate_handler![
            browse_project,
            initialize_asset,
            update_asset_brief,
            convert_selected_reference,
            store_project_palette,
            store_project_recipe,
            load_revision,
            compare_revisions,
            record_review,
            patch_revision,
            remap_revision,
            browse_agent_runs,
            load_agent_candidate,
            select_agent_candidate,
            start_agent_task,
            cancel_agent_task
        ])
        .run(tauri::generate_context!())
        .expect("PixelPipe desktop runtime failed");
}
