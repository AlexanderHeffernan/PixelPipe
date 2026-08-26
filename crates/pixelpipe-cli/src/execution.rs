use std::{
    fs,
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

use pixelpipe_app::{
    AgentRunStatus, AgentRuntime, AgentTaskRequest, BrowseAgentRuns, CompareRevisions,
    ConversionMode, ConvertRevision, ConvertSelectedReference, CreateRevision, DeleteAsset,
    ExportAsset, ImportReference, InitializeAsset, InspectRevision, LoadAgentCandidate,
    PreviewSelectedReference, RecordReview, RenameAsset, SelectAgentCandidate, StoreProjectPalette,
    StoreProjectRecipe, UpdateAssetBrief, UpdateAssetSource, approve_agent_connector,
    browse_agent_runs, compare_revisions, convert_revision, convert_selected_reference,
    create_revision, delete_asset, detect_agent_connectors, export_asset, import_reference,
    initialize_asset, inspect_revision, load_agent_candidate, preview_selected_reference,
    record_review, rename_asset, select_agent_candidate, store_project_palette,
    store_project_recipe, update_asset_brief, update_asset_source,
};
use pixelpipe_core::{ConversionSettings, SheetSettings};
use pixelpipe_project::ProjectStore;
use serde_json::json;

use crate::args::{
    AgentCommand, AssetCommand, Cli, Command, ConversionKind, ProjectCommand, ReferenceCommand,
    RevisionCommand,
};
use crate::edit::run_edit_revision;
use crate::guide::agent_guide;
use crate::pixelize::pixelize_command;

pub(crate) fn run(cli: Cli) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match cli.command {
        Command::Guide { root } => agent_guide(&root),
        Command::Init { root, name } => {
            let store = ProjectStore::init(&root, &name)?;
            Ok(
                json!({ "ok": true, "project_root": store.root(), "schema": store.manifest()?.schema }),
            )
        }
        Command::Project { command } => run_project(command),
        Command::Revision { command } => run_revision(command),
        Command::Asset { command } => run_asset(command),
        Command::Agent { command } => run_agent(command),
        Command::Reference { command } => run_reference(command),
    }
}

fn run_reference(
    command: ReferenceCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let selection = match command {
        ReferenceCommand::Import { root, asset, file } => import_reference(ImportReference {
            start: root,
            asset,
            file,
        })?,
        ReferenceCommand::Select {
            root,
            asset,
            run,
            candidate,
        } => select_agent_candidate(SelectAgentCandidate {
            start: root,
            asset,
            run,
            candidate,
        })?,
    };
    Ok(json!({ "ok": true, "selection": selection }))
}

fn run_project(command: ProjectCommand) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        ProjectCommand::Show { root } => {
            let store = ProjectStore::discover(&root)?;
            Ok(
                json!({ "ok": true, "project_root": store.root(), "project": store.manifest()?, "recipes": store.conversion_recipes()? }),
            )
        }
        ProjectCommand::SetPalette { root, id, file } => {
            store_project_palette(StoreProjectPalette {
                start: root,
                id: id.clone(),
                file,
            })?;
            Ok(json!({ "ok": true, "palette": id }))
        }
        ProjectCommand::SetRecipe { root, file } => {
            let recipe = store_project_recipe(StoreProjectRecipe { start: root, file })?;
            Ok(json!({ "ok": true, "recipe": recipe.id }))
        }
    }
}

fn run_asset(command: AssetCommand) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        AssetCommand::List { root } => {
            let store = ProjectStore::discover(&root)?;
            Ok(json!({
                "ok": true,
                "project_root": store.root(),
                "assets": store.assets()?,
            }))
        }
        AssetCommand::Init {
            root,
            asset,
            kind,
            brief,
        } => Ok(
            json!({ "ok": true, "asset": initialize_asset(InitializeAsset { start: root, asset, kind: kind.into(), brief })? }),
        ),
        AssetCommand::SetBrief { root, asset, brief } => Ok(
            json!({ "ok": true, "asset": update_asset_brief(UpdateAssetBrief { start: root, asset, brief })? }),
        ),
        AssetCommand::Delete { root, asset } => {
            delete_asset(DeleteAsset {
                start: root,
                asset: asset.clone(),
            })?;
            Ok(json!({ "ok": true, "asset": asset, "deleted": true }))
        }
        AssetCommand::Rename { root, asset, name } => Ok(json!({
            "ok": true,
            "asset": rename_asset(RenameAsset { start: root, asset, display_name: name })?
        })),
        AssetCommand::UpdateSource {
            root,
            asset,
            file,
            actor,
        } => Ok(json!({
            "ok": true,
            "update": update_asset_source(UpdateAssetSource {
                start: root,
                asset,
                file,
                actor,
            })?
        })),
        AssetCommand::Export {
            root,
            asset,
            destination,
            overwrite,
        } => Ok(json!({
            "ok": true,
            "export": export_asset(ExportAsset {
                start: root,
                asset,
                destination,
                overwrite,
            })?
        })),
        AssetCommand::Inspect { root, asset } => {
            let store = ProjectStore::discover(&root)?;
            Ok(json!({ "ok": true, "project_root": store.root(), "asset": store.asset(&asset)? }))
        }
    }
}

fn run_agent(command: AgentCommand) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        AgentCommand::Detect => Ok(json!({
            "ok": true,
            "connectors": detect_agent_connectors()?,
        })),
        AgentCommand::Approve { connector } => Ok(json!({
            "ok": true,
            "connector": approve_agent_connector(pixelpipe_app::ApproveAgentConnector {
                id: connector,
            })?,
        })),
        AgentCommand::Run {
            root,
            asset,
            profile,
            operation,
            revision,
            prompt,
        } => {
            let sink = Arc::new(|event| {
                eprintln!(
                    "{}",
                    serde_json::to_string(&event).expect("agent event must serialize")
                );
            });
            let result = AgentRuntime::user_local()?.run(
                AgentTaskRequest {
                    start: root,
                    asset,
                    profile,
                    operation: operation.into(),
                    revision,
                    prompt,
                },
                &AtomicBool::new(false),
                sink,
            )?;
            if result.run.status != AgentRunStatus::Completed {
                return Err(format!(
                    "agent run '{}' ended with {:?}",
                    result.run.id, result.run.status
                )
                .into());
            }
            Ok(json!({ "ok": true, "run": result.run }))
        }
        AgentCommand::Runs { root, asset } => Ok(
            json!({ "ok": true, "runs": browse_agent_runs(BrowseAgentRuns { start: root, asset })? }),
        ),
        AgentCommand::Candidate {
            root,
            run,
            candidate,
            output,
        } => {
            fs::write(
                &output,
                load_agent_candidate(LoadAgentCandidate {
                    start: root,
                    run,
                    candidate,
                })?,
            )?;
            Ok(json!({ "ok": true, "output": output }))
        }
    }
}

fn run_revision(command: RevisionCommand) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        RevisionCommand::Pixelize { options } => pixelize_command(options),
        RevisionCommand::Create {
            root,
            asset,
            kind,
            pixels,
            brief,
            preview_scale,
            actor,
        } => Ok(
            json!({ "ok": true, "revision": create_revision(CreateRevision { start: root, asset, kind: kind.into(), raster_path: pixels, brief_path: brief, preview_scale, actor })? }),
        ),
        command @ RevisionCommand::Convert { .. } => convert_command(command),
        RevisionCommand::ConvertSelected {
            root,
            asset,
            recipe,
            palette,
            colors,
            settings,
            auto_background,
            actor,
        } => Ok(
            json!({ "ok": true, "revision": convert_selected_reference(ConvertSelectedReference { start: root, asset, recipe, palette, color_count: colors, palette_overrides: Vec::new(), settings: read_reference_settings(settings)?, auto_background, actor })? }),
        ),
        RevisionCommand::PreviewSelected {
            root,
            asset,
            recipe,
            palette,
            colors,
            settings,
            auto_background,
            native,
        } => preview_selected_command(
            PreviewSelectedReference {
                start: root,
                asset,
                recipe,
                palette,
                color_count: colors,
                palette_overrides: Vec::new(),
                settings: read_reference_settings(settings)?,
                auto_background,
            },
            &native,
        ),
        command @ (RevisionCommand::Patch { .. }
        | RevisionCommand::Fill { .. }
        | RevisionCommand::Compose { .. }
        | RevisionCommand::SetHead { .. }
        | RevisionCommand::Remap { .. }
        | RevisionCommand::Recolor { .. }
        | RevisionCommand::Draw { .. }) => run_edit_revision(command),
        RevisionCommand::Inspect {
            root,
            asset,
            revision,
        } => Ok(
            json!({ "ok": true, "revision": inspect_revision(InspectRevision { start: root, asset, revision })? }),
        ),
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
        } => Ok(
            json!({ "ok": true, "review": record_review(RecordReview { start: root, asset, revision, actor, actor_kind: actor_kind.into(), decision: decision.into(), note })? }),
        ),
    }
}

fn preview_selected_command(
    request: PreviewSelectedReference,
    native: &std::path::Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let preview = preview_selected_reference(request)?;
    fs::write(native, &preview.native_png)?;
    Ok(json!({ "ok": true, "preview": {
        "inspection": preview.inspection,
        "palette_name": preview.palette_name,
        "native": native,
    }}))
}

fn read_reference_settings(
    path: Option<PathBuf>,
) -> Result<Option<ConversionSettings>, Box<dyn std::error::Error>> {
    path.map(|path| Ok(serde_json::from_slice(&fs::read(path)?)?))
        .transpose()
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
        json!({ "ok": true, "revision": convert_revision(ConvertRevision { start: root, asset, kind: kind.into(), source_path: source, palette_path: palette, mode, brief_path: brief, preview_scale, actor })? }),
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
    Ok(json!({ "ok": true, "comparison": {
        "project_root": comparison.project_root, "asset": comparison.asset,
        "left": comparison.left, "right": comparison.right, "diff": comparison.diff,
        "visual_native": visual_native, "visual_preview": visual_preview,
        "visual_native_sha256": comparison.visual_native_sha256,
        "visual_preview_sha256": comparison.visual_preview_sha256,
    }}))
}
