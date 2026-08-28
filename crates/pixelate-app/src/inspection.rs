use std::path::PathBuf;

use pixelate_core::{
    Palette, RasterInspection, SequenceMotion, inspect_raster, inspect_sequence_motion, render,
};
use pixelate_project::{
    AssetManifest, ProjectError, ProjectManifest, ProjectStore, RevisionManifest,
};
use serde::{Deserialize, Serialize};

use crate::{
    AppError, PixelizationDefaults, pixelization_defaults, revision_commit::resolve_revision,
};

#[derive(Debug, Deserialize)]
pub struct InspectRevision {
    pub start: PathBuf,
    pub asset: String,
    pub revision: Option<String>,
    #[serde(default)]
    pub frame_id: Option<String>,
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
    pub pixelization: PixelizationDefaults,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssetBrowser {
    pub asset: AssetManifest,
    pub revisions: Vec<RevisionManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RevisionInspectionResult {
    pub project_root: PathBuf,
    pub asset: String,
    pub revision: String,
    pub parent: Option<String>,
    pub inspection: RasterInspection,
    pub frames: Vec<FrameMetadata>,
    pub motion: SequenceMotion,
    pub selected_frame_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FrameMetadata {
    pub id: String,
    pub name: Option<String>,
    pub duration_ms: u32,
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
    pub validation: pixelate_core::ValidationReport,
    pub frames: Vec<FrameMetadata>,
    pub selected_frame_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionView {
    pub metadata: RevisionViewMetadata,
    pub native_png: Vec<u8>,
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
        pixelization: pixelization_defaults(),
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
    let selected_frame_id = request
        .frame_id
        .unwrap_or_else(|| snapshot.sequence.frames[0].id.clone());
    let raster = snapshot.sequence.raster(&selected_frame_id)?;
    let inspection = inspect_raster(&raster)?;
    let palette = snapshot.sequence.palette.clone();
    let transparent_index = snapshot.sequence.palette.transparent_index;
    let frames = snapshot
        .sequence
        .frames
        .iter()
        .map(|frame| FrameMetadata {
            id: frame.id.clone(),
            name: frame.name.clone(),
            duration_ms: frame.duration_ms,
        })
        .collect();
    let native_png = render(&raster, 1)?.native_png;
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
            frames,
            selected_frame_id,
        },
        native_png,
    })
}

/// Loads a revision inspection and its separate durable review history.
///
/// # Errors
///
/// Returns an [`AppError`] when project, revision, review, or raster validation fails.
pub fn inspect_revision(request: InspectRevision) -> Result<RevisionInspectionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let revision = resolve_revision(&store, &request.asset, request.revision.clone())?;
    let motion = inspect_sequence_motion(&store.revision(&request.asset, &revision)?.sequence)?;
    let view = load_revision_view(request)?;
    Ok(RevisionInspectionResult {
        project_root: view.metadata.project_root,
        asset: view.metadata.asset,
        revision: view.metadata.revision,
        parent: view.metadata.parent,
        inspection: view.metadata.inspection,
        frames: view.metadata.frames,
        motion,
        selected_frame_id: view.metadata.selected_frame_id,
    })
}
