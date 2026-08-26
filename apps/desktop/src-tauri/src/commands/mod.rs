pub(crate) mod preferences;
pub(crate) mod project;
pub(crate) mod revisions;
pub(crate) mod terminal;

pub(super) type CommandResult<T> = Result<T, String>;

pub(super) fn command_error(error: &pixelpipe_app::AppError) -> String {
    error.to_string()
}

pub(super) async fn blocking<T, F>(action: F) -> CommandResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, pixelpipe_app::AppError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(action)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| command_error(&error))
}
