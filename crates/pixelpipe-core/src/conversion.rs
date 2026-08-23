use std::{collections::VecDeque, io::Cursor};

use png::{ColorType, Transformations};
use serde::{Deserialize, Serialize};

use crate::{CoreError, IndexedRaster, Palette, RASTER_SCHEMA, ValidationCheck};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 4]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum BackdropPolicy {
    Alpha {
        alpha_threshold: u8,
    },
    BorderConnected {
        color: [u8; 3],
        tolerance: u8,
        alpha_threshold: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Registration {
    Center,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentExpectation {
    pub min: u16,
    pub max: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversionSettings {
    pub width: u32,
    pub height: u32,
    pub margin: u16,
    pub coverage_percent: u8,
    pub backdrop: BackdropPolicy,
    pub registration: Registration,
    pub components: ComponentExpectation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SheetSettings {
    pub columns: u16,
    pub rows: u16,
    pub frame: ConversionSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionResult {
    pub raster: IndexedRaster,
    pub checks: Vec<ValidationCheck>,
}

#[derive(Debug, Clone)]
struct PreparedFrame {
    width: u32,
    height: u32,
    pixels: Vec<Option<u8>>,
    source_bounds: Bounds,
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Copy)]
struct Scale {
    numerator: u32,
    denominator: u32,
}

/// Decodes a PNG into deterministic row-major RGBA8 pixels.
///
/// # Errors
///
/// Returns a [`CoreError`] for malformed PNG data, unsupported decoder output,
/// or inconsistent dimensions.
pub fn decode_rgba_png(bytes: &[u8]) -> Result<RgbaImage, CoreError> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer)?;
    let data = &buffer[..info.buffer_size()];
    let pixels = match info.color_type {
        ColorType::Rgba => data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
            .collect(),
        ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        ColorType::Grayscale => data
            .iter()
            .map(|value| [*value, *value, *value, 255])
            .collect(),
        ColorType::GrayscaleAlpha => data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]])
            .collect(),
        ColorType::Indexed => return Err(CoreError::InvalidSourceImage),
    };
    let image = RgbaImage {
        width: info.width,
        height: info.height,
        pixels,
    };
    validate_source(&image)?;
    Ok(image)
}

/// Converts one RGBA reference into a registered indexed raster.
///
/// # Errors
///
/// Returns a [`CoreError`] when the source, settings, palette, or resulting
/// component structure is invalid.
pub fn convert_reference(
    source: &RgbaImage,
    palette: &Palette,
    settings: &ConversionSettings,
) -> Result<ConversionResult, CoreError> {
    validate_settings(settings)?;
    palette.validate()?;
    let frame = prepare_frame(source, palette, &settings.backdrop)?;
    let available = available_size(settings)?;
    let scale = fit_scale(frame.width, frame.height, available.0, available.1);
    let (raster, placement) = render_frame(&frame, palette, settings, scale)?;
    finish_conversion(raster, settings, &[frame.source_bounds], &[placement])
}

/// Converts a regular RGBA sheet with shared frame scale and registration.
///
/// # Errors
///
/// Returns a [`CoreError`] when the source grid, frame settings, palette, or any
/// resulting frame structure is invalid.
pub fn convert_sheet(
    source: &RgbaImage,
    palette: &Palette,
    settings: &SheetSettings,
) -> Result<ConversionResult, CoreError> {
    validate_source(source)?;
    validate_settings(&settings.frame)?;
    palette.validate()?;
    let columns = u32::from(settings.columns);
    let rows = u32::from(settings.rows);
    if columns == 0
        || rows == 0
        || !source.width.is_multiple_of(columns)
        || !source.height.is_multiple_of(rows)
    {
        return Err(CoreError::InvalidSheetGrid);
    }
    let source_frame_width = source.width / columns;
    let source_frame_height = source.height / rows;
    let mut frames = Vec::with_capacity(usize::from(settings.columns) * usize::from(settings.rows));
    for row in 0..rows {
        for column in 0..columns {
            let cell = extract_cell(
                source,
                column * source_frame_width,
                row * source_frame_height,
                source_frame_width,
                source_frame_height,
            )?;
            frames.push(prepare_frame(&cell, palette, &settings.frame.backdrop)?);
        }
    }

    let max_width = frames
        .iter()
        .map(|frame| frame.width)
        .max()
        .ok_or(CoreError::EmptySource)?;
    let max_height = frames
        .iter()
        .map(|frame| frame.height)
        .max()
        .ok_or(CoreError::EmptySource)?;
    let available = available_size(&settings.frame)?;
    let scale = fit_scale(max_width, max_height, available.0, available.1);
    let sheet_width = settings
        .frame
        .width
        .checked_mul(columns)
        .ok_or(CoreError::DimensionOverflow)?;
    let sheet_height = settings
        .frame
        .height
        .checked_mul(rows)
        .ok_or(CoreError::DimensionOverflow)?;
    if sheet_width > 8192 || sheet_height > 8192 {
        return Err(CoreError::InvalidDimensions);
    }
    let sheet_len = pixel_count(sheet_width, sheet_height)?;
    let mut sheet_pixels = vec![palette.transparent_index; sheet_len];
    let mut source_bounds = Vec::with_capacity(frames.len());
    let mut placements = Vec::with_capacity(frames.len());

    for (index, frame) in frames.iter().enumerate() {
        let (raster, placement) = render_frame(frame, palette, &settings.frame, scale)?;
        validate_components(&raster, settings.frame.components)?;
        let column = u32::try_from(index).map_err(|_| CoreError::DimensionOverflow)? % columns;
        let row = u32::try_from(index).map_err(|_| CoreError::DimensionOverflow)? / columns;
        copy_frame(
            &raster,
            &mut sheet_pixels,
            sheet_width,
            column * settings.frame.width,
            row * settings.frame.height,
        )?;
        source_bounds.push(frame.source_bounds);
        placements.push(placement);
    }

    let mut metadata = conversion_metadata(&source_bounds, &placements);
    metadata.insert("sheet_columns".to_owned(), settings.columns.to_string());
    metadata.insert("sheet_rows".to_owned(), settings.rows.to_string());
    metadata.insert("frame_width".to_owned(), settings.frame.width.to_string());
    metadata.insert("frame_height".to_owned(), settings.frame.height.to_string());
    let raster = IndexedRaster {
        schema: RASTER_SCHEMA.to_owned(),
        width: sheet_width,
        height: sheet_height,
        palette: palette.clone(),
        pixels: sheet_pixels,
        pivot: None,
        metadata,
    };
    raster.validate()?;
    Ok(ConversionResult {
        raster,
        checks: conversion_checks(&source_bounds, &placements, frames.len()),
    })
}

fn prepare_frame(
    source: &RgbaImage,
    palette: &Palette,
    backdrop: &BackdropPolicy,
) -> Result<PreparedFrame, CoreError> {
    validate_source(source)?;
    let cleaned = clean_backdrop(source, backdrop);
    let bounds = visible_bounds(&cleaned).ok_or(CoreError::EmptySource)?;
    let capacity = pixel_count(bounds.width, bounds.height)?;
    let mut pixels = Vec::with_capacity(capacity);
    for y in bounds.y..bounds.y + bounds.height {
        for x in bounds.x..bounds.x + bounds.width {
            let pixel = cleaned.pixels[source_offset(cleaned.width, x, y)?];
            pixels.push((pixel[3] > 0).then(|| nearest_palette_index(pixel, palette)));
        }
    }
    Ok(PreparedFrame {
        width: bounds.width,
        height: bounds.height,
        pixels,
        source_bounds: bounds,
    })
}

fn clean_backdrop(source: &RgbaImage, policy: &BackdropPolicy) -> RgbaImage {
    let mut cleaned = source.clone();
    let alpha_threshold = match policy {
        BackdropPolicy::Alpha { alpha_threshold }
        | BackdropPolicy::BorderConnected {
            alpha_threshold, ..
        } => *alpha_threshold,
    };
    for pixel in &mut cleaned.pixels {
        if pixel[3] <= alpha_threshold {
            pixel[3] = 0;
        }
    }
    if let BackdropPolicy::BorderConnected {
        color, tolerance, ..
    } = policy
    {
        remove_border_connected(&mut cleaned, *color, *tolerance);
    }
    cleaned
}

fn remove_border_connected(image: &mut RgbaImage, color: [u8; 3], tolerance: u8) {
    let Ok(length) = usize::try_from(image.width)
        .and_then(|width| usize::try_from(image.height).map(|height| width.saturating_mul(height)))
    else {
        return;
    };
    let mut queued = vec![false; length];
    let mut queue = VecDeque::new();
    for x in 0..image.width {
        enqueue_background(image, x, 0, color, tolerance, &mut queued, &mut queue);
        if image.height > 1 {
            enqueue_background(
                image,
                x,
                image.height - 1,
                color,
                tolerance,
                &mut queued,
                &mut queue,
            );
        }
    }
    for y in 0..image.height {
        enqueue_background(image, 0, y, color, tolerance, &mut queued, &mut queue);
        if image.width > 1 {
            enqueue_background(
                image,
                image.width - 1,
                y,
                color,
                tolerance,
                &mut queued,
                &mut queue,
            );
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let Ok(offset) = source_offset(image.width, x, y) else {
            continue;
        };
        image.pixels[offset][3] = 0;
        if x > 0 {
            enqueue_background(image, x - 1, y, color, tolerance, &mut queued, &mut queue);
        }
        if x + 1 < image.width {
            enqueue_background(image, x + 1, y, color, tolerance, &mut queued, &mut queue);
        }
        if y > 0 {
            enqueue_background(image, x, y - 1, color, tolerance, &mut queued, &mut queue);
        }
        if y + 1 < image.height {
            enqueue_background(image, x, y + 1, color, tolerance, &mut queued, &mut queue);
        }
    }
}

fn enqueue_background(
    image: &RgbaImage,
    x: u32,
    y: u32,
    color: [u8; 3],
    tolerance: u8,
    queued: &mut [bool],
    queue: &mut VecDeque<(u32, u32)>,
) {
    let Ok(offset) = source_offset(image.width, x, y) else {
        return;
    };
    if queued[offset] {
        return;
    }
    let pixel = image.pixels[offset];
    let matches = pixel[3] > 0
        && pixel[..3]
            .iter()
            .zip(color)
            .all(|(actual, expected)| actual.abs_diff(expected) <= tolerance);
    if matches {
        queued[offset] = true;
        queue.push_back((x, y));
    }
}

fn visible_bounds(image: &RgbaImage) -> Option<Bounds> {
    let mut min_x = image.width;
    let mut min_y = image.height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;
    for y in 0..image.height {
        for x in 0..image.width {
            if image.pixels[source_offset(image.width, x, y).ok()?][3] > 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found = true;
            }
        }
    }
    found.then_some(Bounds {
        x: min_x,
        y: min_y,
        width: max_x - min_x + 1,
        height: max_y - min_y + 1,
    })
}

fn fit_scale(source_width: u32, source_height: u32, width: u32, height: u32) -> Scale {
    if u64::from(source_width) * u64::from(height) <= u64::from(source_height) * u64::from(width) {
        Scale {
            numerator: height,
            denominator: source_height,
        }
    } else {
        Scale {
            numerator: width,
            denominator: source_width,
        }
    }
}

fn render_frame(
    frame: &PreparedFrame,
    palette: &Palette,
    settings: &ConversionSettings,
    scale: Scale,
) -> Result<(IndexedRaster, Bounds), CoreError> {
    let destination_width = scaled_dimension(frame.width, scale)?;
    let destination_height = scaled_dimension(frame.height, scale)?;
    let margin = u32::from(settings.margin);
    let x = (settings.width - destination_width) / 2;
    let y = match settings.registration {
        Registration::Center => (settings.height - destination_height) / 2,
        Registration::Bottom => settings.height - margin - destination_height,
    };
    let mut pixels = vec![palette.transparent_index; pixel_count(settings.width, settings.height)?];
    for destination_y in 0..destination_height {
        for destination_x in 0..destination_width {
            let index = dominant_cell(
                frame,
                destination_x,
                destination_y,
                destination_width,
                destination_height,
                settings.coverage_percent,
                palette.transparent_index,
            )?;
            let target_x = x + destination_x;
            let target_y = y + destination_y;
            let offset = source_offset(settings.width, target_x, target_y)?;
            pixels[offset] = index;
        }
    }
    let placement = Bounds {
        x,
        y,
        width: destination_width,
        height: destination_height,
    };
    let raster = IndexedRaster {
        schema: RASTER_SCHEMA.to_owned(),
        width: settings.width,
        height: settings.height,
        palette: palette.clone(),
        pixels,
        pivot: Some([
            i32::try_from(settings.width / 2).map_err(|_| CoreError::DimensionOverflow)?,
            i32::try_from(match settings.registration {
                Registration::Center => settings.height / 2,
                Registration::Bottom => settings.height - margin,
            })
            .map_err(|_| CoreError::DimensionOverflow)?,
        ]),
        metadata: conversion_metadata(&[frame.source_bounds], &[placement]),
    };
    raster.validate()?;
    Ok((raster, placement))
}

fn dominant_cell(
    frame: &PreparedFrame,
    destination_x: u32,
    destination_y: u32,
    destination_width: u32,
    destination_height: u32,
    coverage_percent: u8,
    transparent_index: u8,
) -> Result<u8, CoreError> {
    let start_x = destination_x * frame.width / destination_width;
    let end_x = ((destination_x + 1) * frame.width).div_ceil(destination_width);
    let start_y = destination_y * frame.height / destination_height;
    let end_y = ((destination_y + 1) * frame.height).div_ceil(destination_height);
    let mut counts = [0_u32; 256];
    let mut visible = 0_u32;
    let total = (end_x - start_x) * (end_y - start_y);
    for source_y in start_y..end_y {
        for source_x in start_x..end_x {
            let offset = source_offset(frame.width, source_x, source_y)?;
            if let Some(index) = frame.pixels[offset] {
                counts[usize::from(index)] += 1;
                visible += 1;
            }
        }
    }
    if visible * 100 < total * u32::from(coverage_percent) {
        return Ok(transparent_index);
    }
    let (index, count) = counts
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != usize::from(transparent_index))
        .max_by(|(left_index, left_count), (right_index, right_count)| {
            left_count
                .cmp(right_count)
                .then_with(|| right_index.cmp(left_index))
        })
        .ok_or(CoreError::EmptySource)?;
    if *count == 0 {
        return Err(CoreError::EmptySource);
    }
    u8::try_from(index).map_err(|_| CoreError::DimensionOverflow)
}

fn nearest_palette_index(pixel: [u8; 4], palette: &Palette) -> u8 {
    palette
        .colors
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != usize::from(palette.transparent_index))
        .min_by_key(|(index, color)| {
            let red = i32::from(pixel[0]) - i32::from(color[0]);
            let green = i32::from(pixel[1]) - i32::from(color[1]);
            let blue = i32::from(pixel[2]) - i32::from(color[2]);
            (red * red + green * green + blue * blue, *index)
        })
        .map_or(palette.transparent_index, |(index, _)| {
            u8::try_from(index).expect("validated palettes contain at most 256 colors")
        })
}

fn validate_components(
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
        (x > 0).then_some(y * width + x - 1),
        (x + 1 < width).then_some(y * width + x + 1),
        (y > 0).then_some((y - 1) * width + x),
        (y + 1 < height).then_some((y + 1) * width + x),
    ]
}

fn finish_conversion(
    mut raster: IndexedRaster,
    settings: &ConversionSettings,
    source_bounds: &[Bounds],
    placements: &[Bounds],
) -> Result<ConversionResult, CoreError> {
    let components = validate_components(&raster, settings.components)?;
    raster.metadata = conversion_metadata(source_bounds, placements);
    Ok(ConversionResult {
        raster,
        checks: vec![
            ValidationCheck {
                name: "source_bounds".to_owned(),
                passed: true,
                detail: bounds_detail(source_bounds),
            },
            ValidationCheck {
                name: "registration".to_owned(),
                passed: true,
                detail: bounds_detail(placements),
            },
            ValidationCheck {
                name: "connected_components".to_owned(),
                passed: true,
                detail: components.to_string(),
            },
        ],
    })
}

fn conversion_checks(
    source_bounds: &[Bounds],
    placements: &[Bounds],
    frame_count: usize,
) -> Vec<ValidationCheck> {
    vec![
        ValidationCheck {
            name: "source_bounds".to_owned(),
            passed: true,
            detail: bounds_detail(source_bounds),
        },
        ValidationCheck {
            name: "registration".to_owned(),
            passed: true,
            detail: bounds_detail(placements),
        },
        ValidationCheck {
            name: "sheet_frames".to_owned(),
            passed: true,
            detail: frame_count.to_string(),
        },
    ]
}

fn conversion_metadata(
    source_bounds: &[Bounds],
    placements: &[Bounds],
) -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::from([
        ("source_bounds".to_owned(), bounds_detail(source_bounds)),
        ("placements".to_owned(), bounds_detail(placements)),
    ])
}

fn bounds_detail(bounds: &[Bounds]) -> String {
    bounds
        .iter()
        .map(|bounds| {
            format!(
                "{},{},{},{}",
                bounds.x, bounds.y, bounds.width, bounds.height
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn validate_source(source: &RgbaImage) -> Result<(), CoreError> {
    if source.width == 0
        || source.height == 0
        || source.width > 16384
        || source.height > 16384
        || source.pixels.len() != pixel_count(source.width, source.height)?
    {
        return Err(CoreError::InvalidSourceImage);
    }
    Ok(())
}

fn validate_settings(settings: &ConversionSettings) -> Result<(), CoreError> {
    if settings.width == 0
        || settings.height == 0
        || settings.width > 8192
        || settings.height > 8192
    {
        return Err(CoreError::InvalidDimensions);
    }
    if settings.coverage_percent == 0 || settings.coverage_percent > 100 {
        return Err(CoreError::InvalidCoverage);
    }
    if settings.components.min == 0 || settings.components.min > settings.components.max {
        return Err(CoreError::ComponentCount {
            min: settings.components.min,
            max: settings.components.max,
            actual: 0,
        });
    }
    available_size(settings)?;
    Ok(())
}

fn available_size(settings: &ConversionSettings) -> Result<(u32, u32), CoreError> {
    let doubled_margin = u32::from(settings.margin)
        .checked_mul(2)
        .ok_or(CoreError::InvalidMargin)?;
    let width = settings
        .width
        .checked_sub(doubled_margin)
        .ok_or(CoreError::InvalidMargin)?;
    let height = settings
        .height
        .checked_sub(doubled_margin)
        .ok_or(CoreError::InvalidMargin)?;
    if width == 0 || height == 0 {
        return Err(CoreError::InvalidMargin);
    }
    Ok((width, height))
}

fn scaled_dimension(value: u32, scale: Scale) -> Result<u32, CoreError> {
    let scaled = u64::from(value) * u64::from(scale.numerator) / u64::from(scale.denominator);
    u32::try_from(scaled.max(1)).map_err(|_| CoreError::DimensionOverflow)
}

fn extract_cell(
    source: &RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<RgbaImage, CoreError> {
    let mut pixels = Vec::with_capacity(pixel_count(width, height)?);
    for source_y in y..y + height {
        for source_x in x..x + width {
            pixels.push(source.pixels[source_offset(source.width, source_x, source_y)?]);
        }
    }
    Ok(RgbaImage {
        width,
        height,
        pixels,
    })
}

fn copy_frame(
    frame: &IndexedRaster,
    sheet: &mut [u8],
    sheet_width: u32,
    x: u32,
    y: u32,
) -> Result<(), CoreError> {
    for frame_y in 0..frame.height {
        for frame_x in 0..frame.width {
            let source = source_offset(frame.width, frame_x, frame_y)?;
            let target = source_offset(sheet_width, x + frame_x, y + frame_y)?;
            sheet[target] = frame.pixels[source];
        }
    }
    Ok(())
}

fn source_offset(width: u32, x: u32, y: u32) -> Result<usize, CoreError> {
    let offset = u64::from(y)
        .checked_mul(u64::from(width))
        .and_then(|row| row.checked_add(u64::from(x)))
        .ok_or(CoreError::DimensionOverflow)?;
    usize::try_from(offset).map_err(|_| CoreError::DimensionOverflow)
}

fn pixel_count(width: u32, height: u32) -> Result<usize, CoreError> {
    usize::try_from(u64::from(width) * u64::from(height)).map_err(|_| CoreError::DimensionOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PALETTE_SCHEMA, render, sha256_hex, stable_json};

    #[derive(Deserialize)]
    struct RgbaFixture {
        width: u32,
        height: u32,
        pixels: Vec<Vec<[u8; 4]>>,
    }

    fn palette() -> Palette {
        Palette {
            schema: PALETTE_SCHEMA.to_owned(),
            name: "synthetic".to_owned(),
            transparent_index: 0,
            colors: vec![
                [0, 0, 0, 0],
                [24, 24, 28, 255],
                [220, 60, 40, 255],
                [248, 220, 96, 255],
                [240, 240, 240, 255],
            ],
        }
    }

    fn settings(registration: Registration) -> ConversionSettings {
        ConversionSettings {
            width: 8,
            height: 8,
            margin: 1,
            coverage_percent: 25,
            backdrop: BackdropPolicy::Alpha { alpha_threshold: 0 },
            registration,
            components: ComponentExpectation { min: 1, max: 1 },
        }
    }

    #[test]
    fn decodes_rgba_png() {
        let pixels = [255, 0, 0, 255, 0, 255, 0, 128];
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
            encoder.set_color(ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("PNG header");
            writer.write_image_data(&pixels).expect("PNG pixels");
            writer.finish().expect("PNG finish");
        }
        let decoded = decode_rgba_png(&bytes).expect("decode PNG");
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.pixels, vec![[255, 0, 0, 255], [0, 255, 0, 128]]);
    }

    #[test]
    fn border_cleanup_preserves_enclosed_dark_subject_pixels() {
        let mut pixels = vec![[0, 0, 0, 255]; 25];
        for y in 1..=3 {
            for x in 1..=3 {
                pixels[y * 5 + x] = [240, 240, 240, 255];
            }
        }
        pixels[2 * 5 + 2] = [20, 20, 20, 255];
        let source = RgbaImage {
            width: 5,
            height: 5,
            pixels,
        };
        let mut settings = settings(Registration::Center);
        settings.backdrop = BackdropPolicy::BorderConnected {
            color: [0, 0, 0],
            tolerance: 4,
            alpha_threshold: 0,
        };
        let converted = convert_reference(&source, &palette(), &settings).expect("convert");

        assert!(converted.raster.pixels.contains(&1));
        assert!(converted.raster.pixels.contains(&4));
    }

    #[test]
    fn bottom_registration_uses_shared_baseline() {
        let source = RgbaImage {
            width: 2,
            height: 3,
            pixels: vec![[220, 60, 40, 255]; 6],
        };
        let converted = convert_reference(&source, &palette(), &settings(Registration::Bottom))
            .expect("convert");
        let bounds = visible_bounds_from_raster(&converted.raster);
        assert_eq!(bounds.y + bounds.height, 7);
        assert_eq!(converted.raster.pivot, Some([4, 7]));
    }

    #[test]
    fn dominant_ties_choose_lower_palette_index() {
        let source = RgbaImage {
            width: 2,
            height: 1,
            pixels: vec![[220, 60, 40, 255], [248, 220, 96, 255]],
        };
        let mut settings = settings(Registration::Center);
        settings.width = 3;
        settings.height = 3;
        settings.margin = 1;
        let converted = convert_reference(&source, &palette(), &settings).expect("convert");
        assert!(converted.raster.pixels.contains(&2));
        assert!(!converted.raster.pixels.contains(&3));
    }

    #[test]
    fn palette_distance_ties_choose_lower_palette_index() {
        let palette = Palette::new("tie", 0, vec![[0, 0, 0, 0], [0, 0, 0, 255], [2, 0, 0, 255]]);
        let source = RgbaImage {
            width: 1,
            height: 1,
            pixels: vec![[1, 0, 0, 255]],
        };
        let mut settings = settings(Registration::Center);
        settings.width = 3;
        settings.height = 3;
        settings.margin = 1;
        let converted = convert_reference(&source, &palette, &settings).expect("convert");
        assert!(converted.raster.pixels.contains(&1));
        assert!(!converted.raster.pixels.contains(&2));
    }

    #[test]
    fn sheet_uses_shared_scale_and_bottom_registration() {
        let source = fixture_image(include_bytes!("../../../fixtures/m2/sheet.rgba.json"));
        let palette: Palette =
            serde_json::from_slice(include_bytes!("../../../fixtures/m2/palette.json"))
                .expect("fixture palette");
        let sheet: SheetSettings =
            serde_json::from_slice(include_bytes!("../../../fixtures/m2/sheet.settings.json"))
                .expect("sheet settings");
        let converted = convert_sheet(&source, &palette, &sheet).expect("sheet conversion");
        assert_eq!((converted.raster.width, converted.raster.height), (16, 8));
        let rendered = render(&converted.raster, 8).expect("sheet render");
        assert_eq!(
            rendered.native_png,
            render(&converted.raster, 8).expect("repeat").native_png
        );
        assert_eq!(
            sha256_hex(&rendered.native_png),
            "fce359e660986518efae60130e4227af5b4dc8c0d24070bc1ee91ec8455f1132"
        );
        assert_eq!(
            sha256_hex(&rendered.preview_png),
            "871d96fabd73d3c265f1c0ff05067c1a37d545434678fe0da7241c52d93cbe9a"
        );
    }

    #[test]
    fn synthetic_fixture_matches_golden_hashes() {
        let source = fixture_image(include_bytes!("../../../fixtures/m2/reference.rgba.json"));
        let palette: Palette =
            serde_json::from_slice(include_bytes!("../../../fixtures/m2/palette.json"))
                .expect("fixture palette");
        let settings: ConversionSettings = serde_json::from_slice(include_bytes!(
            "../../../fixtures/m2/reference.settings.json"
        ))
        .expect("fixture settings");
        let converted =
            convert_reference(&source, &palette, &settings).expect("fixture conversion");
        let raster_hash = sha256_hex(&stable_json(&converted.raster).expect("canonical raster"));
        let rendered = render(&converted.raster, 8).expect("fixture render");

        assert_eq!(
            raster_hash,
            "19af2370aeee6415ceec8e2a16aad78d746672a7e202252b72c5aeb37b597b67"
        );
        assert_eq!(
            sha256_hex(&rendered.native_png),
            "9e1345c3b488327bb6839c177830c0f50f5121b21450de9eda42e1d923e4721e"
        );
        assert_eq!(
            sha256_hex(&rendered.preview_png),
            "22d22ac34a6764531e972636f4c0d10e17c01e9752ad01b3b2e9c4a44305f201"
        );
    }

    fn fixture_image(bytes: &[u8]) -> RgbaImage {
        let fixture: RgbaFixture = serde_json::from_slice(bytes).expect("RGBA fixture");
        RgbaImage {
            width: fixture.width,
            height: fixture.height,
            pixels: fixture.pixels.into_iter().flatten().collect(),
        }
    }

    fn visible_bounds_from_raster(raster: &IndexedRaster) -> Bounds {
        let image = RgbaImage {
            width: raster.width,
            height: raster.height,
            pixels: raster
                .pixels
                .iter()
                .map(|index| raster.palette.colors[usize::from(*index)])
                .collect(),
        };
        visible_bounds(&image).expect("visible raster")
    }
}
