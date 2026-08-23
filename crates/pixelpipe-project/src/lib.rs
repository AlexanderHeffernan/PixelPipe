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
    ConversionSettings, IndexedRaster, Palette, PaletteRemap, PixelPatchSet, RECIPE_SCHEMA, Recipe,
    SheetSettings, VALIDATION_SCHEMA, ValidationReport, sha256_hex, stable_json,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PROJECT_SCHEMA: &str = "pixelpipe.project/v1";
pub const ASSET_SCHEMA: &str = "pixelpipe.asset/v2";
const LEGACY_ASSET_SCHEMA: &str = "pixelpipe.asset/v1";
pub const REVISION_SCHEMA: &str = "pixelpipe.revision/v1";
pub const PROVENANCE_SCHEMA: &str = "pixelpipe.provenance/v1";
pub const REVIEW_SCHEMA: &str = "pixelpipe.review/v1";
pub const AGENT_RUN_SCHEMA: &str = "pixelpipe.agent-run/v1";
pub const REFERENCE_SELECTION_SCHEMA: &str = "pixelpipe.reference-selection/v1";
pub const ASSET_BRIEF_SCHEMA: &str = "pixelpipe.asset-brief/v1";
pub const CONVERSION_RECIPE_SCHEMA: &str = "pixelpipe.conversion-recipe/v1";
const REVISION_PAYLOADS: [&str; 7] = [
    "brief.md",
    "native.png",
    "pixels.json",
    "preview.png",
    "provenance.json",
    "recipe.json",
    "validation.json",
];

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
    #[serde(default)]
    pub state: AssetState,
    #[serde(default)]
    pub brief: AssetBrief,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_reference: Option<ReferenceSelection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetState {
    #[default]
    Draft,
    AwaitingReference,
    SelectedReference,
    Revisioned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetBrief {
    pub schema: String,
    pub text: String,
}

impl Default for AssetBrief {
    fn default() -> Self {
        Self {
            schema: ASSET_BRIEF_SCHEMA.to_owned(),
            text: String::new(),
        }
    }
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
pub struct ConversionRecipeDocument {
    pub schema: String,
    pub id: String,
    pub kind: AssetKind,
    pub palette: String,
    pub preview_scale: u16,
    pub mode: StoredConversionMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum StoredConversionMode {
    Reference { settings: ConversionSettings },
    Sheet { settings: SheetSettings },
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionSnapshot {
    pub path: PathBuf,
    pub manifest: RevisionManifest,
    pub raster: IndexedRaster,
    pub recipe: Recipe,
    pub validation: ValidationReport,
    pub provenance: Provenance,
    pub brief: String,
    pub native_png: Vec<u8>,
    pub preview_png: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewActorKind {
    Human,
    Agent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    Reviewed,
    ChangesRequested,
    Accepted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewEvent {
    pub sequence: u64,
    pub created_unix_ms: u64,
    pub actor: String,
    pub actor_kind: ReviewActorKind,
    pub decision: ReviewDecision,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewRecord {
    pub schema: String,
    pub asset: String,
    pub revision: String,
    pub events: Vec<ReviewEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapability {
    GenerateReferences,
    CritiqueAsset,
    ProposeRefinement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentOperation {
    GenerateReferences,
    CritiqueAsset,
    ProposeRefinement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunStatus {
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentIdentity {
    pub adapter: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub capabilities: Vec<AgentCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentCandidate {
    pub id: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub png: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AgentProposal {
    PixelPatch { patch: PixelPatchSet },
    PaletteRemap { remap: PaletteRemap },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRunRecord {
    pub schema: String,
    pub id: String,
    pub asset: String,
    pub operation: AgentOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    pub profile: String,
    pub profile_command_sha256: String,
    pub prompt: String,
    pub started_unix_ms: u64,
    pub duration_ms: u64,
    pub status: AgentRunStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_status: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<AgentIdentity>,
    pub stdout: String,
    pub stderr: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<AgentCandidate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub critique: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal: Option<AgentProposal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceSelection {
    pub schema: String,
    pub asset: String,
    pub run: String,
    pub candidate: String,
    pub sha256: String,
    pub selected_unix_ms: u64,
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
    #[error("asset '{0}' already exists")]
    AssetExists(String),
    #[error("asset '{asset}' is not ready for {operation}: {reason}")]
    AssetNotReady {
        asset: String,
        operation: &'static str,
        reason: &'static str,
    },
    #[error("project resource id '{0}' is invalid")]
    InvalidResourceId(String),
    #[error("project resource '{kind}/{id}' does not exist")]
    ResourceNotFound { kind: &'static str, id: String },
    #[error("project resource identity does not match its path")]
    ResourceIdentityMismatch,
    #[error("revision directory already exists: {0}")]
    RevisionExists(PathBuf),
    #[error("revision '{revision}' does not exist for asset '{asset}'")]
    RevisionNotFound { asset: String, revision: String },
    #[error("revision payload hash mismatch for '{name}'")]
    RevisionHashMismatch { name: String },
    #[error("revision manifest has an invalid payload file set")]
    InvalidRevisionFiles,
    #[error("revision manifest identity does not match asset/revision path")]
    RevisionIdentityMismatch,
    #[error("invalid revision id '{0}'")]
    InvalidRevisionId(String),
    #[error("review actor must not be empty")]
    EmptyReviewActor,
    #[error("review record identity or event sequence is invalid")]
    InvalidReviewRecord,
    #[error("revision brief is not valid UTF-8")]
    InvalidBriefUtf8,
    #[error("rendered output hash does not match bytes for '{name}'")]
    OutputHashMismatch { name: String },
    #[error("stored reference does not match its content-addressed path: {0}")]
    ReferenceHashMismatch(PathBuf),
    #[error("invalid agent run or candidate id '{0}'")]
    InvalidAgentId(String),
    #[error("agent run directory already exists: {0}")]
    AgentRunExists(PathBuf),
    #[error("agent run identity, schema, or candidate manifest is invalid")]
    InvalidAgentRun,
    #[error("agent candidate '{candidate}' does not exist in run '{run}'")]
    AgentCandidateNotFound { run: String, candidate: String },
    #[error("agent candidate bytes do not match their content hash")]
    AgentCandidateHashMismatch,
    #[error("TOML error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("invalid TOML: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
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
        fs::create_dir_all(pixelpipe.join("recipes"))
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
            kind,
            state: state_for(brief, None, None),
            brief: AssetBrief {
                schema: ASSET_BRIEF_SCHEMA.to_owned(),
                text: brief.to_owned(),
            },
            selected_reference: None,
            head: None,
            approved: None,
        };
        write_file(
            &staging.join("asset.toml"),
            toml::to_string_pretty(&asset)?.as_bytes(),
        )?;
        fs::rename(&staging, &path).map_err(|source| io_at(&path, source))?;
        Ok(asset)
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

    /// Atomically stores a versioned project palette resource.
    ///
    /// # Errors
    ///
    /// Returns an error when identity, palette validation, or storage fails.
    pub fn store_palette(&self, id: &str, palette: &Palette) -> Result<(), ProjectError> {
        validate_resource_id(id)?;
        palette.validate()?;
        let _lock = self.lock()?;
        atomic_write(&self.resource_path("palettes", id), &stable_json(palette)?)
    }

    /// Loads and validates a project palette resource.
    ///
    /// # Errors
    ///
    /// Returns an error when the resource is missing, malformed, or invalid.
    pub fn palette(&self, id: &str) -> Result<Palette, ProjectError> {
        validate_resource_id(id)?;
        let path = self.resource_path("palettes", id);
        if !path.is_file() {
            return Err(ProjectError::ResourceNotFound {
                kind: "palettes",
                id: id.to_owned(),
            });
        }
        let palette: Palette = read_json(&path)?;
        palette.validate()?;
        Ok(palette)
    }

    /// Atomically stores a complete project conversion recipe.
    ///
    /// # Errors
    ///
    /// Returns an error when identity, recipe validation, or storage fails.
    pub fn store_conversion_recipe(
        &self,
        recipe: &ConversionRecipeDocument,
    ) -> Result<(), ProjectError> {
        validate_conversion_recipe(recipe)?;
        let _lock = self.lock()?;
        let recipes = self.root.join(".pixelpipe/recipes");
        fs::create_dir_all(&recipes).map_err(|source| io_at(&recipes, source))?;
        atomic_write(
            &self.resource_path("recipes", &recipe.id),
            &stable_json(recipe)?,
        )
    }

    /// Loads and validates a project conversion recipe.
    ///
    /// # Errors
    ///
    /// Returns an error when the resource is missing, malformed, or invalid.
    pub fn conversion_recipe(&self, id: &str) -> Result<ConversionRecipeDocument, ProjectError> {
        validate_resource_id(id)?;
        let path = self.resource_path("recipes", id);
        if !path.is_file() {
            return Err(ProjectError::ResourceNotFound {
                kind: "recipes",
                id: id.to_owned(),
            });
        }
        let recipe: ConversionRecipeDocument = read_json(&path)?;
        validate_conversion_recipe(&recipe)?;
        if recipe.id != id {
            return Err(ProjectError::ResourceIdentityMismatch);
        }
        Ok(recipe)
    }

    /// Lists validated conversion recipes in stable ID order.
    ///
    /// # Errors
    ///
    /// Returns an error when the resource directory or any recipe is invalid.
    pub fn conversion_recipes(&self) -> Result<Vec<ConversionRecipeDocument>, ProjectError> {
        let path = self.root.join(".pixelpipe/recipes");
        if !path.is_dir() {
            return Ok(Vec::new());
        }
        let mut recipes = Vec::new();
        for entry in fs::read_dir(&path).map_err(|source| io_at(&path, source))? {
            let entry = entry.map_err(|source| io_at(&path, source))?;
            if entry.path().extension().and_then(|value| value.to_str()) == Some("json") {
                let id = entry
                    .file_name()
                    .to_string_lossy()
                    .trim_end_matches(".json")
                    .to_owned();
                recipes.push(self.conversion_recipe(&id)?);
            }
        }
        recipes.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(recipes)
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

    /// Loads a revision's durable review record, if review has begun.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when identities, schemas, or review JSON are invalid.
    pub fn review(
        &self,
        asset_id: &str,
        revision: &str,
    ) -> Result<Option<ReviewRecord>, ProjectError> {
        self.revision(asset_id, revision)?;
        let path = self.review_path(asset_id, revision);
        if !path.is_file() {
            return Ok(None);
        }
        let record: ReviewRecord = read_json(&path)?;
        validate_review_record(&record, asset_id, revision)?;
        Ok(Some(record))
    }

    /// Atomically appends an explicit human or agent review event.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when the revision, actor, record, lock, or
    /// durable write is invalid.
    pub fn record_review(
        &self,
        asset_id: &str,
        revision: &str,
        actor: &str,
        actor_kind: ReviewActorKind,
        decision: ReviewDecision,
        note: &str,
    ) -> Result<ReviewRecord, ProjectError> {
        if actor.trim().is_empty() {
            return Err(ProjectError::EmptyReviewActor);
        }
        self.revision(asset_id, revision)?;
        let lock = self.lock()?;
        let path = self.review_path(asset_id, revision);
        let mut record = if path.is_file() {
            let record: ReviewRecord = read_json(&path)?;
            validate_review_record(&record, asset_id, revision)?;
            record
        } else {
            ReviewRecord {
                schema: REVIEW_SCHEMA.to_owned(),
                asset: asset_id.to_owned(),
                revision: revision.to_owned(),
                events: Vec::new(),
            }
        };
        let sequence = u64::try_from(record.events.len())
            .map_err(|_| ProjectError::Clock)?
            .checked_add(1)
            .ok_or(ProjectError::Clock)?;
        record.events.push(ReviewEvent {
            sequence,
            created_unix_ms: now_unix_ms()?,
            actor: actor.to_owned(),
            actor_kind,
            decision,
            note: note.to_owned(),
        });
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| io_at(parent, source))?;
        }
        atomic_write(&path, &stable_json(&record)?)?;
        drop(lock);
        Ok(record)
    }

    /// Atomically stores one finished agent run and its validated candidate PNGs.
    ///
    /// Agent runs are immutable local provenance. Project configuration never
    /// supplies the executable that produced them.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when identities, hashes, locking, or storage fail.
    pub fn store_agent_run(
        &self,
        record: &AgentRunRecord,
        candidate_bytes: &BTreeMap<String, Vec<u8>>,
    ) -> Result<(), ProjectError> {
        validate_agent_run(record)?;
        if record.candidates.len() != candidate_bytes.len() {
            return Err(ProjectError::InvalidAgentRun);
        }
        for candidate in &record.candidates {
            let bytes = candidate_bytes.get(&candidate.id).ok_or_else(|| {
                ProjectError::AgentCandidateNotFound {
                    run: record.id.clone(),
                    candidate: candidate.id.clone(),
                }
            })?;
            if sha256_hex(bytes) != candidate.sha256 {
                return Err(ProjectError::AgentCandidateHashMismatch);
            }
        }

        let lock = self.lock()?;
        let runs = self.root.join(".pixelpipe/runs");
        fs::create_dir_all(&runs).map_err(|source| io_at(&runs, source))?;
        let destination = runs.join(&record.id);
        if destination.exists() {
            return Err(ProjectError::AgentRunExists(destination));
        }
        let staging = self.root.join(".pixelpipe/tmp").join(format!(
            "agent-run-{}-{}",
            record.id,
            std::process::id()
        ));
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(|source| io_at(&staging, source))?;
        }
        fs::create_dir_all(staging.join("candidates")).map_err(|source| io_at(&staging, source))?;
        for candidate in &record.candidates {
            let bytes = candidate_bytes
                .get(&candidate.id)
                .ok_or(ProjectError::InvalidAgentRun)?;
            write_file(&staging.join(&candidate.png), bytes)?;
        }
        write_file(&staging.join("run.json"), &stable_json(record)?)?;
        fs::rename(&staging, &destination).map_err(|source| io_at(&destination, source))?;
        drop(lock);
        Ok(())
    }

    /// Loads and hash-verifies an immutable agent run.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when the run or any candidate is invalid.
    pub fn agent_run(&self, run: &str) -> Result<AgentRunRecord, ProjectError> {
        validate_agent_id(run)?;
        let path = self.root.join(".pixelpipe/runs").join(run);
        let record: AgentRunRecord = read_json(&path.join("run.json"))?;
        validate_agent_run(&record)?;
        if record.id != run {
            return Err(ProjectError::InvalidAgentRun);
        }
        for candidate in &record.candidates {
            let bytes = fs::read(path.join(&candidate.png))
                .map_err(|source| io_at(&path.join(&candidate.png), source))?;
            if sha256_hex(&bytes) != candidate.sha256 {
                return Err(ProjectError::AgentCandidateHashMismatch);
            }
        }
        Ok(record)
    }

    /// Lists verified runs for an asset in stable run-ID order.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when the run store cannot be read or verified.
    pub fn agent_runs(&self, asset: &str) -> Result<Vec<AgentRunRecord>, ProjectError> {
        validate_asset_id(asset)?;
        let runs = self.root.join(".pixelpipe/runs");
        if !runs.is_dir() {
            return Ok(Vec::new());
        }
        let mut records = Vec::new();
        for entry in fs::read_dir(&runs).map_err(|source| io_at(&runs, source))? {
            let entry = entry.map_err(|source| io_at(&runs, source))?;
            if !entry.path().join("run.json").is_file() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().into_owned();
            let record = self.agent_run(&id)?;
            if record.asset == asset {
                records.push(record);
            }
        }
        records.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(records)
    }

    /// Explicitly selects one validated generated candidate as the asset reference.
    ///
    /// This does not create or move a revision.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when run/candidate verification or storage fails.
    pub fn select_agent_candidate(
        &self,
        asset: &str,
        run: &str,
        candidate_id: &str,
    ) -> Result<ReferenceSelection, ProjectError> {
        validate_asset_id(asset)?;
        validate_agent_id(candidate_id)?;
        let asset_manifest = self.asset(asset)?;
        if asset_manifest.head.is_none() && asset_manifest.brief.text.trim().is_empty() {
            return Err(ProjectError::AssetNotReady {
                asset: asset.to_owned(),
                operation: "select reference",
                reason: "write a non-empty brief first",
            });
        }
        let record = self.agent_run(run)?;
        if record.asset != asset {
            return Err(ProjectError::InvalidAgentRun);
        }
        let candidate = record
            .candidates
            .iter()
            .find(|candidate| candidate.id == candidate_id)
            .ok_or_else(|| ProjectError::AgentCandidateNotFound {
                run: run.to_owned(),
                candidate: candidate_id.to_owned(),
            })?;
        let path = self
            .root
            .join(".pixelpipe/runs")
            .join(run)
            .join(&candidate.png);
        let bytes = fs::read(&path).map_err(|source| io_at(&path, source))?;
        let stored = self.import_reference(asset, &bytes)?;
        let selection = ReferenceSelection {
            schema: REFERENCE_SELECTION_SCHEMA.to_owned(),
            asset: asset.to_owned(),
            run: run.to_owned(),
            candidate: candidate_id.to_owned(),
            sha256: stored.sha256,
            selected_unix_ms: now_unix_ms()?,
        };
        let _lock = self.lock()?;
        let mut asset_manifest = self.asset(asset)?;
        if asset_manifest.head.is_none() && asset_manifest.brief.text.trim().is_empty() {
            return Err(ProjectError::AssetNotReady {
                asset: asset.to_owned(),
                operation: "select reference",
                reason: "write a non-empty brief first",
            });
        }
        ASSET_SCHEMA.clone_into(&mut asset_manifest.schema);
        asset_manifest.selected_reference = Some(selection.clone());
        asset_manifest.state = state_for(
            &asset_manifest.brief.text,
            asset_manifest.selected_reference.as_ref(),
            asset_manifest.head.as_deref(),
        );
        atomic_write(
            &self.asset_path(asset).join("asset.toml"),
            toml::to_string_pretty(&asset_manifest)?.as_bytes(),
        )?;
        Ok(selection)
    }

    /// Selects user-imported PNG bytes as the asset reference.
    ///
    /// This explicit import path has the same content-addressed storage and
    /// lifecycle transition as an agent candidate, but no synthetic agent run.
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
                kind,
                state: AssetState::Draft,
                brief: AssetBrief {
                    schema: ASSET_BRIEF_SCHEMA.to_owned(),
                    text: files.brief.clone(),
                },
                selected_reference: None,
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

    fn asset_path(&self, id: &str) -> PathBuf {
        self.root.join(".pixelpipe/assets").join(id)
    }

    fn resource_path(&self, kind: &str, id: &str) -> PathBuf {
        self.root
            .join(".pixelpipe")
            .join(kind)
            .join(format!("{id}.json"))
    }

    fn review_path(&self, asset_id: &str, revision: &str) -> PathBuf {
        self.asset_path(asset_id)
            .join("reviews")
            .join(format!("{revision}.json"))
    }

    fn lock(&self) -> Result<File, ProjectError> {
        let path = self.root.join(".pixelpipe/.lock");
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

fn validate_resource_id(id: &str) -> Result<(), ProjectError> {
    if validate_asset_id(id).is_ok() {
        Ok(())
    } else {
        Err(ProjectError::InvalidResourceId(id.to_owned()))
    }
}

fn state_for(
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

fn validate_conversion_recipe(recipe: &ConversionRecipeDocument) -> Result<(), ProjectError> {
    ensure_schema(&recipe.schema, CONVERSION_RECIPE_SCHEMA)?;
    validate_resource_id(&recipe.id)?;
    validate_resource_id(&recipe.palette)?;
    if recipe.preview_scale == 0 {
        return Err(ProjectError::Core(
            pixelpipe_core::CoreError::InvalidPreviewScale,
        ));
    }
    Ok(())
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

fn validate_agent_id(id: &str) -> Result<(), ProjectError> {
    let valid = !id.is_empty()
        && id.len() <= 96
        && !id.starts_with('-')
        && !id.ends_with('-')
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(ProjectError::InvalidAgentId(id.to_owned()))
    }
}

fn validate_agent_run(record: &AgentRunRecord) -> Result<(), ProjectError> {
    ensure_schema(&record.schema, AGENT_RUN_SCHEMA)?;
    validate_agent_id(&record.id)?;
    validate_asset_id(&record.asset)?;
    validate_agent_id(&record.profile)?;
    if let Some(revision) = &record.revision {
        validate_revision_id(revision)?;
    }
    let candidates_valid = record.candidates.iter().all(|candidate| {
        validate_agent_id(&candidate.id).is_ok()
            && is_sha256(&candidate.sha256)
            && candidate.png == format!("candidates/{}.png", candidate.sha256)
            && candidate.width > 0
            && candidate.height > 0
    });
    let unique_candidates = record.candidates.iter().enumerate().all(|(index, left)| {
        record.candidates[index + 1..]
            .iter()
            .all(|right| left.id != right.id && left.sha256 != right.sha256)
    });
    let operation_valid = match record.operation {
        AgentOperation::GenerateReferences => record.revision.is_none(),
        AgentOperation::CritiqueAsset | AgentOperation::ProposeRefinement => {
            record.revision.is_some()
        }
    };
    let outcome_valid = match (record.status, record.operation) {
        (AgentRunStatus::Completed, AgentOperation::GenerateReferences) => {
            record.adapter.is_some()
                && record.error.is_none()
                && !record.candidates.is_empty()
                && record.critique.is_none()
                && record.proposal.is_none()
        }
        (AgentRunStatus::Completed, AgentOperation::CritiqueAsset) => {
            record.adapter.is_some()
                && record.error.is_none()
                && record.candidates.is_empty()
                && record.critique.is_some()
                && record.proposal.is_none()
        }
        (AgentRunStatus::Completed, AgentOperation::ProposeRefinement) => {
            record.adapter.is_some()
                && record.error.is_none()
                && record.candidates.is_empty()
                && record.critique.is_none()
                && record.proposal.is_some()
        }
        (AgentRunStatus::Failed | AgentRunStatus::Cancelled, _) => {
            record.error.is_some()
                && record.candidates.is_empty()
                && record.critique.is_none()
                && record.proposal.is_none()
        }
    };
    if !is_sha256(&record.profile_command_sha256)
        || !operation_valid
        || !outcome_valid
        || !candidates_valid
        || !unique_candidates
    {
        return Err(ProjectError::InvalidAgentRun);
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
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

fn validate_review_record(
    record: &ReviewRecord,
    asset_id: &str,
    revision: &str,
) -> Result<(), ProjectError> {
    ensure_schema(&record.schema, REVIEW_SCHEMA)?;
    let events_valid = record.events.iter().enumerate().all(|(index, event)| {
        u64::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(1))
            == Some(event.sequence)
            && !event.actor.trim().is_empty()
    });
    if record.asset == asset_id && record.revision == revision && events_valid {
        Ok(())
    } else {
        Err(ProjectError::InvalidReviewRecord)
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

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, ProjectError> {
    let bytes = fs::read(path).map_err(|source| io_at(path, source))?;
    serde_json::from_slice(&bytes).map_err(ProjectError::from)
}

fn now_unix_ms() -> Result<u64, ProjectError> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ProjectError::Clock)?
            .as_millis(),
    )
    .map_err(|_| ProjectError::Clock)
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
