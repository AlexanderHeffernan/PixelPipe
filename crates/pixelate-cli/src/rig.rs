use std::{fs, io};

use pixelate_app::{
    BakeRig, CreateRig, DiscoverRigParts, MutateRig, RIG_DEFINITION_SCHEMA, RigDefinition,
    RigMutation, RigPartDefinition, bake_rig, create_rig, discover_rig_parts, mutate_rig,
};
use pixelate_core::{RigInterpolation, RigNode, RigNodePose, RigPose};
use serde_json::json;

use crate::args::RigCommand;

#[allow(clippy::too_many_lines)] // Exhaustive adapter dispatch is clearer than per-variant wrappers.
pub(crate) fn run_rig(
    command: RigCommand,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let command = match command {
        RigCommand::Parts {
            root,
            asset,
            revision,
            frame,
            min_pixels,
        } => {
            let result = discover_rig_parts(DiscoverRigParts {
                start: root,
                asset,
                revision,
                frame_id: frame,
                minimum_pixels: min_pixels,
            })?;
            return Ok(json!({ "ok": true, "discovery": result }));
        }
        command => command,
    };
    let result = match command {
        RigCommand::Assemble {
            root,
            asset,
            parent,
            source_frame,
            width,
            height,
            parts,
            nodes,
            pose,
            name,
            duration,
            inbetweens,
            looped,
            actor,
        } => {
            let (nodes, posed) = parse_nodes(&nodes)?;
            create_rig(CreateRig {
                start: root,
                asset,
                parent,
                source_frame_id: source_frame,
                definition: RigDefinition {
                    schema: RIG_DEFINITION_SCHEMA.to_owned(),
                    width,
                    height,
                    parts: parts
                        .iter()
                        .map(|part| parse_part(part))
                        .collect::<Result<_, _>>()?,
                    nodes,
                    poses: vec![RigPose {
                        id: pose,
                        name: Some(name),
                        nodes: posed,
                    }],
                    frame_duration_ms: duration,
                    interpolation: RigInterpolation { inbetweens, looped },
                    pivot: None,
                },
                actor,
            })?
        }
        RigCommand::Create {
            root,
            asset,
            parent,
            source_frame,
            definition,
            actor,
        } => create_rig(CreateRig {
            start: root,
            asset,
            parent,
            source_frame_id: source_frame,
            definition: serde_json::from_slice::<RigDefinition>(&fs::read(definition)?)?,
            actor,
        })?,
        RigCommand::Mutate {
            root,
            asset,
            parent,
            mutation,
            actor,
        } => mutate_rig(MutateRig {
            start: root,
            asset,
            parent,
            action: serde_json::from_slice::<RigMutation>(&fs::read(mutation)?)?,
            actor,
        })?,
        RigCommand::Node {
            root,
            asset,
            parent,
            pose,
            node,
            x,
            y,
            rotation,
            width_percent,
            height_percent,
            layer,
            part,
            actor,
        } => mutate(
            root,
            asset,
            parent,
            actor,
            RigMutation::UpdateNode {
                pose_id: pose,
                node_id: node,
                x_millis: x.map(|value| value.saturating_mul(1000)),
                y_millis: y.map(|value| value.saturating_mul(1000)),
                rotation_millidegrees: rotation.map(|value| value.saturating_mul(1000)),
                scale_x_millis: width_percent.map(|value| value.saturating_mul(10)),
                scale_y_millis: height_percent.map(|value| value.saturating_mul(10)),
                depth: layer,
                visible: None,
                part_id: part,
            },
        )?,
        RigCommand::Swap {
            root,
            asset,
            parent,
            first,
            second,
            actor,
        } => mutate(
            root,
            asset,
            parent,
            actor,
            RigMutation::SwapParts {
                first_node_id: first,
                second_node_id: second,
            },
        )?,
        RigCommand::Interpolation {
            root,
            asset,
            parent,
            inbetweens,
            looped,
            actor,
        } => mutate(
            root,
            asset,
            parent,
            actor,
            RigMutation::SetInterpolation { inbetweens, looped },
        )?,
        RigCommand::Duration {
            root,
            asset,
            parent,
            duration,
            actor,
        } => mutate(
            root,
            asset,
            parent,
            actor,
            RigMutation::SetDuration {
                duration_ms: duration,
            },
        )?,
        RigCommand::DuplicatePose {
            root,
            asset,
            parent,
            pose,
            new_pose,
            name,
            actor,
        } => mutate(
            root,
            asset,
            parent,
            actor,
            RigMutation::DuplicatePose {
                pose_id: pose,
                new_pose_id: new_pose,
                name,
            },
        )?,
        RigCommand::DeletePose {
            root,
            asset,
            parent,
            pose,
            actor,
        } => mutate(
            root,
            asset,
            parent,
            actor,
            RigMutation::DeletePose { pose_id: pose },
        )?,
        RigCommand::ReorderPose {
            root,
            asset,
            parent,
            pose,
            position,
            actor,
        } => mutate(
            root,
            asset,
            parent,
            actor,
            RigMutation::ReorderPose {
                pose_id: pose,
                position,
            },
        )?,
        RigCommand::RenamePose {
            root,
            asset,
            parent,
            pose,
            name,
            actor,
        } => mutate(
            root,
            asset,
            parent,
            actor,
            RigMutation::RenamePose {
                pose_id: pose,
                name,
            },
        )?,
        RigCommand::Bake {
            root,
            asset,
            parent,
            actor,
        } => bake_rig(BakeRig {
            start: root,
            asset,
            parent,
            actor,
        })?,
        RigCommand::Parts { .. } => unreachable!("parts returned before mutation dispatch"),
    };
    Ok(json!({ "ok": true, "revision": result }))
}

fn mutate(
    start: std::path::PathBuf,
    asset: String,
    parent: String,
    actor: String,
    action: RigMutation,
) -> Result<pixelate_app::RevisionResult, pixelate_app::AppError> {
    mutate_rig(MutateRig {
        start,
        asset,
        parent,
        action,
        actor,
    })
}

fn fields<'a>(value: &'a str, count: usize, kind: &str) -> io::Result<Vec<&'a str>> {
    let fields = value.split(',').map(str::trim).collect::<Vec<_>>();
    if fields.len() != count {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{kind} must contain exactly {count} comma-separated fields"),
        ));
    }
    Ok(fields)
}

fn number<T: std::str::FromStr>(value: &str, field: &str) -> io::Result<T> {
    value.parse().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid {field} value '{value}'"),
        )
    })
}

fn parse_part(value: &str) -> io::Result<RigPartDefinition> {
    let item = fields(value, 7, "part")?;
    Ok(RigPartDefinition {
        id: item[0].to_owned(),
        source: [
            number(item[1], "part x")?,
            number(item[2], "part y")?,
            number(item[3], "part width")?,
            number(item[4], "part height")?,
        ],
        pivot: [
            number(item[5], "part pivot x")?,
            number(item[6], "part pivot y")?,
        ],
    })
}

fn parse_nodes(values: &[String]) -> io::Result<(Vec<RigNode>, Vec<RigNodePose>)> {
    let mut nodes = Vec::with_capacity(values.len());
    let mut posed = Vec::with_capacity(values.len());
    for value in values {
        let item = fields(value, 6, "node")?;
        nodes.push(RigNode {
            id: item[0].to_owned(),
            parent_id: (!item[1].eq_ignore_ascii_case("none")).then(|| item[1].to_owned()),
        });
        posed.push(RigNodePose {
            node_id: item[0].to_owned(),
            part_id: item[2].to_owned(),
            x_millis: number::<i32>(item[3], "node x")?.saturating_mul(1000),
            y_millis: number::<i32>(item[4], "node y")?.saturating_mul(1000),
            rotation_millidegrees: 0,
            scale_x_millis: 1000,
            scale_y_millis: 1000,
            depth: number(item[5], "node layer")?,
            visible: true,
        });
    }
    Ok((nodes, posed))
}
