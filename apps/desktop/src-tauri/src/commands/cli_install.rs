use std::{env, path::PathBuf};

use serde::Serialize;

use super::CommandResult;

#[cfg(target_os = "linux")]
pub(super) mod linux;
#[cfg(any(target_os = "macos", all(test, unix)))]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(super) mod macos;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CliInstallState {
    Installed,
    NotInstalled,
    NeedsRepair,
    Conflict,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct CliInstallStatus {
    pub(super) state: CliInstallState,
    command: PathBuf,
    pub(in crate::commands) managed: bool,
}

#[tauri::command]
pub(crate) fn cli_installation_status() -> CliInstallStatus {
    current_status()
}

#[tauri::command]
pub(crate) async fn install_cli() -> CommandResult<CliInstallStatus> {
    tauri::async_runtime::spawn_blocking(install)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn uninstall_cli() -> CommandResult<CliInstallStatus> {
    tauri::async_runtime::spawn_blocking(uninstall)
        .await
        .map_err(|error| error.to_string())?
}

pub(super) fn bundled_cli_path() -> Option<PathBuf> {
    env::current_exe()
        .ok()
        .and_then(|executable| {
            executable
                .parent()
                .map(|directory| directory.join("pixelate"))
        })
        .filter(|path| path.is_file())
}

pub(super) fn status(state: CliInstallState, command: PathBuf, managed: bool) -> CliInstallStatus {
    CliInstallStatus {
        state,
        command,
        managed,
    }
}

fn current_status() -> CliInstallStatus {
    let source = bundled_cli_path();
    #[cfg(target_os = "macos")]
    return macos::current_status(source.as_deref());
    #[cfg(target_os = "linux")]
    return linux::current_status(source.as_deref());
    #[allow(unreachable_code)]
    status(
        CliInstallState::Unavailable,
        PathBuf::from("pixelate"),
        false,
    )
}

fn install() -> CommandResult<CliInstallStatus> {
    #[cfg(target_os = "macos")]
    return macos::install(bundled_cli_path());
    #[cfg(target_os = "linux")]
    return linux::install(bundled_cli_path());
    #[allow(unreachable_code)]
    Err("CLI installation is unavailable on this platform".to_owned())
}

fn uninstall() -> CommandResult<CliInstallStatus> {
    #[cfg(target_os = "macos")]
    return macos::uninstall(bundled_cli_path());
    #[cfg(target_os = "linux")]
    return linux::uninstall(bundled_cli_path());
    #[allow(unreachable_code)]
    Err("CLI installation is unavailable on this platform".to_owned())
}
