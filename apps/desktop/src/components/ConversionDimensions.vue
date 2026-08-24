<script setup lang="ts">
import { computed, ref } from "vue";
import { useWorkspace } from "../workspace/context";

const workspace = useWorkspace();
const settings = computed(() => workspace.settings.value!);
const ratioLocked = ref(settings.value.width === settings.value.height);
const presets = [16, 32, 48, 64];
const maximumDimension = 512;

const maximumMargin = computed(() =>
  Math.max(
    0,
    Math.floor((Math.min(settings.value.width, settings.value.height) - 1) / 2),
  ),
);

function setPreset(size: number) {
  ratioLocked.value = true;
  workspace.updateSettings({
    width: size,
    height: size,
    margin: Math.min(settings.value.margin, Math.floor((size - 1) / 2)),
  });
}

function setDimension(field: "width" | "height", event: Event) {
  const input = event.target as HTMLInputElement;
  const value = Math.min(
    maximumDimension,
    Math.max(1, Number(input.value) || 1),
  );
  input.value = String(value);
  const dimensions = ratioLocked.value
    ? { width: value, height: value }
    : {
        width: settings.value.width,
        height: settings.value.height,
        [field]: value,
      };
  const margin = Math.min(
    settings.value.margin,
    Math.floor((Math.min(dimensions.width, dimensions.height) - 1) / 2),
  );
  workspace.updateSettings({ ...dimensions, margin });
}
</script>

<template>
  <section
    class="inspector-section output-size"
    aria-labelledby="output-size-label"
  >
    <div class="control-heading">
      <label id="output-size-label">Output size</label>
      <button
        class="ratio-lock"
        :aria-pressed="ratioLocked"
        :title="
          ratioLocked ? 'Unlock width and height' : 'Lock width and height'
        "
        @click="ratioLocked = !ratioLocked"
      >
        {{ ratioLocked ? "Linked" : "Independent" }}
      </button>
    </div>
    <div class="segmented-control">
      <button
        v-for="size in presets"
        :key="size"
        :aria-pressed="settings.width === size && settings.height === size"
        @click="setPreset(size)"
      >
        {{ size }}
      </button>
    </div>
    <div class="dimension-fields">
      <label>
        <span>Width</span>
        <input
          type="number"
          min="1"
          :max="maximumDimension"
          :value="settings.width"
          @change="setDimension('width', $event)"
        />
      </label>
      <span aria-hidden="true">×</span>
      <label>
        <span>Height</span>
        <input
          type="number"
          min="1"
          :max="maximumDimension"
          :value="settings.height"
          @change="setDimension('height', $event)"
        />
      </label>
    </div>
    <p class="control-help">
      Final sprite dimensions from 1–512 pixels. Smaller sizes produce stronger,
      simpler shapes.
    </p>
  </section>

  <label class="inspector-slider">
    <span>Edge padding</span>
    <input
      type="range"
      aria-label="Edge padding"
      min="0"
      :max="maximumMargin"
      :value="settings.margin"
      @input="
        workspace.updateSettings({
          margin: Number(($event.target as HTMLInputElement).value),
        })
      "
    />
    <output>{{ settings.margin }}px</output>
    <small>Keeps the subject away from the canvas edge.</small>
  </label>
</template>
