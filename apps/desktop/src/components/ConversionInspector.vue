<script setup lang="ts">
import { computed } from "vue";
import BackdropControls from "./BackdropControls.vue";
import ConversionDimensions from "./ConversionDimensions.vue";
import { useWorkspace } from "../workspace/context";

const workspace = useWorkspace();
const settings = computed(() => workspace.settings.value);

function setNumber(field: "coverage_percent", event: Event) {
  workspace.updateSettings({
    [field]: Number((event.target as HTMLInputElement).value),
  });
}

function setRegistration(event: Event) {
  const registration = (event.target as HTMLSelectElement).value as
    | "center"
    | "bottom";
  workspace.updateSettings({ registration });
}

function setRecipe(event: Event) {
  workspace.chooseRecipe((event.target as HTMLSelectElement).value);
}

function setComponent(field: "min" | "max", event: Event) {
  if (!settings.value) return;
  const input = event.target as HTMLInputElement;
  const value = Math.min(65535, Math.max(1, Number(input.value) || 1));
  input.value = String(value);
  const current = settings.value.components;
  workspace.updateSettings({
    components:
      field === "min"
        ? { min: value, max: Math.max(current.max, value) }
        : { min: Math.min(current.min, value), max: value },
  });
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

      <p v-if="workspace.previewError.value" class="preview-error" role="alert">
        Preview could not update. {{ workspace.previewError.value }}
      </p>

      <label v-if="workspace.recipes.value.length > 1" class="inspector-row">
        <span>Starting recipe</span>
        <select :value="workspace.recipeId.value" @change="setRecipe">
          <option
            v-for="recipe in workspace.recipes.value"
            :key="recipe.id"
            :value="recipe.id"
          >
            {{ recipe.id.replaceAll("-", " ") }}
          </option>
        </select>
      </label>

      <ConversionDimensions />

      <label class="inspector-row">
        <span>Registration</span>
        <select :value="settings.registration" @change="setRegistration">
          <option value="center">Center</option>
          <option value="bottom">Bottom Center</option>
        </select>
      </label>
      <p class="control-help">
        Bottom alignment keeps characters and props on a shared ground line.
      </p>

      <BackdropControls />

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
        <span>Shape coverage</span>
        <input
          type="range"
          aria-label="Shape coverage"
          min="1"
          max="100"
          :value="settings.coverage_percent"
          @input="setNumber('coverage_percent', $event)"
        />
        <output>{{ settings.coverage_percent }}%</output>
        <small>
          Lower keeps fine details; higher produces cleaner, stronger
          silhouettes.
        </small>
      </label>

      <details class="advanced-controls">
        <summary>Structure validation</summary>
        <p class="control-help">
          Reject results outside this connected-shape range. This catches
          accidental fragments; it does not change the artwork.
        </p>
        <div class="component-fields">
          <label>
            <span>Minimum</span>
            <input
              type="number"
              min="1"
              max="65535"
              :value="settings.components.min"
              @change="setComponent('min', $event)"
            />
          </label>
          <label>
            <span>Maximum</span>
            <input
              type="number"
              min="1"
              max="65535"
              :value="settings.components.max"
              @change="setComponent('max', $event)"
            />
          </label>
        </div>
      </details>
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
