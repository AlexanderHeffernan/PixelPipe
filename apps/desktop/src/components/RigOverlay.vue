<script setup lang="ts">
import { ref } from "vue";
import { useWorkspace } from "../workspace/context";

const workspace = useWorkspace();
const dragging = ref("");
const dragMoved = ref(false);
const overlay = ref<SVGSVGElement>();

function point(event: PointerEvent) {
  const bounds = overlay.value!.getBoundingClientRect();
  const width = workspace.inspection.value?.width ?? 1;
  const height = workspace.inspection.value?.height ?? 1;
  return {
    x: ((event.clientX - bounds.left) / bounds.width) * width,
    y: ((event.clientY - bounds.top) / bounds.height) * height,
  };
}

function pointerDown(event: PointerEvent, nodeId: string) {
  if (event.button !== 0) return;
  event.stopPropagation();
  workspace.animation.pause();
  dragging.value = nodeId;
  dragMoved.value = false;
  workspace.rig.beginNodeDrag(nodeId);
  overlay.value?.setPointerCapture?.(event.pointerId);
}

function pointerMove(event: PointerEvent) {
  if (!dragging.value) return;
  event.stopPropagation();
  dragMoved.value = true;
  const position = point(event);
  workspace.rig.previewNodeDrag(dragging.value, position.x, position.y);
}

function pointerUp(event: PointerEvent) {
  if (!dragging.value) return;
  event.stopPropagation();
  const node = dragging.value;
  dragging.value = "";
  if (dragMoved.value) void workspace.rig.finishNodeDrag(node);
  else workspace.rig.cancelNodeDrag();
}

function cancelDrag() {
  dragging.value = "";
  dragMoved.value = false;
  workspace.rig.cancelNodeDrag();
}

function points(corners: { x: number; y: number }[]) {
  return corners.map((corner) => `${corner.x},${corner.y}`).join(" ");
}

function moveWithKeyboard(event: KeyboardEvent, nodeId: string) {
  const movement: Record<string, [number, number]> = {
    ArrowLeft: [-1, 0],
    ArrowRight: [1, 0],
    ArrowUp: [0, -1],
    ArrowDown: [0, 1],
  };
  const delta = movement[event.key];
  const handle = workspace.rig.handles.value.find(
    (candidate) => candidate.node_id === nodeId,
  );
  if (!delta || !handle) return;
  event.preventDefault();
  workspace.animation.pause();
  void workspace.rig.moveNode(nodeId, handle.x + delta[0], handle.y + delta[1]);
}
</script>

<template>
  <svg
    v-if="
      workspace.rig.active.value &&
      workspace.rig.guidesVisible.value &&
      workspace.rig.currentPose.value &&
      !workspace.animation.playing.value
    "
    ref="overlay"
    class="rig-overlay"
    :viewBox="`0 0 ${workspace.inspection.value?.width ?? 1} ${workspace.inspection.value?.height ?? 1}`"
    aria-label="Editable pixel rig"
    @pointermove="pointerMove"
    @pointerup="pointerUp"
    @pointercancel="cancelDrag"
  >
    <template
      v-for="handle in workspace.rig.handles.value"
      :key="handle.node_id"
    >
      <line
        v-if="handle.parent"
        :x1="handle.parent.x"
        :y1="handle.parent.y"
        :x2="handle.x"
        :y2="handle.y"
      />
      <circle
        v-if="
          handle.parent && workspace.rig.selectedNodeId.value === handle.node_id
        "
        class="rig-reach"
        :cx="handle.parent.x"
        :cy="handle.parent.y"
        :r="Math.hypot(handle.x - handle.parent.x, handle.y - handle.parent.y)"
      />
      <polygon
        v-if="workspace.rig.selectedNodeId.value === handle.node_id"
        class="rig-selection"
        :points="points(handle.corners)"
      />
      <g
        role="button"
        tabindex="0"
        :aria-label="`Adjust rig joint ${handle.node_id}`"
        :class="{
          selected: workspace.rig.selectedNodeId.value === handle.node_id,
          hidden: !handle.visible,
        }"
        @pointerdown="pointerDown($event, handle.node_id)"
        @keydown="moveWithKeyboard($event, handle.node_id)"
      >
        <circle :cx="handle.x" :cy="handle.y" r="1.35" />
        <circle :cx="handle.x" :cy="handle.y" r="0.42" class="rig-pivot" />
      </g>
    </template>
  </svg>
</template>
