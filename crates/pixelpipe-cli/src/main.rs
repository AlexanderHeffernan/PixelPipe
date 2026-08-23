use std::{path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand, ValueEnum};
use pixelpipe_app::{CreateRevision, create_revision};
use pixelpipe_project::{AssetKind, ProjectStore};
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
        Command::Revision {
            command:
                RevisionCommand::Create {
                    root,
                    asset,
                    kind,
                    pixels,
                    brief,
                    preview_scale,
                    actor,
                },
        } => Ok(json!({
            "ok": true,
            "revision": create_revision(CreateRevision {
                start: root,
                asset,
                kind: kind.into(),
                raster_path: pixels,
                brief_path: brief,
                preview_scale,
                actor,
            })?,
        })),
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

fn print_json(value: &impl Serialize) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("CLI response must serialize")
    );
}
