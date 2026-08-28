pub(crate) mod cli_install;
pub(crate) mod preferences;
pub(crate) mod project;
pub(crate) mod revisions;
pub(crate) mod terminal;

#[cfg(all(test, unix))]
mod cli_install_tests;
#[cfg(test)]
mod terminal_tests;

pub(super) type CommandResult<T> = Result<T, String>;

pub(super) fn command_error(error: &pixelate_app::AppError) -> String {
    error.to_string()
}

pub(super) async fn blocking<T, F>(action: F) -> CommandResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, pixelate_app::AppError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(action)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| command_error(&error))
}
