use std::path::PathBuf;

use pixelate_project::ProjectError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Project(#[from] ProjectError),
    #[error(transparent)]
    Core(#[from] pixelate_core::CoreError),
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
    #[error("unsupported conversion request: {0}")]
    UnsupportedConversion(String),
    #[error("image could not be decoded: {0}")]
    Image(String),
    #[error("brief is not valid UTF-8: {path}")]
    BriefUtf8 { path: PathBuf },
    #[error("invalid export destination: {0}")]
    InvalidExportDestination(String),
    #[error("export already exists: {0}")]
    ExportExists(PathBuf),
    #[error("unsupported export format: {0}")]
    UnsupportedExportFormat(String),
}
