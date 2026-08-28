<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { suggestedAssetId } from "../workspace/catalog-actions";
import { useWorkspace } from "../workspace/context";
import AppButton from "./AppButton.vue";

const workspace = useWorkspace();
const path = computed(() => workspace.projectImagePath.value);
const dimensions = ref<{ width: number; height: number }>();
const tooLargeForPixelArt = computed(
  () =>
    !dimensions.value ||
    dimensions.value.width > 256 ||
    dimensions.value.height > 256,
);

watch(
  path,
  () => {
    dimensions.value = undefined;
  },
  { immediate: true },
);

async function hide() {
  const selected = path.value;
  await workspace.catalog.setIgnored(selected, true);
  workspace.clearProjectImage();
}

function identity() {
  const filename = path.value.split("/").at(-1) || path.value;
  const stem = filename.replace(/\.[^.]+$/, "");
  const folder = path.value.includes("/")
    ? path.value.slice(0, path.value.lastIndexOf("/") + 1)
    : "";
  const base = suggestedAssetId(path.value);
  const existing = new Set(
    workspace.project.value?.assets.map(({ asset }) => asset.id) || [],
  );
  let id = base;
  let suffix = 2;
  while (existing.has(id)) id = `${base}-${suffix++}`;
  return {
    id,
    name: stem.replaceAll(/[-_]+/g, " "),
    destination: `${folder}${stem}-pixel.png`,
  };
}

async function adoptReference() {
  const selected = identity();
  await workspace.catalog.adoptReference(
    path.value,
    selected.id,
    selected.name,
    selected.destination,
  );
}

async function adoptPixelArt() {
  if (tooLargeForPixelArt.value) return;
  const selected = identity();
  await workspace.catalog.adoptPixelArt(path.value, selected.id, selected.name);
}

function readDimensions(event: Event) {
  const image = event.currentTarget as HTMLImageElement;
  dimensions.value = { width: image.naturalWidth, height: image.naturalHeight };
}
</script>

<template>
  <section class="project-image-stage" aria-labelledby="project-image-title">
    <div class="project-image-stage__preview checker">
      <img
        :src="workspace.projectImagePreview.value"
        :alt="`${path} preview`"
        @load="readDimensions"
      />
    </div>
    <div class="project-image-stage__details">
      <h1 id="project-image-title">{{ path.split("/").at(-1) }}</h1>
      <p>{{ path }}</p>
      <div class="project-image-stage__actions">
        <AppButton
          variant="quiet"
          :disabled="workspace.busy.value"
          @click="hide"
        >
          Hide from Assets
        </AppButton>
        <AppButton
          variant="secondary"
          :disabled="workspace.busy.value"
          @click="adoptReference"
        >
          Use as Reference
        </AppButton>
        <AppButton
          variant="primary"
          :disabled="tooLargeForPixelArt || workspace.busy.value"
          :title="
            tooLargeForPixelArt
              ? 'Exact pixel art import supports images up to 256 × 256 pixels'
              : undefined
          "
          @click="adoptPixelArt"
        >
          Import as Pixel Art
        </AppButton>
      </div>
      <p v-if="tooLargeForPixelArt && dimensions" class="pixel-art-limit">
        {{ dimensions.width }} × {{ dimensions.height }} is too large for exact
        pixel-art editing. The limit is 256 × 256.
      </p>
    </div>
  </section>
</template>
