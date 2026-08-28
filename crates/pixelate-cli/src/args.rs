use std::path::PathBuf;

use clap::{Parser, Subcommand};

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
    /// Print the installed CLI version as JSON.
    Version,
    /// Install the latest signed standalone CLI release.
    Update,
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
    /// Mutate or import the ordered frames of one asset clip.
    Frame {
        #[command(subcommand)]
        command: FrameCommand,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ProjectCommand {
    Show {
        #[arg(long, default_value = ".")]
        root: PathBuf,
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
        #[arg(long)]
        frame: Option<String>,
        /// Integer nearest-neighbour scale; defaults toward a 512px longest edge.
        #[arg(long, value_parser = clap::value_parser!(u16).range(1..=64))]
        scale: Option<u16>,
        #[arg(long)]
        output: PathBuf,
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
        /// Stable frame ID; required when the revision has multiple frames.
        #[arg(long)]
        frame: Option<String>,
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
        /// Stable frame ID; required when the revision has multiple frames.
        #[arg(long)]
        frame: Option<String>,
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
        #[arg(long)]
        frame: Option<String>,
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
    /// Export the current revision to a named PNG or WebP file.
    ExportFile {
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

#[derive(Debug, Subcommand)]
pub(crate) enum FrameCommand {
    Add {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        position: Option<usize>,
        #[arg(long)]
        duration: Option<u32>,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    Duplicate {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        frame: String,
        #[arg(long)]
        position: Option<usize>,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    Delete {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        frame: String,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    Reorder {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        frame: String,
        /// Zero-based destination position.
        #[arg(long)]
        position: usize,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    Duration {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        frame: String,
        #[arg(long)]
        duration: u32,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    Import {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        position: Option<usize>,
        #[arg(long)]
        duration: Option<u32>,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    ImportSequence {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: Option<String>,
        /// Ordered image path; repeat in the intended playback order.
        #[arg(long = "file", required = true)]
        files: Vec<PathBuf>,
        #[arg(long, default_value_t = 100)]
        duration: u32,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
    ImportSheet {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        frame_width: u32,
        #[arg(long)]
        frame_height: u32,
        /// Zero-based cell index; repeat in the intended playback order.
        #[arg(long = "cell", required = true)]
        order: Vec<usize>,
        #[arg(long, default_value_t = 100)]
        duration: u32,
        #[arg(long, default_value = "agent")]
        actor: String,
    },
}
