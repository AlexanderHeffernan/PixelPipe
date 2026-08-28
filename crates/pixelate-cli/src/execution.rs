use std::fs;

use pixelate_app::{
    AdoptPixelArt, AdoptProjectImage, BrowseProject, CreateFolder, DeleteAsset, DeleteFolder,
    ExportAsset, ExportAssetFile, ImportReference, InitializeAsset, InspectRevision, MoveAsset,
    MoveFolder, OpenProject, PreviewRevision, RelinkAsset, RenameAsset, SetProjectImageIgnored,
    UpdateAssetBrief, UpdateAssetSource, UpdateLinkedSource, adopt_pixel_art, adopt_project_image,
    browse_project, create_folder, delete_asset, delete_folder, export_asset, export_asset_file,
    import_reference, initialize_asset, inspect_revision, move_asset, move_folder, open_project,
    preview_revision, relink_asset, rename_asset, set_project_image_ignored, update_asset_brief,
    update_asset_source, update_linked_source,
};
use pixelate_project::ProjectStore;
use serde_json::json;

use crate::args::{AssetCommand, Cli, Command, ProjectCommand, ReferenceCommand, RevisionCommand};
use crate::edit::run_edit_revision;
use crate::guide::agent_guide;
use crate::pixelize::pixelize_command;
use crate::update::update_cli;

pub(crate) fn run(cli: Cli) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match cli.command {
        Command::Version => Ok(json!({
            "ok": true,
            "version": env!("CARGO_PKG_VERSION"),
            "target": format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
        })),
        Command::Update => update_cli(),
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
    match command {
        ProjectCommand::Show { root } => {
            let store = ProjectStore::discover(&root)?;
            Ok(json!({ "ok": true, "project_root": store.root(), "project": store.manifest()? }))
        }
        ProjectCommand::Catalog { root } => {
            let browser = browse_project(&BrowseProject { start: root })?;
            Ok(
                json!({ "ok": true, "project_root": browser.project_root, "catalog": browser.catalog }),
            )
        }
        ProjectCommand::CreateFolder { root, path } => {
            create_folder(CreateFolder {
                start: root,
                path: path.clone(),
            })?;
            Ok(json!({ "ok": true, "path": path }))
        }
        ProjectCommand::MoveFolder {
            root,
            source,
            destination,
        } => {
            let assets = move_folder(MoveFolder {
                start: root,
                source: source.clone(),
                destination: destination.clone(),
            })?;
            Ok(
                json!({ "ok": true, "source": source, "destination": destination, "assets": assets }),
            )
        }
        ProjectCommand::DeleteFolder { root, path } => {
            delete_folder(DeleteFolder {
                start: root,
                path: path.clone(),
            })?;
            Ok(json!({ "ok": true, "path": path, "deleted": true }))
        }
        ProjectCommand::HideImage { root, path } => {
            set_project_image_ignored(SetProjectImageIgnored {
                start: root,
                path: path.clone(),
                ignored: true,
            })?;
            Ok(json!({ "ok": true, "path": path, "hidden": true }))
        }
        ProjectCommand::ShowImage { root, path } => {
            set_project_image_ignored(SetProjectImageIgnored {
                start: root,
                path: path.clone(),
                ignored: false,
            })?;
            Ok(json!({ "ok": true, "path": path, "hidden": false }))
        }
    }
}

fn run_asset(command: AssetCommand) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        AssetCommand::List { root } => list_assets_command(&root),
        AssetCommand::Init {
            root,
            asset,
            brief,
            path,
        } => initialize_asset_command(root, asset, brief, path),
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
        AssetCommand::Adopt {
            root,
            path,
            asset,
            brief,
            destination,
        } => adopt_reference_command(root, path, asset, brief, destination),
        AssetCommand::AdoptPixelArt {
            root,
            path,
            asset,
            brief,
            actor,
        } => adopt_pixel_art_command(root, path, asset, brief, actor),
        AssetCommand::Relink { root, asset, path } => Ok(json!({
            "ok": true,
            "asset": relink_asset(RelinkAsset { start: root, asset, path })?
        })),
        AssetCommand::Move {
            root,
            asset,
            destination,
        } => Ok(json!({
            "ok": true,
            "asset": move_asset(MoveAsset { start: root, asset, destination })?
        })),
        AssetCommand::UpdateLinkedSource { root, asset } => Ok(json!({
            "ok": true,
            "asset": update_linked_source(UpdateLinkedSource { start: root, asset })?
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

fn list_assets_command(
    root: &std::path::Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let store = ProjectStore::discover(root)?;
    Ok(json!({
        "ok": true,
        "project_root": store.root(),
        "assets": store.assets()?,
    }))
}

fn initialize_asset_command(
    root: std::path::PathBuf,
    asset: String,
    brief: String,
    project_path: Option<String>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(json!({
        "ok": true,
        "asset": initialize_asset(InitializeAsset { start: root, asset, brief, project_path })?
    }))
}

fn adopt_reference_command(
    root: std::path::PathBuf,
    path: String,
    asset: String,
    brief: String,
    destination: String,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(json!({
        "ok": true,
        "asset": adopt_project_image(AdoptProjectImage {
            start: root, path, asset, brief, destination
        })?
    }))
}

fn adopt_pixel_art_command(
    root: std::path::PathBuf,
    path: String,
    asset: String,
    brief: String,
    actor: String,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(json!({
        "ok": true,
        "revision": adopt_pixel_art(AdoptPixelArt {
            start: root, path, asset, brief, actor
        })?
    }))
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
