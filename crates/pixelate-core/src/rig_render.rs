use std::collections::BTreeMap;

use crate::{
    CoreError, IndexedFrame, IndexedSequence, PixelRig, RigNode, RigNodePose, RigPart, RigPose,
    SEQUENCE_SCHEMA,
    rig::{GENERATED_PREFIX, invalid},
};

const FIXED: i128 = 1_000_000;

pub(crate) fn render_rig(rig: &PixelRig) -> Result<IndexedSequence, CoreError> {
    rig.validate()?;
    let mut frames = Vec::new();
    let mut generated = 1_u32;
    for (index, pose) in rig.poses.iter().enumerate() {
        frames.push(render_pose(rig, pose, pose.id.clone(), pose.name.clone())?);
        let next = rig
            .poses
            .get(index + 1)
            .or_else(|| (rig.interpolation.looped && rig.poses.len() > 1).then(|| &rig.poses[0]));
        if let Some(next) = next {
            for step in 1..=u32::from(rig.interpolation.inbetweens) {
                let pose = interpolate_pose(
                    pose,
                    next,
                    step,
                    u32::from(rig.interpolation.inbetweens) + 1,
                );
                frames.push(render_pose(
                    rig,
                    &pose,
                    format!("{GENERATED_PREFIX}{generated:04}"),
                    None,
                )?);
                generated += 1;
            }
        }
    }
    let sequence = IndexedSequence {
        schema: SEQUENCE_SCHEMA.to_owned(),
        width: rig.width,
        height: rig.height,
        palette: rig.palette.clone(),
        frames,
        pivot: rig.pivot,
        metadata: rig.metadata.clone(),
    };
    sequence.validate()?;
    Ok(sequence)
}

fn render_pose(
    rig: &PixelRig,
    pose: &RigPose,
    id: String,
    name: Option<String>,
) -> Result<IndexedFrame, CoreError> {
    let count = usize::try_from(u64::from(rig.width) * u64::from(rig.height))
        .map_err(|_| CoreError::DimensionOverflow)?;
    let mut pixels = vec![rig.palette.transparent_index; count];
    let posed = pose
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let mut ordered = pose.nodes.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left.depth
            .cmp(&right.depth)
            .then_with(|| left.node_id.cmp(&right.node_id))
    });
    let mut matrices = BTreeMap::new();
    for node in ordered {
        if !node.visible {
            continue;
        }
        let matrix = resolve_matrix(&node.node_id, &rig.nodes, &posed, &mut matrices)?;
        let part = rig
            .parts
            .iter()
            .find(|part| part.id == node.part_id)
            .ok_or_else(|| invalid("posed node part is missing"))?;
        composite_part(rig, part, matrix, &mut pixels)?;
    }
    Ok(IndexedFrame {
        id,
        name,
        duration_ms: rig.frame_duration_ms,
        pixels,
    })
}

pub(crate) fn interpolate_pose(from: &RigPose, to: &RigPose, step: u32, steps: u32) -> RigPose {
    let targets = to
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    RigPose {
        id: String::new(),
        name: None,
        nodes: from
            .nodes
            .iter()
            .map(|node| {
                let target = targets[&node.node_id.as_str()];
                RigNodePose {
                    node_id: node.node_id.clone(),
                    part_id: node.part_id.clone(),
                    x_millis: lerp(node.x_millis, target.x_millis, step, steps),
                    y_millis: lerp(node.y_millis, target.y_millis, step, steps),
                    rotation_millidegrees: lerp(
                        node.rotation_millidegrees,
                        target.rotation_millidegrees,
                        step,
                        steps,
                    ),
                    scale_x_millis: lerp(node.scale_x_millis, target.scale_x_millis, step, steps),
                    scale_y_millis: lerp(node.scale_y_millis, target.scale_y_millis, step, steps),
                    depth: lerp(node.depth, target.depth, step, steps),
                    visible: node.visible,
                }
            })
            .collect(),
    }
}

fn lerp(from: i32, to: i32, step: u32, steps: u32) -> i32 {
    let difference = i64::from(to) - i64::from(from);
    let rounded = difference * i64::from(step);
    let adjustment = if rounded >= 0 {
        i64::from(steps) / 2
    } else {
        -(i64::from(steps) / 2)
    };
    i32::try_from(i64::from(from) + (rounded + adjustment) / i64::from(steps))
        .expect("interpolation remains between two i32 values")
}

#[derive(Clone, Copy)]
struct Matrix {
    a: i128,
    b: i128,
    c: i128,
    d: i128,
    tx: i128,
    ty: i128,
}

fn resolve_matrix(
    node_id: &str,
    nodes: &[RigNode],
    posed: &BTreeMap<&str, &RigNodePose>,
    matrices: &mut BTreeMap<String, Matrix>,
) -> Result<Matrix, CoreError> {
    if let Some(matrix) = matrices.get(node_id) {
        return Ok(*matrix);
    }
    let node = nodes
        .iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| invalid("node is missing"))?;
    let pose = posed
        .get(node_id)
        .ok_or_else(|| invalid("posed node is missing"))?;
    let local = local_matrix(pose);
    let matrix = match &node.parent_id {
        Some(parent) => multiply(resolve_matrix(parent, nodes, posed, matrices)?, local),
        None => local,
    };
    matrices.insert(node_id.to_owned(), matrix);
    Ok(matrix)
}

fn local_matrix(pose: &RigNodePose) -> Matrix {
    let (sin, cos) = sin_cos(pose.rotation_millidegrees);
    Matrix {
        a: cos * i128::from(pose.scale_x_millis) / 1_000,
        b: sin * i128::from(pose.scale_x_millis) / 1_000,
        c: -sin * i128::from(pose.scale_y_millis) / 1_000,
        d: cos * i128::from(pose.scale_y_millis) / 1_000,
        tx: i128::from(pose.x_millis) * FIXED / 1_000,
        ty: i128::from(pose.y_millis) * FIXED / 1_000,
    }
}

fn multiply(parent: Matrix, child: Matrix) -> Matrix {
    Matrix {
        a: (parent.a * child.a + parent.c * child.b) / FIXED,
        b: (parent.b * child.a + parent.d * child.b) / FIXED,
        c: (parent.a * child.c + parent.c * child.d) / FIXED,
        d: (parent.b * child.c + parent.d * child.d) / FIXED,
        tx: (parent.a * child.tx + parent.c * child.ty) / FIXED + parent.tx,
        ty: (parent.b * child.tx + parent.d * child.ty) / FIXED + parent.ty,
    }
}

fn sin_cos(rotation: i32) -> (i128, i128) {
    const ANGLES: [i32; 20] = [
        45_000, 26_565, 14_036, 7_125, 3_576, 1_790, 895, 448, 224, 112, 56, 28, 14, 7, 3, 2, 1, 0,
        0, 0,
    ];
    let mut angle = rotation.rem_euclid(360_000);
    if angle > 180_000 {
        angle -= 360_000;
    }
    let sign = if angle > 90_000 {
        angle -= 180_000;
        -1
    } else if angle < -90_000 {
        angle += 180_000;
        -1
    } else {
        1
    };
    let mut x = 607_252_i128;
    let mut y = 0_i128;
    let mut remaining = angle;
    for (shift, turn) in ANGLES.into_iter().enumerate() {
        let direction = if remaining >= 0 { 1 } else { -1 };
        let next_x = x - i128::from(direction) * (y >> shift);
        y += i128::from(direction) * (x >> shift);
        x = next_x;
        remaining -= direction * turn;
    }
    (y * sign, x * sign)
}

fn composite_part(
    rig: &PixelRig,
    part: &RigPart,
    matrix: Matrix,
    output: &mut [u8],
) -> Result<(), CoreError> {
    let pivot_x = i128::from(part.pivot[0]) * FIXED;
    let pivot_y = i128::from(part.pivot[1]) * FIXED;
    let corners = [
        (0, 0),
        (i128::from(part.width) * FIXED, 0),
        (0, i128::from(part.height) * FIXED),
        (
            i128::from(part.width) * FIXED,
            i128::from(part.height) * FIXED,
        ),
    ]
    .map(|(x, y)| transform(matrix, x - pivot_x, y - pivot_y));
    let min_x = corners.iter().map(|point| point.0).min().unwrap_or(0);
    let max_x = corners.iter().map(|point| point.0).max().unwrap_or(0);
    let min_y = corners.iter().map(|point| point.1).min().unwrap_or(0);
    let max_y = corners.iter().map(|point| point.1).max().unwrap_or(0);
    let start_x = min_x.div_euclid(FIXED).clamp(0, i128::from(rig.width));
    let end_x = div_ceil(max_x, FIXED).clamp(0, i128::from(rig.width));
    let start_y = min_y.div_euclid(FIXED).clamp(0, i128::from(rig.height));
    let end_y = div_ceil(max_y, FIXED).clamp(0, i128::from(rig.height));
    let determinant = matrix.a * matrix.d - matrix.b * matrix.c;
    if determinant == 0 {
        return Err(invalid("node transform must be invertible"));
    }
    for y in start_y..end_y {
        for x in start_x..end_x {
            let world_x = x * FIXED + FIXED / 2 - matrix.tx;
            let world_y = y * FIXED + FIXED / 2 - matrix.ty;
            let local_x = (matrix.d * world_x - matrix.c * world_y) * FIXED / determinant + pivot_x;
            let local_y =
                (-matrix.b * world_x + matrix.a * world_y) * FIXED / determinant + pivot_y;
            let source_x = local_x.div_euclid(FIXED);
            let source_y = local_y.div_euclid(FIXED);
            if source_x < 0
                || source_y < 0
                || source_x >= i128::from(part.width)
                || source_y >= i128::from(part.height)
            {
                continue;
            }
            let source = usize::try_from(source_y * i128::from(part.width) + source_x)
                .map_err(|_| CoreError::DimensionOverflow)?;
            let index = part.pixels[source];
            if index == rig.palette.transparent_index {
                continue;
            }
            let target = usize::try_from(y * i128::from(rig.width) + x)
                .map_err(|_| CoreError::DimensionOverflow)?;
            output[target] = index;
        }
    }
    Ok(())
}

fn transform(matrix: Matrix, x: i128, y: i128) -> (i128, i128) {
    (
        (matrix.a * x + matrix.c * y) / FIXED + matrix.tx,
        (matrix.b * x + matrix.d * y) / FIXED + matrix.ty,
    )
}

fn div_ceil(value: i128, divisor: i128) -> i128 {
    -(-value).div_euclid(divisor)
}
