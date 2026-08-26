use std::fs;

use pixelpipe_core::{Palette, stable_json};

use crate::{
    CONVERSION_RECIPE_SCHEMA, ConversionRecipeDocument, ProjectError, ProjectStore,
    assets::validate_asset_id,
    persistence::{atomic_write, ensure_schema, io_at, read_json},
};

impl ProjectStore {
    /// Atomically stores a versioned project palette resource.
    ///
    /// # Errors
    ///
    /// Returns an error when identity, palette validation, or storage fails.
    pub fn store_palette(&self, id: &str, palette: &Palette) -> Result<(), ProjectError> {
        validate_resource_id(id)?;
        palette.validate()?;
        let _lock = self.lock()?;
        atomic_write(&self.resource_path("palettes", id), &stable_json(palette)?)
    }

    /// Loads and validates a project palette resource.
    ///
    /// # Errors
    ///
    /// Returns an error when the resource is missing, malformed, or invalid.
    pub fn palette(&self, id: &str) -> Result<Palette, ProjectError> {
        validate_resource_id(id)?;
        let path = self.resource_path("palettes", id);
        if !path.is_file() {
            return Err(ProjectError::ResourceNotFound {
                kind: "palettes",
                id: id.to_owned(),
            });
        }
        let palette: Palette = read_json(&path)?;
        palette.validate()?;
        Ok(palette)
    }

    /// Lists validated project palettes in stable resource-ID order.
    ///
    /// # Errors
    ///
    /// Returns an error when the palette directory or any resource is invalid.
    pub fn palettes(&self) -> Result<Vec<(String, Palette)>, ProjectError> {
        let path = self.root.join(".pixelpipe/palettes");
        if !path.is_dir() {
            return Ok(Vec::new());
        }
        let mut palettes = Vec::new();
        for entry in fs::read_dir(&path).map_err(|source| io_at(&path, source))? {
            let entry = entry.map_err(|source| io_at(&path, source))?;
            if entry.path().extension().and_then(|value| value.to_str()) == Some("json") {
                let id = entry
                    .file_name()
                    .to_string_lossy()
                    .trim_end_matches(".json")
                    .to_owned();
                palettes.push((id.clone(), self.palette(&id)?));
            }
        }
        palettes.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(palettes)
    }

    /// Atomically stores a complete project conversion recipe.
    ///
    /// # Errors
    ///
    /// Returns an error when identity, recipe validation, or storage fails.
    pub fn store_conversion_recipe(
        &self,
        recipe: &ConversionRecipeDocument,
    ) -> Result<(), ProjectError> {
        validate_conversion_recipe(recipe)?;
        let _lock = self.lock()?;
        let recipes = self.root.join(".pixelpipe/recipes");
        fs::create_dir_all(&recipes).map_err(|source| io_at(&recipes, source))?;
        atomic_write(
            &self.resource_path("recipes", &recipe.id),
            &stable_json(recipe)?,
        )
    }

    /// Loads and validates a project conversion recipe.
    ///
    /// # Errors
    ///
    /// Returns an error when the resource is missing, malformed, or invalid.
    pub fn conversion_recipe(&self, id: &str) -> Result<ConversionRecipeDocument, ProjectError> {
        validate_resource_id(id)?;
        let path = self.resource_path("recipes", id);
        if !path.is_file() {
            return Err(ProjectError::ResourceNotFound {
                kind: "recipes",
                id: id.to_owned(),
            });
        }
        let recipe: ConversionRecipeDocument = read_json(&path)?;
        validate_conversion_recipe(&recipe)?;
        if recipe.id != id {
            return Err(ProjectError::ResourceIdentityMismatch);
        }
        Ok(recipe)
    }

    /// Lists validated conversion recipes in stable ID order.
    ///
    /// # Errors
    ///
    /// Returns an error when the resource directory or any recipe is invalid.
    pub fn conversion_recipes(&self) -> Result<Vec<ConversionRecipeDocument>, ProjectError> {
        let path = self.root.join(".pixelpipe/recipes");
        if !path.is_dir() {
            return Ok(Vec::new());
        }
        let mut recipes = Vec::new();
        for entry in fs::read_dir(&path).map_err(|source| io_at(&path, source))? {
            let entry = entry.map_err(|source| io_at(&path, source))?;
            if entry.path().extension().and_then(|value| value.to_str()) == Some("json") {
                let id = entry
                    .file_name()
                    .to_string_lossy()
                    .trim_end_matches(".json")
                    .to_owned();
                recipes.push(self.conversion_recipe(&id)?);
            }
        }
        recipes.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(recipes)
    }
}

fn validate_resource_id(id: &str) -> Result<(), ProjectError> {
    if validate_asset_id(id).is_ok() {
        Ok(())
    } else {
        Err(ProjectError::InvalidResourceId(id.to_owned()))
    }
}

fn validate_conversion_recipe(recipe: &ConversionRecipeDocument) -> Result<(), ProjectError> {
    ensure_schema(&recipe.schema, CONVERSION_RECIPE_SCHEMA)?;
    validate_resource_id(&recipe.id)?;
    validate_resource_id(&recipe.palette)?;
    if recipe.preview_scale == 0 {
        return Err(ProjectError::Core(
            pixelpipe_core::CoreError::InvalidPreviewScale,
        ));
    }
    Ok(())
}
