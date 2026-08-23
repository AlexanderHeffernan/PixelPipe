use std::{collections::BTreeMap, fs, path::PathBuf};

use pixelpipe_core::{
    ConversionSettings, IndexedRaster, Operation, Palette, RECIPE_SCHEMA, Recipe, SheetSettings,
    ValidationCheck, convert_reference, convert_sheet, decode_rgba_png, render, sha256_hex,
    stable_json,
};
use pixelpipe_project::{AssetKind, ProjectError, ProjectStore, RevisionFiles, StoredRevision};
use serde::Serialize;
use thiserror::Error;

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
        },
    )
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
    let stored = store.create_revision(
        &commit.asset,
        commit.kind,
        RevisionFiles {
            raster: commit.raster,
            recipe: commit.recipe,
            validation: rendered.validation,
            native_png: rendered.native_png,
            preview_png: rendered.preview_png,
            brief: commit.brief,
            actor: commit.actor,
            input_hashes: commit.input_hashes,
            output_hashes,
        },
    )?;

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
