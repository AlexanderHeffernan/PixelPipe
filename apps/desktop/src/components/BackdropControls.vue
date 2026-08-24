<script setup lang="ts">
import { computed, ref } from "vue";
import type { BackdropPolicy } from "../types";
import { useWorkspace } from "../workspace/context";

const workspace = useWorkspace();
const backdrop = computed(() => workspace.settings.value!.backdrop);
type BorderBackdrop = Extract<BackdropPolicy, { type: "border_connected" }>;
const defaultBorder: BorderBackdrop = {
  type: "border_connected",
  color: [255, 255, 255],
  tolerance: 28,
  alpha_threshold: backdrop.value.alpha_threshold,
};
const lastBorder = ref(
  backdrop.value.type === "border_connected"
    ? copyBorder(backdrop.value)
    : defaultBorder,
);

function copyBorder(value: BorderBackdrop): BorderBackdrop {
  return {
    ...value,
    color: [value.color[0], value.color[1], value.color[2]],
  };
}

function setMode(event: Event) {
  const type = (event.target as HTMLSelectElement).value;
  if (backdrop.value.type === "border_connected") {
    lastBorder.value = copyBorder(backdrop.value);
  }
  const next: BackdropPolicy =
    type === "alpha"
      ? { type: "alpha", alpha_threshold: backdrop.value.alpha_threshold }
      : {
          ...lastBorder.value,
          alpha_threshold: backdrop.value.alpha_threshold,
        };
  workspace.updateSettings({ backdrop: next });
}

function updateAlpha(event: Event) {
  workspace.updateSettings({
    backdrop: {
      ...backdrop.value,
      alpha_threshold: Number((event.target as HTMLInputElement).value),
    },
  });
}

function updateTolerance(event: Event) {
  if (backdrop.value.type !== "border_connected") return;
  updateBorder({
    ...backdrop.value,
    tolerance: Number((event.target as HTMLInputElement).value),
  });
}

function updateColor(event: Event) {
  if (backdrop.value.type !== "border_connected") return;
  const value = (event.target as HTMLInputElement).value;
  const color = [1, 3, 5].map((offset) =>
    Number.parseInt(value.slice(offset, offset + 2), 16),
  ) as [number, number, number];
  updateBorder({ ...backdrop.value, color });
}

function updateBorder(next: BorderBackdrop) {
  lastBorder.value = next;
  workspace.updateSettings({ backdrop: next });
}

const hexColor = computed(() =>
  backdrop.value.type === "border_connected"
    ? `#${backdrop.value.color.map((value) => value.toString(16).padStart(2, "0")).join("")}`
    : "#ffffff",
);
</script>

<template>
  <section class="inspector-section" aria-labelledby="background-label">
    <label class="inspector-row">
      <span id="background-label">Background</span>
      <select :value="backdrop.type" @change="setMode">
        <option value="border_connected">Solid edge colour</option>
        <option value="alpha">Transparency only</option>
      </select>
    </label>
    <p class="control-help">
      <template v-if="backdrop.type === 'border_connected'">
        Removes matching colour connected to the image edge, while preserving it
        inside the subject.
      </template>
      <template v-else
        >Uses the source image’s transparency without removing a
        colour.</template
      >
    </p>

    <label v-if="backdrop.type === 'border_connected'" class="inspector-row">
      <span>Edge colour</span>
      <span class="color-control">
        <input type="color" :value="hexColor" @input="updateColor" />
        <code>{{ hexColor.toUpperCase() }}</code>
      </span>
    </label>
    <label v-if="backdrop.type === 'border_connected'" class="inspector-slider">
      <span>Colour tolerance</span>
      <input
        type="range"
        aria-label="Colour tolerance"
        min="0"
        max="255"
        :value="backdrop.tolerance"
        @input="updateTolerance"
      />
      <output>{{ backdrop.tolerance }}</output>
      <small
        >Higher values remove a wider range of shades around the edge
        colour.</small
      >
    </label>
    <label class="inspector-slider">
      <span>Alpha cutoff</span>
      <input
        type="range"
        aria-label="Alpha cutoff"
        min="0"
        max="255"
        :value="backdrop.alpha_threshold"
        @input="updateAlpha"
      />
      <output>{{ backdrop.alpha_threshold }}</output>
      <small>Pixels at or below this opacity are treated as transparent.</small>
    </label>
  </section>
</template>
