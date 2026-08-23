use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use pixelpipe_app::AgentOperation;
use pixelpipe_project::{
    AssetKind, ReviewActorKind as ProjectReviewActorKind, ReviewDecision as ProjectReviewDecision,
};

#[derive(Debug, Parser)]
#[command(
    name = "pixelpipe",
    version,
    about = "Deterministic pixel-art project pipeline"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
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
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
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
        #[arg(long, default_value = "cli")]
        actor: String,
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
    Inspect {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum AgentCommand {
    Run {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        profile: String,
        #[arg(long, value_enum)]
        operation: AgentOperationKind,
        #[arg(long)]
        revision: Option<String>,
        #[arg(long)]
        prompt: String,
    },
    Runs {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
    },
    Candidate {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        run: String,
        #[arg(long)]
        candidate: String,
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ReferenceCommand {
    Select {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
        #[arg(long)]
        run: String,
        #[arg(long)]
        candidate: String,
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
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum AgentOperationKind {
    Generate,
    Critique,
    Propose,
}

impl From<AgentOperationKind> for AgentOperation {
    fn from(value: AgentOperationKind) -> Self {
        match value {
            AgentOperationKind::Generate => Self::GenerateReferences,
            AgentOperationKind::Critique => Self::CritiqueAsset,
            AgentOperationKind::Propose => Self::ProposeRefinement,
        }
    }
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
