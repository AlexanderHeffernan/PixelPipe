use std::fs;

use crate::{
    ASSET_BRIEF_SCHEMA, ASSET_SCHEMA, AssetBrief, AssetManifest, ProjectError, ProjectStore,
    persistence::{atomic_write, ensure_schema, io_at, write_file},
};

impl ProjectStore {
    /// Loads and schema-checks an asset manifest.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when the ID is invalid or the manifest cannot
    /// be read, parsed, or schema-checked.
    pub fn asset(&self, id: &str) -> Result<AssetManifest, ProjectError> {
        validate_asset_id(id)?;
        let path = self.asset_path(id).join("asset.toml");
        let contents = fs::read_to_string(&path).map_err(|source| io_at(&path, source))?;
        let asset: AssetManifest = toml::from_str(&contents)?;
        ensure_schema(&asset.schema, ASSET_SCHEMA)?;
        if asset.id != id {
            return Err(ProjectError::AssetIdentityMismatch {
                expected: id.to_owned(),
                actual: asset.id,
            });
        }
        validate_asset_manifest(&asset)?;
        Ok(asset)
    }

    /// Loads an asset when its manifest exists without treating absence as corruption.
    ///
    /// # Errors
    ///
    /// Returns an error when the ID or an existing manifest is invalid.
    pub fn optional_asset(&self, id: &str) -> Result<Option<AssetManifest>, ProjectError> {
        validate_asset_id(id)?;
        if self.asset_path(id).join("asset.toml").is_file() {
            self.asset(id).map(Some)
        } else {
            Ok(None)
        }
    }

    /// Creates a stable asset identity before any revision exists.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid/duplicate ID or failed atomic storage.
    pub fn create_asset(&self, id: &str, brief: &str) -> Result<AssetManifest, ProjectError> {
        validate_asset_id(id)?;
        let _lock = self.lock()?;
        let path = self.asset_path(id);
        if path.exists() {
            return Err(ProjectError::AssetExists(id.to_owned()));
        }
        let staging = self
            .root
            .join(".pixelate/tmp")
            .join(format!("asset-{id}-{}", std::process::id()));
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(|source| io_at(&staging, source))?;
        }
        fs::create_dir_all(staging.join("revisions")).map_err(|source| io_at(&staging, source))?;
        let asset = AssetManifest {
            schema: ASSET_SCHEMA.to_owned(),
            id: id.to_owned(),
            display_name: None,
            project_path: None,
            project_file_sha256: None,
            brief: AssetBrief {
                schema: ASSET_BRIEF_SCHEMA.to_owned(),
                text: brief.to_owned(),
            },
            selected_reference: None,
            head: None,
            style: None,
        };
        write_file(
            &staging.join("asset.toml"),
            toml::to_string_pretty(&asset)?.as_bytes(),
        )?;
        fs::rename(&staging, &path).map_err(|source| io_at(&path, source))?;
        Ok(asset)
    }

    /// Permanently removes one complete project asset and its immutable history.
    ///
    /// # Errors
    ///
    /// Returns an error when the asset ID is invalid, missing, or cannot be removed.
    pub fn delete_asset(&self, id: &str) -> Result<(), ProjectError> {
        validate_asset_id(id)?;
        let _lock = self.lock()?;
        let path = self.asset_path(id);
        if !path.join("asset.toml").is_file() {
            return Err(ProjectError::AssetNotReady {
                asset: id.to_owned(),
                operation: "delete",
                reason: "the asset does not exist",
            });
        }
        fs::remove_dir_all(&path).map_err(|source| io_at(&path, source))
    }

    /// Replaces the project-owned brief without changing a revision or head.
    ///
    /// # Errors
    ///
    /// Returns an error when the asset cannot be loaded, validated, or stored.
    pub fn set_asset_brief(&self, id: &str, brief: &str) -> Result<AssetManifest, ProjectError> {
        let _lock = self.lock()?;
        let mut asset = self.asset(id)?;
        if asset.head.is_none() && asset.selected_reference.is_some() && brief.trim().is_empty() {
            return Err(ProjectError::AssetNotReady {
                asset: id.to_owned(),
                operation: "update brief",
                reason: "a selected reference requires a non-empty brief",
            });
        }
        ASSET_SCHEMA.clone_into(&mut asset.schema);
        asset.brief = AssetBrief {
            schema: ASSET_BRIEF_SCHEMA.to_owned(),
            text: brief.to_owned(),
        };
        atomic_write(
            &self.asset_path(id).join("asset.toml"),
            toml::to_string_pretty(&asset)?.as_bytes(),
        )?;
        Ok(asset)
    }

    /// Changes the project-owned display name without changing stable asset identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the name is empty or the manifest cannot be stored.
    pub fn set_asset_display_name(
        &self,
        id: &str,
        display_name: &str,
    ) -> Result<AssetManifest, ProjectError> {
        let display_name = display_name.trim();
        if display_name.is_empty() {
            return Err(ProjectError::AssetNotReady {
                asset: id.to_owned(),
                operation: "rename",
                reason: "the display name cannot be empty",
            });
        }
        let _lock = self.lock()?;
        let mut asset = self.asset(id)?;
        ASSET_SCHEMA.clone_into(&mut asset.schema);
        asset.display_name = Some(display_name.to_owned());
        atomic_write(
            &self.asset_path(id).join("asset.toml"),
            toml::to_string_pretty(&asset)?.as_bytes(),
        )?;
        Ok(asset)
    }

    /// Links an asset to a verified project file without changing its identity or history.
    ///
    /// # Errors
    ///
    /// Returns an error when the path is unsafe, missing, unsupported, or cannot be stored.
    pub fn link_asset_project_path(
        &self,
        id: &str,
        project_path: &str,
    ) -> Result<AssetManifest, ProjectError> {
        let relative = self.validate_project_file(project_path)?;
        let hash = pixelate_core::sha256_hex(
            &fs::read(self.root.join(&relative))
                .map_err(|source| io_at(&self.root.join(&relative), source))?,
        );
        let _lock = self.lock()?;
        if self.assets()?.iter().any(|asset| {
            asset.id != id && asset.project_path.as_deref() == Some(path_string(&relative).as_str())
        }) {
            return Err(ProjectError::ProjectPathExists(path_string(&relative)));
        }
        let mut asset = self.asset(id)?;
        asset.project_path = Some(path_string(&relative));
        asset.project_file_sha256 = Some(hash);
        self.write_asset_manifest(&asset)?;
        Ok(asset)
    }

    /// Assigns a future project image location without creating or overwriting the file.
    ///
    /// # Errors
    ///
    /// Returns an error when the path is unsafe, unsupported, already claimed, or cannot be stored.
    pub fn plan_asset_project_path(
        &self,
        id: &str,
        project_path: &str,
    ) -> Result<AssetManifest, ProjectError> {
        let relative = crate::catalog::validate_relative(project_path)?;
        if !crate::catalog::is_supported_image(&relative) {
            return Err(ProjectError::InvalidProjectPath(project_path.to_owned()));
        }
        let path = path_string(&relative);
        let target = self.root.join(&relative);
        self.ensure_contained_parent(&target, &path)?;
        if target.exists() {
            return Err(ProjectError::ProjectPathExists(path));
        }
        let _lock = self.lock()?;
        if self
            .assets()?
            .iter()
            .any(|asset| asset.id != id && asset.project_path.as_deref() == Some(path.as_str()))
        {
            return Err(ProjectError::ProjectPathExists(path));
        }
        let mut asset = self.asset(id)?;
        asset.project_path = Some(path);
        asset.project_file_sha256 = None;
        self.write_asset_manifest(&asset)?;
        Ok(asset)
    }

    /// Clears an asset's project-file link while retaining Pixelate history.
    ///
    /// # Errors
    ///
    /// Returns an error when the asset cannot be loaded or stored.
    pub fn unlink_asset_project_path(&self, id: &str) -> Result<AssetManifest, ProjectError> {
        let _lock = self.lock()?;
        let mut asset = self.asset(id)?;
        asset.project_path = None;
        asset.project_file_sha256 = None;
        self.write_asset_manifest(&asset)?;
        Ok(asset)
    }

    /// Lists initialized asset manifests in stable asset-ID order.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when the asset directory or a manifest is invalid.
    pub fn assets(&self) -> Result<Vec<AssetManifest>, ProjectError> {
        let path = self.root.join(".pixelate/assets");
        let mut ids = Vec::new();
        for entry in fs::read_dir(&path).map_err(|source| io_at(&path, source))? {
            let entry = entry.map_err(|source| io_at(&path, source))?;
            if entry.path().join("asset.toml").is_file() {
                ids.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        ids.sort();
        ids.into_iter().map(|id| self.asset(&id)).collect()
    }
}

impl ProjectStore {
    pub(crate) fn write_asset_manifest(&self, asset: &AssetManifest) -> Result<(), ProjectError> {
        atomic_write(
            &self.asset_path(&asset.id).join("asset.toml"),
            toml::to_string_pretty(asset)?.as_bytes(),
        )
    }
}

pub(crate) fn path_string(path: &std::path::Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub(crate) fn validate_asset_id(id: &str) -> Result<(), ProjectError> {
    let valid = !id.is_empty()
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(ProjectError::InvalidAssetId(id.to_owned()))
    }
}

fn validate_asset_manifest(asset: &AssetManifest) -> Result<(), ProjectError> {
    ensure_schema(&asset.brief.schema, ASSET_BRIEF_SCHEMA)?;
    if asset.head.is_none()
        && asset.brief.text.trim().is_empty()
        && asset.selected_reference.is_some()
    {
        return Err(ProjectError::AssetNotReady {
            asset: asset.id.clone(),
            operation: "load asset",
            reason: "a selected reference requires a non-empty brief",
        });
    }
    Ok(())
}
