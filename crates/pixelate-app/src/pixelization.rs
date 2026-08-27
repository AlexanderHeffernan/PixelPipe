use std::{collections::BTreeMap, path::PathBuf};

use pixelate_core::{
    BackdropPolicy, ComponentExpectation, ConversionSettings, Operation, RECIPE_SCHEMA, Recipe,
    Registration, decode_rgba_png, sha256_hex, stable_json,
};
use pixelate_project::{AssetBrief, AssetManifest, AssetStyle, ProjectStore, ReferenceSelection};
use serde::{Deserialize, Serialize};

use crate::{
    AppError, PaletteColorOverride,
    revision_commit::{CommitRaster, RevisionResult, commit_raster},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PixelizationDefaults {
    pub color_count: u8,
    pub settings: ConversionSettings,
}

#[derive(Debug, Deserialize)]
pub struct ConvertSelectedReference {
    pub start: PathBuf,
    pub asset: String,
    #[serde(default)]
    pub color_count: Option<u8>,
    #[serde(default)]
    pub palette_overrides: Vec<PaletteColorOverride>,
    #[serde(default)]
    pub settings: Option<ConversionSettings>,
    #[serde(default)]
    pub auto_background: bool,
    pub actor: String,
}

#[must_use]
pub fn pixelization_defaults() -> PixelizationDefaults {
    PixelizationDefaults {
        color_count: 16,
        settings: ConversionSettings {
            width: 32,
            height: 32,
            color_treatment: pixelate_core::ColorTreatment::Original,
            color_adjustments: pixelate_core::ColorAdjustments::default(),
            margin: 0,
            subject_scale_percent: 100,
            offset_x: 0,
            offset_y: 0,
            coverage_percent: 10,
            backdrop: BackdropPolicy::BorderConnected {
                color: [255, 255, 255],
                tolerance: 28,
                alpha_threshold: 8,
            },
            registration: Registration::Center,
            components: ComponentExpectation { min: 1, max: 32 },
        },
    }
}

/// Converts the selected source into an immutable revision using direct settings.
///
/// # Errors
///
/// Returns an [`AppError`] when the asset, source, conversion, or storage is invalid.
pub fn convert_selected_reference(
    request: ConvertSelectedReference,
) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let asset = store.asset(&request.asset)?;
    require_brief(&asset)?;
    let (selection, source_bytes) = store.selected_reference(&asset.id)?;
    let source = decode_rgba_png(&source_bytes)?;
    let defaults = pixelization_defaults();
    let color_count = request.color_count.unwrap_or(defaults.color_count);
    let (converted, settings, palette) = crate::conversion_preview::convert_source_reference(
        &source,
        request.settings,
        defaults.settings,
        request.auto_background,
        color_count,
        &request.palette_overrides,
    )?;
    let palette_sha256 = sha256_hex(&stable_json(&palette)?);
    let recipe = Recipe {
        schema: RECIPE_SCHEMA.to_owned(),
        input_sha256: selection.sha256.clone(),
        palette_sha256: palette_sha256.clone(),
        operations: vec![Operation::ConvertReference {
            settings: settings.clone(),
        }],
    };
    let input_hashes = conversion_input_hashes(
        &asset.brief,
        &selection,
        color_count,
        &settings,
        &palette_sha256,
    )?;
    commit_raster(
        &store,
        CommitRaster {
            asset: asset.id,
            raster: converted.raster,
            recipe,
            brief: asset.brief.text,
            actor: request.actor,
            input_hashes,
            additional_checks: converted.checks,
            parent: None,
            style: Some(AssetStyle {
                color_count,
                settings,
            }),
        },
    )
}

fn require_brief(asset: &AssetManifest) -> Result<(), AppError> {
    if asset.brief.text.trim().is_empty() {
        return Err(pixelate_project::ProjectError::AssetNotReady {
            asset: asset.id.clone(),
            operation: "pixelize selected source",
            reason: "write a non-empty brief first",
        }
        .into());
    }
    Ok(())
}

fn conversion_input_hashes(
    brief: &AssetBrief,
    selection: &ReferenceSelection,
    color_count: u8,
    settings: &ConversionSettings,
    palette_hash: &str,
) -> Result<BTreeMap<String, String>, AppError> {
    Ok(BTreeMap::from([
        ("brief".to_owned(), sha256_hex(&stable_json(brief)?)),
        ("palette".to_owned(), palette_hash.to_owned()),
        (
            format!("source_palette:{color_count}"),
            palette_hash.to_owned(),
        ),
        (
            "pixelization_settings".to_owned(),
            sha256_hex(&stable_json(settings)?),
        ),
        ("reference".to_owned(), selection.sha256.clone()),
        (
            "reference_selection".to_owned(),
            sha256_hex(&stable_json(selection)?),
        ),
    ]))
}
