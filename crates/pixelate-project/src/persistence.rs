use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use atomicwrites::{AllowOverwrite, AtomicFile};
use fs2::FileExt;
use serde::de::DeserializeOwned;

use crate::{PROJECT_SCHEMA, ProjectError, ProjectManifest};

#[derive(Debug, Clone)]
pub struct ProjectStore {
    pub(crate) root: PathBuf,
}

impl ProjectStore {
    /// Creates a new `.pixelate` project at `root`.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] if a project already exists or its directories
    /// and initial manifests cannot be written.
    pub fn init(root: &Path, name: &str) -> Result<Self, ProjectError> {
        let root = absolute(root)?;
        let pixelate = root.join(".pixelate");
        if pixelate.exists() {
            return Err(ProjectError::AlreadyExists(pixelate));
        }

        fs::create_dir_all(pixelate.join("assets")).map_err(|source| io_at(&pixelate, source))?;
        fs::create_dir_all(pixelate.join("tmp")).map_err(|source| io_at(&pixelate, source))?;

        let manifest = ProjectManifest {
            schema: PROJECT_SCHEMA.to_owned(),
            name: name.to_owned(),
            ignored_project_images: Vec::new(),
        };
        atomic_write(
            &pixelate.join("project.toml"),
            toml::to_string_pretty(&manifest)?.as_bytes(),
        )?;
        atomic_write(
            &pixelate.join(".gitignore"),
            b"/.lock\n/cache/\n/tmp/\n/assets/*/references/\n",
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
            if candidate.join(".pixelate/project.toml").is_file() {
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
        let path = self.root.join(".pixelate/project.toml");
        let contents = fs::read_to_string(&path).map_err(|source| io_at(&path, source))?;
        let manifest: ProjectManifest = toml::from_str(&contents)?;
        ensure_schema(&manifest.schema, PROJECT_SCHEMA)?;
        Ok(manifest)
    }

    pub(crate) fn asset_path(&self, id: &str) -> PathBuf {
        self.root.join(".pixelate/assets").join(id)
    }

    pub(crate) fn lock(&self) -> Result<File, ProjectError> {
        let path = self.root.join(".pixelate/.lock");
        let lock = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| io_at(&path, source))?;
        lock.lock_exclusive()
            .map_err(|source| io_at(&path, source))?;
        Ok(lock)
    }
}

pub(crate) fn ensure_schema(actual: &str, expected: &'static str) -> Result<(), ProjectError> {
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

pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, ProjectError> {
    let bytes = fs::read(path).map_err(|source| io_at(path, source))?;
    serde_json::from_slice(&bytes).map_err(ProjectError::from)
}

pub(crate) fn now_unix_ms() -> Result<u64, ProjectError> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ProjectError::Clock)?
            .as_millis(),
    )
    .map_err(|_| ProjectError::Clock)
}

pub(crate) fn write_file(path: &Path, bytes: &[u8]) -> Result<(), ProjectError> {
    let mut file = File::create(path).map_err(|source| io_at(path, source))?;
    file.write_all(bytes)
        .map_err(|source| io_at(path, source))?;
    file.sync_all().map_err(|source| io_at(path, source))
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ProjectError> {
    AtomicFile::new(path, AllowOverwrite)
        .write(|file| file.write_all(bytes))
        .map_err(|error| ProjectError::Atomic(error.to_string()))
}

pub(crate) fn io_at(path: &Path, source: io::Error) -> ProjectError {
    ProjectError::Io {
        path: path.to_path_buf(),
        source,
    }
}
