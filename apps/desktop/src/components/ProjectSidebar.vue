<script setup lang="ts">
import { PhFolderPlus, PhPlus } from "@phosphor-icons/vue";
import { ref } from "vue";
import { useAssetTree } from "../workspace/asset-tree";
import { useWorkspace } from "../workspace/context";
import { useSidebarResize } from "../workspace/sidebar-resize";
import AssetBrowserFolder from "./AssetBrowserFolder.vue";
import AssetBrowserFile from "./AssetBrowserFile.vue";

const workspace = useWorkspace();
const search = ref("");
const addMenu = ref(false);
const folderForm = ref(false);
const folderPath = ref("");
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

function createFolder() {
  if (!folderPath.value.trim()) return;
  void workspace.catalog.createFolder(folderPath.value.trim());
  folderForm.value = false;
  folderPath.value = "";
}
function importReference() {
  addMenu.value = false;
  void workspace.importAssets();
}
function importPixelArt() {
  addMenu.value = false;
  void workspace.importPixelArt();
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
          <button
            aria-label="New Asset"
            title="New Asset"
            @click="
              addMenu = !addMenu;
              folderForm = false;
            "
          >
            <PhPlus /> Asset
          </button>
          <button
            aria-label="New Folder"
            title="New Folder"
            @click="
              folderForm = !folderForm;
              addMenu = false;
            "
          >
            <PhFolderPlus /> Folder
          </button>
        </div>
        <div v-if="addMenu" class="asset-add-menu" role="menu">
          <button role="menuitem" @click="importReference">
            Reference image…
          </button>
          <button role="menuitem" @click="importPixelArt">
            Pixel art in project…
          </button>
        </div>
        <form
          v-if="folderForm"
          class="asset-inline-form"
          @submit.prevent="createFolder"
        >
          <label
            >Project-relative folder
            <input v-model="folderPath" autofocus required
          /></label>
          <div>
            <button type="button" @click="folderForm = false">Cancel</button
            ><button type="submit">Create</button>
          </div>
        </form>
        <input
          v-model="search"
          type="search"
          aria-label="Search assets"
          placeholder="Search names and paths"
        />
      </header>
      <nav class="asset-tree" aria-label="Project assets">
        <div role="tree" aria-label="Project image folders">
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
        <p v-if="!folders.length && !rootFiles.length" class="sidebar-empty">
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
            <button
              :aria-label="`Restore ${path}`"
              @click="workspace.catalog.setIgnored(path, false)"
            >
              Restore
            </button>
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
