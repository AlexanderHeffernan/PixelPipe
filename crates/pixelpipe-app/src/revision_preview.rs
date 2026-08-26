use std::path::PathBuf;

use pixelpipe_core::{render, sha256_hex};
use pixelpipe_project::ProjectStore;
use serde::{Deserialize, Serialize};

use crate::{AppError, resolve_revision};

const TARGET_LONG_EDGE: u32 = 512;
const MAX_PREVIEW_SCALE: u16 = 64;

#[derive(Debug, Deserialize)]
pub struct PreviewRevision {
    pub start: PathBuf,
    pub asset: String,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub scale: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RevisionPreview {
    pub project_root: PathBuf,
    pub asset: String,
    pub revision: String,
    pub native_width: u32,
    pub native_height: u32,
    pub scale: u16,
    pub width: u32,
    pub height: u32,
    pub sha256: String,
    #[serde(skip)]
    pub png: Vec<u8>,
}

/// Renders a verified revision for visual inspection without changing project state.
///
/// The default exact nearest-neighbour scale targets a 512-pixel longest edge,
/// capped at the deterministic renderer's maximum 64× scale.
///
/// # Errors
///
/// Returns an [`AppError`] when project discovery, revision verification, or
/// deterministic PNG rendering fails.
pub fn preview_revision(request: PreviewRevision) -> Result<RevisionPreview, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let revision = resolve_revision(&store, &request.asset, request.revision)?;
    let snapshot = store.revision(&request.asset, &revision)?;
    let native_width = snapshot.raster.width;
    let native_height = snapshot.raster.height;
    let scale = request.scale.unwrap_or_else(|| {
        let longest_edge = native_width.max(native_height);
        u16::try_from((TARGET_LONG_EDGE / longest_edge).clamp(1, u32::from(MAX_PREVIEW_SCALE)))
            .unwrap_or(MAX_PREVIEW_SCALE)
    });
    let png = render(&snapshot.raster, scale)?.preview_png;
    Ok(RevisionPreview {
        project_root: store.root().to_path_buf(),
        asset: request.asset,
        revision,
        native_width,
        native_height,
        scale,
        width: native_width * u32::from(scale),
        height: native_height * u32::from(scale),
        sha256: sha256_hex(&png),
        png,
    })
}
