<script setup lang="ts">
import { computed } from "vue";
import { pngDataUrl } from "../../api";
import { useWorkspace } from "../../workspace/context";
const workspace = useWorkspace();
const native = computed(() =>
  workspace.view.value
    ? pngDataUrl(workspace.view.value.native_png_base64)
    : "",
);
const preview = computed(() =>
  workspace.view.value
    ? pngDataUrl(workspace.view.value.preview_png_base64)
    : "",
);
</script>

<template>
  <div v-if="workspace.view.value" class="stage-view review-stage">
    <div class="review-toolbar">
      <div>
        <p class="eyebrow">
          Revision {{ workspace.view.value.metadata.revision }}
        </p>
        <h2>Does it read at native size?</h2>
      </div>
      <button class="primary" @click="workspace.stage.value = 'export'">
        Export Sprite
      </button>
    </div>
    <div class="preview-grid">
      <figure>
        <figcaption>
          <strong>Native</strong
          ><span
            >{{ workspace.view.value.metadata.inspection.width }}×{{
              workspace.view.value.metadata.inspection.height
            }}</span
          >
        </figcaption>
        <div class="canvas checker native">
          <img :src="native" alt="Sprite at native resolution" />
        </div>
      </figure>
      <figure>
        <figcaption>
          <strong>Nearest preview</strong><span>No smoothing</span>
        </figcaption>
        <div class="canvas checker enlarged">
          <img :src="preview" alt="Nearest-neighbour enlarged sprite" />
        </div>
      </figure>
    </div>
    <div class="review-facts">
      <span
        ><strong>{{
          workspace.view.value.metadata.inspection.visible_pixels
        }}</strong>
        visible pixels</span
      ><span
        ><strong>{{
          workspace.view.value.metadata.inspection.palette.length
        }}</strong>
        colors used</span
      ><span
        ><strong>{{
          workspace.view.value.metadata.validation.valid
            ? "Valid"
            : "Needs work"
        }}</strong>
        machine checks</span
      >
    </div>
  </div>
</template>
