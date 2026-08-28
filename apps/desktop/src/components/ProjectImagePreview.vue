<script setup lang="ts">
import { PhFolderOpen } from "@phosphor-icons/vue";
import { computed } from "vue";
import { suggestedAssetId } from "../workspace/catalog-actions";
import { useWorkspace } from "../workspace/context";
import AppButton from "./AppButton.vue";

const workspace = useWorkspace();
const path = computed(() => workspace.projectImagePath.value);

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
  if (!workspace.projectImagePixelArtImportable.value) return;
  const selected = identity();
  await workspace.catalog.adoptPixelArt(path.value, selected.id, selected.name);
}
</script>

<template>
  <section class="project-image-stage" aria-labelledby="project-image-title">
    <div class="project-image-stage__preview checker">
      <img
        :src="workspace.projectImagePreview.value"
        :alt="`${path} preview`"
      />
    </div>
    <div class="project-image-stage__details">
      <h1 id="project-image-title" :title="path.split('/').at(-1)">
        {{ path.split("/").at(-1) }}
      </h1>
      <div class="project-image-stage__actions">
        <AppButton
          variant="quiet"
          :disabled="workspace.busy.value"
          @click="workspace.catalog.revealProjectImage(path)"
        >
          <PhFolderOpen aria-hidden="true" />
          Show in Finder
        </AppButton>
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
          v-if="workspace.projectImagePixelArtImportable.value"
          variant="primary"
          :disabled="workspace.busy.value"
          @click="adoptPixelArt"
        >
          Import as Pixel Art
        </AppButton>
      </div>
    </div>
  </section>
</template>
