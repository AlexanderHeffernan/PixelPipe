<script setup lang="ts">
import BriefStage from "./stages/BriefStage.vue";
import ExportStage from "./stages/ExportStage.vue";
import PixelizeStage from "./stages/PixelizeStage.vue";
import ReferenceStage from "./stages/ReferenceStage.vue";
import ReviewStage from "./stages/ReviewStage.vue";
import { useWorkspace } from "../workspace/context";

const workspace = useWorkspace();
const stages = {
  brief: BriefStage,
  reference: ReferenceStage,
  pixelize: PixelizeStage,
  review: ReviewStage,
  export: ExportStage,
};
</script>

<template>
  <section v-if="workspace.selectedAsset.value" class="asset-workspace">
    <header class="asset-header">
      <div>
        <small>{{ workspace.selectedAsset.value.asset.kind }}</small>
        <h1>{{ workspace.selectedAsset.value.asset.id }}</h1>
      </div>
      <span class="state-badge">{{
        workspace.selectedAsset.value.asset.state.replaceAll("_", " ")
      }}</span>
    </header>
    <component :is="stages[workspace.stage.value]" />
  </section>
  <section v-else class="empty-project">
    <div class="empty-glyph">◇</div>
    <h1>Create your first sprite</h1>
    <p>
      Give it a name and describe what you want. PixelPipe handles the project
      setup.
    </p>
    <button class="primary large" @click="workspace.creatingAsset.value = true">
      New Sprite
    </button>
  </section>
</template>
