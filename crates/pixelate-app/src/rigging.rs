use std::{collections::BTreeMap, path::PathBuf};

use pixelate_core::{
    CoreError, Operation, PixelRig, RECIPE_SCHEMA, Recipe, RigOperation, ValidationCheck,
    sha256_hex, stable_json,
};
use pixelate_project::{ProjectStore, RevisionSnapshot};
use serde::Deserialize;

use crate::{
    AppError, CommitSequence, RevisionResult, RigDefinition, commit_sequence,
    rig_definition::build_rig,
};

#[derive(Debug, Deserialize)]
pub struct CreateRig {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    #[serde(default)]
    pub source_frame_id: Option<String>,
    pub definition: RigDefinition,
    #[serde(default = "default_actor")]
    pub actor: String,
}

#[derive(Debug, Deserialize)]
pub struct MutateRig {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    pub action: RigMutation,
    #[serde(default = "default_actor")]
    pub actor: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum RigMutation {
    UpdateNode {
        pose_id: String,
        node_id: String,
        x_millis: Option<i32>,
        y_millis: Option<i32>,
        rotation_millidegrees: Option<i32>,
        scale_x_millis: Option<i32>,
        scale_y_millis: Option<i32>,
        depth: Option<i32>,
        visible: Option<bool>,
        part_id: Option<String>,
    },
    SwapParts {
        first_node_id: String,
        second_node_id: String,
    },
    SetInterpolation {
        inbetweens: u16,
        looped: bool,
    },
    SetDuration {
        duration_ms: u32,
    },
    DuplicatePose {
        pose_id: String,
        new_pose_id: String,
        name: Option<String>,
    },
    DeletePose {
        pose_id: String,
    },
    ReorderPose {
        pose_id: String,
        position: usize,
    },
    RenamePose {
        pose_id: String,
        name: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct BakeRig {
    pub start: PathBuf,
    pub asset: String,
    pub parent: String,
    #[serde(default = "default_actor")]
    pub actor: String,
}

/// Creates a generic pixel-part rig by cropping an indexed revision frame.
///
/// # Errors
/// Returns [`AppError`] when the parent, definition, crop, rig, or commit is invalid.
pub fn create_rig(request: CreateRig) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let parent = store.revision(&request.asset, &request.parent)?;
    let rig = build_rig(
        &parent,
        request.source_frame_id.as_deref(),
        request.definition,
    )?;
    commit_rig(
        &store,
        &request.asset,
        &request.parent,
        parent,
        rig,
        RigOperation::Create,
        request.actor,
    )
}

/// Mutates one complete generic rig and commits its deterministically rendered sequence.
///
/// # Errors
/// Returns [`AppError`] when the parent has no rig or the mutation is invalid.
pub fn mutate_rig(request: MutateRig) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let parent = store.revision(&request.asset, &request.parent)?;
    let mut rig = parent
        .rig
        .clone()
        .ok_or_else(|| CoreError::InvalidRig("revision has no editable rig".to_owned()))?;
    let operation = apply_mutation(&mut rig, request.action)?;
    commit_rig(
        &store,
        &request.asset,
        &request.parent,
        parent,
        rig,
        operation,
        request.actor,
    )
}

/// Removes rig authoring data while preserving its exact rendered sequence.
///
/// # Errors
/// Returns [`AppError`] when the parent has no rig or commit fails.
pub fn bake_rig(request: BakeRig) -> Result<RevisionResult, AppError> {
    let store = ProjectStore::discover(&request.start)?;
    let parent = store.revision(&request.asset, &request.parent)?;
    if parent.rig.is_none() {
        return Err(CoreError::InvalidRig("revision has no editable rig".to_owned()).into());
    }
    commit(
        &store,
        &request.asset,
        &request.parent,
        parent.clone(),
        parent.sequence,
        None,
        RigOperation::Bake,
        request.actor,
    )
}

fn apply_mutation(rig: &mut PixelRig, action: RigMutation) -> Result<RigOperation, AppError> {
    let operation = match action {
        RigMutation::UpdateNode {
            pose_id,
            node_id,
            x_millis,
            y_millis,
            rotation_millidegrees,
            scale_x_millis,
            scale_y_millis,
            depth,
            visible,
            part_id,
        } => {
            let node = posed_node_mut(rig, &pose_id, &node_id)?;
            if let Some(value) = x_millis {
                node.x_millis = value;
            }
            if let Some(value) = y_millis {
                node.y_millis = value;
            }
            if let Some(value) = rotation_millidegrees {
                node.rotation_millidegrees = value;
            }
            if let Some(value) = scale_x_millis {
                node.scale_x_millis = value;
            }
            if let Some(value) = scale_y_millis {
                node.scale_y_millis = value;
            }
            if let Some(value) = depth {
                node.depth = value;
            }
            if let Some(value) = visible {
                node.visible = value;
            }
            if let Some(value) = part_id {
                node.part_id = value;
            }
            RigOperation::UpdateNode { pose_id, node_id }
        }
        RigMutation::SwapParts {
            first_node_id,
            second_node_id,
        } => {
            for pose in &mut rig.poses {
                let first = pose
                    .nodes
                    .iter()
                    .position(|node| node.node_id == first_node_id)
                    .ok_or_else(|| invalid_id("node", &first_node_id))?;
                let second = pose
                    .nodes
                    .iter()
                    .position(|node| node.node_id == second_node_id)
                    .ok_or_else(|| invalid_id("node", &second_node_id))?;
                let first_part = pose.nodes[first].part_id.clone();
                pose.nodes[first].part_id = pose.nodes[second].part_id.clone();
                pose.nodes[second].part_id = first_part;
            }
            RigOperation::SwapParts {
                first_node_id,
                second_node_id,
            }
        }
        RigMutation::SetInterpolation { inbetweens, looped } => {
            rig.interpolation.inbetweens = inbetweens;
            rig.interpolation.looped = looped;
            RigOperation::SetInterpolation { inbetweens, looped }
        }
        RigMutation::SetDuration { duration_ms } => {
            rig.frame_duration_ms = duration_ms;
            RigOperation::SetDuration { duration_ms }
        }
        RigMutation::DuplicatePose {
            pose_id,
            new_pose_id,
            name,
        } => {
            let index = pose_index(rig, &pose_id)?;
            let mut pose = rig.poses[index].clone();
            pose.id.clone_from(&new_pose_id);
            pose.name = name;
            rig.poses.insert(index + 1, pose);
            RigOperation::DuplicatePose {
                pose_id,
                new_pose_id,
            }
        }
        RigMutation::DeletePose { pose_id } => {
            if rig.poses.len() == 1 {
                return Err(
                    CoreError::InvalidRig("cannot delete the final rig pose".to_owned()).into(),
                );
            }
            rig.poses.remove(pose_index(rig, &pose_id)?);
            RigOperation::DeletePose { pose_id }
        }
        RigMutation::ReorderPose { pose_id, position } => {
            if position >= rig.poses.len() {
                return Err(
                    CoreError::InvalidRig("pose position is outside the rig".to_owned()).into(),
                );
            }
            let pose = rig.poses.remove(pose_index(rig, &pose_id)?);
            rig.poses.insert(position, pose);
            RigOperation::ReorderPose { pose_id, position }
        }
        RigMutation::RenamePose { pose_id, name } => {
            let name = name.trim().to_owned();
            let index = pose_index(rig, &pose_id)?;
            rig.poses[index].name = Some(name.clone());
            RigOperation::RenamePose { pose_id, name }
        }
    };
    rig.validate()?;
    Ok(operation)
}

fn commit_rig(
    store: &ProjectStore,
    asset: &str,
    parent_id: &str,
    parent: RevisionSnapshot,
    rig: PixelRig,
    operation: RigOperation,
    actor: String,
) -> Result<RevisionResult, AppError> {
    let sequence = rig.render_sequence()?;
    commit(
        store,
        asset,
        parent_id,
        parent,
        sequence,
        Some(rig),
        operation,
        actor,
    )
}

fn commit(
    store: &ProjectStore,
    asset: &str,
    parent_id: &str,
    parent: RevisionSnapshot,
    sequence: pixelate_core::IndexedSequence,
    rig: Option<PixelRig>,
    operation: RigOperation,
    actor: String,
) -> Result<RevisionResult, AppError> {
    let input_hash = sha256_hex(&stable_json(&parent.sequence)?);
    let palette_hash = sha256_hex(&stable_json(&sequence.palette)?);
    commit_sequence(
        store,
        CommitSequence {
            asset: asset.to_owned(),
            sequence,
            rig,
            recipe: Recipe {
                schema: RECIPE_SCHEMA.to_owned(),
                input_sha256: input_hash.clone(),
                palette_sha256: palette_hash.clone(),
                operations: vec![Operation::EditRig { action: operation }],
            },
            brief: parent.brief,
            actor,
            input_hashes: BTreeMap::from([
                ("palette".to_owned(), palette_hash),
                ("parent_pixels".to_owned(), input_hash),
            ]),
            additional_checks: vec![ValidationCheck {
                name: "pixel_rig".to_owned(),
                passed: true,
                detail: "generic rig rendered deterministically".to_owned(),
            }],
            parent: Some(parent_id.to_owned()),
            style: None,
        },
    )
}

fn posed_node_mut<'a>(
    rig: &'a mut PixelRig,
    pose_id: &str,
    node_id: &str,
) -> Result<&'a mut pixelate_core::RigNodePose, AppError> {
    let pose = rig
        .poses
        .iter_mut()
        .find(|pose| pose.id == pose_id)
        .ok_or_else(|| invalid_id("pose", pose_id))?;
    pose.nodes
        .iter_mut()
        .find(|node| node.node_id == node_id)
        .ok_or_else(|| invalid_id("node", node_id))
}

fn pose_index(rig: &PixelRig, pose_id: &str) -> Result<usize, AppError> {
    rig.poses
        .iter()
        .position(|pose| pose.id == pose_id)
        .ok_or_else(|| invalid_id("pose", pose_id))
}

fn invalid_id(kind: &str, id: &str) -> AppError {
    CoreError::InvalidRig(format!("{kind} '{id}' does not exist")).into()
}

fn default_actor() -> String {
    "agent".to_owned()
}
