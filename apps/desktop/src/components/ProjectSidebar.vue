<script setup lang="ts">
import { PhFolder, PhFolderPlus, PhPlus } from "@phosphor-icons/vue";
import { nextTick, ref } from "vue";
import {
  acceptsAssetDrop,
  basename,
  droppedItem,
} from "../workspace/asset-drag";
import { useAssetTree } from "../workspace/asset-tree";
import { useWorkspace } from "../workspace/context";
import { useSidebarResize } from "../workspace/sidebar-resize";
import AppButton from "./AppButton.vue";
import AssetBrowserFolder from "./AssetBrowserFolder.vue";
import AssetBrowserFile from "./AssetBrowserFile.vue";
import PopupMenu from "./PopupMenu.vue";

const workspace = useWorkspace();
const search = ref("");
const addMenu = ref(false);
const addingFolder = ref(false);
const folderPath = ref("");
const folderInput = ref<HTMLInputElement>();
const { folders, rootFiles } = useAssetTree(workspace.project, search);
const MIN_WIDTH = 240;
const MAX_WIDTH = 420;
const { width, isResizing, startResize, resizeWithKeyboard } = useSidebarResize(
  {
    edge: "left",
    initialWidth: 280,
    minWidth: MIN_WIDTH,
    maxWidth: MAX_WIDTH,
  },
);

async function createFolder() {
  if (!folderPath.value.trim()) {
    addingFolder.value = false;
    return;
  }
  await workspace.catalog.createFolder(folderPath.value.trim());
  addingFolder.value = false;
  folderPath.value = "";
}

function beginFolder() {
  addMenu.value = false;
  addingFolder.value = true;
  folderPath.value = "";
  void nextTick(() => folderInput.value?.focus());
}

function importReference() {
  addMenu.value = false;
  void workspace.importAssets();
}

function importPixelArt() {
  addMenu.value = false;
  void workspace.importPixelArt();
}

function dropAtRoot(event: DragEvent) {
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
    if (file && file.includes("/"))
      void workspace.catalog.moveAsset(item.asset, basename(file));
  } else if (item.folder && item.folder.includes("/")) {
    void workspace.catalog.moveFolder(item.folder, basename(item.folder));
  }
}
</script>

<template>
  <aside
    class="project-sidebar"
    :class="{ 'is-resizing': isResizing }"
    :style="{ '--sidebar-width': `${width}px` }"
  >
    <div class="project-sidebar__body">
      <header class="asset-browser-header">
        <h2>Assets</h2>
        <div class="asset-browser-actions">
          <AppButton
            size="small"
            aria-label="Add Asset"
            title="Add Asset"
            @pointerdown.stop
            @click="addMenu = !addMenu"
          >
            <PhPlus /> Add Asset
          </AppButton>
          <AppButton
            size="small"
            aria-label="Add Folder"
            title="Add Folder"
            @click="beginFolder"
          >
            <PhFolderPlus /> Add Folder
          </AppButton>
        </div>
        <PopupMenu
          v-if="addMenu"
          class="asset-add-menu"
          @close="addMenu = false"
        >
          <button role="menuitem" @click="importReference">
            Reference image…
          </button>
          <button role="menuitem" @click="importPixelArt">
            Pixel art in project…
          </button>
        </PopupMenu>
        <input
          v-model="search"
          type="search"
          aria-label="Search assets"
          placeholder="Search names and paths"
        />
      </header>
      <nav class="asset-tree" aria-label="Project assets">
        <div
          role="tree"
          aria-label="Project image folders"
          @dragover.prevent
          @drop="dropAtRoot"
        >
          <form
            v-if="addingFolder"
            class="browser-new-folder"
            role="treeitem"
            @submit.prevent="createFolder"
          >
            <PhFolder aria-hidden="true" />
            <input
              ref="folderInput"
              v-model="folderPath"
              aria-label="New folder name"
              placeholder="Folder name"
              @keydown.escape.prevent="addingFolder = false"
              @blur="folderPath.trim() ? undefined : (addingFolder = false)"
            />
          </form>
          <AssetBrowserFolder
            v-for="folder in folders"
            :key="folder.path"
            :folder="folder"
          />
          <AssetBrowserFile
            v-for="file in rootFiles"
            :key="file.path"
            :file="file"
            :level="0"
          />
        </div>
        <p
          v-if="!addingFolder && !folders.length && !rootFiles.length"
          class="sidebar-empty"
        >
          No supported raster assets
        </p>
        <details
          v-if="workspace.project.value?.project.ignored_project_images.length"
          class="hidden-images"
        >
          <summary>
            Hidden images ({{
              workspace.project.value.project.ignored_project_images.length
            }})
          </summary>
          <div
            v-for="path in workspace.project.value.project
              .ignored_project_images"
            :key="path"
          >
            <span :title="path">{{ path }}</span>
            <AppButton
              size="small"
              :aria-label="`Restore ${path}`"
              @click="workspace.catalog.setIgnored(path, false)"
            >
              Restore
            </AppButton>
          </div>
        </details>
      </nav>
    </div>
    <div
      class="sidebar-resize-handle sidebar-resize-handle--right"
      role="separator"
      aria-label="Resize asset sidebar"
      aria-orientation="vertical"
      :aria-valuemin="MIN_WIDTH"
      :aria-valuemax="MAX_WIDTH"
      :aria-valuenow="width"
      tabindex="0"
      @pointerdown="startResize"
      @keydown="resizeWithKeyboard"
    />
  </aside>
</template>
