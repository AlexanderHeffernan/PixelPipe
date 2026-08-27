use std::path::PathBuf;

use pixelpipe_core::{Palette, RasterDiff, RasterInspection, compare_rasters, inspect_raster};
use pixelpipe_project::{
    AssetManifest, ConversionRecipeDocument, ProjectError, ProjectManifest, ProjectStore,
    ReviewActorKind, ReviewDecision, ReviewRecord, RevisionManifest,
};
use serde::{Deserialize, Serialize};

use crate::{AppError, revision_commit::resolve_revision};

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
    pub recipes: Vec<ConversionRecipeDocument>,
    pub palettes: Vec<ProjectPalette>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectPalette {
    pub id: String,
    pub palette: Palette,
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
    pub palette: Palette,
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
        recipes: store.conversion_recipes()?,
        palettes: store
            .palettes()?
            .into_iter()
            .map(|(id, palette)| ProjectPalette { id, palette })
            .collect(),
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
    let palette = snapshot.raster.palette.clone();
    let transparent_index = snapshot.raster.palette.transparent_index;
    let review = store.review(&request.asset, &revision)?;
    Ok(RevisionView {
        metadata: RevisionViewMetadata {
            project_root: store.root().to_path_buf(),
            asset: request.asset,
            revision,
            parent: snapshot.manifest.parent,
            inspection,
            palette,
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
