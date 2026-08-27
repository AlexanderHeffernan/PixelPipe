use std::path::PathBuf;

use pixelate_core::{
    BackdropPolicy, ConversionResult, ConversionSettings, Palette, RasterInspection, RgbaImage,
    convert_reference, convert_sheet, decode_rgba_png, derive_source_palette, detect_border_color,
    inspect_raster, render,
};
use pixelate_project::{ProjectError, ProjectStore, StoredConversionMode};
use serde::{Deserialize, Serialize};

use crate::AppError;

#[derive(Debug, Deserialize)]
pub struct PreviewSelectedReference {
    pub start: PathBuf,
    pub asset: String,
    pub recipe: String,
    #[serde(default)]
    pub palette: Option<String>,
    #[serde(default)]
    pub color_count: Option<u8>,
    #[serde(default)]
    pub palette_overrides: Vec<PaletteColorOverride>,
    #[serde(default)]
    pub settings: Option<ConversionSettings>,
    #[serde(default)]
    pub auto_background: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaletteColorOverride {
    pub index: u8,
    pub rgba: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionPreview {
    pub inspection: RasterInspection,
    pub palette_name: String,
    pub native_png: Vec<u8>,
    pub background_removed: bool,
}

/// Renders a selected smooth reference without creating or advancing a revision.
///
/// # Errors
///
/// Returns an [`AppError`] when project discovery, resource resolution, image
/// decoding, deterministic conversion, or PNG rendering fails.
pub fn preview_selected_reference(
    request: PreviewSelectedReference,
) -> Result<ConversionPreview, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let asset = store.asset(&request.asset)?;
    let (_, source_bytes) = store.selected_reference(&asset.id)?;
    let source = decode_rgba_png(&source_bytes)?;
    let recipe = store.conversion_recipe(&request.recipe)?;
    if recipe.kind != asset.kind {
        return Err(ProjectError::AssetKindMismatch {
            asset: asset.id,
            existing: asset.kind,
            requested: recipe.kind,
        }
        .into());
    }
    let (raster, palette_name, background_removed) = match recipe.mode {
        StoredConversionMode::Reference { settings } => {
            let (converted, resolved, palette) = convert_source_reference(
                &source,
                request.settings,
                settings,
                request.auto_background,
                request.color_count.unwrap_or(16),
                &request.palette_overrides,
            )?;
            let name = palette.name.clone();
            (
                converted.raster,
                name,
                matches!(resolved.backdrop, BackdropPolicy::BorderConnected { .. }),
            )
        }
        StoredConversionMode::Sheet { settings } => {
            if request.settings.is_some() || !request.palette_overrides.is_empty() {
                return Err(AppError::UnsupportedConversion(
                    "sheet recipes do not accept reference settings overrides".to_owned(),
                ));
            }
            let palette = store.palette(request.palette.as_deref().unwrap_or(&recipe.palette))?;
            let name = palette.name.clone();
            (
                convert_sheet(&source, &palette, &settings)?.raster,
                name,
                false,
            )
        }
    };
    let inspection = inspect_raster(&raster)?;
    let rendered = render(&raster, 1)?;
    Ok(ConversionPreview {
        inspection,
        palette_name,
        native_png: rendered.native_png,
        background_removed,
    })
}

pub(crate) fn convert_source_reference(
    source: &RgbaImage,
    requested: Option<ConversionSettings>,
    fallback: ConversionSettings,
    automatic: bool,
    color_count: u8,
    overrides: &[PaletteColorOverride],
) -> Result<(ConversionResult, ConversionSettings, Palette), AppError> {
    let mut settings = requested.unwrap_or(fallback);
    if automatic {
        let detection = if let BackdropPolicy::BorderConnected {
            tolerance,
            alpha_threshold,
            ..
        } = &settings.backdrop
        {
            Some((*alpha_threshold, *tolerance))
        } else {
            None
        };
        if let Some((alpha_threshold, tolerance)) = detection {
            settings.backdrop = detect_border_color(source, alpha_threshold, tolerance)?.map_or(
                BackdropPolicy::Alpha { alpha_threshold },
                |color| BackdropPolicy::BorderConnected {
                    color,
                    tolerance,
                    alpha_threshold,
                },
            );
        }
    }
    let palette = source_palette(
        source,
        &settings.backdrop,
        color_count,
        settings.color_treatment,
        settings.color_adjustments,
        overrides,
    )?;
    let converted = convert_reference(source, &palette, &settings)?;
    Ok((converted, settings, palette))
}

pub(crate) fn source_palette(
    source: &RgbaImage,
    backdrop: &BackdropPolicy,
    color_count: u8,
    treatment: pixelate_core::ColorTreatment,
    adjustments: pixelate_core::ColorAdjustments,
    overrides: &[PaletteColorOverride],
) -> Result<Palette, AppError> {
    let mut palette = derive_source_palette(source, backdrop, color_count, treatment, adjustments)?;
    for entry in overrides {
        if entry.index == palette.transparent_index {
            return Err(AppError::UnsupportedConversion(
                "the transparent palette entry cannot be replaced".to_owned(),
            ));
        }
        let Some(color) = palette.colors.get_mut(usize::from(entry.index)) else {
            return Err(AppError::UnsupportedConversion(format!(
                "palette colour {} is not available for this source",
                entry.index
            )));
        };
        *color = entry.rgba;
    }
    palette.validate()?;
    Ok(palette)
}
