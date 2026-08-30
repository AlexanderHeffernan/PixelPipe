use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    CoreError, IndexedRaster, IndexedSequence, Palette, RASTER_SCHEMA, ValidationCheck,
    ValidationReport,
};

pub const RIG_SCHEMA: &str = "pixelate.rig/v1";
pub(crate) const GENERATED_PREFIX: &str = "__generated-";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PixelRig {
    pub schema: String,
    pub width: u32,
    pub height: u32,
    pub palette: Palette,
    pub parts: Vec<RigPart>,
    pub nodes: Vec<RigNode>,
    pub poses: Vec<RigPose>,
    pub frame_duration_ms: u32,
    #[serde(default)]
    pub interpolation: RigInterpolation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pivot: Option<[i32; 2]>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RigPart {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub pivot: [i32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RigNode {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RigPose {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub nodes: Vec<RigNodePose>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RigNodePose {
    pub node_id: String,
    pub part_id: String,
    pub x_millis: i32,
    pub y_millis: i32,
    pub rotation_millidegrees: i32,
    pub scale_x_millis: i32,
    pub scale_y_millis: i32,
    pub depth: i32,
    #[serde(default = "visible")]
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RigInterpolation {
    pub inbetweens: u16,
    pub looped: bool,
}

fn visible() -> bool {
    true
}

impl PixelRig {
    /// Validates a generic indexed-pixel rig and all explicit poses.
    ///
    /// # Errors
    /// Returns [`CoreError`] when an identity, reference, hierarchy, part,
    /// transform, palette, canvas, pose, or timing invariant is invalid.
    pub fn validate(&self) -> Result<ValidationReport, CoreError> {
        if self.schema != RIG_SCHEMA {
            return Err(CoreError::Schema {
                expected: RIG_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        if self.width == 0 || self.height == 0 || self.width > 8192 || self.height > 8192 {
            return Err(invalid(
                "canvas dimensions must be between 1 and 8192 pixels",
            ));
        }
        self.palette.validate()?;
        if self.frame_duration_ms == 0 {
            return Err(invalid("frame duration must be greater than zero"));
        }
        if self.parts.is_empty() || self.nodes.is_empty() || self.poses.is_empty() {
            return Err(invalid("at least one part, node, and pose is required"));
        }
        if self.interpolation.inbetweens > 120 {
            return Err(invalid("at most 120 in-between frames are supported"));
        }

        let part_ids = unique_ids(self.parts.iter().map(|part| part.id.as_str()), "part")?;
        for part in &self.parts {
            IndexedRaster {
                schema: RASTER_SCHEMA.to_owned(),
                width: part.width,
                height: part.height,
                palette: self.palette.clone(),
                pixels: part.pixels.clone(),
                pivot: Some(part.pivot),
                metadata: BTreeMap::new(),
            }
            .validate()?;
            if part.pivot[0].unsigned_abs() > 16_384 || part.pivot[1].unsigned_abs() > 16_384 {
                return Err(invalid(
                    "part pivots must remain within the supported range",
                ));
            }
        }

        let node_ids = unique_ids(self.nodes.iter().map(|node| node.id.as_str()), "node")?;
        for node in &self.nodes {
            if node
                .parent_id
                .as_ref()
                .is_some_and(|parent| parent == &node.id || !node_ids.contains(parent.as_str()))
            {
                return Err(invalid("every node parent must reference a different node"));
            }
            validate_parent_chain(&node.id, &self.nodes)?;
        }

        unique_ids(self.poses.iter().map(|pose| pose.id.as_str()), "pose")?;
        for pose in &self.poses {
            if pose.id.starts_with(GENERATED_PREFIX) {
                return Err(invalid("pose IDs may not use the generated-frame prefix"));
            }
            if pose
                .name
                .as_ref()
                .is_some_and(|name| name.trim().is_empty())
            {
                return Err(invalid("pose names must not be empty"));
            }
            let posed = unique_ids(
                pose.nodes.iter().map(|node| node.node_id.as_str()),
                "posed node",
            )?;
            if posed != node_ids {
                return Err(invalid(
                    "every pose must contain every rig node exactly once",
                ));
            }
            for node in &pose.nodes {
                if !part_ids.contains(node.part_id.as_str()) {
                    return Err(invalid("every posed node must reference an existing part"));
                }
                if node.scale_x_millis == 0
                    || node.scale_y_millis == 0
                    || node.scale_x_millis.unsigned_abs() > 8_000
                    || node.scale_y_millis.unsigned_abs() > 8_000
                {
                    return Err(invalid(
                        "node scales must be nonzero and at most 8000 millis",
                    ));
                }
                if node.x_millis.unsigned_abs() > 16_384_000
                    || node.y_millis.unsigned_abs() > 16_384_000
                {
                    return Err(invalid(
                        "node positions must remain within the supported range",
                    ));
                }
            }
        }

        Ok(ValidationReport {
            schema: crate::VALIDATION_SCHEMA.to_owned(),
            valid: true,
            checks: vec![
                ValidationCheck {
                    name: "rig_parts".to_owned(),
                    passed: true,
                    detail: format!("{} indexed parts", self.parts.len()),
                },
                ValidationCheck {
                    name: "rig_nodes".to_owned(),
                    passed: true,
                    detail: format!("{} generic nodes", self.nodes.len()),
                },
                ValidationCheck {
                    name: "rig_poses".to_owned(),
                    passed: true,
                    detail: format!("{} manual poses", self.poses.len()),
                },
            ],
        })
    }

    /// Deterministically renders manual poses and configured derived in-betweens.
    ///
    /// # Errors
    /// Returns [`CoreError`] when the rig is invalid or fixed-point transform
    /// arithmetic exceeds the supported range.
    pub fn render_sequence(&self) -> Result<IndexedSequence, CoreError> {
        crate::rig_render::render_rig(self)
    }
}

fn unique_ids<'a>(
    ids: impl Iterator<Item = &'a str>,
    kind: &str,
) -> Result<BTreeSet<&'a str>, CoreError> {
    let mut result = BTreeSet::new();
    for id in ids {
        if id.trim().is_empty() || !result.insert(id) {
            return Err(CoreError::InvalidRig(format!(
                "{kind} IDs must be nonempty and unique"
            )));
        }
    }
    Ok(result)
}

fn validate_parent_chain(id: &str, nodes: &[RigNode]) -> Result<(), CoreError> {
    let mut seen = BTreeSet::new();
    let mut current = Some(id);
    while let Some(id) = current {
        if !seen.insert(id) {
            return Err(invalid("node hierarchy must not contain a cycle"));
        }
        current = nodes
            .iter()
            .find(|node| node.id == id)
            .and_then(|node| node.parent_id.as_deref());
    }
    Ok(())
}

pub(crate) fn invalid(message: &str) -> CoreError {
    CoreError::InvalidRig(message.to_owned())
}
