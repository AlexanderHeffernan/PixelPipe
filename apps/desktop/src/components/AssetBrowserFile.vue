<script setup lang="ts">
import { PhImageSquare } from "@phosphor-icons/vue";
import { nextTick, ref } from "vue";
import type { AssetTreeFile } from "../workspace/asset-tree";
import { beginFileDrag } from "../workspace/asset-drag";
import { useWorkspace } from "../workspace/context";
import PopupMenu from "./PopupMenu.vue";

const props = defineProps<{ file: AssetTreeFile; level: number }>();
const workspace = useWorkspace();
const menuOpen = ref(false);
const relinking = ref(false);
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
  relinking.value = false;
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

async function deleteFile() {
  menuOpen.value = false;
  if (
    !window.confirm(
      `Permanently delete “${props.file.path}” from the project? Pixelate history is retained. This cannot be undone.`,
    )
  )
    return;
  await workspace.catalog.deleteProjectImage(props.file.path);
  if (!props.file.managed) workspace.clearProjectImage();
}
</script>

<template>
  <div
    class="browser-file"
    :class="{ 'is-project-file': !file.managed }"
    role="treeitem"
    :aria-current="selected() ? 'page' : undefined"
    :style="{ '--tree-level': level }"
    draggable="true"
    @dragstart.stop="beginFileDrag($event, file.path, file.managed?.asset.id)"
    @contextmenu.stop.prevent="openMenu"
    @keydown.shift.f10.stop.prevent="openMenu"
  >
    <form
      v-if="renaming"
      class="browser-inline-rename"
      @submit.prevent="rename"
    >
      <input
        ref="renameInput"
        v-model="value"
        aria-label="Rename asset"
        @keydown.escape.prevent="renaming = false"
        @blur="rename"
      />
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
          v-if="!file.managed || file.catalog.status !== 'unexported'"
          class="asset-path"
        >
          {{ file.path }}
        </span>
      </span>
      <span
        v-if="
          file.managed &&
          (file.catalog.status === 'missing' ||
            file.catalog.status === 'modified')
        "
        class="asset-status-label"
      >
        {{ file.catalog.status === "missing" ? "Missing" : "Changed" }}
      </span>
    </button>
    <PopupMenu
      v-if="menuOpen"
      class="asset-context-menu"
      @close="menuOpen = false"
    >
      <template v-if="file.managed && !relinking">
        <button role="menuitem" @click="beginRename">Rename</button>
        <button
          v-if="file.catalog.status === 'missing'"
          role="menuitem"
          @click="
            relinking = true;
            value = file.path;
          "
        >
          Relink…
        </button>
        <button
          v-if="file.catalog.status === 'modified'"
          role="menuitem"
          @click="
            workspace.catalog.updateLinkedSource(file.managed.asset.id);
            menuOpen = false;
          "
        >
          Import external changes
        </button>
        <button
          role="menuitem"
          class="danger"
          @click="
            workspace.deleteAsset(file.managed.asset.id);
            menuOpen = false;
          "
        >
          Remove from Pixelate…
        </button>
        <button
          v-if="
            file.catalog.status === 'current' ||
            file.catalog.status === 'modified'
          "
          role="menuitem"
          class="danger"
          @click="deleteFile"
        >
          Delete file…
        </button>
      </template>
      <template v-else-if="!file.managed">
        <button
          role="menuitem"
          @click="
            workspace.catalog.setIgnored(file.path, true);
            menuOpen = false;
          "
        >
          Hide from Assets
        </button>
        <button role="menuitem" class="danger" @click="deleteFile">
          Delete file…
        </button>
      </template>
      <form
        v-else
        @submit.prevent="
          workspace.catalog.relink(file.managed!.asset.id, value.trim());
          menuOpen = false;
        "
      >
        <label>Project path<input v-model="value" autofocus /></label>
        <div>
          <button type="button" @click="relinking = false">Back</button>
          <button type="submit">Relink</button>
        </div>
      </form>
    </PopupMenu>
  </div>
</template>
