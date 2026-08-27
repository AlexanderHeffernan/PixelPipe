use std::collections::VecDeque;

use super::model::ComponentExpectation;
use crate::{CoreError, IndexedRaster};

/// Counts four-connected visible components and enforces an inclusive range.
///
/// # Errors
///
/// Returns a [`CoreError`] when dimensions overflow or the count is outside the
/// expected range.
pub fn validate_component_expectation(
    raster: &IndexedRaster,
    expectation: ComponentExpectation,
) -> Result<u16, CoreError> {
    let count = component_count(raster)?;
    if count < expectation.min || count > expectation.max {
        return Err(CoreError::ComponentCount {
            min: expectation.min,
            max: expectation.max,
            actual: count,
        });
    }
    Ok(count)
}

fn component_count(raster: &IndexedRaster) -> Result<u16, CoreError> {
    let mut visited = vec![false; raster.pixels.len()];
    let mut count = 0_u16;
    let mut queue = VecDeque::new();
    for offset in 0..raster.pixels.len() {
        if visited[offset] || raster.pixels[offset] == raster.palette.transparent_index {
            continue;
        }
        count = count.checked_add(1).ok_or(CoreError::DimensionOverflow)?;
        visited[offset] = true;
        queue.push_back(offset);
        while let Some(current) = queue.pop_front() {
            let x = current
                % usize::try_from(raster.width).map_err(|_| CoreError::DimensionOverflow)?;
            let y = current
                / usize::try_from(raster.width).map_err(|_| CoreError::DimensionOverflow)?;
            for neighbor in component_neighbors(x, y, raster.width, raster.height) {
                let Some(neighbor) = neighbor else { continue };
                if !visited[neighbor] && raster.pixels[neighbor] != raster.palette.transparent_index
                {
                    visited[neighbor] = true;
                    queue.push_back(neighbor);
                }
            }
        }
    }
    Ok(count)
}

fn component_neighbors(x: usize, y: usize, width: u32, height: u32) -> [Option<usize>; 4] {
    let width = usize::try_from(width).unwrap_or(0);
    let height = usize::try_from(height).unwrap_or(0);
    [
        (x > 0).then(|| y * width + x - 1),
        (x + 1 < width).then(|| y * width + x + 1),
        (y > 0).then(|| (y - 1) * width + x),
        (y + 1 < height).then(|| (y + 1) * width + x),
    ]
}
