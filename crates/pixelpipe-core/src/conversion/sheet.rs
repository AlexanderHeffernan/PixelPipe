use super::{
    components::validate_component_expectation,
    frame::{
        available_size, conversion_checks, conversion_metadata, fit_scale, prepare_frame,
        render_frame, subject_scale, validate_settings,
    },
    image::{pixel_count, source_offset, validate_source},
    model::{ConversionResult, RgbaImage, SheetSettings},
};
use crate::{CoreError, IndexedRaster, Palette, RASTER_SCHEMA};

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
            frames.push(prepare_frame(
                &cell,
                palette,
                &settings.frame.backdrop,
                settings.frame.color_treatment,
                settings.frame.color_adjustments,
            )?);
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
    let scale = subject_scale(
        fit_scale(max_width, max_height, available.0, available.1),
        settings.frame.subject_scale_percent,
    );
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
        validate_component_expectation(&raster, settings.frame.components)?;
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
