use std::fs;

use crate::{
    ASSET_BRIEF_SCHEMA, ASSET_SCHEMA, AssetBrief, AssetKind, AssetManifest, AssetState,
    LEGACY_ASSET_SCHEMA, ProjectError, ProjectStore, REFERENCE_SELECTION_SCHEMA,
    ReferenceSelection,
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
        let mut asset: AssetManifest = toml::from_str(&contents)?;
        if asset.schema != ASSET_SCHEMA && asset.schema != LEGACY_ASSET_SCHEMA {
            return Err(ProjectError::Schema {
                expected: ASSET_SCHEMA,
                actual: asset.schema,
            });
        }
        if asset.id != id {
            return Err(ProjectError::AssetIdentityMismatch {
                expected: id.to_owned(),
                actual: asset.id,
            });
        }
        if asset.schema == LEGACY_ASSET_SCHEMA {
            asset.state = if asset.head.is_some() {
                AssetState::Revisioned
            } else {
                AssetState::Draft
            };
        } else {
            validate_asset_state(&asset)?;
        }
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
    pub fn create_asset(
        &self,
        id: &str,
        kind: AssetKind,
        brief: &str,
    ) -> Result<AssetManifest, ProjectError> {
        validate_asset_id(id)?;
        let _lock = self.lock()?;
        let path = self.asset_path(id);
        if path.exists() {
            return Err(ProjectError::AssetExists(id.to_owned()));
        }
        let staging = self
            .root
            .join(".pixelpipe/tmp")
            .join(format!("asset-{id}-{}", std::process::id()));
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(|source| io_at(&staging, source))?;
        }
        fs::create_dir_all(staging.join("revisions")).map_err(|source| io_at(&staging, source))?;
        let asset = AssetManifest {
            schema: ASSET_SCHEMA.to_owned(),
            id: id.to_owned(),
            display_name: None,
            kind,
            state: state_for(brief, None, None),
            brief: AssetBrief {
                schema: ASSET_BRIEF_SCHEMA.to_owned(),
                text: brief.to_owned(),
            },
            selected_reference: None,
            head: None,
            approved: None,
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
        asset.state = state_for(
            brief,
            asset.selected_reference.as_ref(),
            asset.head.as_deref(),
        );
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

    /// Lists initialized asset manifests in stable asset-ID order.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when the asset directory or a manifest is invalid.
    pub fn assets(&self) -> Result<Vec<AssetManifest>, ProjectError> {
        let path = self.root.join(".pixelpipe/assets");
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

pub(crate) fn state_for(
    brief: &str,
    selection: Option<&ReferenceSelection>,
    head: Option<&str>,
) -> AssetState {
    if head.is_some() {
        AssetState::Revisioned
    } else if selection.is_some() {
        AssetState::SelectedReference
    } else if brief.trim().is_empty() {
        AssetState::Draft
    } else {
        AssetState::AwaitingReference
    }
}

fn validate_asset_state(asset: &AssetManifest) -> Result<(), ProjectError> {
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
    let expected = state_for(
        &asset.brief.text,
        asset.selected_reference.as_ref(),
        asset.head.as_deref(),
    );
    if asset.state != expected {
        return Err(ProjectError::AssetNotReady {
            asset: asset.id.clone(),
            operation: "load asset",
            reason: "serialized lifecycle state does not match brief, selection, and head",
        });
    }
    if let Some(selection) = &asset.selected_reference {
        ensure_schema(&selection.schema, REFERENCE_SELECTION_SCHEMA)?;
        if selection.asset != asset.id {
            return Err(ProjectError::AssetIdentityMismatch {
                expected: asset.id.clone(),
                actual: selection.asset.clone(),
            });
        }
    }
    Ok(())
}
