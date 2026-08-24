<script setup lang="ts">
import { computed } from "vue";
import { useWorkspace } from "../workspace/context";

const workspace = useWorkspace();
const dimensions = computed(() => {
  const inspection = workspace.inspection.value;
  return inspection ? `${inspection.width} × ${inspection.height}` : "—";
});
const colors = computed(() => workspace.inspection.value?.palette.length ?? 0);
</script>

<template>
  <section class="asset-workspace">
    <div v-if="workspace.canvasImage.value" class="canvas-viewport checker">
      <img
        :src="workspace.canvasImage.value"
        :alt="`${workspace.selectedAsset.value?.asset.id} pixel art`"
      />
      <span v-if="workspace.previewBusy.value" class="preview-indicator"
        >Updating…</span
      >
    </div>

    <div v-else-if="workspace.selectedAsset.value" class="canvas-empty">
      <svg viewBox="0 0 32 32" aria-hidden="true">
        <path
          d="M7 5h18a2 2 0 0 1 2 2v18a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2Z"
        />
        <path d="m9 22 5-6 4 4 3-4 3 6M21 11h.01" />
      </svg>
      <h1>Add a source image</h1>
      <p>
        Upload a smooth reference now, or ask your coding agent to add options
        through the PixelPipe CLI.
      </p>
      <button class="primary" @click="workspace.importReference">
        Choose Image…
      </button>
    </div>

    <div v-else class="canvas-empty">
      <h1>Create your first asset</h1>
      <p>
        Start from a reference image or prepare an asset for your coding agent.
      </p>
      <button class="primary" @click="workspace.createAssetOpen.value = true">
        Create Asset
      </button>
    </div>

    <footer v-if="workspace.inspection.value" class="canvas-status">
      <span>{{ dimensions }}</span
      ><i></i><span>Fit</span><i></i>
      <span>{{ colors }} colours</span>
    </footer>
  </section>
</template>
