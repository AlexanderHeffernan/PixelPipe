use std::{collections::BTreeMap, io, path::PathBuf};

use pixelate_core::{ConversionSettings, IndexedRaster, Recipe, SheetSettings, ValidationReport};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PROJECT_SCHEMA: &str = "pixelate.project/v1";
pub const ASSET_SCHEMA: &str = "pixelate.asset/v2";
pub const REVISION_SCHEMA: &str = "pixelate.revision/v1";
pub const PROVENANCE_SCHEMA: &str = "pixelate.provenance/v1";
pub const REVIEW_SCHEMA: &str = "pixelate.review/v1";
pub const REFERENCE_SELECTION_SCHEMA: &str = "pixelate.reference-selection/v1";
pub const ASSET_BRIEF_SCHEMA: &str = "pixelate.asset-brief/v1";
pub const CONVERSION_RECIPE_SCHEMA: &str = "pixelate.conversion-recipe/v1";
pub(crate) const REVISION_PAYLOADS: [&str; 7] = [
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<AssetStyle>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetStyle {
    pub recipe: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_count: Option<u8>,
    pub settings: ConversionSettings,
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
    pub style: Option<AssetStyle>,
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
    #[error("no .pixelate/project.toml found from {0}")]
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
    #[error("TOML error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("invalid TOML: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("core encoding error: {0}")]
    Core(#[from] pixelate_core::CoreError),
    #[error("system clock is before the Unix epoch")]
    Clock,
    #[error("atomic write failed: {0}")]
    Atomic(String),
}
