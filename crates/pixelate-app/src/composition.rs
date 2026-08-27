use std::{collections::BTreeMap, path::PathBuf};

use pixelate_core::{
    CanvasSettings, Operation, RECIPE_SCHEMA, RasterInspection, Recipe, ValidationCheck,
    compose_canvas, inspect_raster, render, sha256_hex, stable_json,
};
use pixelate_project::ProjectStore;
use serde::Deserialize;

use crate::{AppError, CommitRaster, RevisionResult, commit_raster};

#[derive(Debug, Deserialize)]
pub struct PreviewComposition {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    pub settings: CanvasSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionPreview {
    pub inspection: RasterInspection,
    pub native_png: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub struct CommitComposition {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    pub settings: CanvasSettings,
    pub actor: String,
}

/// Places an indexed sprite on a canvas without mutating project state.
///
/// # Errors
///
/// Returns [`AppError`] when the project, revision, settings, or render is invalid.
pub fn preview_composition(request: PreviewComposition) -> Result<CompositionPreview, AppError> {
    let PreviewComposition {
        start,
        asset,
        parent,
        settings,
    } = request;
    let store = ProjectStore::discover(&start)?;
    let parent = store.revision(&asset, &parent)?;
    let raster = compose_canvas(&parent.raster, settings)?;
    let inspection = inspect_raster(&raster)?;
    let rendered = render(&raster, 1)?;
    Ok(CompositionPreview {
        inspection,
        native_png: rendered.native_png,
    })
}

/// Commits one deterministic canvas placement as an immutable revision.
///
/// # Errors
///
/// Returns [`AppError`] when the project, revision, settings, render, or commit fails.
pub fn commit_composition(request: CommitComposition) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let parent = store.revision(&request.asset, &request.parent)?;
    let raster = compose_canvas(&parent.raster, request.settings)?;
    let input_hash = sha256_hex(&stable_json(&parent.raster)?);
    let palette_hash = sha256_hex(&stable_json(&raster.palette)?);
    let recipe = Recipe {
        schema: RECIPE_SCHEMA.to_owned(),
        input_sha256: input_hash.clone(),
        palette_sha256: palette_hash.clone(),
        operations: vec![Operation::ComposeCanvas {
            settings: request.settings,
        }],
    };
    commit_raster(
        &store,
        CommitRaster {
            asset: request.asset,
            raster,
            recipe,
            brief: parent.brief,
            actor: request.actor,
            input_hashes: BTreeMap::from([
                ("palette".to_owned(), palette_hash),
                ("parent_pixels".to_owned(), input_hash),
            ]),
            additional_checks: vec![ValidationCheck {
                name: "canvas_composition".to_owned(),
                passed: true,
                detail: format!(
                    "{}x{}:{}%@{},{}",
                    request.settings.width,
                    request.settings.height,
                    request.settings.scale_percent,
                    request.settings.offset_x,
                    request.settings.offset_y
                ),
            }],
            parent: Some(request.parent),
            style: None,
        },
    )
}
