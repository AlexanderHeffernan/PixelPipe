<script setup lang="ts">
import { PhArrowRight, PhCaretRight, PhImageSquare } from "@phosphor-icons/vue";
import { computed, ref } from "vue";
import { useWorkspace } from "../workspace/context";
import PixelizeBackground from "./PixelizeBackground.vue";
import PixelizeColour from "./PixelizeColour.vue";
import HintTip from "./HintTip.vue";

const workspace = useWorkspace();
const settings = computed(() => workspace.settings.value);
const resolutions = [32, 64, 128, 256];
const customResolution = ref(
  !resolutions.includes(settings.value?.width ?? 32) ||
    settings.value?.width !== settings.value?.height,
);

function setResolution(value: number) {
  customResolution.value = !resolutions.includes(value);
  workspace.updateSettings({
    width: value,
    height: value,
    margin: 0,
    subject_scale_percent: 100,
    offset_x: 0,
    offset_y: 0,
    registration: "center",
  });
}

function setCustomResolution(event: Event) {
  const input = event.target as HTMLInputElement;
  const value = Math.min(256, Math.max(32, Number(input.value) || 32));
  input.value = String(value);
  setResolution(value);
}
</script>

<template>
  <header class="inspector-heading">
    <div><span>Step 1</span><strong>Pixelize</strong></div>
  </header>

  <p v-if="workspace.previewError.value" class="preview-error" role="alert">
    Preview could not update. {{ workspace.previewError.value }}
  </p>

  <details class="intent-section" open>
    <summary class="intent-heading">
      <strong id="resolution-label">Sprite resolution</strong>
      <HintTip
        text="Choose the rough pixel density. Canvas placement comes next."
      />
      <output>{{ settings?.width ?? 0 }} px</output>
      <PhCaretRight class="section-chevron" aria-hidden="true" />
    </summary>
    <div class="intent-content">
      <div class="choice-grid resolution-choices">
        <button
          v-for="value in resolutions"
          :key="value"
          :aria-pressed="
            !customResolution &&
            settings?.width === value &&
            settings?.height === value
          "
          @click="setResolution(value)"
        >
          {{ value }}
        </button>
        <button
          aria-label="Custom resolution"
          :aria-pressed="customResolution"
          @click="customResolution = true"
        >
          Custom
        </button>
      </div>
      <label v-if="customResolution" class="inline-number">
        <span>Resolution</span>
        <input
          type="number"
          aria-label="Custom resolution value"
          min="32"
          max="256"
          :value="settings?.width"
          @change="setCustomResolution"
        />
      </label>
    </div>
  </details>

  <PixelizeColour />

  <PixelizeBackground />

  <div class="inspector-spacer"></div>
  <footer class="phase-action canvas-actions">
    <button
      class="back-button continue-button"
      :disabled="workspace.busy.value || workspace.canvasLoading.value"
      @click="workspace.replaceSource"
    >
      <PhImageSquare aria-hidden="true" />
      <span>Replace Source Image…</span>
    </button>
    <button
      class="primary continue-button"
      :disabled="
        workspace.previewBusy.value || Boolean(workspace.previewError.value)
      "
      @click="workspace.setMode('edit')"
    >
      Continue to Canvas
      <PhArrowRight aria-hidden="true" />
    </button>
  </footer>
</template>
