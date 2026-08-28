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
                fs::read(&full)
                    .ok()
                    .is_some_and(|bytes| sha256_hex(&bytes) != expected)
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

pub fn relink_asset(request: RelinkAsset) -> Result<AssetManifest, AppError> {
    Ok(ProjectStore::discover(&request.start)?
        .link_asset_project_path(&request.asset, &request.path)?)
}

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

pub fn create_folder(request: CreateFolder) -> Result<(), AppError> {
    Ok(ProjectStore::discover(&request.start)?.create_project_folder(&request.path)?)
}
pub fn move_folder(request: MoveFolder) -> Result<Vec<AssetManifest>, AppError> {
    Ok(ProjectStore::discover(&request.start)?
        .move_project_folder(&request.source, &request.destination)?)
}
pub fn delete_folder(request: DeleteFolder) -> Result<(), AppError> {
    Ok(ProjectStore::discover(&request.start)?.delete_project_folder(&request.path)?)
}
pub fn move_asset(request: MoveAsset) -> Result<AssetManifest, AppError> {
    Ok(ProjectStore::discover(&request.start)?
        .move_asset_file(&request.asset, &request.destination)?)
}
pub fn load_project_image(request: LoadProjectImage) -> Result<Vec<u8>, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let path = store.root().join(
        store
            .project_images()?
            .into_iter()
            .find(|image| image.path == request.path)
            .ok_or_else(|| {
                pixelate_project::ProjectError::ProjectPathNotFound(request.path.clone())
            })?
            .path,
    );
    fs::read(&path).map_err(|source| AppError::Read { path, source })
}
