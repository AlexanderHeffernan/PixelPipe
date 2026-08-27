use std::fs;

use pixelate_app::{
    DeleteAsset, ExportAsset, ExportAssetFile, ImportReference, InitializeAsset, InspectRevision,
    OpenProject, PreviewRevision, RenameAsset, UpdateAssetBrief, UpdateAssetSource, delete_asset,
    export_asset, export_asset_file, import_reference, initialize_asset, inspect_revision,
    open_project, preview_revision, rename_asset, update_asset_brief, update_asset_source,
};
use pixelate_project::ProjectStore;
use serde_json::json;

use crate::args::{AssetCommand, Cli, Command, ProjectCommand, ReferenceCommand, RevisionCommand};
use crate::edit::run_edit_revision;
use crate::guide::agent_guide;
use crate::pixelize::pixelize_command;

pub(crate) fn run(cli: Cli) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match cli.command {
        Command::Guide { root } => agent_guide(&root),
        Command::Init { root, name } => {
            let store = ProjectStore::init(&root, &name)?;
            let project_root = store.root().to_path_buf();
            let browser = open_project(OpenProject {
                start: project_root,
            })?;
            Ok(json!({
                "ok": true,
                "project_root": browser.project_root,
                "schema": browser.project.schema,
            }))
        }
        Command::Project { command } => run_project(command),
        Command::Revision { command } => run_revision(command),
        Command::Asset { command } => run_asset(command),
        Command::Reference { command } => run_reference(command),
    }
}

fn run_reference(
    command: ReferenceCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let ReferenceCommand::Import { root, asset, file } = command;
    let selection = import_reference(ImportReference {
        start: root,
        asset,
        file,
    })?;
    Ok(json!({ "ok": true, "selection": selection }))
}

fn run_project(command: ProjectCommand) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let ProjectCommand::Show { root } = command;
    let store = ProjectStore::discover(&root)?;
    Ok(json!({ "ok": true, "project_root": store.root(), "project": store.manifest()? }))
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
        AssetCommand::Init { root, asset, brief } => Ok(
            json!({ "ok": true, "asset": initialize_asset(InitializeAsset { start: root, asset, brief })? }),
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
        AssetCommand::ExportFile {
            root,
            asset,
            destination,
            overwrite,
        } => Ok(json!({
            "ok": true,
            "export": export_asset_file(ExportAssetFile {
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

fn run_revision(command: RevisionCommand) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        RevisionCommand::Pixelize { options } => pixelize_command(options),
        command @ RevisionCommand::Preview { .. } => preview_command(command),
        command @ (RevisionCommand::Fill { .. }
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
    }
}

fn preview_command(
    command: RevisionCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let RevisionCommand::Preview {
        root,
        asset,
        revision,
        scale,
        output,
    } = command
    else {
        unreachable!("preview_command only receives preview commands")
    };
    let preview = preview_revision(PreviewRevision {
        start: root,
        asset,
        revision,
        scale,
    })?;
    fs::write(&output, &preview.png)?;
    Ok(json!({ "ok": true, "preview": {
        "project_root": preview.project_root,
        "asset": preview.asset,
        "revision": preview.revision,
        "native_width": preview.native_width,
        "native_height": preview.native_height,
        "scale": preview.scale,
        "width": preview.width,
        "height": preview.height,
        "sha256": preview.sha256,
        "output": output,
    }}))
}
