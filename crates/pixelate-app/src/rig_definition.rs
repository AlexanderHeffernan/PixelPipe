use pixelate_core::{CoreError, PixelRig, RIG_SCHEMA, RigInterpolation, RigNode, RigPart, RigPose};
use pixelate_project::RevisionSnapshot;
use serde::{Deserialize, Serialize};

use crate::AppError;

pub const RIG_DEFINITION_SCHEMA: &str = "pixelate.rig-definition/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RigDefinition {
    pub schema: String,
    pub width: u32,
    pub height: u32,
    pub parts: Vec<RigPartDefinition>,
    pub nodes: Vec<RigNode>,
    pub poses: Vec<RigPose>,
    pub frame_duration_ms: u32,
    #[serde(default)]
    pub interpolation: RigInterpolation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pivot: Option<[i32; 2]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RigPartDefinition {
    pub id: String,
    /// X, Y, width, and height in the indexed source frame.
    pub source: [u32; 4],
    /// Rotation origin in part-local pixel coordinates.
    pub pivot: [i32; 2],
}

pub(crate) fn build_rig(
    parent: &RevisionSnapshot,
    source_frame_id: Option<&str>,
    definition: RigDefinition,
) -> Result<PixelRig, AppError> {
    if definition.schema != RIG_DEFINITION_SCHEMA {
        return Err(CoreError::InvalidRig(format!(
            "unsupported definition schema '{}', expected '{RIG_DEFINITION_SCHEMA}'",
            definition.schema
        ))
        .into());
    }
    let frame_id = match source_frame_id {
        Some(frame_id) => frame_id,
        None if parent.sequence.frames.len() == 1 => &parent.sequence.frames[0].id,
        None => return Err(AppError::AmbiguousFrameTarget),
    };
    let source = parent.sequence.raster(frame_id)?;
    let parts = definition
        .parts
        .into_iter()
        .map(|part| crop_part(&source, part))
        .collect::<Result<Vec<_>, _>>()?;
    let rig = PixelRig {
        schema: RIG_SCHEMA.to_owned(),
        width: definition.width,
        height: definition.height,
        palette: source.palette,
        parts,
        nodes: definition.nodes,
        poses: definition.poses,
        frame_duration_ms: definition.frame_duration_ms,
        interpolation: definition.interpolation,
        pivot: definition.pivot,
        metadata: source.metadata,
    };
    rig.validate()?;
    Ok(rig)
}

fn crop_part(
    source: &pixelate_core::IndexedRaster,
    definition: RigPartDefinition,
) -> Result<RigPart, AppError> {
    let [x, y, width, height] = definition.source;
    if width == 0
        || height == 0
        || x.checked_add(width)
            .is_none_or(|right| right > source.width)
        || y.checked_add(height)
            .is_none_or(|bottom| bottom > source.height)
    {
        return Err(CoreError::InvalidRig(format!(
            "part '{}' source rectangle is outside the indexed frame",
            definition.id
        ))
        .into());
    }
    let mut pixels = Vec::with_capacity(
        usize::try_from(u64::from(width) * u64::from(height))
            .map_err(|_| CoreError::DimensionOverflow)?,
    );
    for source_y in y..y + height {
        let start = usize::try_from(u64::from(source_y) * u64::from(source.width) + u64::from(x))
            .map_err(|_| CoreError::DimensionOverflow)?;
        let count = usize::try_from(width).map_err(|_| CoreError::DimensionOverflow)?;
        pixels.extend_from_slice(&source.pixels[start..start + count]);
    }
    Ok(RigPart {
        id: definition.id,
        width,
        height,
        pixels,
        pivot: definition.pivot,
    })
}
