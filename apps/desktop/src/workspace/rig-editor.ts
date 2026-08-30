import { computed, ref, watch, type Ref } from "vue";
import * as api from "../api";
import type {
  ProjectBrowser,
  RevisionViewResponse,
  RigNodePose,
} from "../types";

interface RigContext {
  project: Ref<ProjectBrowser | undefined>;
  assetId: Ref<string>;
  view: Ref<RevisionViewResponse | undefined>;
  refresh: () => Promise<void>;
  run: (action: () => Promise<void>) => Promise<void>;
  notice: (message: string) => void;
  onMutation: () => void;
}

interface Matrix {
  a: number;
  b: number;
  c: number;
  d: number;
  tx: number;
  ty: number;
}

export function createRigEditor(context: RigContext) {
  const active = ref(false);
  const selectedNodeId = ref("");
  const rig = computed(() => context.view.value?.metadata.rig);
  const currentPose = computed(() =>
    rig.value?.poses.find(
      (pose) => pose.id === context.view.value?.metadata.selected_frame_id,
    ),
  );
  const selectedNode = computed(() =>
    currentPose.value?.nodes.find(
      (node) => node.node_id === selectedNodeId.value,
    ),
  );
  const manualFrames = computed(() => {
    const metadata = context.view.value?.metadata;
    if (!metadata?.rig) return metadata?.frames ?? [];
    return metadata.rig.poses.flatMap((pose) => {
      const frame = metadata.frames.find((frame) => frame.id === pose.id);
      return frame ? [frame] : [];
    });
  });
  const handles = computed(() => {
    if (!active.value || !currentPose.value || !rig.value) return [];
    const matrices = worldMatrices(rig.value.nodes, currentPose.value.nodes);
    return currentPose.value.nodes.map((node) => {
      const point = apply(matrices.get(node.node_id)!, 0, 0);
      const parentId = rig.value!.nodes.find(
        (candidate) => candidate.id === node.node_id,
      )?.parent_id;
      const parent = parentId
        ? apply(matrices.get(parentId)!, 0, 0)
        : undefined;
      return { ...node, x: point.x, y: point.y, parent };
    });
  });

  watch(
    () => `${context.assetId.value}:${Boolean(rig.value)}`,
    () => {
      active.value = Boolean(rig.value);
      selectedNodeId.value = rig.value?.nodes[0]?.id ?? "";
    },
    { immediate: true },
  );

  async function mutate(
    action: api.RigMutationAction,
    preferredPose = currentPose.value?.id,
  ) {
    const root = context.project.value?.project_root;
    const parent = context.view.value?.metadata.revision;
    if (!root || !parent) return;
    await context.run(async () => {
      const result = await api.mutateRig(
        root,
        context.assetId.value,
        parent,
        action,
        "user",
      );
      context.onMutation();
      await context.refresh();
      let loaded = await api.loadRevision(
        root,
        context.assetId.value,
        result.revision,
      );
      if (
        preferredPose &&
        loaded.metadata.rig?.poses.some((pose) => pose.id === preferredPose)
      )
        loaded = await api.loadRevision(
          root,
          context.assetId.value,
          result.revision,
          preferredPose,
        );
      context.view.value = loaded;
      context.notice("Rig change saved as a new revision");
    });
  }

  function updateSelected(values: Partial<RigNodePose>) {
    const pose = currentPose.value;
    const node = selectedNode.value;
    if (!pose || !node) return Promise.resolve();
    return mutate({
      type: "update_node",
      pose_id: pose.id,
      node_id: node.node_id,
      ...values,
    });
  }

  function moveNode(nodeId: string, worldX: number, worldY: number) {
    const pose = currentPose.value;
    const metadata = rig.value;
    if (!pose || !metadata) return;
    selectedNodeId.value = nodeId;
    const definition = metadata.nodes.find((node) => node.id === nodeId);
    let point = { x: worldX, y: worldY };
    if (definition?.parent_id) {
      const matrices = worldMatrices(metadata.nodes, pose.nodes);
      point = invert(matrices.get(definition.parent_id)!, worldX, worldY);
    }
    return updateSelected({
      x_millis: Math.round(point.x * 1000),
      y_millis: Math.round(point.y * 1000),
    });
  }

  function duplicatePose(poseId: string) {
    const existing = new Set(rig.value?.poses.map((pose) => pose.id));
    let number = 1;
    while (existing.has(`pose-${String(number).padStart(4, "0")}`)) number++;
    const id = `pose-${String(number).padStart(4, "0")}`;
    return mutate(
      { type: "duplicate_pose", pose_id: poseId, new_pose_id: id },
      id,
    );
  }

  async function bake() {
    const root = context.project.value?.project_root;
    const parent = context.view.value?.metadata.revision;
    if (!root || !parent) return;
    await context.run(async () => {
      const result = await api.bakeRig(
        root,
        context.assetId.value,
        parent,
        "user",
      );
      await context.refresh();
      context.view.value = await api.loadRevision(
        root,
        context.assetId.value,
        result.revision,
      );
      active.value = false;
      context.notice("Rig baked; frames are ready for pixel refinement");
    });
  }

  return {
    active,
    rig,
    currentPose,
    selectedNodeId,
    selectedNode,
    manualFrames,
    handles,
    mutate,
    updateSelected,
    moveNode,
    duplicatePose,
    bake,
  };
}

function worldMatrices(
  nodes: { id: string; parent_id?: string }[],
  poses: RigNodePose[],
) {
  const result = new Map<string, Matrix>();
  const resolve = (id: string): Matrix => {
    const existing = result.get(id);
    if (existing) return existing;
    const definition = nodes.find((node) => node.id === id)!;
    const pose = poses.find((node) => node.node_id === id)!;
    const angle = (pose.rotation_millidegrees * Math.PI) / 180_000;
    const cosine = Math.cos(angle);
    const sine = Math.sin(angle);
    const local: Matrix = {
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

function multiply(parent: Matrix, child: Matrix): Matrix {
  return {
    a: parent.a * child.a + parent.c * child.b,
    b: parent.b * child.a + parent.d * child.b,
    c: parent.a * child.c + parent.c * child.d,
    d: parent.b * child.c + parent.d * child.d,
    tx: parent.a * child.tx + parent.c * child.ty + parent.tx,
    ty: parent.b * child.tx + parent.d * child.ty + parent.ty,
  };
}

function apply(matrix: Matrix, x: number, y: number) {
  return {
    x: matrix.a * x + matrix.c * y + matrix.tx,
    y: matrix.b * x + matrix.d * y + matrix.ty,
  };
}

function invert(matrix: Matrix, x: number, y: number) {
  const determinant = matrix.a * matrix.d - matrix.b * matrix.c;
  const offsetX = x - matrix.tx;
  const offsetY = y - matrix.ty;
  return {
    x: (matrix.d * offsetX - matrix.c * offsetY) / determinant,
    y: (-matrix.b * offsetX + matrix.a * offsetY) / determinant,
  };
}
