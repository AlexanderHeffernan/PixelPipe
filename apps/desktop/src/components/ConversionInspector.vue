<script setup lang="ts">
import { computed } from "vue";
import { useWorkspace } from "../workspace/context";
import type { BackdropPolicy } from "../types";

const workspace = useWorkspace();
const settings = computed(() => workspace.settings.value);
const sizes = [16, 32, 48, 64];

function setSize(size: number) {
  workspace.updateSettings({ width: size, height: size });
}

function setNumber(field: "margin" | "coverage_percent", event: Event) {
  workspace.updateSettings({
    [field]: Number((event.target as HTMLInputElement).value),
  });
}

function setBackdrop(event: Event) {
  const type = (event.target as HTMLSelectElement).value;
  const backdrop: BackdropPolicy =
    type === "alpha"
      ? { type: "alpha", alpha_threshold: 8 }
      : {
          type: "border_connected",
          color: [255, 255, 255],
          tolerance: 28,
          alpha_threshold: 8,
        };
  workspace.updateSettings({ backdrop });
}

function setRegistration(event: Event) {
  const registration = (event.target as HTMLSelectElement).value as
    | "center"
    | "bottom";
  workspace.updateSettings({ registration });
}

const color = (rgba: number[]) => `rgba(${rgba.join(",")})`;
</script>

<template>
  <aside class="conversion-inspector">
    <template v-if="workspace.mode.value === 'convert' && settings">
      <header class="inspector-heading">
        <strong>Conversion</strong>
        <span><i :class="{ busy: workspace.previewBusy.value }"></i>Live</span>
      </header>

      <div class="inspector-control output-size">
        <label>Output Size</label>
        <div class="segmented-control">
          <button
            v-for="size in sizes"
            :key="size"
            :aria-pressed="settings.width === size && settings.height === size"
            @click="setSize(size)"
          >
            {{ size }}
          </button>
        </div>
      </div>

      <label class="inspector-row"
        ><span>Crop</span><span class="static-value">Tight</span></label
      >
      <label class="inspector-row">
        <span>Registration</span>
        <select :value="settings.registration" @change="setRegistration">
          <option value="center">Center</option>
          <option value="bottom">Bottom Center</option>
        </select>
      </label>

      <label class="inspector-slider">
        <span>Padding</span>
        <input
          type="range"
          min="0"
          max="8"
          :value="settings.margin"
          @input="setNumber('margin', $event)"
        />
        <output>{{ settings.margin }}px</output>
      </label>

      <label class="inspector-row">
        <span>Background</span>
        <select :value="settings.backdrop.type" @change="setBackdrop">
          <option value="border_connected">Auto detect</option>
          <option value="alpha">Transparency</option>
        </select>
      </label>
      <label class="inspector-row">
        <span>Palette</span
        ><span class="static-value">{{ workspace.paletteName.value }}</span>
      </label>
      <label class="inspector-row">
        <span>Colours</span
        ><span class="static-value">{{
          workspace.inspection.value?.palette.length ?? 0
        }}</span>
      </label>

      <label class="inspector-slider">
        <span>Coverage</span>
        <input
          type="range"
          min="1"
          max="100"
          :value="settings.coverage_percent"
          @input="setNumber('coverage_percent', $event)"
        />
        <output>{{ settings.coverage_percent }}%</output>
      </label>
    </template>

    <template v-else>
      <header class="inspector-heading"><strong>Pixel Editing</strong></header>
      <div class="edit-summary">
        <span>Revision</span
        ><strong>{{ workspace.view.value?.metadata.revision ?? "—" }}</strong>
        <span>Canvas</span
        ><strong
          >{{ workspace.inspection.value?.width }} ×
          {{ workspace.inspection.value?.height }}</strong
        >
        <span>Palette</span><strong>{{ workspace.paletteName.value }}</strong>
      </div>
      <p class="inspector-note">
        Pencil and fill tools are the next editing slice. Your conversion is
        safely stored as an immutable revision.
      </p>
    </template>

    <div
      v-if="workspace.inspection.value?.palette.length"
      class="palette-strip"
      aria-label="Used palette colours"
    >
      <span
        v-for="entry in workspace.inspection.value.palette"
        :key="entry.index"
        :style="{ backgroundColor: color(entry.rgba) }"
        :title="`Index ${entry.index}`"
      ></span>
    </div>
  </aside>
</template>
