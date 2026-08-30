use std::path::PathBuf;

use pixelate_core::{DetectedPart, detect_parts};
use pixelate_project::ProjectStore;
use serde::{Deserialize, Serialize};

use crate::{AppError, resolve_revision};

#[derive(Debug, Deserialize)]
pub struct DiscoverRigParts {
    pub start: PathBuf,
    pub asset: String,
    pub revision: Option<String>,
    pub frame_id: Option<String>,
    pub minimum_pixels: u32,
}

#[derive(Debug, Serialize)]
pub struct RigPartsResult {
    pub project_root: PathBuf,
    pub asset: String,
    pub revision: String,
    pub frame_id: String,
    pub parts: Vec<DetectedPart>,
}

/// Discovers exact reusable-part bounds in a verified indexed revision.
///
/// # Errors
/// Returns [`AppError`] when the revision or requested frame is unavailable or invalid.
pub fn discover_rig_parts(request: DiscoverRigParts) -> Result<RigPartsResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let revision = resolve_revision(&store, &request.asset, request.revision)?;
    let snapshot = store.revision(&request.asset, &revision)?;
    let frame_id = match request.frame_id {
        Some(frame_id) => frame_id,
        None if snapshot.sequence.frames.len() == 1 => snapshot.sequence.frames[0].id.clone(),
        None => return Err(AppError::AmbiguousFrameTarget),
    };
    let parts = detect_parts(
        &snapshot.sequence.raster(&frame_id)?,
        request.minimum_pixels,
    )?;
    Ok(RigPartsResult {
        project_root: store.root().to_path_buf(),
        asset: request.asset,
        revision,
        frame_id,
        parts,
    })
}
