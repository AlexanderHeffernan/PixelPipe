<script setup lang="ts">
import { PhCaretRight } from "@phosphor-icons/vue";
import { computed, nextTick, ref, watch } from "vue";
import type { ColorAdjustments } from "../types";
import { useWorkspace } from "../workspace/context";
import HintTip from "./HintTip.vue";

const workspace = useWorkspace();
const colorCounts = [4, 8, 16, 36];
type MoodId = "original" | "warm" | "cool" | "vivid" | "muted" | "custom";
const treatments: { id: MoodId; label: string }[] = [
  { id: "original", label: "Original" },
  { id: "warm", label: "Warm" },
  { id: "cool", label: "Cool" },
  { id: "vivid", label: "Vivid" },
  { id: "muted", label: "Muted" },
  { id: "custom", label: "Custom" },
];
const moodAdjustments: Record<Exclude<MoodId, "custom">, ColorAdjustments> = {
  original: { brightness: 0, contrast: 0, saturation: 0, warmth: 0 },
  warm: { brightness: 0, contrast: 0, saturation: 5, warmth: 30 },
  cool: { brightness: 0, contrast: 0, saturation: 5, warmth: -30 },
  vivid: { brightness: 0, contrast: 10, saturation: 35, warmth: 0 },
  muted: { brightness: 2, contrast: -5, saturation: -35, warmth: 0 },
};
const customColors = ref(!colorCounts.includes(workspace.colorCount.value));
const customMood = ref(false);
const fineTune = ref<HTMLDetailsElement>();
const adjustments = computed<ColorAdjustments>(
  () =>
    workspace.settings.value?.color_adjustments ?? {
      brightness: 0,
      contrast: 0,
      saturation: 0,
      warmth: 0,
    },
);
const selectedMood = computed<MoodId>(() => {
  if (customMood.value) return "custom";
  return (
    (Object.entries(moodAdjustments).find(([, values]) =>
      sameAdjustments(values, adjustments.value),
    )?.[0] as MoodId | undefined) ?? "custom"
  );
});
const fineControls: { id: keyof ColorAdjustments; label: string }[] = [
  { id: "brightness", label: "Brightness" },
  { id: "contrast", label: "Contrast" },
  { id: "saturation", label: "Saturation" },
  { id: "warmth", label: "Warmth" },
];

watch(workspace.colorCount, (count) => {
  customColors.value = !colorCounts.includes(count);
});
watch(
  () => workspace.settings.value?.color_treatment,
  (legacy) => {
    if (legacy && legacy !== "original") setMood(legacy);
  },
  { immediate: true },
);

function setColorCount(value: number) {
  customColors.value = !colorCounts.includes(value);
  workspace.setColorCount(value);
}

function setCustomColors(event: Event) {
  const input = event.target as HTMLInputElement;
  const value = Math.min(64, Math.max(4, Number(input.value) || 4));
  input.value = String(value);
  setColorCount(value);
}

function setAdjustment(field: keyof ColorAdjustments, event: Event) {
  const input = event.target as HTMLInputElement;
  const value = Math.min(100, Math.max(-100, Number(input.value) || 0));
  input.value = String(value);
  const next = { ...adjustments.value, [field]: value };
  customMood.value = !Object.values(moodAdjustments).some((preset) =>
    sameAdjustments(preset, next),
  );
  workspace.updateSettings({
    color_treatment: "original",
    color_adjustments: next,
  });
}

function resetAdjustments() {
  setMood("original");
}

function sameAdjustments(a: ColorAdjustments, b: ColorAdjustments) {
  return (
    a.brightness === b.brightness &&
    a.contrast === b.contrast &&
    a.saturation === b.saturation &&
    a.warmth === b.warmth
  );
}

function setMood(mood: MoodId) {
  if (mood === "custom") {
    customMood.value = true;
    void nextTick(() => {
      if (fineTune.value) fineTune.value.open = true;
    });
    return;
  }
  customMood.value = false;
  workspace.updateSettings({
    color_treatment: "original",
    color_adjustments: { ...moodAdjustments[mood] },
  });
}
</script>

<template>
  <details class="intent-section" open>
    <summary class="intent-heading">
      <strong>Colour</strong>
      <HintTip
        text="Set how much source colour survives, then shift the overall mood in one step."
      />
      <output>Up to {{ workspace.colorCount.value }}</output>
      <PhCaretRight class="section-chevron" aria-hidden="true" />
    </summary>
    <div class="intent-content">
      <span class="control-label">Colour detail</span>
      <div class="choice-grid">
        <button
          v-for="value in colorCounts"
          :key="value"
          :aria-pressed="!customColors && workspace.colorCount.value === value"
          @click="setColorCount(value)"
        >
          {{ value }}
        </button>
        <button
          aria-label="Custom colour detail"
          :aria-pressed="customColors"
          @click="customColors = true"
        >
          Custom
        </button>
      </div>
      <label v-if="customColors" class="inline-number">
        <span>Maximum colours</span>
        <input
          type="number"
          aria-label="Maximum colours"
          min="4"
          max="64"
          :value="workspace.colorCount.value"
          @change="setCustomColors"
        />
      </label>

      <span class="control-label control-label--spaced">Colour mood</span>
      <div class="choice-grid treatment-choices">
        <button
          v-for="option in treatments"
          :key="option.id"
          :aria-pressed="selectedMood === option.id"
          @click="setMood(option.id)"
        >
          {{ option.label }}
        </button>
      </div>
      <details ref="fineTune" class="fine-tune">
        <summary>Fine Tune</summary>
        <div>
          <label v-for="control in fineControls" :key="control.id">
            <span>{{ control.label }}</span>
            <input
              type="range"
              min="-100"
              max="100"
              :aria-label="control.label"
              :value="adjustments[control.id]"
              @input="setAdjustment(control.id, $event)"
            />
            <input
              class="fine-value"
              type="number"
              min="-100"
              max="100"
              :aria-label="`${control.label} exact value`"
              :value="adjustments[control.id]"
              @change="setAdjustment(control.id, $event)"
            />
          </label>
          <button class="reset-adjustments" @click="resetAdjustments">
            Reset adjustments
          </button>
        </div>
      </details>
    </div>
  </details>
</template>
