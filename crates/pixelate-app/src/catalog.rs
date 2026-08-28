use std::{fs, path::PathBuf};

use pixelate_core::sha256_hex;
use pixelate_project::{AssetManifest, ProjectStore};
use serde::{Deserialize, Serialize};

use crate::{AppError, AssetBrowser, ImportReference, import_reference};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectFileStatus {
    Current,
    Modified,
    Missing,
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
pub struct MoveAsset {
    pub start: PathBuf,
    pub asset: String,
    pub destination: String,
}

#[derive(Debug, Deserialize)]
pub struct LoadProjectImage {
    pub start: PathBuf,
    pub path: String,
}

pub(crate) fn build_catalog(
    store: &ProjectStore,
    assets: &[AssetBrowser],
) -> Result<Vec<CatalogEntry>, AppError> {
    let mut catalog = Vec::new();
    let managed_paths = assets
        .iter()
        .filter_map(|entry| entry.asset.project_path.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    for image in store.project_images()? {
        if managed_paths.contains(image.path.as_str()) {
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
        let status = if !full.is_file() {
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
    catalog.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(catalog)
}

/// Adopts an existing project image by importing a verified internal copy, leaving the file untouched.
///
/// # Errors
///
/// Returns an error when the path, image, identity, import, or link is invalid.
pub fn adopt_project_image(request: AdoptProjectImage) -> Result<AssetManifest, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let relative = store.root().join(&request.path);
    store.create_asset(&request.asset, &request.brief)?;
    let result = (|| {
        import_reference(ImportReference {
            start: request.start,
            asset: request.asset.clone(),
            file: relative,
        })?;
        Ok(store.link_asset_project_path(&request.asset, &request.path)?)
    })();
    if result.is_err() {
        let _ = store.delete_asset(&request.asset);
    }
    result
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
    Ok(ProjectStore::discover(&start)?.move_asset_file(&asset, &destination)?)
}
/// Lazily reads one discovered project image for preview.
///
/// # Errors
///
/// Returns an error when the image is not in the current safe discovery catalog or cannot be read.
pub fn load_project_image(request: LoadProjectImage) -> Result<Vec<u8>, AppError> {
    let LoadProjectImage {
        start,
        path: requested_path,
    } = request;
    let store = ProjectStore::discover(&start)?;
    let path = store.root().join(
        store
            .project_images()?
            .into_iter()
            .find(|image| image.path == requested_path)
            .ok_or(pixelate_project::ProjectError::ProjectPathNotFound(
                requested_path,
            ))?
            .path,
    );
    fs::read(&path).map_err(|source| AppError::Read { path, source })
}
