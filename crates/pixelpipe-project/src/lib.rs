use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use atomicwrites::{AllowOverwrite, AtomicFile};
use fs2::FileExt;
use pixelpipe_core::{
    IndexedRaster, RECIPE_SCHEMA, Recipe, VALIDATION_SCHEMA, ValidationReport, sha256_hex,
    stable_json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PROJECT_SCHEMA: &str = "pixelpipe.project/v1";
pub const ASSET_SCHEMA: &str = "pixelpipe.asset/v1";
pub const REVISION_SCHEMA: &str = "pixelpipe.revision/v1";
pub const PROVENANCE_SCHEMA: &str = "pixelpipe.provenance/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectManifest {
    pub schema: String,
    pub name: String,
    pub preview_scale: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_palette: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exports: Vec<ExportMapping>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportMapping {
    pub asset: String,
    pub png: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetManifest {
    pub schema: String,
    pub id: String,
    pub kind: AssetKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Sprite,
    Sheet,
    Tile,
    Ui,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionManifest {
    pub schema: String,
    pub id: String,
    pub asset: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    pub created_unix_ms: u64,
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub schema: String,
    pub revision: String,
    pub actor: String,
    pub engine_version: String,
    pub created_unix_ms: u64,
    pub inputs: BTreeMap<String, String>,
    pub outputs: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct RevisionFiles {
    pub raster: IndexedRaster,
    pub recipe: Recipe,
    pub validation: ValidationReport,
    pub native_png: Vec<u8>,
    pub preview_png: Vec<u8>,
    pub brief: String,
    pub actor: String,
    pub input_hashes: BTreeMap<String, String>,
    pub output_hashes: BTreeMap<String, String>,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredRevision {
    pub project_root: PathBuf,
    pub asset: String,
    pub revision: String,
    pub parent: Option<String>,
    pub revision_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredReference {
    pub sha256: String,
    pub path: PathBuf,
}

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("project already exists at {0}")]
    AlreadyExists(PathBuf),
    #[error("no .pixelpipe/project.toml found from {0}")]
    NotFound(PathBuf),
    #[error("unsupported schema '{actual}', expected '{expected}'")]
    Schema {
        expected: &'static str,
        actual: String,
    },
    #[error("invalid asset id '{0}'; use lowercase letters, numbers, and single hyphens")]
    InvalidAssetId(String),
    #[error("asset manifest identity '{actual}' does not match path identity '{expected}'")]
    AssetIdentityMismatch { expected: String, actual: String },
    #[error("asset '{asset}' already exists with kind {existing:?}, not {requested:?}")]
    AssetKindMismatch {
        asset: String,
        existing: AssetKind,
        requested: AssetKind,
    },
    #[error("revision directory already exists: {0}")]
    RevisionExists(PathBuf),
    #[error("rendered output hash does not match bytes for '{name}'")]
    OutputHashMismatch { name: String },
    #[error("stored reference does not match its content-addressed path: {0}")]
    ReferenceHashMismatch(PathBuf),
    #[error("TOML error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("invalid TOML: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("core encoding error: {0}")]
    Core(#[from] pixelpipe_core::CoreError),
    #[error("system clock is before the Unix epoch")]
    Clock,
    #[error("atomic write failed: {0}")]
    Atomic(String),
}

#[derive(Debug, Clone)]
pub struct ProjectStore {
    root: PathBuf,
}

impl ProjectStore {
    /// Creates a new `.pixelpipe` project at `root`.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] if a project already exists or its directories
    /// and initial manifests cannot be written.
    pub fn init(root: &Path, name: &str) -> Result<Self, ProjectError> {
        let root = absolute(root)?;
        let pixelpipe = root.join(".pixelpipe");
        if pixelpipe.exists() {
            return Err(ProjectError::AlreadyExists(pixelpipe));
        }

        fs::create_dir_all(pixelpipe.join("assets")).map_err(|source| io_at(&pixelpipe, source))?;
        fs::create_dir_all(pixelpipe.join("palettes"))
            .map_err(|source| io_at(&pixelpipe, source))?;
        fs::create_dir_all(pixelpipe.join("tmp")).map_err(|source| io_at(&pixelpipe, source))?;

        let manifest = ProjectManifest {
            schema: PROJECT_SCHEMA.to_owned(),
            name: name.to_owned(),
            preview_scale: 8,
            default_palette: None,
            exports: Vec::new(),
        };
        atomic_write(
            &pixelpipe.join("project.toml"),
            toml::to_string_pretty(&manifest)?.as_bytes(),
        )?;
        atomic_write(
            &pixelpipe.join(".gitignore"),
            b"/.lock\n/cache/\n/tmp/\n/review/\n/runs/\n/assets/*/references/\n",
        )?;

        Ok(Self { root })
    }

    /// Finds the nearest project by walking from `start` toward the filesystem root.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] if no project exists or its manifest is invalid.
    pub fn discover(start: &Path) -> Result<Self, ProjectError> {
        let start = absolute(start)?;
        let start = if start.is_file() {
            start.parent().unwrap_or(&start).to_path_buf()
        } else {
            start
        };
        for candidate in start.ancestors() {
            if candidate.join(".pixelpipe/project.toml").is_file() {
                let store = Self {
                    root: candidate.to_path_buf(),
                };
                store.manifest()?;
                return Ok(store);
            }
        }
        Err(ProjectError::NotFound(start))
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Loads and schema-checks the project manifest.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when the manifest cannot be read or parsed, or
    /// uses an unsupported schema.
    pub fn manifest(&self) -> Result<ProjectManifest, ProjectError> {
        let path = self.root.join(".pixelpipe/project.toml");
        let contents = fs::read_to_string(&path).map_err(|source| io_at(&path, source))?;
        let manifest: ProjectManifest = toml::from_str(&contents)?;
        ensure_schema(&manifest.schema, PROJECT_SCHEMA)?;
        Ok(manifest)
    }

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
        Ok(asset)
    }

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
        validate_asset_id(asset_id)?;
        files.raster.validate()?;
        ensure_schema(&files.recipe.schema, RECIPE_SCHEMA)?;
        ensure_schema(&files.validation.schema, VALIDATION_SCHEMA)?;
        let lock_path = self.root.join(".pixelpipe/.lock");
        let lock = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| io_at(&lock_path, source))?;
        lock.lock_exclusive()
            .map_err(|source| io_at(&lock_path, source))?;

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
                kind,
                head: None,
                approved: None,
            }
        };
        if asset.kind != kind {
            return Err(ProjectError::AssetKindMismatch {
                asset: asset_id.to_owned(),
                existing: asset.kind,
                requested: kind,
            });
        }

        let parent = asset.head.clone();
        let revision = next_revision(&asset_path)?;
        let revision_path = asset_path.join("revisions").join(&revision);
        if revision_path.exists() {
            return Err(ProjectError::RevisionExists(revision_path));
        }
        let created_unix_ms = u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| ProjectError::Clock)?
                .as_millis(),
        )
        .map_err(|_| ProjectError::Clock)?;
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
        asset.head = Some(revision.clone());
        atomic_write(&manifest_path, toml::to_string_pretty(&asset)?.as_bytes())?;

        Ok(StoredRevision {
            project_root: self.root.clone(),
            asset: asset_id.to_owned(),
            revision,
            parent,
            revision_path,
        })
    }

    fn asset_path(&self, id: &str) -> PathBuf {
        self.root.join(".pixelpipe/assets").join(id)
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

fn validate_asset_id(id: &str) -> Result<(), ProjectError> {
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

fn ensure_schema(actual: &str, expected: &'static str) -> Result<(), ProjectError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ProjectError::Schema {
            expected,
            actual: actual.to_owned(),
        })
    }
}

fn absolute(path: &Path) -> Result<PathBuf, ProjectError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|source| io_at(path, source))
    }
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), ProjectError> {
    let mut file = File::create(path).map_err(|source| io_at(path, source))?;
    file.write_all(bytes)
        .map_err(|source| io_at(path, source))?;
    file.sync_all().map_err(|source| io_at(path, source))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ProjectError> {
    AtomicFile::new(path, AllowOverwrite)
        .write(|file| file.write_all(bytes))
        .map_err(|error| ProjectError::Atomic(error.to_string()))
}

fn io_at(path: &Path, source: io::Error) -> ProjectError {
    ProjectError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn init_and_discover_project() {
        let temp = tempdir().expect("tempdir");
        let store = ProjectStore::init(temp.path(), "Fixture Game").expect("init");
        let nested = temp.path().join("src/deep");
        fs::create_dir_all(&nested).expect("nested directory");

        let discovered = ProjectStore::discover(&nested).expect("discover");
        assert_eq!(discovered.root(), store.root());
        assert_eq!(
            discovered.manifest().expect("manifest").name,
            "Fixture Game"
        );
    }

    #[test]
    fn rejects_path_like_asset_ids() {
        assert!(matches!(
            validate_asset_id("../escape"),
            Err(ProjectError::InvalidAssetId(_))
        ));
    }

    #[test]
    fn imports_references_by_content_hash_without_overwriting() {
        let temp = tempdir().expect("tempdir");
        let store = ProjectStore::init(temp.path(), "Fixture Game").expect("init");
        let bytes = b"synthetic PNG fixture bytes";
        let first = store
            .import_reference("test-sprite", bytes)
            .expect("import");
        let second = store
            .import_reference("test-sprite", bytes)
            .expect("repeat import");

        assert_eq!(first, second);
        assert_eq!(fs::read(&first.path).expect("stored reference"), bytes);
        assert_eq!(first.sha256, sha256_hex(bytes));
    }
}
