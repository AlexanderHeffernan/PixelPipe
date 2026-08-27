use std::collections::BTreeMap;

use super::{
    backdrop::{clean_backdrop, visible_bounds},
    components::validate_component_expectation,
    image::{pixel_count, source_offset, validate_source},
    model::{
        BackdropPolicy, Bounds, ConversionResult, ConversionSettings, PreparedFrame, Registration,
        RgbaImage, Scale,
    },
};
use crate::{
    ColorAdjustments, ColorTreatment, CoreError, IndexedRaster, Palette, RASTER_SCHEMA,
    ValidationCheck,
};

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
    let frame = prepare_frame(
        source,
        palette,
        &settings.backdrop,
        settings.color_treatment,
        settings.color_adjustments,
    )?;
    let available = available_size(settings)?;
    let scale = subject_scale(
        fit_scale(frame.width, frame.height, available.0, available.1),
        settings.subject_scale_percent,
    );
    let (raster, placement) = render_frame(&frame, palette, settings, scale)?;
    finish_conversion(raster, settings, &[frame.source_bounds], &[placement])
}

pub(super) fn prepare_frame(
    source: &RgbaImage,
    palette: &Palette,
    backdrop: &BackdropPolicy,
    treatment: ColorTreatment,
    adjustments: ColorAdjustments,
) -> Result<PreparedFrame, CoreError> {
    validate_source(source)?;
    let cleaned = clean_backdrop(source, backdrop);
    let bounds = visible_bounds(&cleaned).ok_or(CoreError::EmptySource)?;
    let capacity = pixel_count(bounds.width, bounds.height)?;
    let mut pixels = Vec::with_capacity(capacity);
    for y in bounds.y..bounds.y + bounds.height {
        for x in bounds.x..bounds.x + bounds.width {
            let pixel = adjustments
                .apply(treatment.apply(cleaned.pixels[source_offset(cleaned.width, x, y)?]));
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

pub(super) fn available_size(settings: &ConversionSettings) -> Result<(u32, u32), CoreError> {
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

pub(super) fn fit_scale(source_width: u32, source_height: u32, width: u32, height: u32) -> Scale {
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

pub(super) fn subject_scale(scale: Scale, percent: u8) -> Scale {
    Scale {
        numerator: scale.numerator * u32::from(percent),
        denominator: scale.denominator * 100,
    }
}

pub(super) fn render_frame(
    frame: &PreparedFrame,
    palette: &Palette,
    settings: &ConversionSettings,
    scale: Scale,
) -> Result<(IndexedRaster, Bounds), CoreError> {
    let destination_width = scaled_dimension(frame.width, scale)?;
    let destination_height = scaled_dimension(frame.height, scale)?;
    let margin = i64::from(settings.margin);
    let centered_x = (i64::from(settings.width) - i64::from(destination_width)).div_euclid(2);
    let registered_y = match settings.registration {
        Registration::Top => margin,
        Registration::Center => {
            (i64::from(settings.height) - i64::from(destination_height)).div_euclid(2)
        }
        Registration::Bottom => i64::from(settings.height) - margin - i64::from(destination_height),
    };
    let x = centered_x + i64::from(settings.offset_x);
    let y = registered_y - i64::from(settings.offset_y);
    let mut pixels = vec![palette.transparent_index; pixel_count(settings.width, settings.height)?];
    for destination_y in 0..destination_height {
        for destination_x in 0..destination_width {
            let target_x = x + i64::from(destination_x);
            let target_y = y + i64::from(destination_y);
            if target_x < 0
                || target_y < 0
                || target_x >= i64::from(settings.width)
                || target_y >= i64::from(settings.height)
            {
                continue;
            }
            let index = dominant_cell(
                frame,
                destination_x,
                destination_y,
                destination_width,
                destination_height,
                settings.coverage_percent,
                palette.transparent_index,
            )?;
            let target_x = u32::try_from(target_x).map_err(|_| CoreError::DimensionOverflow)?;
            let target_y = u32::try_from(target_y).map_err(|_| CoreError::DimensionOverflow)?;
            let offset = source_offset(settings.width, target_x, target_y)?;
            pixels[offset] = index;
        }
    }
    let placement = clipped_placement(
        x,
        y,
        destination_width,
        destination_height,
        settings.width,
        settings.height,
    );
    let raster = IndexedRaster {
        schema: RASTER_SCHEMA.to_owned(),
        width: settings.width,
        height: settings.height,
        palette: palette.clone(),
        pixels,
        pivot: Some([
            i32::try_from(settings.width / 2).map_err(|_| CoreError::DimensionOverflow)?,
            i32::try_from(match settings.registration {
                Registration::Top => u32::from(settings.margin),
                Registration::Center => settings.height / 2,
                Registration::Bottom => settings.height - u32::from(settings.margin),
            })
            .map_err(|_| CoreError::DimensionOverflow)?,
        ]),
        metadata: conversion_metadata(&[frame.source_bounds], &[placement]),
    };
    raster.validate()?;
    Ok((raster, placement))
}

pub(super) fn conversion_checks(
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

pub(super) fn conversion_metadata(
    source_bounds: &[Bounds],
    placements: &[Bounds],
) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("source_bounds".to_owned(), bounds_detail(source_bounds)),
        ("placements".to_owned(), bounds_detail(placements)),
    ])
}

pub(super) fn validate_settings(settings: &ConversionSettings) -> Result<(), CoreError> {
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
    if !(25..=200).contains(&settings.subject_scale_percent) {
        return Err(CoreError::InvalidSubjectScale);
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

fn finish_conversion(
    mut raster: IndexedRaster,
    settings: &ConversionSettings,
    source_bounds: &[Bounds],
    placements: &[Bounds],
) -> Result<ConversionResult, CoreError> {
    let components = validate_component_expectation(&raster, settings.components)?;
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

fn clipped_placement(
    x: i64,
    y: i64,
    width: u32,
    height: u32,
    canvas_width: u32,
    canvas_height: u32,
) -> Bounds {
    let left = x.clamp(0, i64::from(canvas_width));
    let top = y.clamp(0, i64::from(canvas_height));
    let right = (x + i64::from(width)).clamp(0, i64::from(canvas_width));
    let bottom = (y + i64::from(height)).clamp(0, i64::from(canvas_height));
    Bounds {
        x: u32::try_from(left).expect("clipped x fits u32"),
        y: u32::try_from(top).expect("clipped y fits u32"),
        width: u32::try_from((right - left).max(0)).expect("clipped width fits u32"),
        height: u32::try_from((bottom - top).max(0)).expect("clipped height fits u32"),
    }
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

fn scaled_dimension(value: u32, scale: Scale) -> Result<u32, CoreError> {
    let scaled = u64::from(value) * u64::from(scale.numerator) / u64::from(scale.denominator);
    u32::try_from(scaled.max(1)).map_err(|_| CoreError::DimensionOverflow)
}
