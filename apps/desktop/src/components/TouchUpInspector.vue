<script setup lang="ts">
import { PhArrowLeft, PhCaretRight, PhExport } from "@phosphor-icons/vue";
import { computed } from "vue";
import type { Rgba } from "../types";
import { useWorkspace } from "../workspace/context";
import HintTip from "./HintTip.vue";

const workspace = useWorkspace();
const canvas = computed(() => workspace.composition.settings.value);

function rgbaHex(rgba: Rgba) {
  return `#${rgba
    .slice(0, 3)
    .map((value) => value.toString(16).padStart(2, "0"))
    .join("")}`;
}

function canvasNumber(
  field: "width" | "height" | "offset_x" | "offset_y",
  event: Event,
) {
  const input = event.target as HTMLInputElement;
  const dimension = field === "width" || field === "height";
  const value = dimension
    ? Math.min(512, Math.max(1, Number(input.value) || 1))
    : Math.min(32767, Math.max(-32768, Number(input.value) || 0));
  input.value = String(value);
  workspace.composition.update({ [field]: value });
}
</script>

<template>
  <header class="inspector-heading">
    <div>
      <span
        >Step {{ workspace.view.value?.metadata.rig_ancestor ? 3 : 2 }}</span
      >
      <strong>Canvas &amp; Touch Up</strong>
    </div>
  </header>

  <p
    v-if="workspace.composition.error.value"
    class="preview-error"
    role="alert"
  >
    Canvas could not update. {{ workspace.composition.error.value }}
  </p>

  <details class="inspector-panel" open>
    <summary>
      <strong>Canvas &amp; Placement</strong>
      <HintTip
        text="Placement moves pixels instantly. Artwork may extend beyond the canvas."
      />
      <PhCaretRight class="section-chevron" aria-hidden="true" />
    </summary>
    <div class="panel-content">
      <div class="dimension-fields compact-fields">
        <label>
          <span>Width</span>
          <input
            type="number"
            min="1"
            max="512"
            :value="canvas?.width"
            @change="canvasNumber('width', $event)"
          />
        </label>
        <span aria-hidden="true">×</span>
        <label>
          <span>Height</span>
          <input
            type="number"
            min="1"
            max="512"
            :value="canvas?.height"
            @change="canvasNumber('height', $event)"
          />
        </label>
      </div>
      <div class="position-fields compact-fields">
        <label>
          <span>Move right</span>
          <input
            type="number"
            aria-label="Canvas horizontal position"
            :value="canvas?.offset_x"
            @input="canvasNumber('offset_x', $event)"
          />
        </label>
        <label>
          <span>Move up</span>
          <input
            type="number"
            aria-label="Canvas vertical position"
            :value="canvas?.offset_y"
            @input="canvasNumber('offset_y', $event)"
          />
        </label>
      </div>
    </div>
  </details>

  <details class="inspector-panel" open>
    <summary>
      <strong>Palette Colours</strong>
      <HintTip
        text="Replace a colour everywhere. Choose the drawing colour in the canvas toolbar."
      />
      <PhCaretRight class="section-chevron" aria-hidden="true" />
    </summary>
    <div class="panel-content">
      <div class="palette-editor">
        <label
          v-for="entry in workspace.inspection.value?.palette"
          :key="entry.index"
        >
          <input
            type="color"
            :aria-label="`Replace colour ${entry.index}`"
            :value="rgbaHex(entry.rgba)"
            @change="
              workspace.recolorCurrent(
                entry.index,
                ($event.target as HTMLInputElement).value,
              )
            "
          />
          <span>{{ rgbaHex(entry.rgba).toUpperCase() }}</span>
        </label>
      </div>
    </div>
  </details>

  <div class="inspector-spacer"></div>
  <footer class="phase-action canvas-actions">
    <button
      v-if="workspace.view.value?.metadata.rig_ancestor"
      class="back-button continue-button"
      :disabled="workspace.busy.value"
      @click="workspace.rig.returnToRig"
    >
      <PhArrowLeft aria-hidden="true" />
      <span>Back to Rigging</span>
    </button>
    <button
      v-else-if="workspace.canConvert.value"
      class="back-button continue-button"
      :disabled="workspace.busy.value"
      @click="workspace.reconvert"
    >
      <PhArrowLeft aria-hidden="true" />
      <span>Back to Pixelize</span>
      <HintTip
        embedded
        text="Returning starts a fresh conversion. These canvas changes remain in history but do not carry forward."
      />
    </button>
    <button
      class="primary continue-button"
      :disabled="workspace.busy.value"
      @click="workspace.exportCurrent"
    >
      Export Sprite…
      <PhExport aria-hidden="true" />
    </button>
  </footer>
</template>
