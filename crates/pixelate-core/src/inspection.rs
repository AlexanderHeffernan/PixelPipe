use serde::{Deserialize, Serialize};

use crate::{CoreError, IndexedRaster, Palette, RASTER_SCHEMA, RenderedRaster, render, sha256_hex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RasterBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaletteUsage {
    pub index: u8,
    pub rgba: [u8; 4],
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RasterInspection {
    pub width: u32,
    pub height: u32,
    pub pivot: Option<[i32; 2]>,
    pub visible_bounds: Option<RasterBounds>,
    pub visible_pixels: u64,
    pub palette: Vec<PaletteUsage>,
    pub text_rows: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PixelDifference {
    pub x: u32,
    pub y: u32,
    pub left_index: Option<u8>,
    pub right_index: Option<u8>,
    pub left_rgba: Option<[u8; 4]>,
    pub right_rgba: Option<[u8; 4]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaletteDifference {
    pub index: u16,
    pub left: Option<[u8; 4]>,
    pub right: Option<[u8; 4]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RasterDiff {
    pub left_dimensions: [u32; 2],
    pub right_dimensions: [u32; 2],
    pub changed_bounds: Option<RasterBounds>,
    pub changed_pixels: Vec<PixelDifference>,
    pub palette_differences: Vec<PaletteDifference>,
}

#[derive(Debug)]
pub struct RasterComparison {
    pub diff: RasterDiff,
    pub visual: RenderedRaster,
    pub visual_native_sha256: String,
    pub visual_preview_sha256: String,
}

/// Produces deterministic palette usage, visible bounds, and an indexed text grid.
///
/// # Errors
///
/// Returns a [`CoreError`] when the raster is invalid or dimensions overflow.
pub fn inspect_raster(raster: &IndexedRaster) -> Result<RasterInspection, CoreError> {
    raster.validate()?;
    let mut counts = vec![0_u64; raster.palette.colors.len()];
    let mut visible = 0_u64;
    let mut bounds = BoundsAccumulator::new(raster.width, raster.height);
    let mut text_rows = Vec::with_capacity(
        usize::try_from(raster.height).map_err(|_| CoreError::DimensionOverflow)?,
    );
    for y in 0..raster.height {
        let mut tokens = Vec::with_capacity(
            usize::try_from(raster.width).map_err(|_| CoreError::DimensionOverflow)?,
        );
        for x in 0..raster.width {
            let index = raster.pixels[pixel_offset(raster.width, x, y)?];
            counts[usize::from(index)] += 1;
            if index == raster.palette.transparent_index {
                tokens.push("--".to_owned());
            } else {
                tokens.push(format!("{index:02X}"));
                visible += 1;
                bounds.include(x, y);
            }
        }
        text_rows.push(tokens.join(" "));
    }
    let palette = raster
        .palette
        .colors
        .iter()
        .enumerate()
        .map(|(index, rgba)| {
            Ok(PaletteUsage {
                index: u8::try_from(index).map_err(|_| CoreError::InvalidPaletteSize)?,
                rgba: *rgba,
                count: counts[index],
            })
        })
        .collect::<Result<Vec<_>, CoreError>>()?;
    Ok(RasterInspection {
        width: raster.width,
        height: raster.height,
        pivot: raster.pivot,
        visible_bounds: bounds.finish(),
        visible_pixels: visible,
        palette,
        text_rows,
    })
}

/// Compares canonical pixel colors and returns a machine diff plus indexed visual diff.
///
/// # Errors
///
/// Returns a [`CoreError`] when either raster is invalid or rendering overflows.
pub fn compare_rasters(
    left: &IndexedRaster,
    right: &IndexedRaster,
    preview_scale: u16,
) -> Result<RasterComparison, CoreError> {
    left.validate()?;
    right.validate()?;
    let width = left.width.max(right.width);
    let height = left.height.max(right.height);
    let mut changed_pixels = Vec::new();
    let mut changed_bounds = BoundsAccumulator::new(width, height);
    let mut visual_pixels = Vec::with_capacity(pixel_count(width, height)?);
    for y in 0..height {
        for x in 0..width {
            let left_pixel = pixel_at(left, x, y)?;
            let right_pixel = pixel_at(right, x, y)?;
            let same_color = left_pixel.map(|pixel| pixel.1) == right_pixel.map(|pixel| pixel.1);
            if same_color {
                visual_pixels.push(0);
                continue;
            }
            changed_bounds.include(x, y);
            changed_pixels.push(PixelDifference {
                x,
                y,
                left_index: left_pixel.map(|pixel| pixel.0),
                right_index: right_pixel.map(|pixel| pixel.0),
                left_rgba: left_pixel.map(|pixel| pixel.1),
                right_rgba: right_pixel.map(|pixel| pixel.1),
            });
            visual_pixels.push(
                match (is_visible(left, left_pixel), is_visible(right, right_pixel)) {
                    (true, false) => 1,
                    (false, true) => 2,
                    _ => 3,
                },
            );
        }
    }

    let palette_differences = compare_palettes(&left.palette, &right.palette);
    let visual_raster = IndexedRaster {
        schema: RASTER_SCHEMA.to_owned(),
        width,
        height,
        palette: Palette::new(
            "pixelate-diff",
            0,
            vec![
                [0, 0, 0, 0],
                [239, 68, 68, 255],
                [34, 197, 94, 255],
                [217, 70, 239, 255],
            ],
        ),
        pixels: visual_pixels,
        pivot: None,
        metadata: std::collections::BTreeMap::from([
            ("removed".to_owned(), "palette:1".to_owned()),
            ("added".to_owned(), "palette:2".to_owned()),
            ("changed".to_owned(), "palette:3".to_owned()),
        ]),
    };
    let visual = render(&visual_raster, preview_scale)?;
    let visual_native_sha256 = sha256_hex(&visual.native_png);
    let visual_preview_sha256 = sha256_hex(&visual.preview_png);
    Ok(RasterComparison {
        diff: RasterDiff {
            left_dimensions: [left.width, left.height],
            right_dimensions: [right.width, right.height],
            changed_bounds: changed_bounds.finish(),
            changed_pixels,
            palette_differences,
        },
        visual,
        visual_native_sha256,
        visual_preview_sha256,
    })
}

fn pixel_at(raster: &IndexedRaster, x: u32, y: u32) -> Result<Option<(u8, [u8; 4])>, CoreError> {
    if x >= raster.width || y >= raster.height {
        return Ok(None);
    }
    let index = raster.pixels[pixel_offset(raster.width, x, y)?];
    Ok(Some((index, raster.palette.colors[usize::from(index)])))
}

fn is_visible(raster: &IndexedRaster, pixel: Option<(u8, [u8; 4])>) -> bool {
    pixel.is_some_and(|pixel| pixel.0 != raster.palette.transparent_index)
}

fn compare_palettes(left: &Palette, right: &Palette) -> Vec<PaletteDifference> {
    let count = left.colors.len().max(right.colors.len());
    (0..count)
        .filter_map(|index| {
            let left = left.colors.get(index).copied();
            let right = right.colors.get(index).copied();
            (left != right).then(|| PaletteDifference {
                index: u16::try_from(index).expect("palette comparison has at most 256 entries"),
                left,
                right,
            })
        })
        .collect()
}

struct BoundsAccumulator {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
    found: bool,
}

impl BoundsAccumulator {
    fn new(width: u32, height: u32) -> Self {
        Self {
            min_x: width,
            min_y: height,
            max_x: 0,
            max_y: 0,
            found: false,
        }
    }

    fn include(&mut self, x: u32, y: u32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
        self.found = true;
    }

    fn finish(self) -> Option<RasterBounds> {
        self.found.then_some(RasterBounds {
            x: self.min_x,
            y: self.min_y,
            width: self.max_x - self.min_x + 1,
            height: self.max_y - self.min_y + 1,
        })
    }
}

fn pixel_offset(width: u32, x: u32, y: u32) -> Result<usize, CoreError> {
    usize::try_from(u64::from(y) * u64::from(width) + u64::from(x))
        .map_err(|_| CoreError::DimensionOverflow)
}

fn pixel_count(width: u32, height: u32) -> Result<usize, CoreError> {
    usize::try_from(u64::from(width) * u64::from(height)).map_err(|_| CoreError::DimensionOverflow)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::PALETTE_SCHEMA;

    use super::*;

    fn raster(pixels: Vec<u8>) -> IndexedRaster {
        IndexedRaster {
            schema: RASTER_SCHEMA.to_owned(),
            width: 2,
            height: 2,
            palette: Palette {
                schema: PALETTE_SCHEMA.to_owned(),
                name: "inspect".to_owned(),
                transparent_index: 0,
                colors: vec![[0, 0, 0, 0], [200, 20, 20, 255], [20, 20, 200, 255]],
            },
            pixels,
            pivot: None,
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn inspection_has_stable_grid_bounds_and_counts() {
        let inspection = inspect_raster(&raster(vec![0, 1, 2, 1])).expect("inspect");
        assert_eq!(inspection.text_rows, vec!["-- 01", "02 01"]);
        assert_eq!(
            inspection.visible_bounds,
            Some(RasterBounds {
                x: 0,
                y: 0,
                width: 2,
                height: 2
            })
        );
        assert_eq!(inspection.palette[1].count, 2);
    }

    #[test]
    fn comparison_uses_rgba_semantics_and_stable_coordinate_order() {
        let left = raster(vec![0, 1, 2, 1]);
        let right = raster(vec![1, 1, 1, 0]);
        let comparison = compare_rasters(&left, &right, 4).expect("compare");
        assert_eq!(
            comparison
                .diff
                .changed_pixels
                .iter()
                .map(|pixel| (pixel.x, pixel.y))
                .collect::<Vec<_>>(),
            vec![(0, 0), (0, 1), (1, 1)]
        );
        assert_eq!(
            comparison.diff.changed_bounds,
            Some(RasterBounds {
                x: 0,
                y: 0,
                width: 2,
                height: 2
            })
        );
        assert_eq!(
            comparison.visual.validation.visual_review,
            crate::VisualReview::Required
        );
    }
}
