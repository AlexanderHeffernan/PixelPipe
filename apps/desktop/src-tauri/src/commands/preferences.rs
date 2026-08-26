use std::fs;

use tauri::{AppHandle, Manager};

use super::CommandResult;

const RECENT_PROJECT_FILE: &str = "recent-project.txt";

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn recent_project(app: AppHandle) -> CommandResult<Option<String>> {
    let path = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join(RECENT_PROJECT_FILE);
    let Ok(value) = fs::read_to_string(path) else {
        return Ok(None);
    };
    let project = value.trim();
    Ok((!project.is_empty() && std::path::Path::new(project).is_dir()).then(|| project.to_owned()))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn remember_project(app: AppHandle, path: String) -> CommandResult<()> {
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let destination = directory.join(RECENT_PROJECT_FILE);
    let temporary = directory.join(format!("{RECENT_PROJECT_FILE}.tmp"));
    fs::write(&temporary, path).map_err(|error| error.to_string())?;
    fs::rename(temporary, destination).map_err(|error| error.to_string())
}
