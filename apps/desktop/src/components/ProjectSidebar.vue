<script setup lang="ts">
import {
  PhDownloadSimple,
  PhFolderPlus,
  PhPlus,
  PhTrash,
} from "@phosphor-icons/vue";
import { nextTick, ref } from "vue";
import { useAssetTree } from "../workspace/asset-tree";
import { suggestedAssetId } from "../workspace/catalog-actions";
import { useWorkspace } from "../workspace/context";
import { useSidebarResize } from "../workspace/sidebar-resize";
import AssetBrowserFolder from "./AssetBrowserFolder.vue";

const workspace = useWorkspace();
const search = ref("");
const editing = ref("");
const editName = ref("");
const renameInput = ref<HTMLInputElement | HTMLInputElement[]>();
const { folders, drafts } = useAssetTree(workspace.project, search);
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
const displayName = (id: string) =>
  id.replaceAll("-", " ").replace(/\b\w/g, (letter) => letter.toUpperCase());

function newAsset() {
  const name = window.prompt("New asset name");
  if (!name?.trim()) return;
  const id = window.prompt("Stable asset ID", suggestedAssetId(name));
  if (id?.trim()) void workspace.catalog.createAsset(id.trim(), name.trim());
}
function newFolder() {
  const path = window.prompt("New project-relative folder path");
  if (path?.trim()) void workspace.catalog.createFolder(path.trim());
}
function adoptSelected() {
  const path = workspace.projectImagePath.value;
  const proposed = suggestedAssetId(path);
  const id = window.prompt("Stable asset ID", proposed);
  if (id?.trim())
    void workspace.catalog.adopt(path, id.trim(), displayName(id.trim()));
}
async function beginRename(id: string, name: string) {
  editing.value = id;
  editName.value = name;
  await nextTick();
  const input = Array.isArray(renameInput.value)
    ? renameInput.value[0]
    : renameInput.value;
  input?.select();
}
async function finishRename(id: string) {
  if (editing.value !== id) return;
  editing.value = "";
  if (editName.value.trim())
    await workspace.renameAsset(id, editName.value.trim());
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
          <button aria-label="New Asset" title="New Asset" @click="newAsset">
            <PhPlus /> Asset
          </button>
          <button aria-label="New Folder" title="New Folder" @click="newFolder">
            <PhFolderPlus /> Folder
          </button>
        </div>
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
        </div>
        <section
          v-if="drafts.length"
          class="drafts-section"
          aria-labelledby="drafts-heading"
        >
          <h3 id="drafts-heading">Drafts</h3>
          <div
            v-for="entry in drafts"
            :key="entry.asset.id"
            class="draft-row"
            :aria-current="
              workspace.assetId.value === entry.asset.id ? 'page' : undefined
            "
          >
            <button
              v-if="editing !== entry.asset.id"
              @click="workspace.selectAsset(entry.asset.id)"
            >
              <span class="asset-kind is-managed" />{{
                entry.asset.display_name || displayName(entry.asset.id)
              }}
            </button>
            <input
              v-else
              ref="renameInput"
              v-model="editName"
              aria-label="Asset name"
              @keydown.enter.prevent="finishRename(entry.asset.id)"
              @keydown.esc.prevent="editing = ''"
              @blur="finishRename(entry.asset.id)"
            />
            <button
              :aria-label="
                editing === entry.asset.id
                  ? `Close rename for ${entry.asset.display_name || displayName(entry.asset.id)}`
                  : `Rename ${entry.asset.display_name || displayName(entry.asset.id)}`
              "
              @pointerdown.prevent
              @click="
                editing === entry.asset.id
                  ? (editing = '')
                  : beginRename(
                      entry.asset.id,
                      entry.asset.display_name || displayName(entry.asset.id),
                    )
              "
            >
              {{ editing === entry.asset.id ? "Close" : "Rename" }}
            </button>
            <button
              :aria-label="`Delete Draft ${entry.asset.id}`"
              @click="workspace.deleteAsset(entry.asset.id)"
            >
              <PhTrash />
            </button>
          </div>
        </section>
        <p v-if="!folders.length && !drafts.length" class="sidebar-empty">
          No supported raster assets
        </p>
      </nav>
      <section
        v-if="workspace.projectImagePath.value"
        class="project-image-preview"
        aria-label="Project image preview"
      >
        <img
          :src="workspace.projectImagePreview.value"
          :alt="`${workspace.projectImagePath.value} preview`"
        />
        <p>{{ workspace.projectImagePath.value }}</p>
        <button @click="adoptSelected">Edit in Pixelate</button>
      </section>
      <button
        class="import-asset-button"
        :disabled="workspace.importing.value"
        @click="workspace.importAssets"
      >
        <PhDownloadSimple aria-hidden="true" /> Import Asset
      </button>
      <p class="git-folder-note">Empty folders are not retained by Git.</p>
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
