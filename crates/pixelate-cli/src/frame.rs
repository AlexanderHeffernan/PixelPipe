use std::path::Path;

use pixelate_app::{
    FrameMutation, FrameMutationAction, ImportImageSequence, ImportSpritesheet,
    import_image_sequence, import_spritesheet, mutate_frames,
};
use pixelate_project::ProjectStore;
use serde_json::json;

use crate::args::FrameCommand;

pub(crate) fn run_frame(
    command: FrameCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let revision = match command {
        command @ (FrameCommand::Import { .. }
        | FrameCommand::Replace { .. }
        | FrameCommand::ImportSequence { .. }
        | FrameCommand::ImportSheet { .. }) => run_import(command)?,
        command @ (FrameCommand::Duration { .. } | FrameCommand::Rename { .. }) => {
            run_metadata_mutation(command)?
        }
        command => run_mutation(command)?,
    };
    Ok(json!({ "ok": true, "revision": revision }))
}

fn run_mutation(
    command: FrameCommand,
) -> Result<pixelate_app::RevisionResult, Box<dyn std::error::Error>> {
    match command {
        FrameCommand::Add {
            root,
            asset,
            parent,
            position,
            duration,
            actor,
        } => mutation(
            root,
            asset,
            parent,
            FrameMutationAction::AddBlank {
                position,
                duration_ms: duration,
            },
            actor,
        ),
        FrameCommand::Duplicate {
            root,
            asset,
            parent,
            frame,
            position,
            actor,
        } => mutation(
            root,
            asset,
            parent,
            FrameMutationAction::Duplicate {
                frame_id: frame,
                position,
            },
            actor,
        ),
        FrameCommand::Delete {
            root,
            asset,
            parent,
            frame,
            actor,
        } => mutation(
            root,
            asset,
            parent,
            FrameMutationAction::Delete { frame_id: frame },
            actor,
        ),
        FrameCommand::Reorder {
            root,
            asset,
            parent,
            frame,
            position,
            actor,
        } => mutation(
            root,
            asset,
            parent,
            FrameMutationAction::Reorder {
                frame_id: frame,
                position,
            },
            actor,
        ),
        FrameCommand::Duration { .. } | FrameCommand::Rename { .. } => {
            unreachable!("metadata commands are routed separately")
        }
        FrameCommand::Import { .. }
        | FrameCommand::Replace { .. }
        | FrameCommand::ImportSequence { .. }
        | FrameCommand::ImportSheet { .. } => {
            unreachable!("import commands are routed separately")
        }
    }
}

fn run_metadata_mutation(
    command: FrameCommand,
) -> Result<pixelate_app::RevisionResult, Box<dyn std::error::Error>> {
    let (root, asset, parent, action, actor) = match command {
        FrameCommand::Duration {
            root,
            asset,
            parent,
            frame,
            duration,
            actor,
        } => (
            root,
            asset,
            parent,
            match frame {
                Some(frame_id) => FrameMutationAction::SetDuration {
                    frame_id,
                    duration_ms: duration,
                },
                None => FrameMutationAction::SetAllDurations {
                    duration_ms: duration,
                },
            },
            actor,
        ),
        FrameCommand::Rename {
            root,
            asset,
            parent,
            frame,
            name,
            actor,
        } => (
            root,
            asset,
            parent,
            FrameMutationAction::Rename {
                frame_id: frame,
                name,
            },
            actor,
        ),
        _ => unreachable!("only frame metadata commands are routed here"),
    };
    mutation(root, asset, parent, action, actor)
}

fn run_import(
    command: FrameCommand,
) -> Result<pixelate_app::RevisionResult, Box<dyn std::error::Error>> {
    match command {
        FrameCommand::Import {
            root,
            asset,
            parent,
            file,
            position,
            duration,
            actor,
        } => mutation(
            root,
            asset,
            parent,
            FrameMutationAction::ImportFrame {
                file,
                position,
                duration_ms: duration,
            },
            actor,
        ),
        FrameCommand::Replace {
            root,
            asset,
            parent,
            frame,
            file,
            actor,
        } => mutation(
            root,
            asset,
            parent,
            FrameMutationAction::ReplaceFrame {
                frame_id: frame,
                file,
            },
            actor,
        ),
        FrameCommand::ImportSequence {
            root,
            asset,
            parent,
            files,
            duration,
            actor,
        } => {
            let parent = resolve_parent(&root, &asset, parent)?;
            Ok(import_image_sequence(ImportImageSequence {
                start: root,
                asset,
                parent,
                files,
                duration_ms: duration,
                actor,
            })?)
        }
        FrameCommand::ImportSheet {
            root,
            asset,
            parent,
            file,
            frame_width,
            frame_height,
            order,
            duration,
            actor,
        } => {
            let parent = resolve_parent(&root, &asset, parent)?;
            Ok(import_spritesheet(ImportSpritesheet {
                start: root,
                asset,
                parent,
                file,
                frame_width,
                frame_height,
                order,
                duration_ms: duration,
                actor,
            })?)
        }
        _ => unreachable!("only import commands are routed here"),
    }
}

fn mutation(
    root: std::path::PathBuf,
    asset: String,
    parent: Option<String>,
    action: FrameMutationAction,
    actor: String,
) -> Result<pixelate_app::RevisionResult, Box<dyn std::error::Error>> {
    let parent = resolve_parent(&root, &asset, parent)?;
    Ok(mutate_frames(FrameMutation {
        start: root,
        asset,
        parent,
        action,
        actor,
    })?)
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
