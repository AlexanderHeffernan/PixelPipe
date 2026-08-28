<script setup lang="ts">
import { PhCaretRight, PhFolder } from "@phosphor-icons/vue";
import { nextTick, ref } from "vue";
import type { AssetTreeFolder } from "../workspace/asset-tree";
import {
  acceptsAssetDrop,
  basename,
  beginFolderDrag,
  droppedItem,
} from "../workspace/asset-drag";
import { useWorkspace } from "../workspace/context";
import AssetBrowserFile from "./AssetBrowserFile.vue";
import PopupMenu from "./PopupMenu.vue";

const props = defineProps<{ folder: AssetTreeFolder; level?: number }>();
const workspace = useWorkspace();
const open = ref(true);
const menuOpen = ref(false);
const renaming = ref(false);
const value = ref("");
const renameInput = ref<HTMLInputElement>();

function beginRename() {
  menuOpen.value = false;
  renaming.value = true;
  value.value = props.folder.name;
  void nextTick(() => renameInput.value?.select());
}

async function rename() {
  if (!value.value.trim()) {
    renaming.value = false;
    return;
  }
  const parent = props.folder.path.includes("/")
    ? props.folder.path.slice(0, props.folder.path.lastIndexOf("/") + 1)
    : "";
  await workspace.catalog.moveFolder(
    props.folder.path,
    `${parent}${value.value.trim()}`,
  );
  renaming.value = false;
}

function remove() {
  if (
    window.confirm(
      `Delete empty folder “${props.folder.path}”? Non-empty folders are always refused.`,
    )
  )
    void workspace.catalog.deleteFolder(props.folder.path);
  menuOpen.value = false;
}

function drop(event: DragEvent) {
  if (!acceptsAssetDrop(event)) return;
  event.preventDefault();
  const item = droppedItem(event);
  if (item.asset) {
    const file =
      workspace.project.value?.assets.find(
        ({ asset }) => asset.id === item.asset,
      )?.asset.project_path ||
      workspace.project.value?.catalog.find(
        ({ asset_id }) => asset_id === item.asset,
      )?.path ||
      `${item.asset}.png`;
    if (file)
      void workspace.catalog.moveAsset(
        item.asset,
        `${props.folder.path}/${basename(file)}`,
      );
  } else if (item.folder && item.folder !== props.folder.path) {
    void workspace.catalog.moveFolder(
      item.folder,
      `${props.folder.path}/${basename(item.folder)}`,
    );
  }
}
</script>

<template>
  <div
    class="browser-folder"
    role="treeitem"
    :aria-expanded="open"
    draggable="true"
    @dragstart.stop="beginFolderDrag($event, folder.path)"
    @dragover.prevent=""
    @drop.stop="drop"
    @contextmenu.prevent="menuOpen = true"
    @keydown.shift.f10.prevent="menuOpen = true"
  >
    <div
      class="browser-folder__heading"
      :style="{ '--tree-level': level || 0 }"
    >
      <form
        v-if="renaming"
        class="browser-inline-rename"
        @submit.prevent="rename"
      >
        <input
          ref="renameInput"
          v-model="value"
          aria-label="Rename folder"
          @keydown.escape.prevent="renaming = false"
          @blur="rename"
        />
      </form>
      <button
        v-else
        class="folder-toggle"
        :aria-label="`${open ? 'Collapse' : 'Expand'} ${folder.name}`"
        @click="open = !open"
      >
        <PhCaretRight :class="{ 'is-open': open }" aria-hidden="true" />
        <PhFolder aria-hidden="true" />
        <span>{{ folder.name }}</span>
      </button>
      <PopupMenu
        v-if="menuOpen"
        class="asset-context-menu"
        @close="menuOpen = false"
      >
        <button role="menuitem" @click="beginRename">Rename</button>
        <button role="menuitem" class="danger" @click="remove">
          Delete empty folder…
        </button>
      </PopupMenu>
    </div>
    <div v-show="open" role="group">
      <AssetBrowserFolder
        v-for="child in folder.folders"
        :key="child.path"
        :folder="child"
        :level="(level || 0) + 1"
      />
      <AssetBrowserFile
        v-for="file in folder.files"
        :key="file.path"
        :file="file"
        :level="(level || 0) + 1"
      />
    </div>
  </div>
</template>
