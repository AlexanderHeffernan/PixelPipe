use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::{CoreError, IndexedRaster};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedPart {
    pub source: [u32; 4],
    pub pixels: u32,
    pub suggested_pivot: [i32; 2],
}

/// Finds 4-connected opaque components in stable top-to-bottom, left-to-right order.
///
/// # Errors
/// Returns [`CoreError`] when the raster is invalid or dimensions overflow the host.
pub fn detect_parts(
    raster: &IndexedRaster,
    minimum_pixels: u32,
) -> Result<Vec<DetectedPart>, CoreError> {
    raster.validate()?;
    let width = usize::try_from(raster.width).map_err(|_| CoreError::DimensionOverflow)?;
    let height = usize::try_from(raster.height).map_err(|_| CoreError::DimensionOverflow)?;
    let transparent = raster.palette.transparent_index;
    let mut visited = vec![false; raster.pixels.len()];
    let mut parts = Vec::new();

    for start in 0..raster.pixels.len() {
        if visited[start] || raster.pixels[start] == transparent {
            continue;
        }
        visited[start] = true;
        let mut queue = VecDeque::from([start]);
        let (mut left, mut right) = (start % width, start % width);
        let (mut top, mut bottom) = (start / width, start / width);
        let mut count = 0_u32;
        while let Some(index) = queue.pop_front() {
            count = count.checked_add(1).ok_or(CoreError::DimensionOverflow)?;
            let x = index % width;
            let y = index / width;
            left = left.min(x);
            right = right.max(x);
            top = top.min(y);
            bottom = bottom.max(y);
            let neighbours = [
                x.checked_sub(1).map(|next| y * width + next),
                (x + 1 < width).then_some(y * width + x + 1),
                y.checked_sub(1).map(|next| next * width + x),
                (y + 1 < height).then_some((y + 1) * width + x),
            ];
            for neighbour in neighbours.into_iter().flatten() {
                if !visited[neighbour] && raster.pixels[neighbour] != transparent {
                    visited[neighbour] = true;
                    queue.push_back(neighbour);
                }
            }
        }
        if count >= minimum_pixels {
            let part_width = right - left + 1;
            let part_height = bottom - top + 1;
            parts.push(DetectedPart {
                source: [
                    u32::try_from(left).map_err(|_| CoreError::DimensionOverflow)?,
                    u32::try_from(top).map_err(|_| CoreError::DimensionOverflow)?,
                    u32::try_from(part_width).map_err(|_| CoreError::DimensionOverflow)?,
                    u32::try_from(part_height).map_err(|_| CoreError::DimensionOverflow)?,
                ],
                pixels: count,
                suggested_pivot: [
                    i32::try_from(part_width / 2).map_err(|_| CoreError::DimensionOverflow)?,
                    i32::try_from(part_height / 2).map_err(|_| CoreError::DimensionOverflow)?,
                ],
            });
        }
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{PALETTE_SCHEMA, Palette, RASTER_SCHEMA};

    use super::*;

    #[test]
    fn finds_stably_ordered_opaque_components() {
        let raster = IndexedRaster {
            schema: RASTER_SCHEMA.to_owned(),
            width: 5,
            height: 3,
            palette: Palette {
                schema: PALETTE_SCHEMA.to_owned(),
                name: "parts".to_owned(),
                transparent_index: 0,
                colors: vec![[0, 0, 0, 0], [255, 255, 255, 255]],
            },
            pixels: vec![1, 1, 0, 0, 1, 1, 0, 0, 0, 1, 0, 0, 1, 1, 0],
            pivot: None,
            metadata: BTreeMap::new(),
        };
        assert_eq!(
            detect_parts(&raster, 2).unwrap(),
            vec![
                DetectedPart {
                    source: [0, 0, 2, 2],
                    pixels: 3,
                    suggested_pivot: [1, 1],
                },
                DetectedPart {
                    source: [4, 0, 1, 2],
                    pixels: 2,
                    suggested_pivot: [0, 1],
                },
                DetectedPart {
                    source: [2, 2, 2, 1],
                    pixels: 2,
                    suggested_pivot: [1, 0],
                },
            ]
        );
    }
}
