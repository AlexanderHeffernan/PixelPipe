use std::{collections::BTreeMap, fs, path::PathBuf};

use pixelpipe_core::{
    ComponentRule, ConversionSettings, IndexedRaster, Operation, Palette, PaletteRemap,
    PixelPatchSet, RECIPE_SCHEMA, RasterDiff, RasterInspection, Recipe, SheetSettings,
    ValidationCheck, apply_palette_remap, apply_pixel_patch, compare_rasters, convert_reference,
    convert_sheet, decode_rgba_png, inspect_raster, render, sha256_hex, stable_json,
};
use pixelpipe_project::{
    AssetKind, AssetManifest, ProjectError, ProjectManifest, ProjectStore, RevisionFiles,
    RevisionManifest, StoredRevision,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use pixelpipe_project::{ReviewActorKind, ReviewDecision, ReviewRecord};

#[derive(Debug)]
pub struct CreateRevision {
    pub start: PathBuf,
    pub asset: String,
    pub kind: AssetKind,
    pub raster_path: PathBuf,
    pub brief_path: Option<PathBuf>,
    pub preview_scale: Option<u16>,
    pub actor: String,
}

#[derive(Debug, Clone)]
pub enum ConversionMode {
    Reference(ConversionSettings),
    Sheet(SheetSettings),
}

#[derive(Debug)]
pub struct ConvertRevision {
    pub start: PathBuf,
    pub asset: String,
    pub kind: AssetKind,
    pub source_path: PathBuf,
    pub palette_path: PathBuf,
    pub mode: ConversionMode,
    pub brief_path: Option<PathBuf>,
    pub preview_scale: Option<u16>,
    pub actor: String,
}

#[derive(Debug)]
pub struct PatchRevision {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    pub patch_path: PathBuf,
    pub brief_path: Option<PathBuf>,
    pub preview_scale: Option<u16>,
    pub actor: String,
}

#[derive(Debug)]
pub struct RemapRevision {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    pub remap_path: PathBuf,
    pub brief_path: Option<PathBuf>,
    pub preview_scale: Option<u16>,
    pub actor: String,
}

#[derive(Debug, Deserialize)]
pub struct PatchRevisionDocument {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    pub patch: PixelPatchSet,
    pub brief: Option<String>,
    pub preview_scale: Option<u16>,
    pub actor: String,
}

#[derive(Debug, Deserialize)]
pub struct RemapRevisionDocument {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    pub remap: PaletteRemap,
    pub brief: Option<String>,
    pub preview_scale: Option<u16>,
    pub actor: String,
}

#[derive(Debug, Deserialize)]
pub struct CompareRevisions {
    pub start: PathBuf,
    pub asset: String,
    pub left: String,
    pub right: String,
    pub preview_scale: Option<u16>,
}

#[derive(Debug, Deserialize)]
pub struct InspectRevision {
    pub start: PathBuf,
    pub asset: String,
    pub revision: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BrowseProject {
    pub start: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectBrowser {
    pub project_root: PathBuf,
    pub project: ProjectManifest,
    pub assets: Vec<AssetBrowser>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssetBrowser {
    pub asset: AssetManifest,
    pub revisions: Vec<RevisionManifest>,
}

#[derive(Debug)]
pub struct RevisionComparisonResult {
    pub project_root: PathBuf,
    pub asset: String,
    pub left: String,
    pub right: String,
    pub diff: RasterDiff,
    pub visual_native_png: Vec<u8>,
    pub visual_preview_png: Vec<u8>,
    pub visual_native_sha256: String,
    pub visual_preview_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RevisionComparisonMetadata {
    pub project_root: PathBuf,
    pub asset: String,
    pub left: String,
    pub right: String,
    pub diff: RasterDiff,
    pub visual_native_sha256: String,
    pub visual_preview_sha256: String,
}

impl RevisionComparisonResult {
    #[must_use]
    pub fn metadata(&self) -> RevisionComparisonMetadata {
        RevisionComparisonMetadata {
            project_root: self.project_root.clone(),
            asset: self.asset.clone(),
            left: self.left.clone(),
            right: self.right.clone(),
            diff: self.diff.clone(),
            visual_native_sha256: self.visual_native_sha256.clone(),
            visual_preview_sha256: self.visual_preview_sha256.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RevisionInspectionResult {
    pub project_root: PathBuf,
    pub asset: String,
    pub revision: String,
    pub parent: Option<String>,
    pub inspection: RasterInspection,
    pub review: Option<ReviewRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RevisionViewMetadata {
    pub project_root: PathBuf,
    pub asset: String,
    pub revision: String,
    pub parent: Option<String>,
    pub inspection: RasterInspection,
    pub palette_name: String,
    pub transparent_index: u8,
    pub validation: pixelpipe_core::ValidationReport,
    pub review: Option<ReviewRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionView {
    pub metadata: RevisionViewMetadata,
    pub native_png: Vec<u8>,
    pub preview_png: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub struct RecordReview {
    pub start: PathBuf,
    pub asset: String,
    pub revision: String,
    pub actor: String,
    pub actor_kind: ReviewActorKind,
    pub decision: ReviewDecision,
    pub note: String,
}

struct CommitRaster {
    asset: String,
    kind: AssetKind,
    raster: IndexedRaster,
    recipe: Recipe,
    preview_scale: u16,
    brief: String,
    actor: String,
    input_hashes: BTreeMap<String, String>,
    additional_checks: Vec<ValidationCheck>,
    parent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RevisionResult {
    pub project_root: PathBuf,
    pub asset: String,
    pub revision: String,
    pub parent: Option<String>,
    pub revision_path: PathBuf,
    pub native_sha256: String,
    pub preview_sha256: String,
    pub validation: String,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Project(#[from] ProjectError),
    #[error(transparent)]
    Core(#[from] pixelpipe_core::CoreError),
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid raster JSON in {path}: {source}")]
    RasterJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid palette JSON in {path}: {source}")]
    PaletteJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid operation JSON in {path}: {source}")]
    OperationJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("asset '{0}' has no head revision")]
    NoHead(String),
    #[error("operation structure rule conflicts with its inherited revision rule")]
    StructureRuleConflict,
    #[error("brief is not valid UTF-8: {path}")]
    BriefUtf8 { path: PathBuf },
}

/// Validates a structured raster, renders it, and commits an immutable revision.
///
/// # Errors
///
/// Returns an [`AppError`] when project discovery, input decoding, deterministic
/// rendering, or revision storage fails.
pub fn create_revision(request: CreateRevision) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let raster_bytes = read(&request.raster_path)?;
    let raster: IndexedRaster =
        serde_json::from_slice(&raster_bytes).map_err(|source| AppError::RasterJson {
            path: request.raster_path.clone(),
            source,
        })?;
    let manifest = store.manifest()?;
    let preview_scale = request.preview_scale.unwrap_or(manifest.preview_scale);
    let canonical_raster = stable_json(&raster)?;
    let palette_bytes = stable_json(&raster.palette)?;
    let recipe = Recipe {
        schema: RECIPE_SCHEMA.to_owned(),
        input_sha256: sha256_hex(&canonical_raster),
        palette_sha256: sha256_hex(&palette_bytes),
        operations: vec![Operation::RenderIndexed { preview_scale }],
    };
    let brief = read_brief(request.brief_path)?;

    let input_hashes = BTreeMap::from([
        ("palette".to_owned(), recipe.palette_sha256.clone()),
        ("pixels".to_owned(), recipe.input_sha256.clone()),
    ]);
    commit_raster(
        &store,
        CommitRaster {
            asset: request.asset,
            kind: request.kind,
            raster,
            recipe,
            preview_scale,
            brief,
            actor: request.actor,
            input_hashes,
            additional_checks: Vec::new(),
            parent: None,
        },
    )
}

/// Converts a smooth RGBA reference and commits the result as an immutable revision.
///
/// # Errors
///
/// Returns an [`AppError`] when project discovery, input decoding, deterministic
/// conversion/rendering, or revision storage fails.
pub fn convert_revision(request: ConvertRevision) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let source_bytes = read(&request.source_path)?;
    let source = decode_rgba_png(&source_bytes)?;
    let palette_bytes = read(&request.palette_path)?;
    let palette: Palette =
        serde_json::from_slice(&palette_bytes).map_err(|source| AppError::PaletteJson {
            path: request.palette_path.clone(),
            source,
        })?;
    let canonical_palette = stable_json(&palette)?;
    let (converted, operation) = match request.mode {
        ConversionMode::Reference(settings) => {
            let converted = convert_reference(&source, &palette, &settings)?;
            (converted, Operation::ConvertReference { settings })
        }
        ConversionMode::Sheet(settings) => {
            let converted = convert_sheet(&source, &palette, &settings)?;
            (converted, Operation::ConvertSheet { settings })
        }
    };
    let manifest = store.manifest()?;
    let preview_scale = request.preview_scale.unwrap_or(manifest.preview_scale);
    let stored_reference = store.import_reference(&request.asset, &source_bytes)?;
    let recipe = Recipe {
        schema: RECIPE_SCHEMA.to_owned(),
        input_sha256: stored_reference.sha256,
        palette_sha256: sha256_hex(&canonical_palette),
        operations: vec![operation, Operation::RenderIndexed { preview_scale }],
    };
    let brief = read_brief(request.brief_path)?;
    let input_hashes = BTreeMap::from([
        ("palette".to_owned(), recipe.palette_sha256.clone()),
        ("reference".to_owned(), recipe.input_sha256.clone()),
    ]);
    commit_raster(
        &store,
        CommitRaster {
            asset: request.asset,
            kind: request.kind,
            raster: converted.raster,
            recipe,
            preview_scale,
            brief,
            actor: request.actor,
            input_hashes,
            additional_checks: converted.checks,
            parent: None,
        },
    )
}

/// Applies a validated pixel patch to an explicit parent and creates a new revision.
///
/// # Errors
///
/// Returns an [`AppError`] when loading, patch validation, rendering, or atomic
/// revision storage fails.
pub fn patch_revision(request: PatchRevision) -> Result<RevisionResult, AppError> {
    let operation_path = request.patch_path;
    let patch: PixelPatchSet =
        serde_json::from_slice(&read(&operation_path)?).map_err(|source| {
            AppError::OperationJson {
                path: operation_path.clone(),
                source,
            }
        })?;
    let brief = read_optional_brief(request.brief_path)?;
    patch_revision_document(PatchRevisionDocument {
        start: request.start,
        asset: request.asset,
        parent: request.parent,
        patch,
        brief,
        preview_scale: request.preview_scale,
        actor: request.actor,
    })
}

/// Applies a typed patch document through the same application use case as the CLI.
///
/// # Errors
///
/// Returns an [`AppError`] when loading, validation, rendering, or storage fails.
pub fn patch_revision_document(request: PatchRevisionDocument) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let parent = store.revision(&request.asset, &request.parent)?;
    let mut patch = request.patch;
    inherit_structure(&mut patch.structure, component_rule(&parent.recipe))?;
    let raster = apply_pixel_patch(&parent.raster, &patch)?;
    let preview_scale = request
        .preview_scale
        .unwrap_or(store.manifest()?.preview_scale);
    let recipe = Recipe {
        schema: RECIPE_SCHEMA.to_owned(),
        input_sha256: sha256_hex(&stable_json(&parent.raster)?),
        palette_sha256: sha256_hex(&stable_json(&raster.palette)?),
        operations: vec![
            Operation::PatchPixels {
                patch: patch.clone(),
            },
            Operation::RenderIndexed { preview_scale },
        ],
    };
    let brief = request.brief.unwrap_or(parent.brief);
    let input_hashes = BTreeMap::from([
        ("palette".to_owned(), recipe.palette_sha256.clone()),
        ("parent_pixels".to_owned(), recipe.input_sha256.clone()),
    ]);
    let kind = store.asset(&request.asset)?.kind;
    commit_raster(
        &store,
        CommitRaster {
            asset: request.asset,
            kind,
            raster,
            recipe,
            preview_scale,
            brief,
            actor: request.actor,
            input_hashes,
            additional_checks: vec![ValidationCheck {
                name: "pixel_patch".to_owned(),
                passed: true,
                detail: patch.edits.len().to_string(),
            }],
            parent: Some(request.parent),
        },
    )
}

/// Applies an explicit palette index map to a parent and creates a new revision.
///
/// # Errors
///
/// Returns an [`AppError`] when loading, remap validation, rendering, or atomic
/// revision storage fails.
pub fn remap_revision(request: RemapRevision) -> Result<RevisionResult, AppError> {
    let path = request.remap_path;
    let remap: PaletteRemap =
        serde_json::from_slice(&read(&path)?).map_err(|source| AppError::OperationJson {
            path: path.clone(),
            source,
        })?;
    let brief = read_optional_brief(request.brief_path)?;
    remap_revision_document(RemapRevisionDocument {
        start: request.start,
        asset: request.asset,
        parent: request.parent,
        remap,
        brief,
        preview_scale: request.preview_scale,
        actor: request.actor,
    })
}

/// Applies a typed palette remap through the same application use case as the CLI.
///
/// # Errors
///
/// Returns an [`AppError`] when loading, validation, rendering, or storage fails.
pub fn remap_revision_document(request: RemapRevisionDocument) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let parent = store.revision(&request.asset, &request.parent)?;
    let mut remap = request.remap;
    inherit_structure(&mut remap.structure, component_rule(&parent.recipe))?;
    let raster = apply_palette_remap(&parent.raster, &remap)?;
    let preview_scale = request
        .preview_scale
        .unwrap_or(store.manifest()?.preview_scale);
    let recipe = Recipe {
        schema: RECIPE_SCHEMA.to_owned(),
        input_sha256: sha256_hex(&stable_json(&parent.raster)?),
        palette_sha256: sha256_hex(&stable_json(&raster.palette)?),
        operations: vec![
            Operation::RemapPalette {
                remap: remap.clone(),
            },
            Operation::RenderIndexed { preview_scale },
        ],
    };
    let brief = request.brief.unwrap_or(parent.brief);
    let input_hashes = BTreeMap::from([
        ("palette".to_owned(), recipe.palette_sha256.clone()),
        ("parent_pixels".to_owned(), recipe.input_sha256.clone()),
    ]);
    let kind = store.asset(&request.asset)?.kind;
    commit_raster(
        &store,
        CommitRaster {
            asset: request.asset,
            kind,
            raster,
            recipe,
            preview_scale,
            brief,
            actor: request.actor,
            input_hashes,
            additional_checks: vec![ValidationCheck {
                name: "palette_remap".to_owned(),
                passed: true,
                detail: remap.index_map.len().to_string(),
            }],
            parent: Some(request.parent),
        },
    )
}

/// Lists the discovered project, assets, and immutable revision history.
///
/// # Errors
///
/// Returns an [`AppError`] when discovery or any manifest is invalid.
pub fn browse_project(request: &BrowseProject) -> Result<ProjectBrowser, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let project = store.manifest()?;
    let assets = store
        .assets()?
        .into_iter()
        .map(|asset| {
            let revisions = store.revisions(&asset.id)?;
            Ok(AssetBrowser { asset, revisions })
        })
        .collect::<Result<Vec<_>, ProjectError>>()?;
    Ok(ProjectBrowser {
        project_root: store.root().to_path_buf(),
        project,
        assets,
    })
}

/// Loads verified revision metadata, inspection, review, and rendered PNG bytes.
///
/// # Errors
///
/// Returns an [`AppError`] when project, revision, review, or raster validation fails.
pub fn load_revision_view(request: InspectRevision) -> Result<RevisionView, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let revision = resolve_revision(&store, &request.asset, request.revision)?;
    let snapshot = store.revision(&request.asset, &revision)?;
    let inspection = inspect_raster(&snapshot.raster)?;
    let palette_name = snapshot.raster.palette.name.clone();
    let transparent_index = snapshot.raster.palette.transparent_index;
    let review = store.review(&request.asset, &revision)?;
    Ok(RevisionView {
        metadata: RevisionViewMetadata {
            project_root: store.root().to_path_buf(),
            asset: request.asset,
            revision,
            parent: snapshot.manifest.parent,
            inspection,
            palette_name,
            transparent_index,
            validation: snapshot.validation,
            review,
        },
        native_png: snapshot.native_png,
        preview_png: snapshot.preview_png,
    })
}

/// Loads a revision inspection and its separate durable review history.
///
/// # Errors
///
/// Returns an [`AppError`] when project, revision, review, or raster validation fails.
pub fn inspect_revision(request: InspectRevision) -> Result<RevisionInspectionResult, AppError> {
    let view = load_revision_view(request)?;
    Ok(RevisionInspectionResult {
        project_root: view.metadata.project_root,
        asset: view.metadata.asset,
        revision: view.metadata.revision,
        parent: view.metadata.parent,
        inspection: view.metadata.inspection,
        review: view.metadata.review,
    })
}

/// Compares two immutable revisions and produces machine and visual diffs.
///
/// # Errors
///
/// Returns an [`AppError`] when either revision cannot be verified or diff
/// rendering fails.
pub fn compare_revisions(request: CompareRevisions) -> Result<RevisionComparisonResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let left = store.revision(&request.asset, &request.left)?;
    let right = store.revision(&request.asset, &request.right)?;
    let preview_scale = request
        .preview_scale
        .unwrap_or(store.manifest()?.preview_scale);
    let comparison = compare_rasters(&left.raster, &right.raster, preview_scale)?;
    Ok(RevisionComparisonResult {
        project_root: store.root().to_path_buf(),
        asset: request.asset,
        left: request.left,
        right: request.right,
        diff: comparison.diff,
        visual_native_png: comparison.visual.native_png,
        visual_preview_png: comparison.visual.preview_png,
        visual_native_sha256: comparison.visual_native_sha256,
        visual_preview_sha256: comparison.visual_preview_sha256,
    })
}

/// Appends one explicit human or agent review event without changing the revision.
///
/// # Errors
///
/// Returns an [`AppError`] when the project, revision, actor, lock, or record is invalid.
pub fn record_review(request: RecordReview) -> Result<ReviewRecord, AppError> {
    let RecordReview {
        start,
        asset,
        revision,
        actor,
        actor_kind,
        decision,
        note,
    } = request;
    let store = ProjectStore::discover(&start)?;
    store
        .record_review(&asset, &revision, &actor, actor_kind, decision, &note)
        .map_err(AppError::from)
}

fn component_rule(recipe: &Recipe) -> Option<ComponentRule> {
    recipe
        .operations
        .iter()
        .rev()
        .find_map(|operation| match operation {
            Operation::ConvertReference { settings } => Some(ComponentRule::Raster {
                expectation: settings.components,
            }),
            Operation::ConvertSheet { settings } => Some(ComponentRule::SheetFrames {
                columns: settings.columns,
                rows: settings.rows,
                expectation: settings.frame.components,
            }),
            Operation::PatchPixels { patch } => patch.structure,
            Operation::RemapPalette { remap } => remap.structure,
            Operation::RenderIndexed { .. } => None,
        })
}

fn resolve_revision(
    store: &ProjectStore,
    asset: &str,
    revision: Option<String>,
) -> Result<String, AppError> {
    match revision {
        Some(revision) => Ok(revision),
        None => store
            .asset(asset)?
            .head
            .ok_or_else(|| AppError::NoHead(asset.to_owned())),
    }
}

fn inherit_structure(
    operation: &mut Option<ComponentRule>,
    inherited: Option<ComponentRule>,
) -> Result<(), AppError> {
    match (*operation, inherited) {
        (Some(operation), Some(inherited)) if operation != inherited => {
            Err(AppError::StructureRuleConflict)
        }
        (None, inherited) => {
            *operation = inherited;
            Ok(())
        }
        _ => Ok(()),
    }
}

fn read_optional_brief(path: Option<PathBuf>) -> Result<Option<String>, AppError> {
    match path {
        Some(path) => String::from_utf8(read(&path)?)
            .map(Some)
            .map_err(|_| AppError::BriefUtf8 { path }),
        None => Ok(None),
    }
}

fn commit_raster(store: &ProjectStore, commit: CommitRaster) -> Result<RevisionResult, AppError> {
    let mut rendered = render(&commit.raster, commit.preview_scale)?;
    rendered.validation.checks.extend(commit.additional_checks);
    let native_sha256 = sha256_hex(&rendered.native_png);
    let preview_sha256 = sha256_hex(&rendered.preview_png);
    let output_hashes = BTreeMap::from([
        ("native.png".to_owned(), native_sha256.clone()),
        ("preview.png".to_owned(), preview_sha256.clone()),
    ]);
    let files = RevisionFiles {
        raster: commit.raster,
        recipe: commit.recipe,
        validation: rendered.validation,
        native_png: rendered.native_png,
        preview_png: rendered.preview_png,
        brief: commit.brief,
        actor: commit.actor,
        input_hashes: commit.input_hashes,
        output_hashes,
    };
    let stored = match commit.parent {
        Some(parent) => store.create_revision_from(&commit.asset, commit.kind, &parent, files)?,
        None => store.create_revision(&commit.asset, commit.kind, files)?,
    };

    Ok(result(stored, native_sha256, preview_sha256))
}

fn read_brief(path: Option<PathBuf>) -> Result<String, AppError> {
    match path {
        Some(path) => String::from_utf8(read(&path)?).map_err(|_| AppError::BriefUtf8 { path }),
        None => Ok(String::new()),
    }
}

fn result(stored: StoredRevision, native_sha256: String, preview_sha256: String) -> RevisionResult {
    RevisionResult {
        project_root: stored.project_root,
        asset: stored.asset,
        revision: stored.revision,
        parent: stored.parent,
        revision_path: stored.revision_path,
        native_sha256,
        preview_sha256,
        validation: "valid_visual_review_required".to_owned(),
    }
}

fn read(path: &PathBuf) -> Result<Vec<u8>, AppError> {
    fs::read(path).map_err(|source| AppError::Read {
        path: path.clone(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use pixelpipe_core::{IndexedRaster, Palette, RASTER_SCHEMA};
    use pixelpipe_project::RevisionManifest;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn use_case_creates_immutable_revision_chain() {
        let temp = tempdir().expect("tempdir");
        ProjectStore::init(temp.path(), "Test").expect("init");
        let input = temp.path().join("pixels.json");
        let raster = IndexedRaster {
            schema: RASTER_SCHEMA.to_owned(),
            width: 2,
            height: 1,
            palette: Palette::new("fixture", 0, vec![[0, 0, 0, 0], [255, 0, 0, 255]]),
            pixels: vec![0, 1],
            pivot: None,
            metadata: BTreeMap::new(),
        };
        fs::write(&input, stable_json(&raster).expect("json")).expect("write fixture");

        let create = || CreateRevision {
            start: temp.path().to_path_buf(),
            asset: "test-sprite".to_owned(),
            kind: AssetKind::Sprite,
            raster_path: input.clone(),
            brief_path: None,
            preview_scale: Some(2),
            actor: "test".to_owned(),
        };
        let first = create_revision(create()).expect("first revision");
        let first_native = fs::read(first.revision_path.join("native.png")).expect("first PNG");
        let second = create_revision(create()).expect("second revision");

        assert_eq!(first.revision, "r000001");
        assert_eq!(second.revision, "r000002");
        assert_eq!(second.parent.as_deref(), Some("r000001"));
        assert_eq!(
            fs::read(first.revision_path.join("native.png")).expect("first PNG after second"),
            first_native
        );
        assert_eq!(first.native_sha256, second.native_sha256);
        assert_eq!(first.preview_sha256, second.preview_sha256);

        let browser = browse_project(&BrowseProject {
            start: temp.path().join("nested"),
        })
        .expect("browse project from descendant");
        assert_eq!(browser.assets.len(), 1);
        assert_eq!(browser.assets[0].revisions.len(), 2);
        assert_eq!(browser.assets[0].asset.head.as_deref(), Some("r000002"));

        let view = load_revision_view(InspectRevision {
            start: temp.path().to_path_buf(),
            asset: "test-sprite".to_owned(),
            revision: Some("r000001".to_owned()),
        })
        .expect("verified revision view");
        assert_eq!(view.metadata.revision, "r000001");
        assert_eq!(view.metadata.palette_name, "fixture");
        assert_eq!(view.native_png, first_native);

        record_review(RecordReview {
            start: temp.path().to_path_buf(),
            asset: "test-sprite".to_owned(),
            revision: "r000001".to_owned(),
            actor: "reviewer".to_owned(),
            actor_kind: ReviewActorKind::Human,
            decision: ReviewDecision::Reviewed,
            note: "native size inspected".to_owned(),
        })
        .expect("record review");
        assert_eq!(
            ProjectStore::discover(temp.path())
                .expect("store")
                .asset("test-sprite")
                .expect("asset")
                .head
                .as_deref(),
            Some("r000002")
        );

        let manifest: RevisionManifest = serde_json::from_slice(
            &fs::read(first.revision_path.join("revision.json")).expect("revision manifest"),
        )
        .expect("revision manifest JSON");
        for (name, expected_hash) in manifest.files {
            let contents = fs::read(first.revision_path.join(name)).expect("hashed payload");
            assert_eq!(sha256_hex(&contents), expected_hash);
        }
    }
}
