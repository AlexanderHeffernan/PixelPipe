use serde::{Deserialize, Serialize};

use crate::{CoreError, IndexedRaster};

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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{PALETTE_SCHEMA, Palette, RASTER_SCHEMA};

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
}
