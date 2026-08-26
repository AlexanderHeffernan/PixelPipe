<script setup lang="ts">
import {
  PhDownloadSimple,
  PhImageSquare,
  PhPencilSimple,
  PhTrash,
  PhX,
} from "@phosphor-icons/vue";
import { nextTick, ref } from "vue";
import { useWorkspace } from "../workspace/context";
import { useSidebarResize } from "../workspace/sidebar-resize";

const workspace = useWorkspace();
const editing = ref("");
const editName = ref("");
const renameInput = ref<HTMLInputElement | HTMLInputElement[]>();
const MIN_WIDTH = 220;
const MAX_WIDTH = 380;
const { width, isResizing, startResize, resizeWithKeyboard } = useSidebarResize(
  {
    edge: "left",
    initialWidth: 256,
    minWidth: MIN_WIDTH,
    maxWidth: MAX_WIDTH,
  },
);
const label = (id: string) =>
  id.replaceAll("-", " ").replace(/\b\w/g, (letter) => letter.toUpperCase());

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
  if (!editing.value) return;
  const name = editName.value.trim();
  editing.value = "";
  if (name) await workspace.renameAsset(id, name);
}

function toggleRename(id: string, name: string) {
  if (editing.value === id) {
    editing.value = "";
    return;
  }
  void beginRename(id, name);
}
</script>

<template>
  <aside
    class="project-sidebar"
    :class="{ 'is-resizing': isResizing }"
    :style="{ '--sidebar-width': `${width}px` }"
  >
    <div class="project-sidebar__body">
      <header class="sidebar-heading">Assets</header>
      <nav class="asset-list" aria-label="Project assets">
        <div
          v-for="entry in workspace.project.value!.assets"
          :key="entry.asset.id"
          :aria-current="
            workspace.assetId.value === entry.asset.id ? 'page' : undefined
          "
          class="asset-row"
          :class="{ 'is-renaming': editing === entry.asset.id }"
        >
          <button
            class="asset-select"
            @click="workspace.selectAsset(entry.asset.id)"
          >
            <span class="asset-thumbnail checker">
              <img
                v-if="workspace.thumbnails.value[entry.asset.id]"
                :src="workspace.thumbnails.value[entry.asset.id]"
                alt=""
              />
              <PhImageSquare v-else aria-hidden="true" />
            </span>
            <input
              v-if="editing === entry.asset.id"
              ref="renameInput"
              v-model="editName"
              class="asset-rename-input"
              aria-label="Asset name"
              @click.stop
              @keydown.enter.prevent="finishRename(entry.asset.id)"
              @keydown.esc.prevent="editing = ''"
              @blur="finishRename(entry.asset.id)"
            />
            <span v-else class="asset-name">{{
              entry.asset.display_name || label(entry.asset.id)
            }}</span>
          </button>
          <button
            class="asset-rename"
            :aria-label="
              editing === entry.asset.id
                ? `Close rename for ${entry.asset.display_name || label(entry.asset.id)}`
                : `Rename ${entry.asset.display_name || label(entry.asset.id)}`
            "
            :title="
              editing === entry.asset.id ? 'Close rename' : 'Rename asset'
            "
            @pointerdown.prevent
            @click="
              toggleRename(
                entry.asset.id,
                entry.asset.display_name || label(entry.asset.id),
              )
            "
          >
            <PhX v-if="editing === entry.asset.id" />
            <PhPencilSimple v-else />
          </button>
          <button
            class="asset-delete"
            :aria-label="`Delete ${label(entry.asset.id)}`"
            title="Delete asset"
            @click="workspace.deleteAsset(entry.asset.id)"
          >
            <PhTrash weight="regular" />
          </button>
        </div>
        <p v-if="!workspace.project.value!.assets.length" class="sidebar-empty">
          No assets yet
        </p>
      </nav>
      <button
        class="create-asset-button"
        :disabled="workspace.importing.value"
        @click="workspace.importAssets"
      >
        <PhDownloadSimple aria-hidden="true" /> Import Asset
      </button>
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
