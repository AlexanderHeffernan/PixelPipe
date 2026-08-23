<script setup lang="ts">
import { ref, watch } from "vue";
import { useWorkspace } from "../../workspace/context";
const workspace = useWorkspace();
const brief = ref(workspace.selectedAsset.value?.asset.brief.text ?? "");
watch(
  () => workspace.assetId.value,
  () => {
    brief.value = workspace.selectedAsset.value?.asset.brief.text ?? "";
  },
);
</script>

<template>
  <div class="stage-view narrow-stage">
    <div class="stage-intro">
      <span class="step-number">1</span>
      <div>
        <p class="eyebrow">Define the sprite</p>
        <h2>What are we making?</h2>
        <p>
          A strong brief gives your agent one camera, silhouette, and purpose to
          solve.
        </p>
      </div>
    </div>
    <form class="brief-editor" @submit.prevent="workspace.saveBrief(brief)">
      <label for="brief">Sprite brief</label>
      <textarea
        id="brief"
        v-model="brief"
        rows="9"
        required
        placeholder="A strict overhead field medic, facing north, compact silhouette, no cast shadow, readable at 32×32…"
      ></textarea>
      <div class="form-footer">
        <small
          >Keep it concrete. PixelPipe preserves this with every
          revision.</small
        ><button
          class="primary"
          :disabled="workspace.busy.value || !brief.trim()"
        >
          Save & Continue
        </button>
      </div>
    </form>
  </div>
</template>
