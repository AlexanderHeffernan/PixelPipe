<script setup lang="ts">
import { PhCaretRight } from "@phosphor-icons/vue";
import { computed, ref, watch } from "vue";
import type { BackdropPolicy } from "../types";
import { useWorkspace } from "../workspace/context";
import HintTip from "./HintTip.vue";

const workspace = useWorkspace();
const backdrop = computed(() => workspace.settings.value?.backdrop);
function copyBorder(
  value: Extract<BackdropPolicy, { type: "border_connected" }>,
) {
  return { ...value, color: [...value.color] as [number, number, number] };
}
const savedBorder = ref<Extract<BackdropPolicy, { type: "border_connected" }>>(
  backdrop.value?.type === "border_connected"
    ? copyBorder(backdrop.value)
    : {
        type: "border_connected",
        color: [255, 255, 255],
        tolerance: 28,
        alpha_threshold: 8,
      },
);
const none = computed(() => backdrop.value?.type === "alpha");
const automatic = computed(
  () => !none.value && workspace.backgroundAutomatic.value,
);
const custom = computed(() => !none.value && !automatic.value);
const automaticResult = computed(
  () => workspace.preview.value?.background_removed,
);
const summary = computed(() => {
  if (none.value) return "None";
  if (custom.value) return "Chosen colour";
  return automaticResult.value === false ? "None found" : "Automatic";
});

watch(backdrop, (value) => {
  if (value?.type === "border_connected") savedBorder.value = copyBorder(value);
});
const colour = computed(() => {
  const value = backdrop.value;
  if (!value || value.type !== "border_connected") return "#ffffff";
  return `#${value.color
    .map((channel) => channel.toString(16).padStart(2, "0"))
    .join("")}`;
});

function setColour(event: Event) {
  const current = backdrop.value;
  if (!current || current.type !== "border_connected") return;
  const value = (event.target as HTMLInputElement).value;
  const color = [1, 3, 5].map((offset) =>
    Number.parseInt(value.slice(offset, offset + 2), 16),
  ) as [number, number, number];
  workspace.setBackgroundAutomatic(false);
  workspace.updateSettings({ backdrop: { ...current, color } });
}

function setTolerance(event: Event) {
  const current = backdrop.value;
  if (!current || current.type !== "border_connected") return;
  workspace.updateSettings({
    backdrop: {
      ...current,
      tolerance: Number((event.target as HTMLInputElement).value),
    },
  });
}

function setAutomatic() {
  workspace.updateSettings({ backdrop: copyBorder(savedBorder.value) });
  workspace.setBackgroundAutomatic(true);
}

function setCustom() {
  workspace.updateSettings({ backdrop: copyBorder(savedBorder.value) });
  workspace.setBackgroundAutomatic(false);
}

function setNone() {
  const alphaThreshold =
    backdrop.value?.type === "border_connected"
      ? backdrop.value.alpha_threshold
      : (backdrop.value?.alpha_threshold ?? 8);
  workspace.setBackgroundAutomatic(false);
  workspace.updateSettings({
    backdrop: { type: "alpha", alpha_threshold: alphaThreshold },
  });
}
</script>

<template>
  <details class="intent-section" open>
    <summary class="intent-heading">
      <strong>Background</strong>
      <HintTip
        text="Automatic removes a confident edge-connected backdrop while preserving edge-to-edge artwork."
      />
      <output>{{ summary }}</output>
      <PhCaretRight class="section-chevron" aria-hidden="true" />
    </summary>
    <div class="intent-content">
      <div class="background-mode" role="group" aria-label="Background colour">
        <button :aria-pressed="automatic" @click="setAutomatic">
          Automatic
        </button>
        <button :aria-pressed="none" @click="setNone">No background</button>
        <button :aria-pressed="custom" @click="setCustom">Pick colour</button>
      </div>
      <template v-if="backdrop?.type === 'border_connected'">
        <p v-if="automatic && automaticResult === false" class="detection-note">
          No clear background was found, so the whole image is being kept.
        </p>
        <label v-if="custom" class="background-colour">
          <span>Background colour</span>
          <span>
            <input
              type="color"
              aria-label="Background colour picker"
              :value="colour"
              @change="setColour"
            />
            <code>{{ colour.toUpperCase() }}</code>
          </span>
        </label>
        <label class="background-range">
          <span>Background range</span>
          <input
            type="range"
            aria-label="Background range"
            min="0"
            max="80"
            :value="backdrop.tolerance"
            @input="setTolerance"
          />
          <output>{{ backdrop.tolerance }}</output>
          <small><span>Exact</span><span>More forgiving</span></small>
        </label>
      </template>
      <HintTip
        v-else
        text="Existing transparency is preserved and no colour is removed."
      />
    </div>
  </details>
</template>
