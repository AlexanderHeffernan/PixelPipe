use std::{fs, path::PathBuf};

use pixelpipe_core::decode_rgba_png;
use pixelpipe_project::{ProjectStore, ReferenceSelection};
use serde::Deserialize;

use crate::AppError;

#[derive(Debug, Deserialize)]
pub struct ImportReference {
    pub start: PathBuf,
    pub asset: String,
    pub file: PathBuf,
}

/// Validates and content-addresses a user-selected PNG as the asset reference.
///
/// # Errors
///
/// Returns an error when the file, PNG, project, or asset transition is invalid.
pub fn import_reference(request: ImportReference) -> Result<ReferenceSelection, AppError> {
    let bytes = fs::read(&request.file).map_err(|source| AppError::Read {
        path: request.file,
        source,
    })?;
    decode_rgba_png(&bytes)?;
    let store = ProjectStore::discover(&request.start)?;
    store
        .select_imported_reference(&request.asset, &bytes)
        .map_err(AppError::from)
}
