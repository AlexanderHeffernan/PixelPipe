use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use pixelpipe_app::{
    AgentRuntime, AgentTaskEvent, AgentTaskEventKind, AgentTaskRequest, ApproveAgentConnector,
    BrowseAgentRuns, LoadAgentCandidate, SelectAgentCandidate,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use super::{CommandResult, command_error};

#[derive(Default)]
pub(crate) struct AgentTasks {
    cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AgentCandidateResponse {
    png_base64: String,
}

#[tauri::command]
pub(crate) fn detect_agent_connectors() -> CommandResult<Vec<pixelpipe_app::AgentConnector>> {
    pixelpipe_app::detect_agent_connectors().map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn approve_agent_connector(
    request: ApproveAgentConnector,
) -> CommandResult<pixelpipe_app::AgentConnector> {
    pixelpipe_app::approve_agent_connector(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn browse_agent_runs(
    request: BrowseAgentRuns,
) -> CommandResult<Vec<pixelpipe_app::AgentRunRecord>> {
    pixelpipe_app::browse_agent_runs(request).map_err(|error| command_error(&error))
}

#[tauri::command]
pub(crate) fn load_agent_candidate(
    request: LoadAgentCandidate,
) -> CommandResult<AgentCandidateResponse> {
    let bytes =
        pixelpipe_app::load_agent_candidate(request).map_err(|error| command_error(&error))?;
    Ok(AgentCandidateResponse {
        png_base64: STANDARD.encode(bytes),
    })
}

#[tauri::command]
pub(crate) fn select_agent_candidate(
    request: SelectAgentCandidate,
) -> CommandResult<pixelpipe_app::ReferenceSelection> {
    pixelpipe_app::select_agent_candidate(request).map_err(|error| command_error(&error))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn start_agent_task(
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
    spawn_agent(app, runtime, request, task.clone(), cancel, task_registry);
    Ok(task)
}

fn spawn_agent(
    app: AppHandle,
    runtime: AgentRuntime,
    request: AgentTaskRequest,
    task: String,
    cancel: Arc<AtomicBool>,
    cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
) {
    std::thread::spawn(move || {
        let event_app = app.clone();
        let sink = Arc::new(move |event: AgentTaskEvent| {
            let _ = event_app.emit("pixelpipe://agent-task", event);
        });
        if let Err(error) = runtime.run_with_task(request, task.clone(), cancel.as_ref(), sink) {
            let _ = app.emit(
                "pixelpipe://agent-task",
                AgentTaskEvent {
                    schema: "pixelpipe.agent-task-event/v1".to_owned(),
                    task: task.clone(),
                    sequence: 1,
                    event: AgentTaskEventKind::Failed {
                        run: None,
                        error: command_error(&error),
                    },
                },
            );
        }
        if let Ok(mut tasks) = cancellations.lock() {
            tasks.remove(&task);
        }
    });
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn cancel_agent_task(tasks: State<'_, AgentTasks>, task: String) -> CommandResult<()> {
    let tasks = tasks
        .cancellations
        .lock()
        .map_err(|_| "agent task registry is unavailable".to_owned())?;
    let cancel = tasks
        .get(&task)
        .ok_or_else(|| format!("agent task '{task}' is not running"))?;
    cancel.store(true, Ordering::Relaxed);
    Ok(())
}
