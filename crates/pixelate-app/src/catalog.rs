use std::{collections::BTreeMap, fs, path::PathBuf};

use pixelate_core::{
    Operation, RECIPE_SCHEMA, Recipe, decode_rgba_png, import_pixel_art, sha256_hex, stable_json,
};
use pixelate_project::{AssetManifest, ProjectStore};
use serde::{Deserialize, Serialize};

use crate::{
    AppError, AssetBrowser, CommitRaster, ImportReference, RevisionResult, commit_raster,
    import_reference,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectFileStatus {
    Current,
    Modified,
    Missing,
    Unexported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatalogEntry {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    pub status: ProjectFileStatus,
}

#[derive(Debug, Deserialize)]
pub struct AdoptProjectImage {
    pub start: PathBuf,
    pub path: String,
    pub asset: String,
    #[serde(default)]
    pub brief: String,
    pub destination: String,
}

#[derive(Debug, Deserialize)]
pub struct AdoptPixelArt {
    pub start: PathBuf,
    pub path: String,
    pub asset: String,
    #[serde(default)]
    pub brief: String,
    pub actor: String,
}

#[derive(Debug, Deserialize)]
pub struct SetProjectImageIgnored {
    pub start: PathBuf,
    pub path: String,
    pub ignored: bool,
}

#[derive(Debug, Deserialize)]
pub struct RelinkAsset {
    pub start: PathBuf,
    pub asset: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLinkedSource {
    pub start: PathBuf,
    pub asset: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateFolder {
    pub start: PathBuf,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct MoveFolder {
    pub start: PathBuf,
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteFolder {
    pub start: PathBuf,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteProjectImage {
    pub start: PathBuf,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct MoveProjectImage {
    pub start: PathBuf,
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Deserialize)]
pub struct MoveAsset {
    pub start: PathBuf,
    pub asset: String,
    pub destination: String,
}

pub(crate) fn build_catalog(
    store: &ProjectStore,
    assets: &[AssetBrowser],
) -> Result<Vec<CatalogEntry>, AppError> {
    let mut catalog = Vec::new();
    let ignored = store
        .manifest()?
        .ignored_project_images
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let managed_paths = assets
        .iter()
        .filter_map(|entry| entry.asset.project_path.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    for image in store.project_images()? {
        if managed_paths.contains(image.path.as_str()) || ignored.contains(&image.path) {
            continue;
        }
        catalog.push(CatalogEntry {
            path: image.path,
            asset_id: None,
            status: ProjectFileStatus::Current,
        });
    }
    for entry in assets {
        let Some(path) = entry.asset.project_path.as_deref() else {
            continue;
        };
        let full = store.root().join(path);
        let status = if !full.is_file() && entry.asset.project_file_sha256.is_none() {
            ProjectFileStatus::Unexported
        } else if !full.is_file() {
            ProjectFileStatus::Missing
        } else if entry
            .asset
            .project_file_sha256
            .as_deref()
            .is_some_and(|expected| {
                fs::read(&full).is_ok_and(|bytes| sha256_hex(&bytes) != expected)
            })
        {
            ProjectFileStatus::Modified
        } else {
            ProjectFileStatus::Current
        };
        catalog.push(CatalogEntry {
            path: path.to_owned(),
            asset_id: Some(entry.asset.id.clone()),
            status,
        });
    }
    for entry in assets {
        if entry.asset.project_path.is_some() {
            continue;
        }
        catalog.push(CatalogEntry {
            path: format!("{}.png", entry.asset.id),
            asset_id: Some(entry.asset.id.clone()),
            status: ProjectFileStatus::Unexported,
        });
    }
    catalog.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(catalog)
}

/// Adopts an existing project image as a reference and hides the source from the asset catalog.
///
/// # Errors
///
/// Returns an error when the path, image, identity, import, or link is invalid.
pub fn adopt_project_image(request: AdoptProjectImage) -> Result<AssetManifest, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let relative = store.root().join(&request.path);
    store.create_asset(&request.asset, &request.brief)?;
    let result = (|| {
        store.plan_asset_project_path(&request.asset, &request.destination)?;
        import_reference(ImportReference {
            start: request.start,
            asset: request.asset.clone(),
            file: relative,
        })?;
        store.ignore_project_image(&request.path)?;
        Ok(store.asset(&request.asset)?)
    })();
    if result.is_err() {
        let _ = store.delete_asset(&request.asset);
    }
    result
}

/// Imports an existing project image as exact editable pixel art and opens at revision step 2.
///
/// # Errors
///
/// Returns an error when the image cannot be decoded exactly into at most 256 indexed colours.
pub fn adopt_pixel_art(request: AdoptPixelArt) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let file = store.root().join(&request.path);
    store.create_asset(&request.asset, &request.brief)?;
    let result = (|| {
        let selection = import_reference(ImportReference {
            start: request.start,
            asset: request.asset.clone(),
            file,
        })?;
        store.unignore_project_image(&request.path)?;
        store.link_asset_project_path(&request.asset, &request.path)?;
        let (_, source) = store.selected_reference(&request.asset)?;
        let raster = import_pixel_art(&decode_rgba_png(&source)?)?;
        let palette_sha256 = sha256_hex(&stable_json(&raster.palette)?);
        commit_raster(
            &store,
            CommitRaster {
                asset: request.asset.clone(),
                recipe: Recipe {
                    schema: RECIPE_SCHEMA.to_owned(),
                    input_sha256: selection.sha256.clone(),
                    palette_sha256: palette_sha256.clone(),
                    operations: vec![Operation::ImportPixelArt],
                },
                raster,
                brief: request.brief,
                actor: request.actor,
                input_hashes: BTreeMap::from([
                    ("source".to_owned(), selection.sha256),
                    ("palette".to_owned(), palette_sha256),
                ]),
                additional_checks: Vec::new(),
                parent: None,
                style: None,
            },
        )
    })();
    if result.is_err() {
        let _ = store.delete_asset(&request.asset);
    }
    result
}

/// Hides or restores one unmanaged image in the project asset catalog.
///
/// # Errors
///
/// Returns an error when the project image or project manifest is invalid.
pub fn set_project_image_ignored(
    request: SetProjectImageIgnored,
) -> Result<pixelate_project::ProjectManifest, AppError> {
    let SetProjectImageIgnored {
        start,
        path,
        ignored,
    } = request;
    let store = ProjectStore::discover(&start)?;
    if ignored {
        Ok(store.ignore_project_image(&path)?)
    } else {
        Ok(store.unignore_project_image(&path)?)
    }
}

/// Relinks a managed asset to an existing supported project image.
///
/// # Errors
///
/// Returns an error when the project, asset, path, or image is invalid.
pub fn relink_asset(request: RelinkAsset) -> Result<AssetManifest, AppError> {
    let RelinkAsset { start, asset, path } = request;
    Ok(ProjectStore::discover(&start)?.link_asset_project_path(&asset, &path)?)
}

/// Imports the current linked project file as the asset's selected source.
///
/// # Errors
///
/// Returns an error when the link is missing or the image cannot be verified and imported.
pub fn update_linked_source(request: UpdateLinkedSource) -> Result<AssetManifest, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let asset = store.asset(&request.asset)?;
    let path = asset
        .project_path
        .ok_or_else(|| pixelate_project::ProjectError::AssetNotReady {
            asset: request.asset.clone(),
            operation: "update linked source",
            reason: "the asset is a Draft",
        })?;
    import_reference(ImportReference {
        start: request.start,
        asset: request.asset.clone(),
        file: store.root().join(&path),
    })?;
    Ok(store.link_asset_project_path(&request.asset, &path)?)
}

/// Creates one real project folder.
///
/// # Errors
///
/// Returns an error when the path is unsafe, exists, or cannot be created.
pub fn create_folder(request: CreateFolder) -> Result<(), AppError> {
    let CreateFolder { start, path } = request;
    Ok(ProjectStore::discover(&start)?.create_project_folder(&path)?)
}
/// Moves or renames one real project folder and its managed links.
///
/// # Errors
///
/// Returns an error when a path is unsafe, collides, or cannot be moved atomically.
pub fn move_folder(request: MoveFolder) -> Result<Vec<AssetManifest>, AppError> {
    let MoveFolder {
        start,
        source,
        destination,
    } = request;
    Ok(ProjectStore::discover(&start)?.move_project_folder(&source, &destination)?)
}
/// Deletes one empty real project folder.
///
/// # Errors
///
/// Returns an error when the folder is unsafe, missing, non-empty, or cannot be deleted.
pub fn delete_folder(request: DeleteFolder) -> Result<(), AppError> {
    let DeleteFolder { start, path } = request;
    Ok(ProjectStore::discover(&start)?.delete_project_folder(&path)?)
}
/// Deletes one supported project image while retaining any Pixelate asset and revision history.
///
/// # Errors
///
/// Returns an error when the path is unsafe, missing, escaped, or unsupported.
pub fn delete_project_image(request: DeleteProjectImage) -> Result<(), AppError> {
    let DeleteProjectImage { start, path } = request;
    Ok(ProjectStore::discover(&start)?.delete_project_image(&path)?)
}
/// Moves an unmanaged project image without changing or creating Pixelate history.
///
/// # Errors
///
/// Returns an error when the source is managed, or either path is unsafe, missing, or occupied.
pub fn move_project_image(request: MoveProjectImage) -> Result<(), AppError> {
    let MoveProjectImage {
        start,
        source,
        destination,
    } = request;
    let store = ProjectStore::discover(&start)?;
    Ok(store.move_project_image(&source, &destination)?)
}
/// Moves a linked asset's project image without changing its stable identity.
///
/// # Errors
///
/// Returns an error when the asset is a Draft or a path is unsafe, missing, or occupied.
pub fn move_asset(request: MoveAsset) -> Result<AssetManifest, AppError> {
    let MoveAsset {
        start,
        asset,
        destination,
    } = request;
    let store = ProjectStore::discover(&start)?;
    let manifest = store.asset(&asset)?;
    if manifest.project_path.is_none()
        || (manifest.project_file_sha256.is_none()
            && manifest
                .project_path
                .as_deref()
                .is_some_and(|path| !store.root().join(path).is_file()))
    {
        Ok(store.plan_asset_project_path(&asset, &destination)?)
    } else {
        Ok(store.move_asset_file(&asset, &destination)?)
    }
}
