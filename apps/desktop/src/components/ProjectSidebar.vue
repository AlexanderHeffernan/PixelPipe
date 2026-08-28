<script setup lang="ts">
import { PhFolder, PhFolderPlus, PhImageSquare } from "@phosphor-icons/vue";
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
const promotedFolders = ref<ReadonlySet<string>>(new Set());
const revealedFolder = ref("");
const { managedFolders, unmanagedFolders, managedRootFiles, projectRootFiles } =
  useAssetTree(workspace.project, search, promotedFolders);
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
  const path = folderPath.value.trim();
  if (!path) {
    addingFolder.value = false;
    return;
  }
  await workspace.catalog.createFolder(path);
  if (!workspace.project.value?.folders.includes(path)) return;
  promotedFolders.value = new Set([...promotedFolders.value, path]);
  revealedFolder.value = path;
  addingFolder.value = false;
  folderPath.value = "";
  await nextTick();
  const row = Array.from(
    document.querySelectorAll<HTMLElement>("[data-folder-path]"),
  ).find((entry) => entry.dataset.folderPath === path);
  row?.scrollIntoView?.({ block: "nearest" });
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
    if (item.image.includes("/"))
      void workspace.catalog.moveAsset(item.asset, basename(item.image));
  } else if (item.image && item.image.includes("/")) {
    void workspace.catalog.moveProjectImage(item.image, basename(item.image));
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
            <PhImageSquare /> Add Asset
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
            <strong>Convert a reference image…</strong>
            <span>Pixelize a smooth image into a new asset</span>
          </button>
          <button role="menuitem" @click="importPixelArt">
            <strong>Import finished pixel art…</strong>
            <span>Keep exact pixels and start in the editor</span>
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
            v-for="folder in managedFolders"
            :key="folder.path"
            :folder="folder"
            :force-open="Boolean(search)"
            :reveal-path="revealedFolder"
          />
          <AssetBrowserFile
            v-for="file in managedRootFiles"
            :key="file.path"
            :file="file"
            :level="0"
          />
          <AssetBrowserFolder
            v-for="folder in unmanagedFolders"
            :key="folder.path"
            :folder="folder"
            :force-open="Boolean(search)"
            :reveal-path="revealedFolder"
          />
          <AssetBrowserFile
            v-for="file in projectRootFiles"
            :key="file.path"
            :file="file"
            :level="0"
          />
        </div>
        <p
          v-if="
            !addingFolder &&
            !managedFolders.length &&
            !unmanagedFolders.length &&
            !managedRootFiles.length &&
            !projectRootFiles.length
          "
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
