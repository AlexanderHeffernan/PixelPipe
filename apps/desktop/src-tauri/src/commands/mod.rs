pub(crate) mod agents;
pub(crate) mod project;
pub(crate) mod revisions;

pub(super) type CommandResult<T> = Result<T, String>;

pub(super) fn command_error(error: &pixelpipe_app::AppError) -> String {
    error.to_string()
}
