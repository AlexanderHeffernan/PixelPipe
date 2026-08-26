use std::path::PathBuf;

use pixelpipe_core::{
    BackdropPolicy, ComponentExpectation, ConversionSettings, PALETTE_SCHEMA, Palette, Registration,
};
use pixelpipe_project::{
    AssetKind, CONVERSION_RECIPE_SCHEMA, ConversionRecipeDocument, ProjectError, ProjectStore,
    StoredConversionMode,
};
use serde::Deserialize;

use crate::{AppError, BrowseProject, ProjectBrowser, browse_project};

const STARTER_PALETTE: &str = "pixelpipe-starter";

#[derive(Debug, Deserialize)]
pub struct OpenProject {
    pub start: PathBuf,
}

/// Opens a `PixelPipe` project or initializes the selected folder with starter resources.
///
/// # Errors
///
/// Returns an error when project discovery, initialization, or resource storage fails.
pub fn open_project(request: OpenProject) -> Result<ProjectBrowser, AppError> {
    let OpenProject { start } = request;
    let store = match ProjectStore::discover(&start) {
        Ok(store) => store,
        Err(ProjectError::NotFound(_)) => {
            let name = start
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.trim().is_empty())
                .unwrap_or("PixelPipe Project");
            ProjectStore::init(&start, name)?
        }
        Err(error) => return Err(error.into()),
    };
    install_starter_resources(&store)?;
    browse_project(&BrowseProject {
        start: store.root().to_path_buf(),
    })
}

fn install_starter_resources(store: &ProjectStore) -> Result<(), AppError> {
    for (id, palette) in built_in_palettes() {
        match store.palette(id) {
            Ok(_) => {}
            Err(ProjectError::ResourceNotFound { .. }) => {
                store.store_palette(id, &palette)?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    let existing = store
        .conversion_recipes()?
        .into_iter()
        .map(|recipe| recipe.id)
        .collect::<Vec<_>>();
    for size in [16, 32, 64] {
        let recipe = starter_recipe(size);
        if !existing.contains(&recipe.id) {
            store.store_conversion_recipe(&recipe)?;
        }
    }
    Ok(())
}

fn built_in_palettes() -> Vec<(&'static str, Palette)> {
    vec![(STARTER_PALETTE, starter_palette())]
}

fn starter_palette() -> Palette {
    Palette {
        schema: PALETTE_SCHEMA.to_owned(),
        name: "PixelPipe Starter".to_owned(),
        transparent_index: 0,
        colors: vec![
            [0, 0, 0, 0],
            [30, 24, 37, 255],
            [60, 38, 55, 255],
            [91, 55, 70, 255],
            [126, 73, 73, 255],
            [164, 96, 77, 255],
            [206, 128, 81, 255],
            [238, 174, 94, 255],
            [255, 225, 133, 255],
            [75, 105, 75, 255],
            [96, 149, 91, 255],
            [129, 190, 113, 255],
            [66, 91, 125, 255],
            [81, 130, 164, 255],
            [121, 177, 191, 255],
            [221, 232, 224, 255],
        ],
    }
}

fn starter_recipe(size: u32) -> ConversionRecipeDocument {
    ConversionRecipeDocument {
        schema: CONVERSION_RECIPE_SCHEMA.to_owned(),
        id: format!("sprite-{size}"),
        kind: AssetKind::Sprite,
        palette: STARTER_PALETTE.to_owned(),
        preview_scale: if size <= 16 { 12 } else { 8 },
        mode: StoredConversionMode::Reference {
            settings: ConversionSettings {
                width: size,
                height: size,
                color_treatment: pixelpipe_core::ColorTreatment::Original,
                color_adjustments: pixelpipe_core::ColorAdjustments::default(),
                margin: 1,
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
        },
    }
}
