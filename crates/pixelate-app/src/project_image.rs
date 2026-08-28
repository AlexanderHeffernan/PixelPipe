use std::{fs, path::PathBuf};

use pixelate_core::{RgbaImage, import_pixel_art};
use pixelate_project::ProjectStore;
use serde::Deserialize;

use crate::AppError;

#[derive(Debug, Deserialize)]
pub struct LoadProjectImage {
    pub start: PathBuf,
    pub path: String,
}

#[derive(Debug)]
pub struct ProjectImageView {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub pixel_art_importable: bool,
}

/// Lazily reads and inspects one discovered project image for preview.
///
/// Exact pixel-art eligibility uses the same deterministic importer as adoption,
/// with the desktop's intentionally small 256 × 256 editing boundary.
///
/// # Errors
///
/// Returns an error when the image is outside safe discovery or cannot be decoded.
pub fn load_project_image(request: LoadProjectImage) -> Result<ProjectImageView, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let relative = store
        .project_images()?
        .into_iter()
        .find(|image| image.path == request.path)
        .ok_or(pixelate_project::ProjectError::ProjectPathNotFound(
            request.path,
        ))?
        .path;
    let path = store.root().join(relative);
    let bytes = fs::read(&path).map_err(|source| AppError::Read {
        path: path.clone(),
        source,
    })?;
    let decoded = image::load_from_memory(&bytes)
        .map_err(|error| AppError::Image(error.to_string()))?
        .to_rgba8();
    let (width, height) = decoded.dimensions();
    let source = RgbaImage {
        width,
        height,
        pixels: decoded.pixels().map(|pixel| pixel.0).collect(),
    };
    let pixel_art_importable = width <= 256 && height <= 256 && import_pixel_art(&source).is_ok();
    Ok(ProjectImageView {
        path,
        bytes,
        width,
        height,
        pixel_art_importable,
    })
}
