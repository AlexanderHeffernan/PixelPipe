<script setup lang="ts">
import { PhCaretRight, PhFolder } from "@phosphor-icons/vue";
import { ref } from "vue";
import type { AssetTreeFolder } from "../workspace/asset-tree";
import { useWorkspace } from "../workspace/context";
import AssetBrowserFile from "./AssetBrowserFile.vue";

const props = defineProps<{ folder: AssetTreeFolder; level?: number }>();
const workspace = useWorkspace();
const open = ref(true);
const menuOpen = ref(false);
const moving = ref(false);
const destination = ref("");
function showMenu() {
  menuOpen.value = true;
  moving.value = false;
}
async function move() {
  if (!destination.value.trim() || destination.value === props.folder.path)
    return;
  await workspace.catalog.moveFolder(
    props.folder.path,
    destination.value.trim(),
  );
  menuOpen.value = false;
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
</script>

<template>
  <div
    class="browser-folder"
    role="treeitem"
    :aria-expanded="open"
    @contextmenu.prevent="showMenu"
    @keydown.shift.f10.prevent="showMenu"
  >
    <div
      class="browser-folder__heading"
      :style="{ '--tree-level': level || 0 }"
    >
      <button
        class="folder-toggle"
        :aria-label="`${open ? 'Collapse' : 'Expand'} ${folder.name}`"
        @click="open = !open"
      >
        <PhCaretRight :class="{ 'is-open': open }" aria-hidden="true" />
        <PhFolder aria-hidden="true" />
        <span>{{ folder.name }}</span>
      </button>
      <div
        v-if="menuOpen"
        class="asset-context-menu"
        role="menu"
        @mouseleave="menuOpen = false"
      >
        <template v-if="!moving">
          <button
            role="menuitem"
            @click="
              moving = true;
              destination = folder.path;
            "
          >
            Move or rename…
          </button>
          <button role="menuitem" class="danger" @click="remove">
            Delete empty folder…
          </button>
        </template>
        <form v-else @submit.prevent="move">
          <label>Project path<input v-model="destination" autofocus /></label>
          <div>
            <button type="button" @click="moving = false">Back</button
            ><button type="submit">Save</button>
          </div>
        </form>
      </div>
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
