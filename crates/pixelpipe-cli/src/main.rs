use std::{fs, path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand, ValueEnum};
use pixelpipe_app::{
    CompareRevisions, ConversionMode, ConvertRevision, CreateRevision, InspectRevision,
    PatchRevision, RecordReview, RemapRevision, compare_revisions, convert_revision,
    create_revision, inspect_revision, patch_revision, record_review, remap_revision,
};
use pixelpipe_core::{ConversionSettings, SheetSettings};
use pixelpipe_project::{
    AssetKind, ProjectStore, ReviewActorKind as ProjectReviewActorKind,
    ReviewDecision as ProjectReviewDecision,
};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Parser)]
#[command(
    name = "pixelpipe",
    version,
    about = "Deterministic pixel-art project pipeline"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
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
}

#[derive(Debug, Subcommand)]
enum ProjectCommand {
    Show {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum RevisionCommand {
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
enum AssetCommand {
    Inspect {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        asset: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Kind {
    Sprite,
    Sheet,
    Tile,
    Ui,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ConversionKind {
    Reference,
    Sheet,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ReviewActorKind {
    Human,
    Agent,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ReviewDecision {
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

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(value) => {
            print_json(&value);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": false,
                    "error": error.to_string(),
                }))
                .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"serialization failure\"}".to_owned())
            );
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match cli.command {
        Command::Init { root, name } => {
            let store = ProjectStore::init(&root, &name)?;
            Ok(json!({
                "ok": true,
                "project_root": store.root(),
                "schema": store.manifest()?.schema,
            }))
        }
        Command::Project {
            command: ProjectCommand::Show { root },
        } => {
            let store = ProjectStore::discover(&root)?;
            Ok(json!({
                "ok": true,
                "project_root": store.root(),
                "project": store.manifest()?,
            }))
        }
        Command::Revision { command } => run_revision(command),
        Command::Asset {
            command: AssetCommand::Inspect { root, asset },
        } => {
            let store = ProjectStore::discover(&root)?;
            Ok(json!({
                "ok": true,
                "project_root": store.root(),
                "asset": store.asset(&asset)?,
            }))
        }
    }
}

fn run_revision(command: RevisionCommand) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        RevisionCommand::Create {
            root,
            asset,
            kind,
            pixels,
            brief,
            preview_scale,
            actor,
        } => Ok(
            json!({ "ok": true, "revision": create_revision(CreateRevision {
            start: root, asset, kind: kind.into(), raster_path: pixels,
            brief_path: brief, preview_scale, actor,
        })? }),
        ),
        command @ RevisionCommand::Convert { .. } => convert_command(command),
        RevisionCommand::Patch {
            root,
            asset,
            parent,
            patch,
            brief,
            preview_scale,
            actor,
        } => Ok(
            json!({ "ok": true, "revision": patch_revision(PatchRevision {
            start: root, asset, parent, patch_path: patch, brief_path: brief,
            preview_scale, actor,
        })? }),
        ),
        RevisionCommand::Remap {
            root,
            asset,
            parent,
            remap,
            brief,
            preview_scale,
            actor,
        } => Ok(
            json!({ "ok": true, "revision": remap_revision(RemapRevision {
            start: root, asset, parent, remap_path: remap, brief_path: brief,
            preview_scale, actor,
        })? }),
        ),
        RevisionCommand::Inspect {
            root,
            asset,
            revision,
        } => Ok(json!({
            "ok": true,
            "revision": inspect_revision(InspectRevision { start: root, asset, revision })?,
        })),
        RevisionCommand::Compare {
            root,
            asset,
            left,
            right,
            preview_scale,
            visual_native,
            visual_preview,
        } => compare_command(
            CompareRevisions {
                start: root,
                asset,
                left,
                right,
                preview_scale,
            },
            visual_native,
            visual_preview,
        ),
        RevisionCommand::Review {
            root,
            asset,
            revision,
            decision,
            actor_kind,
            actor,
            note,
        } => Ok(json!({ "ok": true, "review": record_review(RecordReview {
            start: root, asset, revision, actor, actor_kind: actor_kind.into(),
            decision: decision.into(), note,
        })? })),
    }
}

fn convert_command(
    command: RevisionCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let RevisionCommand::Convert {
        root,
        asset,
        kind,
        source,
        palette,
        settings,
        conversion,
        brief,
        preview_scale,
        actor,
    } = command
    else {
        unreachable!("convert_command only receives convert commands")
    };
    let settings = fs::read(settings)?;
    let mode = match conversion {
        ConversionKind::Reference => {
            ConversionMode::Reference(serde_json::from_slice::<ConversionSettings>(&settings)?)
        }
        ConversionKind::Sheet => {
            ConversionMode::Sheet(serde_json::from_slice::<SheetSettings>(&settings)?)
        }
    };
    Ok(
        json!({ "ok": true, "revision": convert_revision(ConvertRevision {
        start: root, asset, kind: kind.into(), source_path: source,
        palette_path: palette, mode, brief_path: brief, preview_scale, actor,
    })? }),
    )
}

fn compare_command(
    request: CompareRevisions,
    visual_native: Option<PathBuf>,
    visual_preview: Option<PathBuf>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let comparison = compare_revisions(request)?;
    let visual_native = visual_native
        .map(|path| fs::write(&path, &comparison.visual_native_png).map(|()| path))
        .transpose()?;
    let visual_preview = visual_preview
        .map(|path| fs::write(&path, &comparison.visual_preview_png).map(|()| path))
        .transpose()?;
    Ok(json!({
        "ok": true,
        "comparison": {
            "project_root": comparison.project_root,
            "asset": comparison.asset,
            "left": comparison.left,
            "right": comparison.right,
            "diff": comparison.diff,
            "visual_native": visual_native,
            "visual_preview": visual_preview,
            "visual_native_sha256": comparison.visual_native_sha256,
            "visual_preview_sha256": comparison.visual_preview_sha256,
        },
    }))
}

fn print_json(value: &impl Serialize) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("CLI response must serialize")
    );
}
