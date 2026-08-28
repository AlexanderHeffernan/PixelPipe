<script setup lang="ts">
import { PhImageSquare } from "@phosphor-icons/vue";
import SpriteCanvas from "./SpriteCanvas.vue";
import FrameTimeline from "./FrameTimeline.vue";
import { useWorkspace } from "../workspace/context";

const workspace = useWorkspace();
</script>

<template>
  <section class="asset-workspace">
    <div
      v-if="workspace.canvasImage.value || workspace.loadingArtwork.value"
      class="canvas-viewport"
      :class="{ 'is-loading': workspace.canvasLoading.value }"
    >
      <SpriteCanvas />
      <div
        v-if="workspace.canvasLoading.value"
        class="canvas-loading"
        role="status"
        aria-live="polite"
      >
        <i aria-hidden="true"></i>
        <strong>{{ workspace.loadingMessage.value }}</strong>
      </div>
    </div>

    <div v-else-if="workspace.selectedAsset.value" class="canvas-empty">
      <PhImageSquare aria-hidden="true" />
      <h1>Add a source image</h1>
      <p>
        Upload a smooth reference now, or ask your coding agent to add options
        through the Pixelate CLI.
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
      <button
        class="primary"
        :disabled="workspace.importing.value"
        @click="workspace.importAssets"
      >
        Import Asset…
      </button>
    </div>

    <FrameTimeline />

    <span
      v-if="workspace.importing.value"
      class="preview-indicator import-indicator"
      role="status"
      aria-live="polite"
    >
      <i></i>Importing…
    </span>

    <div
      v-if="workspace.error.value || workspace.notice.value"
      class="toast workspace-toast"
      :class="{ error: workspace.error.value }"
      :role="workspace.error.value ? 'alert' : 'status'"
    >
      {{ workspace.error.value || workspace.notice.value }}
    </div>
  </section>
</template>
