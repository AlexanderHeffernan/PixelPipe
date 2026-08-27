use std::{collections::BTreeMap, path::PathBuf};

use pixelpipe_core::{
    ConversionSettings, Operation, RECIPE_SCHEMA, Recipe, convert_sheet, decode_rgba_png,
    sha256_hex, stable_json,
};
use pixelpipe_project::{
    AssetBrief, AssetManifest, AssetStyle, ConversionRecipeDocument, ProjectError, ProjectStore,
    ReferenceSelection, StoredConversionMode,
};
use serde::Deserialize;

use crate::{
    AppError, PaletteColorOverride,
    revision_commit::{CommitRaster, RevisionResult, commit_raster},
};

#[derive(Debug, Deserialize)]
pub struct ConvertSelectedReference {
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
    pub actor: String,
}

/// Resolves project resources and converts the selected reference into a revision.
///
/// The resolved brief, palette, conversion settings, and their content hashes are
/// frozen into the immutable revision. Resource edits after this call cannot alter it.
///
/// # Errors
///
/// Returns an [`AppError`] when lifecycle/resources/input/conversion or storage fails.
pub fn convert_selected_reference(
    request: ConvertSelectedReference,
) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let asset = store.asset(&request.asset)?;
    require_brief(&asset)?;
    let (selection, source_bytes) = store.selected_reference(&asset.id)?;
    let source = decode_rgba_png(&source_bytes)?;
    let resource_recipe = store.conversion_recipe(&request.recipe)?;
    if resource_recipe.kind != asset.kind {
        return Err(ProjectError::AssetKindMismatch {
            asset: asset.id,
            existing: asset.kind,
            requested: resource_recipe.kind,
        }
        .into());
    }
    let palette_id = request
        .palette
        .as_deref()
        .unwrap_or(&resource_recipe.palette);
    let reference_color_count = request.color_count.unwrap_or(16);
    let (converted, operation, style, palette) = match resource_recipe.mode.clone() {
        StoredConversionMode::Reference { settings } => {
            let (converted, settings, palette) =
                crate::conversion_preview::convert_source_reference(
                    &source,
                    request.settings,
                    settings,
                    request.auto_background,
                    reference_color_count,
                    &request.palette_overrides,
                )?;
            let style = AssetStyle {
                recipe: resource_recipe.id.clone(),
                palette: None,
                color_count: Some(reference_color_count),
                settings: settings.clone(),
            };
            (
                converted,
                Operation::ConvertReference { settings },
                Some(style),
                palette,
            )
        }
        StoredConversionMode::Sheet { settings } => {
            ensure_no_reference_overrides(&request)?;
            let palette = store.palette(palette_id)?;
            let converted = convert_sheet(&source, &palette, &settings)?;
            (
                converted,
                Operation::ConvertSheet { settings },
                None,
                palette,
            )
        }
    };
    let canonical_palette = stable_json(&palette)?;
    let recipe = Recipe {
        schema: RECIPE_SCHEMA.to_owned(),
        input_sha256: selection.sha256.clone(),
        palette_sha256: sha256_hex(&canonical_palette),
        operations: vec![
            operation,
            Operation::RenderIndexed {
                preview_scale: resource_recipe.preview_scale,
            },
        ],
    };
    let provenance_color_count =
        matches!(resource_recipe.mode, StoredConversionMode::Reference { .. })
            .then_some(reference_color_count);
    let input_hashes = conversion_input_hashes(
        &asset.brief,
        &selection,
        &resource_recipe,
        palette_id,
        provenance_color_count,
        &recipe.palette_sha256,
    )?;
    commit_raster(
        &store,
        CommitRaster {
            asset: asset.id,
            kind: asset.kind,
            raster: converted.raster,
            recipe,
            preview_scale: resource_recipe.preview_scale,
            brief: asset.brief.text,
            actor: request.actor,
            input_hashes,
            additional_checks: converted.checks,
            parent: None,
            style,
        },
    )
}

fn require_brief(asset: &AssetManifest) -> Result<(), AppError> {
    if asset.brief.text.trim().is_empty() {
        return Err(ProjectError::AssetNotReady {
            asset: asset.id.clone(),
            operation: "convert selected reference",
            reason: "write a non-empty brief first",
        }
        .into());
    }
    Ok(())
}

fn ensure_no_reference_overrides(request: &ConvertSelectedReference) -> Result<(), AppError> {
    if request.settings.is_some()
        || request.color_count.is_some()
        || !request.palette_overrides.is_empty()
    {
        return Err(AppError::UnsupportedConversion(
            "sheet recipes do not accept reference-only overrides".to_owned(),
        ));
    }
    Ok(())
}

fn conversion_input_hashes(
    brief: &AssetBrief,
    selection: &ReferenceSelection,
    recipe: &ConversionRecipeDocument,
    palette_id: &str,
    color_count: Option<u8>,
    palette_hash: &str,
) -> Result<BTreeMap<String, String>, AppError> {
    let palette_resource = color_count.map_or_else(
        || format!("project_palette:{palette_id}"),
        |count| format!("source_palette:{count}"),
    );
    Ok(BTreeMap::from([
        ("brief".to_owned(), sha256_hex(&stable_json(brief)?)),
        ("palette".to_owned(), palette_hash.to_owned()),
        (palette_resource, palette_hash.to_owned()),
        (
            format!("project_recipe:{}", recipe.id),
            sha256_hex(&stable_json(recipe)?),
        ),
        ("reference".to_owned(), selection.sha256.clone()),
        (
            "reference_selection".to_owned(),
            sha256_hex(&stable_json(selection)?),
        ),
    ]))
}
