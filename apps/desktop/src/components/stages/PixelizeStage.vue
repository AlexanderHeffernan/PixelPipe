<script setup lang="ts">
import { ref } from "vue";
import { useWorkspace } from "../../workspace/context";
const workspace = useWorkspace();
const recipe = ref(
  workspace.recipes.value.find(({ id }) => id === "sprite-32")?.id ??
    workspace.recipes.value[0]?.id ??
    "",
);
</script>

<template>
  <div class="stage-view narrow-stage">
    <div class="stage-intro">
      <span class="step-number">3</span>
      <div>
        <p class="eyebrow">Deterministic conversion</p>
        <h2>Choose the sprite size</h2>
        <p>
          We crop, register, hard-reduce, palette-map, validate, and preserve
          the exact recipe.
        </p>
      </div>
    </div>
    <div class="size-options" role="radiogroup" aria-label="Sprite size">
      <button
        v-for="option in workspace.recipes.value"
        :key="option.id"
        role="radio"
        :aria-checked="recipe === option.id"
        :class="{ selected: recipe === option.id }"
        @click="recipe = option.id"
      >
        <strong
          >{{ option.id.replace("sprite-", "") }}×{{
            option.id.replace("sprite-", "")
          }}</strong
        ><small>{{
          option.id === "sprite-32"
            ? "Recommended"
            : option.id === "sprite-16"
              ? "Small icons"
              : "Detailed sprites"
        }}</small>
      </button>
    </div>
    <div class="conversion-summary">
      <span>✓ Hard pixel reduction</span><span>✓ Fixed 16-color palette</span
      ><span>✓ No blur or dithering</span><span>✓ Immutable revision</span>
    </div>
    <button
      class="primary large"
      :disabled="workspace.busy.value || !recipe"
      @click="workspace.pixelize(recipe)"
    >
      {{ workspace.busy.value ? "Pixelizing…" : "Create Sprite" }}
    </button>
  </div>
</template>
