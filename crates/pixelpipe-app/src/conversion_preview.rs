use std::path::PathBuf;

use pixelpipe_core::{
    ConversionSettings, RasterInspection, convert_reference, convert_sheet, decode_rgba_png,
    inspect_raster, render,
};
use pixelpipe_project::{ProjectError, ProjectStore, StoredConversionMode};
use serde::Deserialize;

use crate::AppError;

#[derive(Debug, Deserialize)]
pub struct PreviewSelectedReference {
    pub start: PathBuf,
    pub asset: String,
    pub recipe: String,
    #[serde(default)]
    pub settings: Option<ConversionSettings>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionPreview {
    pub inspection: RasterInspection,
    pub palette_name: String,
    pub native_png: Vec<u8>,
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
    let palette = store.palette(&recipe.palette)?;
    let raster = match recipe.mode {
        StoredConversionMode::Reference { settings } => {
            convert_reference(&source, &palette, &request.settings.unwrap_or(settings))?.raster
        }
        StoredConversionMode::Sheet { settings } => {
            if request.settings.is_some() {
                return Err(AppError::UnsupportedConversion(
                    "sheet recipes do not accept reference settings overrides".to_owned(),
                ));
            }
            convert_sheet(&source, &palette, &settings)?.raster
        }
    };
    let inspection = inspect_raster(&raster)?;
    let rendered = render(&raster, 1)?;
    Ok(ConversionPreview {
        inspection,
        palette_name: palette.name,
        native_png: rendered.native_png,
    })
}
