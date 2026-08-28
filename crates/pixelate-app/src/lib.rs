mod animation;
mod composition;
mod conversion_preview;
mod error;
mod export;
mod inspection;
mod onboarding;
mod palette_editing;
mod pixel_editing;
mod pixelization;
mod project;
mod reference;
mod revision_commit;
mod revision_preview;

pub use composition::{
    CommitComposition, CompositionPreview, PreviewComposition, commit_composition,
    preview_composition,
};
pub use conversion_preview::{
    ConversionPreview, PaletteColorOverride, PreviewSelectedReference, preview_selected_reference,
};
pub use error::AppError;
pub use export::{
    ExportAsset, ExportAssetFile, ExportFileResult, ExportResult, export_asset, export_asset_file,
};
pub use inspection::*;
pub use onboarding::{OpenProject, open_project};
pub use palette_editing::*;
pub use pixel_editing::*;
pub use pixelization::{
    ConvertSelectedReference, PixelizationDefaults, convert_selected_reference,
    pixelization_defaults,
};
pub use project::*;
pub use reference::{
    ImportReference, UpdateAssetSource, UpdateAssetSourceResult, import_reference,
    update_asset_source,
};
pub use revision_commit::RevisionResult;
pub use revision_preview::{PreviewRevision, RevisionPreview, preview_revision};

pub use pixelate_core::{Palette, RasterInspection};
pub use pixelate_project::{AssetManifest, ReferenceSelection};

pub use animation::*;
pub(crate) use revision_commit::{CommitSequence, commit_sequence, resolve_revision};
