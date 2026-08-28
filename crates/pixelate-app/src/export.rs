use std::{io::Write, path::PathBuf};

use atomicwrites::{AllowOverwrite, AtomicFile};
use image::{ExtendedColorType, codecs::webp::WebPEncoder};
use pixelate_core::stable_json;
use pixelate_project::ProjectStore;
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

#[derive(Debug, Deserialize)]
pub struct ExportAssetFile {
    pub start: PathBuf,
    pub asset: String,
    pub destination: PathBuf,
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportFileResult {
    pub asset: String,
    pub revision: String,
    pub file: PathBuf,
    pub format: String,
    pub width: u32,
    pub height: u32,
}

/// Exports the verified head raster and canonical indexed data to an existing folder.
///
/// # Errors
///
/// Returns an error when the asset has no head or verification/atomic output fails.
pub fn export_asset(request: ExportAsset) -> Result<ExportResult, AppError> {
    if !request.destination.is_dir() {
        return Err(AppError::InvalidExportDestination(
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
    refresh_link_hash(&store, &request.asset, &png)?;
    Ok(ExportResult {
        asset: request.asset,
        revision,
        png,
        metadata,
    })
}

/// Exports the verified head image to an explicit native-resolution file.
///
/// # Errors
///
/// Returns an error for a missing head, unsupported extension, existing file,
/// image encoding failure, or atomic output failure.
pub fn export_asset_file(request: ExportAssetFile) -> Result<ExportFileResult, AppError> {
    let parent = request.destination.parent().ok_or_else(|| {
        AppError::UnsupportedExportFormat("destination must include a file name".to_owned())
    })?;
    if !parent.is_dir() {
        return Err(AppError::UnsupportedExportFormat(
            "export folder does not exist".to_owned(),
        ));
    }
    if request.destination.exists() && !request.overwrite {
        return Err(AppError::ExportExists(request.destination));
    }
    let store = ProjectStore::discover(&request.start)?;
    let revision = store
        .asset(&request.asset)?
        .head
        .ok_or_else(|| AppError::NoHead(request.asset.clone()))?;
    let snapshot = store.revision(&request.asset, &revision)?;
    let extension = request
        .destination
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let (bytes, format) = match extension.as_str() {
        "png" => (snapshot.native_png, "png"),
        "webp" => {
            let image = image::load_from_memory(&snapshot.native_png)
                .map_err(|error| AppError::Image(error.to_string()))?
                .into_rgba8();
            let mut bytes = Vec::new();
            WebPEncoder::new_lossless(&mut bytes)
                .encode(
                    image.as_raw(),
                    image.width(),
                    image.height(),
                    ExtendedColorType::Rgba8,
                )
                .map_err(|error| AppError::Image(error.to_string()))?;
            (bytes, "webp")
        }
        _ => {
            return Err(AppError::UnsupportedExportFormat(
                "choose a PNG or WebP file".to_owned(),
            ));
        }
    };
    atomic_write(&request.destination, &bytes)?;
    refresh_link_hash(&store, &request.asset, &request.destination)?;
    Ok(ExportFileResult {
        asset: request.asset,
        revision,
        file: request.destination,
        format: format.to_owned(),
        width: snapshot.raster.width,
        height: snapshot.raster.height,
    })
}

fn refresh_link_hash(
    store: &ProjectStore,
    asset: &str,
    destination: &PathBuf,
) -> Result<(), AppError> {
    let manifest = store.asset(asset)?;
    let Some(path) = manifest.project_path else {
        return Ok(());
    };
    let linked = store.root().join(&path);
    let linked = std::fs::canonicalize(&linked).map_err(|source| AppError::Read {
        path: linked,
        source,
    })?;
    let destination = std::fs::canonicalize(destination).map_err(|source| AppError::Read {
        path: destination.clone(),
        source,
    })?;
    if linked == destination {
        store.link_asset_project_path(asset, &path)?;
    }
    Ok(())
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
