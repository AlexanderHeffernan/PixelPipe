use std::path::Path;

use pixelate_app::{
    CommitComposition, FillRevisionDocument, RemapRevision, SetAssetHead, commit_composition,
    fill_revision_document, remap_revision, set_asset_head,
};
use pixelate_core::CanvasSettings;
use pixelate_project::ProjectStore;
use serde_json::json;

use crate::{args::RevisionCommand, draw::draw_command, recolor::recolor_command};

pub(crate) fn run_edit_revision(
    command: RevisionCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        RevisionCommand::Fill {
            root,
            asset,
            parent,
            x,
            y,
            index,
            frame,
            actor,
        } => {
            let parent = resolve_parent(&root, &asset, parent)?;
            Ok(
                json!({ "ok": true, "revision": fill_revision_document(FillRevisionDocument {
            start: root, asset, parent, x, y, index, frame_id: frame, actor
        })? }),
            )
        }
        RevisionCommand::Compose {
            root,
            asset,
            parent,
            width,
            height,
            scale,
            offset_x,
            offset_y,
            actor,
        } => {
            let parent = resolve_parent(&root, &asset, parent)?;
            Ok(
                json!({ "ok": true, "revision": commit_composition(CommitComposition {
            start: root,
            asset,
            parent,
            settings: CanvasSettings {
                width,
                height,
                scale_percent: scale,
                offset_x,
                offset_y,
            },
            actor,
        })? }),
            )
        }
        RevisionCommand::SetHead {
            root,
            asset,
            revision,
        } => Ok(json!({ "ok": true, "asset": set_asset_head(SetAssetHead {
            start: root, asset, revision
        })? })),
        RevisionCommand::Remap {
            root,
            asset,
            parent,
            remap,
            brief,
            actor,
        } => Ok(
            json!({ "ok": true, "revision": remap_revision(RemapRevision { start: root, asset, parent, remap_path: remap, brief_path: brief, actor })? }),
        ),
        RevisionCommand::Recolor {
            root,
            asset,
            parent,
            replacements,
            actor,
        } => recolor_command(root, asset, parent, &replacements, actor),
        RevisionCommand::Draw {
            root,
            asset,
            parent,
            pixels,
            frame,
            actor,
        } => draw_command(root, asset, parent, &pixels, frame, actor),
        _ => unreachable!("editing revision command is filtered by run_revision"),
    }
}

fn resolve_parent(
    root: &Path,
    asset: &str,
    parent: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(parent) = parent {
        return Ok(parent);
    }
    ProjectStore::discover(root)?
        .asset(asset)?
        .head
        .ok_or_else(|| format!("asset '{asset}' has no current revision").into())
}
