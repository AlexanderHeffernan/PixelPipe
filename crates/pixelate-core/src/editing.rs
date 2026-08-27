use std::collections::{BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{
    ComponentExpectation, CoreError, IndexedRaster, Palette, ensure_schema,
    validate_component_expectation,
};

pub const PATCH_SCHEMA: &str = "pixelate.patch/v1";
pub const PALETTE_REMAP_SCHEMA: &str = "pixelate.palette-remap/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PixelPatch {
    pub x: u32,
    pub y: u32,
    pub index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ComponentRule {
    Raster { expectation: ComponentExpectation },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PixelPatchSet {
    pub schema: String,
    pub edits: Vec<PixelPatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structure: Option<ComponentRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaletteRemap {
    pub schema: String,
    pub palette: Palette,
    pub index_map: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structure: Option<ComponentRule>,
}

/// Applies a complete coordinate patch to a cloned raster after validating all edits.
///
/// # Errors
///
/// Returns a [`CoreError`] without producing a raster when the document, any
/// coordinate/index, duplicate, or resulting component structure is invalid.
pub fn apply_pixel_patch(
    raster: &IndexedRaster,
    patch: &PixelPatchSet,
) -> Result<IndexedRaster, CoreError> {
    raster.validate()?;
    ensure_schema(&patch.schema, PATCH_SCHEMA)?;
    let mut coordinates = BTreeSet::new();
    for edit in &patch.edits {
        if edit.x >= raster.width || edit.y >= raster.height {
            return Err(CoreError::PatchOutOfBounds {
                x: edit.x,
                y: edit.y,
            });
        }
        if usize::from(edit.index) >= raster.palette.colors.len() {
            return Err(CoreError::InvalidPatchIndex { index: edit.index });
        }
        if !coordinates.insert((edit.y, edit.x)) {
            return Err(CoreError::DuplicatePatch {
                x: edit.x,
                y: edit.y,
            });
        }
    }

    let mut result = raster.clone();
    for edit in &patch.edits {
        let offset = pixel_offset(raster.width, edit.x, edit.y)?;
        result.pixels[offset] = edit.index;
    }
    result.validate()?;
    if let Some(rule) = patch.structure {
        validate_structure(&result, rule)?;
    }
    Ok(result)
}

/// Resolves a four-connected flood fill into one deterministic atomic patch.
///
/// # Errors
///
/// Returns a [`CoreError`] when the raster, coordinate, or palette index is invalid.
pub fn flood_fill_patch(
    raster: &IndexedRaster,
    x: u32,
    y: u32,
    index: u8,
) -> Result<PixelPatchSet, CoreError> {
    raster.validate()?;
    if x >= raster.width || y >= raster.height {
        return Err(CoreError::PatchOutOfBounds { x, y });
    }
    if usize::from(index) >= raster.palette.colors.len() {
        return Err(CoreError::InvalidPatchIndex { index });
    }
    let start = pixel_offset(raster.width, x, y)?;
    let replaced = raster.pixels[start];
    if replaced == index {
        return Ok(PixelPatchSet {
            schema: PATCH_SCHEMA.to_owned(),
            edits: Vec::new(),
            structure: None,
        });
    }

    let mut visited = vec![false; raster.pixels.len()];
    let mut queue = VecDeque::from([(x, y)]);
    let mut edits = Vec::new();
    while let Some((current_x, current_y)) = queue.pop_front() {
        let offset = pixel_offset(raster.width, current_x, current_y)?;
        if visited[offset] || raster.pixels[offset] != replaced {
            continue;
        }
        visited[offset] = true;
        edits.push(PixelPatch {
            x: current_x,
            y: current_y,
            index,
        });
        if current_x > 0 {
            queue.push_back((current_x - 1, current_y));
        }
        if current_x + 1 < raster.width {
            queue.push_back((current_x + 1, current_y));
        }
        if current_y > 0 {
            queue.push_back((current_x, current_y - 1));
        }
        if current_y + 1 < raster.height {
            queue.push_back((current_x, current_y + 1));
        }
    }
    Ok(PixelPatchSet {
        schema: PATCH_SCHEMA.to_owned(),
        edits,
        structure: None,
    })
}

/// Applies an explicit old-index to new-index map and replaces the palette.
///
/// # Errors
///
/// Returns a [`CoreError`] without producing a raster when either palette, the
/// mapping, transparent-index semantics, or resulting component structure is invalid.
pub fn apply_palette_remap(
    raster: &IndexedRaster,
    remap: &PaletteRemap,
) -> Result<IndexedRaster, CoreError> {
    raster.validate()?;
    ensure_schema(&remap.schema, PALETTE_REMAP_SCHEMA)?;
    remap.palette.validate()?;
    if remap.index_map.len() != raster.palette.colors.len() {
        return Err(CoreError::InvalidRemapLength {
            expected: raster.palette.colors.len(),
            actual: remap.index_map.len(),
        });
    }
    for index in &remap.index_map {
        if usize::from(*index) >= remap.palette.colors.len() {
            return Err(CoreError::InvalidRemapIndex { index: *index });
        }
    }
    if remap.index_map[usize::from(raster.palette.transparent_index)]
        != remap.palette.transparent_index
    {
        return Err(CoreError::InvalidTransparentRemap);
    }

    let mut result = raster.clone();
    result.palette = remap.palette.clone();
    result.pixels = raster
        .pixels
        .iter()
        .map(|index| remap.index_map[usize::from(*index)])
        .collect();
    result.validate()?;
    if let Some(rule) = remap.structure {
        validate_structure(&result, rule)?;
    }
    Ok(result)
}

fn validate_structure(raster: &IndexedRaster, rule: ComponentRule) -> Result<(), CoreError> {
    match rule {
        ComponentRule::Raster { expectation } => {
            validate_component_expectation(raster, expectation)?;
        }
    }
    Ok(())
}

fn pixel_offset(width: u32, x: u32, y: u32) -> Result<usize, CoreError> {
    usize::try_from(u64::from(y) * u64::from(width) + u64::from(x))
        .map_err(|_| CoreError::DimensionOverflow)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{PALETTE_SCHEMA, RASTER_SCHEMA};

    use super::*;

    fn raster() -> IndexedRaster {
        IndexedRaster {
            schema: RASTER_SCHEMA.to_owned(),
            width: 2,
            height: 2,
            palette: Palette {
                schema: PALETTE_SCHEMA.to_owned(),
                name: "old".to_owned(),
                transparent_index: 0,
                colors: vec![[0, 0, 0, 0], [200, 20, 20, 255], [20, 20, 200, 255]],
            },
            pixels: vec![0, 1, 1, 2],
            pivot: Some([1, 2]),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn patch_is_atomic_and_rejects_duplicate_coordinates() {
        let original = raster();
        let patch = PixelPatchSet {
            schema: PATCH_SCHEMA.to_owned(),
            edits: vec![
                PixelPatch {
                    x: 0,
                    y: 0,
                    index: 1,
                },
                PixelPatch {
                    x: 0,
                    y: 0,
                    index: 2,
                },
            ],
            structure: None,
        };
        assert!(matches!(
            apply_pixel_patch(&original, &patch),
            Err(CoreError::DuplicatePatch { x: 0, y: 0 })
        ));
        assert_eq!(original.pixels, vec![0, 1, 1, 2]);
    }

    #[test]
    fn remap_requires_explicit_transparent_mapping() {
        let original = raster();
        let remap = PaletteRemap {
            schema: PALETTE_REMAP_SCHEMA.to_owned(),
            palette: Palette::new("new", 1, vec![[40, 40, 40, 255], [0, 0, 0, 0]]),
            index_map: vec![0, 0, 0],
            structure: None,
        };
        assert!(matches!(
            apply_palette_remap(&original, &remap),
            Err(CoreError::InvalidTransparentRemap)
        ));
    }

    #[test]
    fn remap_replaces_palette_and_indices_without_mutating_input() {
        let original = raster();
        let remap = PaletteRemap {
            schema: PALETTE_REMAP_SCHEMA.to_owned(),
            palette: Palette::new(
                "new",
                2,
                vec![[240, 120, 20, 255], [20, 220, 100, 255], [0, 0, 0, 0]],
            ),
            index_map: vec![2, 0, 1],
            structure: Some(ComponentRule::Raster {
                expectation: ComponentExpectation { min: 1, max: 1 },
            }),
        };
        let result = apply_palette_remap(&original, &remap).expect("remap");
        assert_eq!(result.pixels, vec![2, 0, 0, 1]);
        assert_eq!(result.palette.name, "new");
        assert_eq!(original.palette.name, "old");
    }

    #[test]
    fn flood_fill_resolves_one_connected_region_in_stable_order() {
        let original = raster();
        let patch = flood_fill_patch(&original, 0, 1, 2).expect("fill");
        assert_eq!(
            patch.edits,
            vec![PixelPatch {
                x: 0,
                y: 1,
                index: 2
            }]
        );
        assert_eq!(
            apply_pixel_patch(&original, &patch).unwrap().pixels,
            vec![0, 1, 2, 2]
        );
    }
}
