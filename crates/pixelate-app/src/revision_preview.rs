use std::{io::Cursor, path::PathBuf};

use image::{
    Delay, Frame, RgbaImage,
    codecs::gif::{GifEncoder, Repeat},
};
use pixelate_core::{render, render_sequence_preview, sha256_hex};
use pixelate_project::ProjectStore;
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
    pub frame_id: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct PreviewAnimation {
    pub start: PathBuf,
    pub asset: String,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub scale: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AnimationPreview {
    pub project_root: PathBuf,
    pub asset: String,
    pub revision: String,
    pub frame_count: usize,
    pub scale: u16,
    pub width: u32,
    pub height: u32,
    pub sha256: String,
    #[serde(skip)]
    pub gif: Vec<u8>,
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
    let selected = request
        .frame_id
        .map(|frame_id| snapshot.sequence.raster(&frame_id))
        .transpose()?;
    let sheet_width = snapshot
        .sequence
        .width
        .checked_mul(
            u32::try_from(snapshot.sequence.frames.len())
                .map_err(|_| pixelate_core::CoreError::DimensionOverflow)?,
        )
        .ok_or(pixelate_core::CoreError::DimensionOverflow)?;
    let native_width = selected
        .as_ref()
        .map_or_else(|| sheet_width, |raster| raster.width);
    let native_height = selected
        .as_ref()
        .map_or(snapshot.sequence.height, |raster| raster.height);
    let scale = request.scale.unwrap_or_else(|| {
        let longest_edge = native_width.max(native_height);
        u16::try_from((TARGET_LONG_EDGE / longest_edge).clamp(1, u32::from(MAX_PREVIEW_SCALE)))
            .unwrap_or(MAX_PREVIEW_SCALE)
    });
    let png = match selected {
        Some(raster) => render(&raster, scale)?.preview_png,
        None => render_sequence_preview(&snapshot.sequence, scale)?,
    };
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

/// Encodes a verified sequence as a looping nearest-neighbour GIF for motion review.
///
/// # Errors
/// Returns [`AppError`] when verification, scaling, or GIF encoding fails.
pub fn preview_animation(request: PreviewAnimation) -> Result<AnimationPreview, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let revision = resolve_revision(&store, &request.asset, request.revision)?;
    let snapshot = store.revision(&request.asset, &revision)?;
    let longest_edge = snapshot.sequence.width.max(snapshot.sequence.height);
    let scale = request.scale.unwrap_or_else(|| {
        u16::try_from((TARGET_LONG_EDGE / longest_edge).clamp(1, u32::from(MAX_PREVIEW_SCALE)))
            .unwrap_or(MAX_PREVIEW_SCALE)
    });
    if !(1..=MAX_PREVIEW_SCALE).contains(&scale) {
        return Err(pixelate_core::CoreError::InvalidPreviewScale.into());
    }
    let width = snapshot
        .sequence
        .width
        .checked_mul(u32::from(scale))
        .ok_or(pixelate_core::CoreError::DimensionOverflow)?;
    let height = snapshot
        .sequence
        .height
        .checked_mul(u32::from(scale))
        .ok_or(pixelate_core::CoreError::DimensionOverflow)?;
    let mut gif = Cursor::new(Vec::new());
    {
        let mut encoder = GifEncoder::new(&mut gif);
        encoder
            .set_repeat(Repeat::Infinite)
            .map_err(|error| AppError::Image(error.to_string()))?;
        for frame in &snapshot.sequence.frames {
            let mut rgba = Vec::with_capacity(
                usize::try_from(u64::from(width) * u64::from(height) * 4)
                    .map_err(|_| pixelate_core::CoreError::DimensionOverflow)?,
            );
            for source_y in 0..snapshot.sequence.height {
                for _ in 0..scale {
                    for source_x in 0..snapshot.sequence.width {
                        let index = usize::try_from(
                            u64::from(source_y) * u64::from(snapshot.sequence.width)
                                + u64::from(source_x),
                        )
                        .map_err(|_| pixelate_core::CoreError::DimensionOverflow)?;
                        let color =
                            snapshot.sequence.palette.colors[usize::from(frame.pixels[index])];
                        for _ in 0..scale {
                            rgba.extend_from_slice(&color);
                        }
                    }
                }
            }
            let image = RgbaImage::from_raw(width, height, rgba)
                .ok_or(pixelate_core::CoreError::DimensionOverflow)?;
            encoder
                .encode_frame(Frame::from_parts(
                    image,
                    0,
                    0,
                    Delay::from_numer_denom_ms(frame.duration_ms, 1),
                ))
                .map_err(|error| AppError::Image(error.to_string()))?;
        }
    }
    let gif = gif.into_inner();
    Ok(AnimationPreview {
        project_root: store.root().to_path_buf(),
        asset: request.asset,
        revision,
        frame_count: snapshot.sequence.frames.len(),
        scale,
        width,
        height,
        sha256: sha256_hex(&gif),
        gif,
    })
}
