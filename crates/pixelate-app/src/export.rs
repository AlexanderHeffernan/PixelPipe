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

const SPRITESHEET_SCHEMA: &str = "pixelate.spritesheet/v1";

#[derive(Debug, Serialize)]
struct SpritesheetMetadata<'a> {
    schema: &'static str,
    asset: &'a str,
    revision: &'a str,
    sheet: SheetDimensions,
    canvas: CanvasDimensions,
    pivot: Option<[i32; 2]>,
    frames: Vec<ExportFrame<'a>>,
}

#[derive(Debug, Serialize)]
struct SheetDimensions {
    width: u32,
    height: u32,
}
#[derive(Debug, Serialize)]
struct CanvasDimensions {
    width: u32,
    height: u32,
}
#[derive(Debug, Serialize)]
struct ExportFrame<'a> {
    id: &'a str,
    order: usize,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    duration_ms: u32,
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
    let metadata_bytes = if snapshot.sequence.frames.len() == 1 {
        stable_json(&snapshot.raster)?
    } else {
        stable_json(&spritesheet_metadata(
            &request.asset,
            &revision,
            &snapshot.sequence,
        )?)?
    };
    atomic_write(&metadata, &metadata_bytes)?;
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
    if snapshot.sequence.frames.len() > 1 {
        return Err(AppError::UnsupportedExportFormat(
            "multi-frame assets export as a PNG spritesheet plus JSON; choose asset export"
                .to_owned(),
        ));
    }
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
    Ok(ExportFileResult {
        asset: request.asset,
        revision,
        file: request.destination,
        format: format.to_owned(),
        width: snapshot.raster.width,
        height: snapshot.raster.height,
    })
}

fn spritesheet_metadata<'a>(
    asset: &'a str,
    revision: &'a str,
    sequence: &'a pixelate_core::IndexedSequence,
) -> Result<SpritesheetMetadata<'a>, AppError> {
    let frame_count = u32::try_from(sequence.frames.len())
        .map_err(|_| pixelate_core::CoreError::DimensionOverflow)?;
    let width = sequence
        .width
        .checked_mul(frame_count)
        .ok_or(pixelate_core::CoreError::DimensionOverflow)?;
    Ok(SpritesheetMetadata {
        schema: SPRITESHEET_SCHEMA,
        asset,
        revision,
        sheet: SheetDimensions {
            width,
            height: sequence.height,
        },
        canvas: CanvasDimensions {
            width: sequence.width,
            height: sequence.height,
        },
        pivot: sequence.pivot,
        frames: sequence
            .frames
            .iter()
            .enumerate()
            .map(|(order, frame)| ExportFrame {
                id: &frame.id,
                order,
                x: sequence.width * u32::try_from(order).expect("validated frame count fits u32"),
                y: 0,
                width: sequence.width,
                height: sequence.height,
                duration_ms: frame.duration_ms,
            })
            .collect(),
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
