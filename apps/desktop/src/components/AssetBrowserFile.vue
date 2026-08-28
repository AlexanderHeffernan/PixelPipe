<script setup lang="ts">
import { PhImageSquare } from "@phosphor-icons/vue";
import { nextTick, ref } from "vue";
import type { AssetTreeFile } from "../workspace/asset-tree";
import { beginFileDrag } from "../workspace/asset-drag";
import { useWorkspace } from "../workspace/context";
import AssetBrowserFileMenu from "./AssetBrowserFileMenu.vue";

const props = defineProps<{ file: AssetTreeFile; level: number }>();
const workspace = useWorkspace();
const menuOpen = ref(false);
const renaming = ref(false);
const value = ref("");
const renameInput = ref<HTMLInputElement>();
const selected = () =>
  props.file.managed
    ? workspace.assetId.value === props.file.managed.asset.id
    : workspace.projectImagePath.value === props.file.path;

function choose() {
  if (selected()) {
    if (props.file.managed) workspace.clearAsset();
    else workspace.clearProjectImage();
    return;
  }
  if (props.file.managed)
    void workspace.selectAsset(props.file.managed.asset.id);
  else void workspace.selectProjectImage(props.file.path);
}

function openMenu() {
  menuOpen.value = true;
}

function beginRename() {
  menuOpen.value = false;
  renaming.value = true;
  value.value = props.file.name;
  void nextTick(() => renameInput.value?.select());
}

async function rename() {
  if (!props.file.managed || !value.value.trim()) {
    renaming.value = false;
    return;
  }
  await workspace.renameAsset(props.file.managed.asset.id, value.value.trim());
  renaming.value = false;
}
</script>

<template>
  <div
    class="browser-file"
    :class="{ 'is-project-file': !file.managed }"
    role="treeitem"
    :aria-current="selected() ? 'page' : undefined"
    :style="{ '--tree-level': level }"
    :draggable="!renaming"
    @dragstart.stop="beginFileDrag($event, file.path, file.managed?.asset.id)"
    @contextmenu.stop.prevent="openMenu"
    @keydown.shift.f10.stop.prevent="openMenu"
  >
    <form
      v-if="renaming"
      class="browser-file__select browser-file__rename"
      :title="file.path"
      @submit.prevent="rename"
    >
      <span class="asset-thumbnail checker">
        <img
          v-if="workspace.thumbnails.value[file.managed!.asset.id]"
          :src="workspace.thumbnails.value[file.managed!.asset.id]"
          alt=""
        />
        <PhImageSquare v-else aria-hidden="true" />
      </span>
      <span class="browser-file__label">
        <input
          ref="renameInput"
          v-model="value"
          aria-label="Rename asset"
          @keydown.escape.prevent="renaming = false"
          @blur="rename"
        />
        <span
          v-if="
            file.catalog.status === 'current' ||
            file.catalog.status === 'modified'
          "
          class="asset-path"
        >
          {{ file.path }}
        </span>
      </span>
      <span
        v-if="file.catalog.status === 'modified'"
        class="asset-status-label"
      >
        Changed
      </span>
    </form>
    <button
      v-else
      class="browser-file__select"
      :title="file.path"
      @click="choose"
    >
      <span v-if="file.managed" class="asset-thumbnail checker">
        <img
          v-if="workspace.thumbnails.value[file.managed.asset.id]"
          :src="workspace.thumbnails.value[file.managed.asset.id]"
          alt=""
        />
        <PhImageSquare v-else aria-hidden="true" />
      </span>
      <span class="browser-file__label">
        <span class="asset-name">{{ file.name }}</span>
        <span
          v-if="
            file.managed &&
            (file.catalog.status === 'current' ||
              file.catalog.status === 'modified')
          "
          class="asset-path"
        >
          {{ file.path }}
        </span>
      </span>
      <span
        v-if="file.managed && file.catalog.status === 'modified'"
        class="asset-status-label"
      >
        Changed
      </span>
    </button>
    <AssetBrowserFileMenu
      v-if="menuOpen"
      :file="file"
      @close="menuOpen = false"
      @rename="beginRename"
    />
  </div>
</template>
