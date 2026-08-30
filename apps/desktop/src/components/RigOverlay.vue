<script setup lang="ts">
import { ref } from "vue";
import { useWorkspace } from "../workspace/context";

const workspace = useWorkspace();
const dragging = ref("");
const dragPoint = ref({ x: 0, y: 0 });
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
  dragPoint.value = point(event);
  workspace.rig.selectedNodeId.value = nodeId;
  overlay.value?.setPointerCapture?.(event.pointerId);
}

function pointerMove(event: PointerEvent) {
  if (!dragging.value) return;
  event.stopPropagation();
  dragPoint.value = point(event);
}

function pointerUp(event: PointerEvent) {
  if (!dragging.value) return;
  event.stopPropagation();
  const position = point(event);
  const node = dragging.value;
  dragging.value = "";
  void workspace.rig.moveNode(node, position.x, position.y);
}

function x(handle: { node_id: string; x: number }) {
  return dragging.value === handle.node_id ? dragPoint.value.x : handle.x;
}

function y(handle: { node_id: string; y: number }) {
  return dragging.value === handle.node_id ? dragPoint.value.y : handle.y;
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
      workspace.rig.currentPose.value &&
      !workspace.animation.playing.value
    "
    ref="overlay"
    class="rig-overlay"
    :viewBox="`0 0 ${workspace.inspection.value?.width ?? 1} ${workspace.inspection.value?.height ?? 1}`"
    aria-label="Editable pixel rig"
    @pointermove="pointerMove"
    @pointerup="pointerUp"
    @pointercancel="dragging = ''"
  >
    <template
      v-for="handle in workspace.rig.handles.value"
      :key="handle.node_id"
    >
      <line
        v-if="handle.parent"
        :x1="handle.parent.x"
        :y1="handle.parent.y"
        :x2="x(handle)"
        :y2="y(handle)"
      />
      <g
        role="button"
        tabindex="0"
        :aria-label="`Move rig node ${handle.node_id}`"
        :class="{
          selected: workspace.rig.selectedNodeId.value === handle.node_id,
          hidden: !handle.visible,
        }"
        @pointerdown="pointerDown($event, handle.node_id)"
        @keydown="moveWithKeyboard($event, handle.node_id)"
      >
        <circle :cx="x(handle)" :cy="y(handle)" r="1.35" />
        <circle :cx="x(handle)" :cy="y(handle)" r="0.42" class="rig-pivot" />
      </g>
    </template>
  </svg>
</template>
