<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { suggestedAssetId } from "../workspace/catalog-actions";
import { useWorkspace } from "../workspace/context";

const workspace = useWorkspace();
const choice = ref<"reference" | "pixel" | "">("");
const name = ref("");
const id = ref("");
const destination = ref("");
const path = computed(() => workspace.projectImagePath.value);

watch(
  path,
  (value) => {
    const filename = value.split("/").at(-1) || value;
    const stem = filename.replace(/\.[^.]+$/, "");
    name.value = stem.replaceAll(/[-_]+/g, " ");
    id.value = suggestedAssetId(value);
    const folder = value.includes("/")
      ? `${value.slice(0, value.lastIndexOf("/") + 1)}`
      : "";
    destination.value = `${folder}${stem}-pixel.png`;
    choice.value = "";
  },
  { immediate: true },
);

async function hide() {
  const selected = path.value;
  await workspace.catalog.setIgnored(selected, true);
  workspace.clearProjectImage();
}

async function submit() {
  if (!id.value.trim() || !name.value.trim()) return;
  if (choice.value === "reference") {
    if (!destination.value.trim()) return;
    await workspace.catalog.adoptReference(
      path.value,
      id.value.trim(),
      name.value.trim(),
      destination.value.trim(),
    );
  } else {
    await workspace.catalog.adoptPixelArt(
      path.value,
      id.value.trim(),
      name.value.trim(),
    );
  }
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
      <h1 id="project-image-title">{{ path.split("/").at(-1) }}</h1>
      <p>{{ path }}</p>
      <div class="project-image-stage__actions">
        <button class="secondary" @click="hide">Hide from Assets</button>
        <button class="primary" @click="choice = 'reference'">
          Use as Reference
        </button>
        <button class="primary" @click="choice = 'pixel'">
          Import as Pixel Art
        </button>
      </div>
      <form v-if="choice" class="project-image-import" @submit.prevent="submit">
        <h2>
          {{
            choice === "reference" ? "Reference details" : "Pixel art details"
          }}
        </h2>
        <label>Display name <input v-model="name" required /></label>
        <label
          >Stable asset ID
          <input v-model="id" required pattern="[a-z0-9][a-z0-9-]*"
        /></label>
        <label v-if="choice === 'reference'">
          Pixel art output path
          <input v-model="destination" required />
        </label>
        <div>
          <button type="button" @click="choice = ''">Cancel</button>
          <button class="primary" type="submit">
            {{
              choice === "reference" ? "Import Reference" : "Import Pixel Art"
            }}
          </button>
        </div>
      </form>
    </div>
  </section>
</template>
