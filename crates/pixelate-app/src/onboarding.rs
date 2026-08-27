use std::path::PathBuf;

use pixelate_project::{ProjectError, ProjectStore};
use serde::Deserialize;

use crate::{AppError, BrowseProject, ProjectBrowser, browse_project};

#[derive(Debug, Deserialize)]
pub struct OpenProject {
    pub start: PathBuf,
}

/// Opens a `Pixelate` project or initializes the selected folder.
///
/// # Errors
///
/// Returns an error when project discovery or initialization fails.
pub fn open_project(request: OpenProject) -> Result<ProjectBrowser, AppError> {
    let OpenProject { start } = request;
    let store = match ProjectStore::discover(&start) {
        Ok(store) => store,
        Err(ProjectError::NotFound(_)) => {
            let name = start
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.trim().is_empty())
                .unwrap_or("Pixelate Project");
            ProjectStore::init(&start, name)?
        }
        Err(error) => return Err(error.into()),
    };
    browse_project(&BrowseProject {
        start: store.root().to_path_buf(),
    })
}
