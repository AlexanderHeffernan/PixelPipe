<script setup lang="ts">
import { useWorkspace } from "../../workspace/context";
const workspace = useWorkspace();
</script>

<template>
  <div class="stage-view narrow-stage">
    <div class="stage-intro">
      <span class="step-number">5</span>
      <div>
        <p class="eyebrow">Game-ready output</p>
        <h2>Export the sprite</h2>
        <p>
          Choose a folder in your game. PixelPipe writes the indexed PNG and
          canonical JSON together.
        </p>
      </div>
    </div>
    <div class="export-card">
      <span class="export-icon">↗</span>
      <div>
        <strong>{{ workspace.assetId.value }}.png</strong
        ><small>Indexed runtime PNG + deterministic JSON metadata</small>
      </div>
    </div>
    <button
      class="primary large"
      :disabled="workspace.busy.value"
      @click="workspace.exportSprite"
    >
      {{ workspace.busy.value ? "Exporting…" : "Choose Export Folder…" }}
    </button>
  </div>
</template>
