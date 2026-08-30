use std::collections::BTreeMap;

use crate::{
    PALETTE_SCHEMA, Palette, PixelRig, RIG_SCHEMA, RigInterpolation, RigNode, RigNodePose, RigPart,
    RigPose, rig_render::interpolate_pose,
};

fn fixture() -> PixelRig {
    PixelRig {
        schema: RIG_SCHEMA.to_owned(),
        width: 5,
        height: 3,
        palette: Palette {
            schema: PALETTE_SCHEMA.to_owned(),
            name: "test".to_owned(),
            transparent_index: 0,
            colors: vec![[0, 0, 0, 0], [255, 90, 20, 255], [40, 60, 90, 255]],
        },
        parts: vec![RigPart {
            id: "block".to_owned(),
            width: 1,
            height: 1,
            pixels: vec![1],
            pivot: [0, 0],
        }],
        nodes: vec![RigNode {
            id: "body".to_owned(),
            parent_id: None,
        }],
        poses: vec![
            RigPose {
                id: "left".to_owned(),
                name: Some("Left".to_owned()),
                nodes: vec![pose("body", 1_000, 1_000)],
            },
            RigPose {
                id: "right".to_owned(),
                name: Some("Right".to_owned()),
                nodes: vec![pose("body", 3_000, 1_000)],
            },
        ],
        frame_duration_ms: 80,
        interpolation: RigInterpolation {
            inbetweens: 1,
            looped: false,
        },
        pivot: None,
        metadata: BTreeMap::new(),
    }
}

fn pose(node_id: &str, x_millis: i32, y_millis: i32) -> RigNodePose {
    RigNodePose {
        node_id: node_id.to_owned(),
        part_id: "block".to_owned(),
        x_millis,
        y_millis,
        rotation_millidegrees: 0,
        scale_x_millis: 1_000,
        scale_y_millis: 1_000,
        depth: 0,
        visible: true,
    }
}

#[test]
fn renders_explicit_and_interpolated_frames() {
    let sequence = fixture().render_sequence().unwrap();
    assert_eq!(sequence.frames.len(), 3);
    assert_eq!(sequence.frames[0].id, "left");
    assert_eq!(sequence.frames[1].id, "__generated-0001");
    assert_eq!(sequence.frames[2].id, "right");
    assert_eq!(sequence.frames[0].pixels[6], 1);
    assert_eq!(sequence.frames[1].pixels[7], 1);
    assert_eq!(sequence.frames[2].pixels[8], 1);
}

#[test]
fn interpolation_uses_explicit_rotation_path_and_source_discrete_state() {
    let mut rig = fixture();
    rig.parts.push(RigPart {
        id: "alternate".to_owned(),
        width: 1,
        height: 1,
        pixels: vec![2],
        pivot: [0, 0],
    });
    rig.poses[1].nodes[0].rotation_millidegrees = 270_000;
    rig.poses[1].nodes[0].part_id = "alternate".to_owned();
    let middle = interpolate_pose(&rig.poses[0], &rig.poses[1], 1, 2);
    assert_eq!(middle.nodes[0].rotation_millidegrees, 135_000);
    assert_eq!(middle.nodes[0].part_id, "block");
}

#[test]
fn rejects_cycles_missing_nodes_duplicate_ids_and_invalid_parts() {
    let mut rig = fixture();
    rig.nodes.push(RigNode {
        id: "child".to_owned(),
        parent_id: Some("body".to_owned()),
    });
    assert!(rig.validate().is_err());

    rig.poses.iter_mut().for_each(|frame| {
        frame.nodes.push(pose("child", 0, 0));
    });
    rig.nodes[0].parent_id = Some("child".to_owned());
    assert!(rig.validate().is_err());

    rig.nodes[0].parent_id = None;
    rig.parts[0].id = "alternate".to_owned();
    rig.parts.push(rig.parts[0].clone());
    assert!(rig.validate().is_err());
}

#[test]
fn loop_interpolation_closes_the_final_pose_to_the_first() {
    let mut rig = fixture();
    rig.interpolation.looped = true;
    let sequence = rig.render_sequence().unwrap();
    assert_eq!(sequence.frames.len(), 4);
    assert_eq!(sequence.frames[3].id, "__generated-0002");
    assert_eq!(sequence.frames[3].pixels[7], 1);
}
