use std::collections::{BTreeMap, VecDeque};

use super::{
    image::{pixel_count, source_offset},
    model::ComponentExpectation,
};
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

/// Enforces the same four-connected component range independently per sheet frame.
///
/// # Errors
///
/// Returns a [`CoreError`] when the grid is invalid or any frame count falls
/// outside the expected range.
pub fn validate_sheet_component_expectation(
    raster: &IndexedRaster,
    columns: u16,
    rows: u16,
    expectation: ComponentExpectation,
) -> Result<Vec<u16>, CoreError> {
    let columns = u32::from(columns);
    let rows = u32::from(rows);
    if columns == 0
        || rows == 0
        || !raster.width.is_multiple_of(columns)
        || !raster.height.is_multiple_of(rows)
    {
        return Err(CoreError::InvalidSheetGrid);
    }
    let width = raster.width / columns;
    let height = raster.height / rows;
    let mut counts = Vec::with_capacity(
        usize::try_from(columns * rows).map_err(|_| CoreError::DimensionOverflow)?,
    );
    for row in 0..rows {
        for column in 0..columns {
            let mut pixels = Vec::with_capacity(pixel_count(width, height)?);
            for y in 0..height {
                for x in 0..width {
                    pixels.push(
                        raster.pixels
                            [source_offset(raster.width, column * width + x, row * height + y)?],
                    );
                }
            }
            let frame = IndexedRaster {
                schema: raster.schema.clone(),
                width,
                height,
                palette: raster.palette.clone(),
                pixels,
                pivot: None,
                metadata: BTreeMap::new(),
            };
            counts.push(validate_component_expectation(&frame, expectation)?);
        }
    }
    Ok(counts)
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
