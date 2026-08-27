use std::path::Path;

use pixelate_app::{
    CommitComposition, FillRevisionDocument, PatchRevision, RemapRevision, SetAssetHead,
    commit_composition, fill_revision_document, patch_revision, remap_revision, set_asset_head,
};
use pixelate_core::CanvasSettings;
use pixelate_project::ProjectStore;
use serde_json::json;

use crate::{args::RevisionCommand, draw::draw_command, recolor::recolor_command};

pub(crate) fn run_edit_revision(
    command: RevisionCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        RevisionCommand::Patch {
            root,
            asset,
            parent,
            patch,
            brief,
            preview_scale,
            actor,
        } => Ok(
            json!({ "ok": true, "revision": patch_revision(PatchRevision { start: root, asset, parent, patch_path: patch, brief_path: brief, preview_scale, actor })? }),
        ),
        RevisionCommand::Fill {
            root,
            asset,
            parent,
            x,
            y,
            index,
            actor,
        } => {
            let parent = resolve_parent(&root, &asset, parent)?;
            Ok(
                json!({ "ok": true, "revision": fill_revision_document(FillRevisionDocument {
            start: root, asset, parent, x, y, index, actor
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
            preview_scale,
            actor,
        } => Ok(
            json!({ "ok": true, "revision": remap_revision(RemapRevision { start: root, asset, parent, remap_path: remap, brief_path: brief, preview_scale, actor })? }),
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
            actor,
        } => draw_command(root, asset, parent, &pixels, actor),
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
