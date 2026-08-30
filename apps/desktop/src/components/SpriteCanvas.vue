<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  PhArrowClockwise,
  PhArrowCounterClockwise,
  PhEraser,
  PhEyedropper,
  PhFilmStrip,
  PhPaintBucket,
  PhPencilSimple,
} from "@phosphor-icons/vue";
import { useWorkspace } from "../workspace/context";
import type { PixelTool } from "../workspace/pixel-editor";
import RigOverlay from "./RigOverlay.vue";
import RigToolbar from "./RigToolbar.vue";

const workspace = useWorkspace();
const zoom = ref(1);
const pan = ref({ x: 0, y: 0 });
const panning = ref(false);
const canvasHovered = ref(false);
const panStart = ref({ x: 0, y: 0, originX: 0, originY: 0 });
const pixelCanvas = ref<HTMLElement>();
const editable = computed(
  () =>
    !workspace.canvasLoading.value &&
    workspace.mode.value === "edit" &&
    !workspace.rig.rig.value &&
    workspace.view.value,
);
const displayedImage = computed(
  () => workspace.canvasImage.value || workspace.loadingArtwork.value,
);
const dimensions = computed(() => workspace.inspection.value);
const canvasStyle = computed(() => ({
  aspectRatio: `${dimensions.value?.width ?? 1} / ${dimensions.value?.height ?? 1}`,
  transform: `translate(${pan.value.x}px, ${pan.value.y}px) scale(${zoom.value})`,
}));
const cursorStyle = computed(() => ({
  left: `${((workspace.editor.cursor.value.x - Math.floor((workspace.editor.brushSize.value - 1) / 2)) * 100) / (dimensions.value?.width ?? 1)}%`,
  top: `${((workspace.editor.cursor.value.y - Math.floor((workspace.editor.brushSize.value - 1) / 2)) * 100) / (dimensions.value?.height ?? 1)}%`,
  width: `${(workspace.editor.brushSize.value * 100) / (dimensions.value?.width ?? 1)}%`,
  height: `${(workspace.editor.brushSize.value * 100) / (dimensions.value?.height ?? 1)}%`,
}));

function editStyle(edit: { x: number; y: number; index: number }) {
  const width = dimensions.value?.width ?? 1;
  const height = dimensions.value?.height ?? 1;
  const rgba = workspace.view.value?.metadata.palette.colors[edit.index];
  return {
    left: `${(edit.x * 100) / width}%`,
    top: `${(edit.y * 100) / height}%`,
    width: `${100 / width}%`,
    height: `${100 / height}%`,
    background: rgba
      ? `rgba(${rgba[0]}, ${rgba[1]}, ${rgba[2]}, ${rgba[3] / 255})`
      : "transparent",
  };
}

function coordinate(event: PointerEvent) {
  const rect = pixelCanvas.value?.getBoundingClientRect();
  if (!rect) return { x: 0, y: 0 };
  const width = dimensions.value?.width ?? 1;
  const height = dimensions.value?.height ?? 1;
  return {
    x: Math.max(
      0,
      Math.min(
        width - 1,
        Math.floor(((event.clientX - rect.left) / rect.width) * width),
      ),
    ),
    y: Math.max(
      0,
      Math.min(
        height - 1,
        Math.floor(((event.clientY - rect.top) / rect.height) * height),
      ),
    ),
  };
}

async function pointerDown(event: PointerEvent) {
  if (event.button !== 0 && event.button !== 1) return;
  if ((event.target as HTMLElement).closest("button, input, select, label"))
    return;
  const onCanvas = pixelCanvas.value?.contains(event.target as Node);
  if (!editable.value || !onCanvas || event.button === 1) {
    panning.value = true;
    panStart.value = {
      x: event.clientX,
      y: event.clientY,
      originX: pan.value.x,
      originY: pan.value.y,
    };
    (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
    return;
  }
  (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
  const point = coordinate(event);
  workspace.animation.pause();
  if (!(await workspace.prepareEditing())) return;
  workspace.editor.point(point.x, point.y);
}

function pointerMove(event: PointerEvent) {
  if (panning.value) {
    pan.value = {
      x: panStart.value.originX + event.clientX - panStart.value.x,
      y: panStart.value.originY + event.clientY - panStart.value.y,
    };
    return;
  }
  if (!editable.value) return;
  const point = coordinate(event);
  workspace.editor.drag(point.x, point.y);
}

function pointerUp() {
  if (editable.value) void workspace.editor.finishStroke();
  panning.value = false;
}

function adjustZoom(next: number) {
  zoom.value = Math.min(5, Math.max(0.5, next));
}

function setBrushSize(event: Event) {
  const input = event.target as HTMLInputElement;
  const value = Math.min(32, Math.max(1, Number(input.value) || 1));
  input.value = String(value);
  workspace.editor.brushSize.value = value;
}

function wheel(event: WheelEvent) {
  adjustZoom(zoom.value * (event.deltaY < 0 ? 1.1 : 0.9));
}

function resetView() {
  zoom.value = 1;
  pan.value = { x: 0, y: 0 };
}

watch(() => workspace.assetId.value, resetView);

function keyDown(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "z") {
    event.preventDefault();
    void (event.shiftKey ? workspace.redo() : workspace.undo());
    return;
  }
  const moves: Record<string, [number, number]> = {
    ArrowLeft: [-1, 0],
    ArrowRight: [1, 0],
    ArrowUp: [0, -1],
    ArrowDown: [0, 1],
  };
  if (moves[event.key]) {
    event.preventDefault();
    workspace.editor.moveCursor(...moves[event.key]);
  } else if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    const { x, y } = workspace.editor.cursor.value;
    void workspace.prepareEditing().then((ready) => {
      if (!ready) return;
      workspace.animation.pause();
      workspace.editor.point(x, y);
      void workspace.editor.finishStroke();
    });
  } else if (["p", "e", "f", "i"].includes(event.key.toLowerCase())) {
    const tools: Record<string, PixelTool> = {
      p: "pencil",
      e: "eraser",
      f: "fill",
      i: "eyedropper",
    };
    void workspace.beginTool(tools[event.key.toLowerCase()]);
  }
}
</script>

<template>
  <div
    class="canvas-stage"
    :class="{ panning }"
    @pointerdown="pointerDown"
    @pointermove="pointerMove"
    @pointerup="pointerUp"
    @pointercancel="pointerUp"
    @wheel.prevent="wheel"
  >
    <div v-if="editable" class="canvas-toolbar" aria-label="Pixel tools">
      <div class="tool-group" aria-label="History">
        <button
          aria-label="Undo"
          title="Undo (⌘Z)"
          :disabled="!workspace.canUndo.value || workspace.busy.value"
          @click="workspace.undo"
        >
          <PhArrowCounterClockwise weight="regular" />
        </button>
        <button
          aria-label="Redo"
          title="Redo (⇧⌘Z)"
          :disabled="!workspace.editor.canRedo.value || workspace.busy.value"
          @click="workspace.redo"
        >
          <PhArrowClockwise weight="regular" />
        </button>
      </div>
      <span class="toolbar-divider"></span>
      <button
        v-for="item in [
          ['eyedropper', 'Pick colour', 'I'],
          ['pencil', 'Pencil', 'P'],
          ['eraser', 'Eraser', 'E'],
          ['fill', 'Fill', 'F'],
        ] as const"
        :key="item[0]"
        :aria-pressed="editable && workspace.editor.tool.value === item[0]"
        :title="`${item[1]} (${item[2]})`"
        @click="workspace.beginTool(item[0])"
      >
        <PhEyedropper v-if="item[0] === 'eyedropper'" weight="regular" />
        <PhPencilSimple v-else-if="item[0] === 'pencil'" weight="regular" />
        <PhEraser v-else-if="item[0] === 'eraser'" weight="regular" />
        <PhPaintBucket v-else weight="regular" />
        <span>{{ item[1] }}</span>
      </button>
      <span class="toolbar-divider"></span>
      <label class="brush-size">
        <span>Size</span>
        <input
          type="number"
          min="1"
          max="32"
          aria-label="Brush size"
          :value="workspace.editor.brushSize.value"
          @change="setBrushSize"
        />
        <span>px</span>
      </label>
      <label class="drawing-colour" title="Drawing colour">
        <input
          type="color"
          aria-label="Drawing colour"
          :value="workspace.editor.drawingColor.value"
          @change="
            workspace.setDrawingColor(($event.target as HTMLInputElement).value)
          "
        />
      </label>
      <template v-if="workspace.animation.frames.value.length === 1">
        <span class="toolbar-divider"></span>
        <button
          aria-label="Add frame to create animation"
          title="Add a reference image or pixel art frame"
          @click="workspace.animation.addFrameFromImage"
        >
          <PhFilmStrip weight="regular" />
          <span>Add frame</span>
        </button>
      </template>
    </div>
    <RigToolbar v-if="workspace.rig.rig.value" />
    <div class="preview-navigation" aria-label="Preview zoom">
      <button aria-label="Zoom out" @click="adjustZoom(zoom - 0.25)">−</button>
      <button aria-label="Reset preview view" @click="resetView">
        {{ Math.round(zoom * 100) }}%
      </button>
      <button aria-label="Zoom in" @click="adjustZoom(zoom + 0.25)">+</button>
    </div>
    <div
      ref="pixelCanvas"
      class="pixel-canvas"
      :class="{ editable, checker: editable, previewing: !editable, panning }"
      :style="canvasStyle"
      :tabindex="editable ? 0 : -1"
      aria-label="Sprite canvas"
      @pointerenter="canvasHovered = true"
      @pointerleave="canvasHovered = false"
      @keydown="keyDown"
    >
      <img
        :src="displayedImage"
        :alt="`${workspace.selectedAsset.value?.asset.id} pixel art`"
        draggable="false"
      />
      <RigOverlay />
      <span
        v-for="edit in workspace.editor.pendingEdits.value"
        :key="`${edit.x}:${edit.y}`"
        class="pending-pixel"
        :class="{
          erased:
            edit.index === workspace.view.value?.metadata.transparent_index,
        }"
        :style="editStyle(edit)"
      ></span>
      <span
        v-if="editable && canvasHovered"
        class="grid-cursor"
        :style="cursorStyle"
      ></span>
    </div>
    <span
      v-if="workspace.previewBusy.value || workspace.composition.busy.value"
      class="preview-indicator"
      role="status"
      aria-live="polite"
      ><i></i>Updating
      {{ workspace.mode.value === "convert" ? "preview" : "canvas" }}…</span
    >
  </div>
</template>
