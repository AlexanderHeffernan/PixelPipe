import { computed, ref, watch, type Ref } from "vue";
import * as api from "../api";
import type {
  ProjectBrowser,
  RevisionViewResponse,
  RigNodePose,
} from "../types";
import { apply, invert, worldMatrices } from "./rig-geometry";

interface RigContext {
  project: Ref<ProjectBrowser | undefined>;
  assetId: Ref<string>;
  view: Ref<RevisionViewResponse | undefined>;
  refresh: () => Promise<void>;
  run: (action: () => Promise<void>) => Promise<void>;
  notice: (message: string) => void;
  onMutation: () => void;
}

export function createRigEditor(context: RigContext) {
  const active = ref(false);
  const bakedFromRig = ref(false);
  const selectedNodeId = ref("");
  const draftNodes = ref<RigNodePose[]>();
  const rig = computed(() => context.view.value?.metadata.rig);
  const currentPose = computed(() =>
    rig.value?.poses.find(
      (pose) => pose.id === context.view.value?.metadata.selected_frame_id,
    ),
  );
  const selectedNode = computed(() =>
    renderedNodes.value.find((node) => node.node_id === selectedNodeId.value),
  );
  const renderedNodes = computed(
    () => draftNodes.value ?? currentPose.value?.nodes ?? [],
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
    const matrices = worldMatrices(rig.value.nodes, renderedNodes.value);
    return renderedNodes.value.map((node) => {
      const point = apply(matrices.get(node.node_id)!, 0, 0);
      const parentId = rig.value!.nodes.find(
        (candidate) => candidate.id === node.node_id,
      )?.parent_id;
      const parent = parentId
        ? apply(matrices.get(parentId)!, 0, 0)
        : undefined;
      const part = rig.value!.parts.find((part) => part.id === node.part_id)!;
      const matrix = matrices.get(node.node_id)!;
      const corners = [
        apply(matrix, -part.pivot[0], -part.pivot[1]),
        apply(matrix, part.width - part.pivot[0], -part.pivot[1]),
        apply(matrix, part.width - part.pivot[0], part.height - part.pivot[1]),
        apply(matrix, -part.pivot[0], part.height - part.pivot[1]),
      ];
      return { ...node, x: point.x, y: point.y, parent, corners };
    });
  });
  const artwork = computed(() => {
    if (!rig.value) return [];
    const matrices = worldMatrices(rig.value.nodes, renderedNodes.value);
    return renderedNodes.value
      .filter((node) => node.visible)
      .map((node) => {
        const part = rig.value!.parts.find(
          (candidate) => candidate.id === node.part_id,
        )!;
        return {
          nodeId: node.node_id,
          part,
          matrix: matrices.get(node.node_id)!,
          href: context.view.value?.rig_part_pngs?.[part.id]
            ? api.pngDataUrl(context.view.value.rig_part_pngs[part.id])
            : "",
          depth: node.depth,
        };
      })
      .sort((left, right) =>
        left.depth === right.depth
          ? left.nodeId.localeCompare(right.nodeId)
          : left.depth - right.depth,
      );
  });

  watch(
    () => `${context.assetId.value}:${Boolean(rig.value)}`,
    () => {
      active.value = Boolean(rig.value);
      bakedFromRig.value = false;
      selectedNodeId.value = "";
      draftNodes.value = undefined;
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
    beginNodeDrag(nodeId);
    previewNodeDrag(nodeId, worldX, worldY);
    return finishNodeDrag(nodeId);
  }

  function beginNodeDrag(nodeId: string) {
    if (!currentPose.value) return;
    selectedNodeId.value = nodeId;
    draftNodes.value = currentPose.value.nodes.map((node) => ({ ...node }));
  }

  function previewNodeDrag(nodeId: string, worldX: number, worldY: number) {
    const metadata = rig.value;
    const nodes = draftNodes.value;
    if (!metadata || !nodes) return;
    const node = nodes.find((candidate) => candidate.node_id === nodeId);
    const definition = metadata.nodes.find(
      (candidate) => candidate.id === nodeId,
    );
    if (!node || !definition) return;
    let point = { x: worldX, y: worldY };
    if (definition.parent_id) {
      const matrices = worldMatrices(metadata.nodes, nodes);
      point = invert(matrices.get(definition.parent_id)!, worldX, worldY);
      const currentX = node.x_millis / 1000;
      const currentY = node.y_millis / 1000;
      const reach = Math.hypot(currentX, currentY);
      const requested = Math.hypot(point.x, point.y);
      if (reach > 0 && requested > 0) {
        point.x = (point.x / requested) * reach;
        point.y = (point.y / requested) * reach;
      }
    }
    node.x_millis = Math.round(point.x * 1000);
    node.y_millis = Math.round(point.y * 1000);
    draftNodes.value = [...nodes];
  }

  function finishNodeDrag(nodeId: string) {
    const node = draftNodes.value?.find(
      (candidate) => candidate.node_id === nodeId,
    );
    draftNodes.value = undefined;
    if (!node) return Promise.resolve();
    return updateSelected({
      x_millis: node.x_millis,
      y_millis: node.y_millis,
    });
  }

  function cancelNodeDrag() {
    draftNodes.value = undefined;
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
      bakedFromRig.value = true;
      context.notice("Rig baked; frames are ready for pixel refinement");
    });
  }

  return {
    active,
    bakedFromRig,
    rig,
    currentPose,
    selectedNodeId,
    selectedNode,
    manualFrames,
    handles,
    artwork,
    mutate,
    updateSelected,
    moveNode,
    beginNodeDrag,
    previewNodeDrag,
    finishNodeDrag,
    cancelNodeDrag,
    duplicatePose,
    bake,
  };
}
