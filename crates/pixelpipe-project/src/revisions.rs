use std::{collections::BTreeMap, fs, path::Path};

use pixelpipe_core::{
    IndexedRaster, RECIPE_SCHEMA, Recipe, VALIDATION_SCHEMA, ValidationReport, sha256_hex,
    stable_json,
};

use crate::{
    ASSET_BRIEF_SCHEMA, ASSET_SCHEMA, AssetBrief, AssetKind, AssetManifest, AssetState,
    PROVENANCE_SCHEMA, ProjectError, ProjectStore, Provenance, REVISION_PAYLOADS, REVISION_SCHEMA,
    RevisionFiles, RevisionManifest, RevisionSnapshot, StoredRevision,
    assets::validate_asset_id,
    persistence::{atomic_write, ensure_schema, io_at, now_unix_ms, read_json, write_file},
};

struct PreparedRevision {
    brief: Vec<u8>,
    raster: Vec<u8>,
    recipe: Vec<u8>,
    validation: Vec<u8>,
    provenance: Vec<u8>,
    manifest: Vec<u8>,
    native_png: Vec<u8>,
    preview_png: Vec<u8>,
}

impl ProjectStore {
    /// Lists revision manifests in stable revision-ID order.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when the asset or revision manifests are invalid.
    pub fn revisions(&self, asset_id: &str) -> Result<Vec<RevisionManifest>, ProjectError> {
        validate_asset_id(asset_id)?;
        self.asset(asset_id)?;
        let path = self.asset_path(asset_id).join("revisions");
        let mut revisions = Vec::new();
        for entry in fs::read_dir(&path).map_err(|source| io_at(&path, source))? {
            let entry = entry.map_err(|source| io_at(&path, source))?;
            if !entry.path().is_dir() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().into_owned();
            validate_revision_id(&id)?;
            let manifest: RevisionManifest = read_json(&entry.path().join("revision.json"))?;
            ensure_schema(&manifest.schema, REVISION_SCHEMA)?;
            if manifest.asset != asset_id || manifest.id != id {
                return Err(ProjectError::RevisionIdentityMismatch);
            }
            revisions.push(manifest);
        }
        revisions.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(revisions)
    }

    /// Moves an asset head to an existing immutable revision.
    ///
    /// This powers explicit undo, redo, and branch navigation without changing
    /// revision contents or history.
    ///
    /// # Errors
    ///
    /// Returns an error when the asset or target revision is invalid or storage fails.
    pub fn set_asset_head(
        &self,
        asset_id: &str,
        revision: &str,
    ) -> Result<AssetManifest, ProjectError> {
        self.revision(asset_id, revision)?;
        let _lock = self.lock()?;
        let mut asset = self.asset(asset_id)?;
        ASSET_SCHEMA.clone_into(&mut asset.schema);
        asset.head = Some(revision.to_owned());
        asset.state = AssetState::Revisioned;
        atomic_write(
            &self.asset_path(asset_id).join("asset.toml"),
            toml::to_string_pretty(&asset)?.as_bytes(),
        )?;
        Ok(asset)
    }

    /// Loads and hash-verifies one immutable revision snapshot.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when identity, schema, payload, or file hashes
    /// are invalid, or the revision does not exist.
    pub fn revision(
        &self,
        asset_id: &str,
        revision: &str,
    ) -> Result<RevisionSnapshot, ProjectError> {
        validate_asset_id(asset_id)?;
        validate_revision_id(revision)?;
        let path = self.asset_path(asset_id).join("revisions").join(revision);
        if !path.is_dir() {
            return Err(ProjectError::RevisionNotFound {
                asset: asset_id.to_owned(),
                revision: revision.to_owned(),
            });
        }
        let manifest: RevisionManifest = read_json(&path.join("revision.json"))?;
        ensure_schema(&manifest.schema, REVISION_SCHEMA)?;
        if manifest.asset != asset_id || manifest.id != revision {
            return Err(ProjectError::RevisionIdentityMismatch);
        }
        if manifest.files.len() != REVISION_PAYLOADS.len()
            || !REVISION_PAYLOADS
                .iter()
                .all(|name| manifest.files.contains_key(*name))
        {
            return Err(ProjectError::InvalidRevisionFiles);
        }
        if let Some(parent) = &manifest.parent {
            validate_revision_id(parent)?;
        }
        for (name, expected) in &manifest.files {
            let payload_path = path.join(name);
            let bytes = fs::read(&payload_path).map_err(|source| io_at(&payload_path, source))?;
            if sha256_hex(&bytes) != *expected {
                return Err(ProjectError::RevisionHashMismatch { name: name.clone() });
            }
        }
        let raster: IndexedRaster = read_json(&path.join("pixels.json"))?;
        raster.validate()?;
        let recipe: Recipe = read_json(&path.join("recipe.json"))?;
        ensure_schema(&recipe.schema, RECIPE_SCHEMA)?;
        let validation: ValidationReport = read_json(&path.join("validation.json"))?;
        ensure_schema(&validation.schema, VALIDATION_SCHEMA)?;
        let provenance: Provenance = read_json(&path.join("provenance.json"))?;
        ensure_schema(&provenance.schema, PROVENANCE_SCHEMA)?;
        if provenance.revision != revision {
            return Err(ProjectError::RevisionIdentityMismatch);
        }
        let brief = String::from_utf8(
            fs::read(path.join("brief.md"))
                .map_err(|source| io_at(&path.join("brief.md"), source))?,
        )
        .map_err(|_| ProjectError::InvalidBriefUtf8)?;
        let native_png = fs::read(path.join("native.png"))
            .map_err(|source| io_at(&path.join("native.png"), source))?;
        let preview_png = fs::read(path.join("preview.png"))
            .map_err(|source| io_at(&path.join("preview.png"), source))?;
        Ok(RevisionSnapshot {
            path,
            manifest,
            raster,
            recipe,
            validation,
            provenance,
            brief,
            native_png,
            preview_png,
        })
    }

    /// Atomically publishes a complete immutable revision and advances asset head.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when validation, locking, serialization, or any
    /// filesystem operation fails. Existing revision directories are never
    /// overwritten.
    pub fn create_revision(
        &self,
        asset_id: &str,
        kind: AssetKind,
        files: RevisionFiles,
    ) -> Result<StoredRevision, ProjectError> {
        self.create_revision_selected(asset_id, kind, None, files)
    }

    /// Atomically creates a revision from an explicit immutable parent.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when the parent does not exist or revision
    /// validation, locking, serialization, or storage fails.
    pub fn create_revision_from(
        &self,
        asset_id: &str,
        kind: AssetKind,
        parent_revision: &str,
        files: RevisionFiles,
    ) -> Result<StoredRevision, ProjectError> {
        self.create_revision_selected(asset_id, kind, Some(parent_revision), files)
    }

    fn create_revision_selected(
        &self,
        asset_id: &str,
        kind: AssetKind,
        parent_revision: Option<&str>,
        files: RevisionFiles,
    ) -> Result<StoredRevision, ProjectError> {
        validate_asset_id(asset_id)?;
        files.raster.validate()?;
        ensure_schema(&files.recipe.schema, RECIPE_SCHEMA)?;
        ensure_schema(&files.validation.schema, VALIDATION_SCHEMA)?;
        let lock = self.lock()?;

        let asset_path = self.asset_path(asset_id);
        let manifest_path = asset_path.join("asset.toml");
        let mut asset = if manifest_path.exists() {
            self.asset(asset_id)?
        } else {
            fs::create_dir_all(asset_path.join("revisions"))
                .map_err(|source| io_at(&asset_path, source))?;
            AssetManifest {
                schema: ASSET_SCHEMA.to_owned(),
                id: asset_id.to_owned(),
                display_name: None,
                kind,
                state: AssetState::Draft,
                brief: AssetBrief {
                    schema: ASSET_BRIEF_SCHEMA.to_owned(),
                    text: files.brief.clone(),
                },
                selected_reference: None,
                head: None,
                approved: None,
                style: None,
            }
        };
        if asset.kind != kind {
            return Err(ProjectError::AssetKindMismatch {
                asset: asset_id.to_owned(),
                existing: asset.kind,
                requested: kind,
            });
        }

        let parent = match parent_revision {
            Some(parent) => {
                self.revision(asset_id, parent)?;
                Some(parent.to_owned())
            }
            None => asset.head.clone(),
        };
        let revision = next_revision(&asset_path)?;
        let revision_path = asset_path.join("revisions").join(&revision);
        if revision_path.exists() {
            return Err(ProjectError::RevisionExists(revision_path));
        }
        let created_unix_ms = now_unix_ms()?;
        let project_brief = files.brief.clone();
        let style = files.style.clone();
        let prepared = prepare_revision(
            asset_id,
            &revision,
            parent.as_deref(),
            created_unix_ms,
            files,
        )?;
        let staging = self
            .root
            .join(".pixelpipe/tmp")
            .join(format!("{asset_id}-{revision}-{}", std::process::id()));
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(|source| io_at(&staging, source))?;
        }
        fs::create_dir_all(&staging).map_err(|source| io_at(&staging, source))?;
        write_prepared_revision(&staging, &prepared)?;

        fs::rename(&staging, &revision_path).map_err(|source| io_at(&revision_path, source))?;
        ASSET_SCHEMA.clone_into(&mut asset.schema);
        if asset.brief.text.is_empty() {
            asset.brief = AssetBrief {
                schema: ASSET_BRIEF_SCHEMA.to_owned(),
                text: project_brief,
            };
        }
        asset.head = Some(revision.clone());
        asset.state = AssetState::Revisioned;
        if style.is_some() {
            asset.style = style;
        }
        atomic_write(&manifest_path, toml::to_string_pretty(&asset)?.as_bytes())?;
        drop(lock);

        Ok(StoredRevision {
            project_root: self.root.clone(),
            asset: asset_id.to_owned(),
            revision,
            parent,
            revision_path,
        })
    }
}

fn prepare_revision(
    asset_id: &str,
    revision: &str,
    parent: Option<&str>,
    created_unix_ms: u64,
    files: RevisionFiles,
) -> Result<PreparedRevision, ProjectError> {
    let provenance = Provenance {
        schema: PROVENANCE_SCHEMA.to_owned(),
        revision: revision.to_owned(),
        actor: files.actor,
        engine_version: env!("CARGO_PKG_VERSION").to_owned(),
        created_unix_ms,
        inputs: files.input_hashes,
        outputs: files.output_hashes.clone(),
    };
    let brief = files.brief.into_bytes();
    let raster = stable_json(&files.raster)?;
    let recipe = stable_json(&files.recipe)?;
    let validation = stable_json(&files.validation)?;
    let provenance = stable_json(&provenance)?;
    let persisted_hashes = BTreeMap::from([
        ("brief.md".to_owned(), sha256_hex(&brief)),
        ("native.png".to_owned(), sha256_hex(&files.native_png)),
        ("pixels.json".to_owned(), sha256_hex(&raster)),
        ("preview.png".to_owned(), sha256_hex(&files.preview_png)),
        ("provenance.json".to_owned(), sha256_hex(&provenance)),
        ("recipe.json".to_owned(), sha256_hex(&recipe)),
        ("validation.json".to_owned(), sha256_hex(&validation)),
    ]);
    for (name, expected) in &files.output_hashes {
        if persisted_hashes.get(name) != Some(expected) {
            return Err(ProjectError::OutputHashMismatch { name: name.clone() });
        }
    }
    let manifest = stable_json(&RevisionManifest {
        schema: REVISION_SCHEMA.to_owned(),
        id: revision.to_owned(),
        asset: asset_id.to_owned(),
        parent: parent.map(str::to_owned),
        created_unix_ms,
        files: persisted_hashes,
    })?;

    Ok(PreparedRevision {
        brief,
        raster,
        recipe,
        validation,
        provenance,
        manifest,
        native_png: files.native_png,
        preview_png: files.preview_png,
    })
}

fn write_prepared_revision(path: &Path, files: &PreparedRevision) -> Result<(), ProjectError> {
    write_file(&path.join("brief.md"), &files.brief)?;
    write_file(&path.join("pixels.json"), &files.raster)?;
    write_file(&path.join("recipe.json"), &files.recipe)?;
    write_file(&path.join("validation.json"), &files.validation)?;
    write_file(&path.join("provenance.json"), &files.provenance)?;
    write_file(&path.join("revision.json"), &files.manifest)?;
    write_file(&path.join("native.png"), &files.native_png)?;
    write_file(&path.join("preview.png"), &files.preview_png)
}

fn next_revision(asset_path: &Path) -> Result<String, ProjectError> {
    let revisions = asset_path.join("revisions");
    fs::create_dir_all(&revisions).map_err(|source| io_at(&revisions, source))?;
    let mut highest = 0_u32;
    for entry in fs::read_dir(&revisions).map_err(|source| io_at(&revisions, source))? {
        let entry = entry.map_err(|source| io_at(&revisions, source))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(number) = name.strip_prefix('r').and_then(|value| value.parse().ok()) {
            highest = highest.max(number);
        }
    }
    Ok(format!("r{:06}", highest + 1))
}

fn validate_revision_id(id: &str) -> Result<(), ProjectError> {
    let valid =
        id.len() == 7 && id.starts_with('r') && id[1..].bytes().all(|byte| byte.is_ascii_digit());
    if valid {
        Ok(())
    } else {
        Err(ProjectError::InvalidRevisionId(id.to_owned()))
    }
}
