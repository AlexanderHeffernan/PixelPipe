use std::{io::Write, path::PathBuf};

use atomicwrites::{AllowOverwrite, AtomicFile};
use pixelpipe_core::stable_json;
use pixelpipe_project::ProjectStore;
use serde::{Deserialize, Serialize};

use crate::AppError;

#[derive(Debug, Deserialize)]
pub struct ExportAsset {
    pub start: PathBuf,
    pub asset: String,
    pub destination: PathBuf,
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportResult {
    pub asset: String,
    pub revision: String,
    pub png: PathBuf,
    pub metadata: PathBuf,
}

/// Exports the verified head raster and canonical indexed data to an existing folder.
///
/// # Errors
///
/// Returns an error when the asset has no head or verification/atomic output fails.
pub fn export_asset(request: ExportAsset) -> Result<ExportResult, AppError> {
    if !request.destination.is_dir() {
        return Err(AppError::AgentCandidatePath(
            "export destination must be an existing folder".to_owned(),
        ));
    }
    let store = ProjectStore::discover(&request.start)?;
    let manifest = store.asset(&request.asset)?;
    let revision = manifest
        .head
        .ok_or_else(|| AppError::NoHead(request.asset.clone()))?;
    let snapshot = store.revision(&request.asset, &revision)?;
    let png = request.destination.join(format!("{}.png", request.asset));
    let metadata = request.destination.join(format!("{}.json", request.asset));
    if !request.overwrite {
        if png.exists() {
            return Err(AppError::ExportExists(png));
        }
        if metadata.exists() {
            return Err(AppError::ExportExists(metadata));
        }
    }
    atomic_write(&png, &snapshot.native_png)?;
    atomic_write(&metadata, &stable_json(&snapshot.raster)?)?;
    Ok(ExportResult {
        asset: request.asset,
        revision,
        png,
        metadata,
    })
}

fn atomic_write(path: &PathBuf, bytes: &[u8]) -> Result<(), AppError> {
    AtomicFile::new(path, AllowOverwrite)
        .write(|file| file.write_all(bytes))
        .map_err(|error| AppError::Read {
            path: path.clone(),
            source: match error {
                atomicwrites::Error::Internal(source) | atomicwrites::Error::User(source) => source,
            },
        })
}
