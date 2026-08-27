use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use pixelate_project::{
    AssetKind, ReviewActorKind as ProjectReviewActorKind, ReviewDecision as ProjectReviewDecision,
};

use crate::pixelize::PixelizeArgs;

#[derive(Debug, Parser)]
#[command(
    name = "pixelate",
    version,
    about = "Deterministic pixel-art project pipeline"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Print the reliable workflow for a coding agent operating `Pixelate`.
    Guide {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    Init {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        name: String,
    },
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    Revision {
        #[command(subcommand)]
        command: RevisionCommand,
    },
    Asset {
        #[command(subcommand)]
        command: AssetCommand,
    },
    Reference {
        #[command(subcommand)]
        command: ReferenceCommand,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ProjectCommand {
    Show {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    SetPalette {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        file: PathBuf,
    },
    SetRecipe {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        file: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum RevisionCommand {
    /// Pixelize the selected source with direct creative controls.
    Pixelize {
        #[command(flatten)]
        options: PixelizeArgs,
    },
    /// Render a committed revision as a vision-friendly nearest-neighbour PNG.
    Preview {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        /// Revision to preview; defaults to the current head.
        #[arg(long)]
        revision: Option<String>,
        /// Integer nearest-neighbour scale; defaults toward a 512px longest edge.
        #[arg(long, value_parser = clap::value_parser!(u16).range(1..=64))]
        scale: Option<u16>,
        #[arg(long)]
        output: PathBuf,
    },
    Create {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long, value_enum, default_value_t = Kind::Sprite)]
        kind: Kind,
        #[arg(long)]
        pixels: PathBuf,
        #[arg(long)]
        brief: Option<PathBuf>,
        #[arg(long)]
        preview_scale: Option<u16>,
        #[arg(long, default_value = "cli")]
        actor: String,
    },
    Convert {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long, value_enum, default_value_t = Kind::Sprite)]
        kind: Kind,
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        palette: PathBuf,
        #[arg(long)]
        settings: PathBuf,
        #[arg(long, value_enum, default_value_t = ConversionKind::Reference)]
        conversion: ConversionKind,
        #[arg(long)]
        brief: Option<PathBuf>,
        #[arg(long)]
        preview_scale: Option<u16>,
        #[arg(long, default_value = "cli")]
        actor: String,
    },
    ConvertSelected {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        recipe: String,
        #[arg(long)]
        palette: Option<String>,
        #[arg(long, value_parser = clap::value_parser!(u8).range(2..=64))]
        colors: Option<u8>,
        #[arg(long)]
        settings: Option<PathBuf>,
        #[arg(long)]
        auto_background: bool,
        #[arg(long, default_value = "cli")]
        actor: String,
    },
    PreviewSelected {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        recipe: String,
        #[arg(long)]
        palette: Option<String>,
        #[arg(long, value_parser = clap::value_parser!(u8).range(2..=64))]
        colors: Option<u8>,
        #[arg(long)]
        settings: Option<PathBuf>,
        #[arg(long)]
        auto_background: bool,
        #[arg(long)]
        native: PathBuf,
    },
    Patch {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: String,
        #[arg(long)]
        patch: PathBuf,
        #[arg(long)]
        brief: Option<PathBuf>,
        #[arg(long)]
        preview_scale: Option<u16>,
        #[arg(long, default_value = "cli")]
        actor: String,
    },
    Fill {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        x: u32,
        #[arg(long)]
        y: u32,
        #[arg(long)]
        index: u8,
        #[arg(long, default_value = "cli")]
        actor: String,
    },
    Compose {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        width: u32,
        #[arg(long)]
        height: u32,
        #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u16).range(25..=400))]
        scale: u16,
        #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
        offset_x: i16,
        #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
        offset_y: i16,
        #[arg(long, default_value = "cli")]
        actor: String,
    },
    SetHead {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        revision: String,
    },
    Remap {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: String,
        #[arg(long)]
        remap: PathBuf,
        #[arg(long)]
        brief: Option<PathBuf>,
        #[arg(long)]
        preview_scale: Option<u16>,
        #[arg(long, default_value = "cli")]
        actor: String,
    },
    /// Replace palette colours without regenerating the source image.
    Recolor {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        /// Revision to edit; defaults to the current head.
        #[arg(long)]
        parent: Option<String>,
        /// Palette replacement in INDEX=#RRGGBB form. Repeat for multiple colours.
        #[arg(long = "set", required = true)]
        replacements: Vec<String>,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    /// Draw one or more indexed pixels as a single immutable revision.
    Draw {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        /// Revision to edit; defaults to the current head.
        #[arg(long)]
        parent: Option<String>,
        /// Pixel edit in `X,Y=PALETTE_INDEX` form. Repeat to draw a stroke.
        #[arg(long = "pixel", required = true)]
        pixels: Vec<String>,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    Inspect {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        revision: Option<String>,
    },
    Compare {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        left: String,
        #[arg(long)]
        right: String,
        #[arg(long)]
        preview_scale: Option<u16>,
        #[arg(long)]
        visual_native: Option<PathBuf>,
        #[arg(long)]
        visual_preview: Option<PathBuf>,
    },
    Review {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        revision: String,
        #[arg(long)]
        decision: ReviewDecision,
        #[arg(long, value_enum)]
        actor_kind: ReviewActorKind,
        #[arg(long)]
        actor: String,
        #[arg(long, default_value = "")]
        note: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum AssetCommand {
    /// List project assets and their current state.
    List {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    Init {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long, value_enum, default_value_t = Kind::Sprite)]
        kind: Kind,
        #[arg(long, default_value = "")]
        brief: String,
    },
    SetBrief {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        brief: String,
    },
    Delete {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
    },
    Rename {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        name: String,
    },
    /// Replace the source and reconvert with the asset's current style.
    UpdateSource {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        file: PathBuf,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    Export {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        destination: PathBuf,
        #[arg(long)]
        overwrite: bool,
    },
    Inspect {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ReferenceCommand {
    Import {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        file: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Kind {
    Sprite,
    Sheet,
    Tile,
    Ui,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum ConversionKind {
    Reference,
    Sheet,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum ReviewActorKind {
    Human,
    Agent,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum ReviewDecision {
    Reviewed,
    ChangesRequested,
    Accepted,
}
impl From<ReviewActorKind> for ProjectReviewActorKind {
    fn from(value: ReviewActorKind) -> Self {
        match value {
            ReviewActorKind::Human => Self::Human,
            ReviewActorKind::Agent => Self::Agent,
        }
    }
}
impl From<ReviewDecision> for ProjectReviewDecision {
    fn from(value: ReviewDecision) -> Self {
        match value {
            ReviewDecision::Reviewed => Self::Reviewed,
            ReviewDecision::ChangesRequested => Self::ChangesRequested,
            ReviewDecision::Accepted => Self::Accepted,
        }
    }
}
impl From<Kind> for AssetKind {
    fn from(value: Kind) -> Self {
        match value {
            Kind::Sprite => Self::Sprite,
            Kind::Sheet => Self::Sheet,
            Kind::Tile => Self::Tile,
            Kind::Ui => Self::Ui,
        }
    }
}
