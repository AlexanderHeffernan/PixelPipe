import type { RigNodePose } from "../types";

export interface RigMatrix {
  a: number;
  b: number;
  c: number;
  d: number;
  tx: number;
  ty: number;
}

export function worldMatrices(
  nodes: { id: string; parent_id?: string }[],
  poses: RigNodePose[],
) {
  const result = new Map<string, RigMatrix>();
  const resolve = (id: string): RigMatrix => {
    const existing = result.get(id);
    if (existing) return existing;
    const definition = nodes.find((node) => node.id === id)!;
    const pose = poses.find((node) => node.node_id === id)!;
    const angle = (pose.rotation_millidegrees * Math.PI) / 180_000;
    const cosine = Math.cos(angle);
    const sine = Math.sin(angle);
    const local: RigMatrix = {
      a: (cosine * pose.scale_x_millis) / 1000,
      b: (sine * pose.scale_x_millis) / 1000,
      c: (-sine * pose.scale_y_millis) / 1000,
      d: (cosine * pose.scale_y_millis) / 1000,
      tx: pose.x_millis / 1000,
      ty: pose.y_millis / 1000,
    };
    const matrix = definition.parent_id
      ? multiply(resolve(definition.parent_id), local)
      : local;
    result.set(id, matrix);
    return matrix;
  };
  nodes.forEach((node) => resolve(node.id));
  return result;
}

export function apply(matrix: RigMatrix, x: number, y: number) {
  return {
    x: matrix.a * x + matrix.c * y + matrix.tx,
    y: matrix.b * x + matrix.d * y + matrix.ty,
  };
}

export function invert(matrix: RigMatrix, x: number, y: number) {
  const determinant = matrix.a * matrix.d - matrix.b * matrix.c;
  const offsetX = x - matrix.tx;
  const offsetY = y - matrix.ty;
  return {
    x: (matrix.d * offsetX - matrix.c * offsetY) / determinant,
    y: (-matrix.b * offsetX + matrix.a * offsetY) / determinant,
  };
}

function multiply(parent: RigMatrix, child: RigMatrix): RigMatrix {
  return {
    a: parent.a * child.a + parent.c * child.b,
    b: parent.b * child.a + parent.d * child.b,
    c: parent.a * child.c + parent.c * child.d,
    d: parent.b * child.c + parent.d * child.d,
    tx: parent.a * child.tx + parent.c * child.ty + parent.tx,
    ty: parent.b * child.tx + parent.d * child.ty + parent.ty,
  };
}
