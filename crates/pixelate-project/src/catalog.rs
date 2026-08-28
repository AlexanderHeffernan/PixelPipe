use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use ignore::WalkBuilder;

use crate::{AssetManifest, ProjectError, ProjectStore, assets::path_string, persistence::io_at};

const RESERVED: [&str; 9] = [
    ".pixelate",
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectImage {
    pub path: String,
}

impl ProjectStore {
    /// Discovers supported project artwork while honoring ignore files and known output folders.
    ///
    /// # Errors
    ///
    /// Returns an error when a traversed filesystem entry cannot be inspected.
    pub fn project_images(&self) -> Result<Vec<ProjectImage>, ProjectError> {
        let mut images = Vec::new();
        let root = self.root.clone();
        let walker = WalkBuilder::new(&self.root)
            .hidden(false)
            .require_git(false)
            .follow_links(false)
            .filter_entry(move |entry| {
                entry.path() == root
                    || entry
                        .file_name()
                        .to_str()
                        .is_some_and(|name| !name.starts_with('.') && !RESERVED.contains(&name))
            })
            .build();
        for result in walker {
            let entry = result.map_err(|error| ProjectError::Io {
                path: self.root.clone(),
                source: std::io::Error::other(error),
            })?;
            if entry.file_type().is_some_and(|kind| kind.is_file())
                && is_supported_image(entry.path())
            {
                let relative = entry.path().strip_prefix(&self.root).map_err(|_| {
                    ProjectError::InvalidProjectPath(entry.path().display().to_string())
                })?;
                images.push(ProjectImage {
                    path: path_string(relative),
                });
            }
        }
        images.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(images)
    }

    pub(crate) fn validate_project_file(&self, path: &str) -> Result<PathBuf, ProjectError> {
        let relative = validate_relative(path)?;
        if !is_supported_image(&relative) {
            return Err(ProjectError::InvalidProjectPath(path.to_owned()));
        }
        let full = self.root.join(&relative);
        if !full.is_file() {
            return Err(ProjectError::ProjectPathNotFound(path.to_owned()));
        }
        self.ensure_contained_existing(&full, path)?;
        Ok(relative)
    }

    /// Creates a real, empty project folder without adding placeholder files.
    pub fn create_project_folder(&self, path: &str) -> Result<(), ProjectError> {
        let relative = validate_relative(path)?;
        let target = self.root.join(&relative);
        if target.exists() {
            return Err(ProjectError::ProjectPathExists(path.to_owned()));
        }
        self.ensure_contained_parent(&target, path)?;
        fs::create_dir(&target).map_err(|source| io_at(&target, source))
    }

    /// Deletes one empty real project folder. Recursive deletion is never performed.
    pub fn delete_project_folder(&self, path: &str) -> Result<(), ProjectError> {
        let relative = validate_relative(path)?;
        let target = self.root.join(&relative);
        self.ensure_contained_existing(&target, path)?;
        if !target.is_dir() {
            return Err(ProjectError::ProjectPathNotFound(path.to_owned()));
        }
        fs::remove_dir(&target).map_err(|source| {
            if source.kind() == std::io::ErrorKind::DirectoryNotEmpty {
                ProjectError::FolderNotEmpty(path.to_owned())
            } else {
                io_at(&target, source)
            }
        })
    }

    /// Moves a linked project image and updates its authoritative manifest link.
    pub fn move_asset_file(
        &self,
        id: &str,
        destination: &str,
    ) -> Result<AssetManifest, ProjectError> {
        let destination = validate_relative(destination)?;
        if !is_supported_image(&destination) {
            return Err(ProjectError::InvalidProjectPath(path_string(&destination)));
        }
        let _lock = self.lock()?;
        let mut asset = self.asset(id)?;
        let source_name =
            asset
                .project_path
                .clone()
                .ok_or_else(|| ProjectError::AssetNotReady {
                    asset: id.to_owned(),
                    operation: "move",
                    reason: "the asset is a Draft",
                })?;
        let source = self.root.join(validate_relative(&source_name)?);
        let target = self.root.join(&destination);
        self.ensure_contained_existing(&source, &source_name)?;
        self.ensure_new_target(&target, &path_string(&destination))?;
        fs::rename(&source, &target).map_err(|source_error| io_at(&target, source_error))?;
        asset.project_path = Some(path_string(&destination));
        if let Err(error) = self.write_asset_manifest(&asset) {
            let _ = fs::rename(&target, &source);
            return Err(error);
        }
        Ok(asset)
    }

    /// Moves a project folder and atomically updates every linked asset path, rolling back on error.
    pub fn move_project_folder(
        &self,
        source: &str,
        destination: &str,
    ) -> Result<Vec<AssetManifest>, ProjectError> {
        let source_relative = validate_relative(source)?;
        let destination_relative = validate_relative(destination)?;
        if destination_relative.starts_with(&source_relative) {
            return Err(ProjectError::InvalidProjectPath(destination.to_owned()));
        }
        let source_full = self.root.join(&source_relative);
        let destination_full = self.root.join(&destination_relative);
        self.ensure_contained_existing(&source_full, source)?;
        self.ensure_new_target(&destination_full, destination)?;
        if !source_full.is_dir() {
            return Err(ProjectError::ProjectPathNotFound(source.to_owned()));
        }
        let _lock = self.lock()?;
        let originals = self.assets()?;
        let mut updated = originals.clone();
        for asset in &mut updated {
            if let Some(path) = asset.project_path.as_deref() {
                let path = validate_relative(path)?;
                if let Ok(suffix) = path.strip_prefix(&source_relative) {
                    asset.project_path = Some(path_string(&destination_relative.join(suffix)));
                }
            }
        }
        fs::rename(&source_full, &destination_full)
            .map_err(|source_error| io_at(&destination_full, source_error))?;
        for (index, asset) in updated.iter().enumerate() {
            if asset == &originals[index] {
                continue;
            }
            if let Err(error) = self.write_asset_manifest(asset) {
                let _ = fs::rename(&destination_full, &source_full);
                for original in &originals[..index] {
                    let _ = self.write_asset_manifest(original);
                }
                return Err(error);
            }
        }
        Ok(updated
            .into_iter()
            .filter(|asset| {
                originals
                    .iter()
                    .any(|old| old.id == asset.id && old != asset)
            })
            .collect())
    }

    fn ensure_new_target(&self, target: &Path, display: &str) -> Result<(), ProjectError> {
        if target.exists() {
            return Err(ProjectError::ProjectPathExists(display.to_owned()));
        }
        self.ensure_contained_parent(target, display)
    }

    fn ensure_contained_parent(&self, target: &Path, display: &str) -> Result<(), ProjectError> {
        let parent = target
            .parent()
            .ok_or_else(|| ProjectError::InvalidProjectPath(display.to_owned()))?;
        if !parent.is_dir() {
            return Err(ProjectError::ProjectPathNotFound(path_string(
                parent.strip_prefix(&self.root).unwrap_or(parent),
            )));
        }
        self.ensure_contained_existing(parent, display)
    }

    fn ensure_contained_existing(&self, target: &Path, display: &str) -> Result<(), ProjectError> {
        let root = fs::canonicalize(&self.root).map_err(|source| io_at(&self.root, source))?;
        let resolved = fs::canonicalize(target).map_err(|source| io_at(target, source))?;
        if resolved.starts_with(root) {
            Ok(())
        } else {
            Err(ProjectError::SymlinkEscape(display.to_owned()))
        }
    }
}

pub(crate) fn validate_relative(value: &str) -> Result<PathBuf, ProjectError> {
    let path = Path::new(value);
    if value.is_empty() || path.is_absolute() {
        return Err(ProjectError::InvalidProjectPath(value.to_owned()));
    }
    for component in path.components() {
        let Component::Normal(part) = component else {
            return Err(ProjectError::InvalidProjectPath(value.to_owned()));
        };
        let name = part.to_string_lossy();
        if name.starts_with('.') || RESERVED.contains(&name.as_ref()) {
            return Err(ProjectError::ReservedProjectPath(value.to_owned()));
        }
    }
    Ok(path.to_path_buf())
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp"
            )
        })
}
