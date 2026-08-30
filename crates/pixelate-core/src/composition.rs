use serde::{Deserialize, Serialize};

use crate::{CoreError, IndexedRaster};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvasSettings {
    pub width: u32,
    pub height: u32,
    pub scale_percent: u16,
    pub offset_x: i16,
    pub offset_y: i16,
}

/// Places the visible sprite on a new canvas without re-running conversion.
///
/// Positive x moves right and positive y moves up. Pixels outside the target
/// canvas are deterministically clipped.
///
/// # Errors
///
/// Returns [`CoreError`] when the source raster or target dimensions are invalid.
pub fn compose_canvas(
    source: &IndexedRaster,
    settings: CanvasSettings,
) -> Result<IndexedRaster, CoreError> {
    source.validate()?;
    if settings.width == 0
        || settings.height == 0
        || settings.scale_percent < 25
        || settings.scale_percent > 400
        || settings.width > 8192
        || settings.height > 8192
    {
        return Err(CoreError::InvalidDimensions);
    }
    let length = usize::try_from(u64::from(settings.width) * u64::from(settings.height))
        .map_err(|_| CoreError::DimensionOverflow)?;
    let mut pixels = vec![source.palette.transparent_index; length];

    if let Some(bounds) = visible_bounds(source) {
        let scaled_width = scaled_dimension(bounds.width, settings.scale_percent)?;
        let scaled_height = scaled_dimension(bounds.height, settings.scale_percent)?;
        let origin_x = (i64::from(settings.width) - i64::from(scaled_width)).div_euclid(2)
            + i64::from(settings.offset_x);
        let origin_y = (i64::from(settings.height) - i64::from(scaled_height)).div_euclid(2)
            - i64::from(settings.offset_y);
        for y in 0..scaled_height {
            for x in 0..scaled_width {
                let target_x = origin_x + i64::from(x);
                let target_y = origin_y + i64::from(y);
                if target_x < 0
                    || target_y < 0
                    || target_x >= i64::from(settings.width)
                    || target_y >= i64::from(settings.height)
                {
                    continue;
                }
                let source_x =
                    u32::try_from(u64::from(x) * u64::from(bounds.width) / u64::from(scaled_width))
                        .map_err(|_| CoreError::DimensionOverflow)?;
                let source_y = u32::try_from(
                    u64::from(y) * u64::from(bounds.height) / u64::from(scaled_height),
                )
                .map_err(|_| CoreError::DimensionOverflow)?;
                let source_index =
                    pixel_offset(source.width, bounds.x + source_x, bounds.y + source_y)?;
                let target_index = pixel_offset(
                    settings.width,
                    u32::try_from(target_x).map_err(|_| CoreError::DimensionOverflow)?,
                    u32::try_from(target_y).map_err(|_| CoreError::DimensionOverflow)?,
                )?;
                pixels[target_index] = source.pixels[source_index];
            }
        }
    }

    let mut result = IndexedRaster {
        schema: source.schema.clone(),
        width: settings.width,
        height: settings.height,
        palette: source.palette.clone(),
        pixels,
        pivot: Some([
            i32::try_from(settings.width / 2).map_err(|_| CoreError::DimensionOverflow)?,
            i32::try_from(settings.height / 2).map_err(|_| CoreError::DimensionOverflow)?,
        ]),
        metadata: source.metadata.clone(),
    };
    result.metadata.insert(
        "canvas_composition".to_owned(),
        format!(
            "{}x{}:{}%@{},{}",
            settings.width,
            settings.height,
            settings.scale_percent,
            settings.offset_x,
            settings.offset_y
        ),
    );
    result.validate()?;
    Ok(result)
}

fn scaled_dimension(value: u32, percent: u16) -> Result<u32, CoreError> {
    let scaled = (u64::from(value) * u64::from(percent) + 50) / 100;
    u32::try_from(scaled.max(1)).map_err(|_| CoreError::DimensionOverflow)
}

#[derive(Clone, Copy)]
struct Bounds {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn visible_bounds(raster: &IndexedRaster) -> Option<Bounds> {
    let mut left = raster.width;
    let mut top = raster.height;
    let mut right = 0;
    let mut bottom = 0;
    let mut found = false;
    for y in 0..raster.height {
        for x in 0..raster.width {
            let index =
                usize::try_from(u64::from(y) * u64::from(raster.width) + u64::from(x)).ok()?;
            if raster.pixels[index] != raster.palette.transparent_index {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x);
                bottom = bottom.max(y);
                found = true;
            }
        }
    }
    if !found {
        return None;
    }
    Some(Bounds {
        x: left,
        y: top,
        width: right - left + 1,
        height: bottom - top + 1,
    })
}

fn pixel_offset(width: u32, x: u32, y: u32) -> Result<usize, CoreError> {
    usize::try_from(u64::from(y) * u64::from(width) + u64::from(x))
        .map_err(|_| CoreError::DimensionOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PALETTE_SCHEMA, Palette, RASTER_SCHEMA};

    fn raster() -> IndexedRaster {
        IndexedRaster {
            schema: RASTER_SCHEMA.to_owned(),
            width: 4,
            height: 4,
            palette: Palette::new("test", 0, vec![[0, 0, 0, 0], [255, 0, 0, 255]]),
            pixels: vec![0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0],
            pivot: None,
            metadata: std::collections::BTreeMap::default(),
        }
    }

    #[test]
    fn composes_and_clips_without_changing_palette() {
        let source = raster();
        let result = compose_canvas(
            &source,
            CanvasSettings {
                width: 3,
                height: 3,
                scale_percent: 100,
                offset_x: 2,
                offset_y: 1,
            },
        )
        .expect("compose");

        assert_eq!(result.palette, source.palette);
        assert_eq!(result.pixels, vec![0, 0, 1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(result.metadata["canvas_composition"], "3x3:100%@2,1");
        assert_eq!(PALETTE_SCHEMA, result.palette.schema);
    }

    #[test]
    fn composes_a_fully_transparent_frame_without_panicking() {
        let mut source = raster();
        source.pixels.fill(source.palette.transparent_index);
        let result = compose_canvas(
            &source,
            CanvasSettings {
                width: 3,
                height: 2,
                scale_percent: 100,
                offset_x: 0,
                offset_y: 0,
            },
        )
        .expect("compose blank frame");
        assert_eq!(result.pixels, vec![source.palette.transparent_index; 6]);
        assert_eq!(result.pivot, Some([1, 1]));
    }
}
