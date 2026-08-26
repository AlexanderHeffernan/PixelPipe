use std::fs;

use pixelpipe_core::sha256_hex;

use crate::{
    ASSET_SCHEMA, ProjectError, ProjectStore, REFERENCE_SELECTION_SCHEMA, ReferenceSelection,
    StoredReference,
    assets::{state_for, validate_asset_id},
    persistence::{atomic_write, io_at, now_unix_ms},
};

impl ProjectStore {
    /// Imports original PNG bytes into the asset's content-addressed reference store.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when the asset ID is invalid, storage fails, or
    /// an existing immutable object does not match its hash-derived path.
    pub fn import_reference(
        &self,
        asset_id: &str,
        png_bytes: &[u8],
    ) -> Result<StoredReference, ProjectError> {
        validate_asset_id(asset_id)?;
        let sha256 = sha256_hex(png_bytes);
        let references = self.asset_path(asset_id).join("references/selected");
        let path = references.join(format!("{sha256}.png"));
        if path.exists() {
            let existing = fs::read(&path).map_err(|source| io_at(&path, source))?;
            if existing != png_bytes {
                return Err(ProjectError::ReferenceHashMismatch(path));
            }
            return Ok(StoredReference { sha256, path });
        }
        fs::create_dir_all(&references).map_err(|source| io_at(&references, source))?;
        atomic_write(&path, png_bytes)?;
        Ok(StoredReference { sha256, path })
    }

    /// Selects user-imported PNG bytes as the asset reference.
    ///
    /// The bytes are stored by content hash before the asset lifecycle advances.
    ///
    /// # Errors
    ///
    /// Returns an error when the asset is not ready or storage fails.
    pub fn select_imported_reference(
        &self,
        asset: &str,
        png_bytes: &[u8],
    ) -> Result<ReferenceSelection, ProjectError> {
        let _lock = self.lock()?;
        let mut manifest = self.asset(asset)?;
        if manifest.head.is_none() && manifest.brief.text.trim().is_empty() {
            return Err(ProjectError::AssetNotReady {
                asset: asset.to_owned(),
                operation: "select reference",
                reason: "write a non-empty brief first",
            });
        }
        let stored = self.import_reference(asset, png_bytes)?;
        let selection = ReferenceSelection {
            schema: REFERENCE_SELECTION_SCHEMA.to_owned(),
            asset: asset.to_owned(),
            run: "import".to_owned(),
            candidate: "local-file".to_owned(),
            sha256: stored.sha256,
            selected_unix_ms: now_unix_ms()?,
        };
        ASSET_SCHEMA.clone_into(&mut manifest.schema);
        manifest.selected_reference = Some(selection.clone());
        manifest.state = state_for(
            &manifest.brief.text,
            manifest.selected_reference.as_ref(),
            manifest.head.as_deref(),
        );
        atomic_write(
            &self.asset_path(asset).join("asset.toml"),
            toml::to_string_pretty(&manifest)?.as_bytes(),
        )?;
        Ok(selection)
    }

    /// Loads the selected reference only after verifying its content-addressed bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when no selection exists or its stored bytes/hash are invalid.
    pub fn selected_reference(
        &self,
        asset: &str,
    ) -> Result<(ReferenceSelection, Vec<u8>), ProjectError> {
        let manifest = self.asset(asset)?;
        let selection = manifest
            .selected_reference
            .ok_or_else(|| ProjectError::AssetNotReady {
                asset: asset.to_owned(),
                operation: "convert selected reference",
                reason: "select a validated reference first",
            })?;
        let path = self
            .asset_path(asset)
            .join("references/selected")
            .join(format!("{}.png", selection.sha256));
        let bytes = fs::read(&path).map_err(|source| io_at(&path, source))?;
        if sha256_hex(&bytes) != selection.sha256 {
            return Err(ProjectError::ReferenceHashMismatch(path));
        }
        Ok((selection, bytes))
    }
}
