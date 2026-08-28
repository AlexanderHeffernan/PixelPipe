<script setup lang="ts">
import { PhCaretRight, PhFolder } from "@phosphor-icons/vue";
import { ref } from "vue";
import type { AssetTreeFolder } from "../workspace/asset-tree";
import { useWorkspace } from "../workspace/context";
import AssetBrowserFile from "./AssetBrowserFile.vue";

const props = defineProps<{ folder: AssetTreeFolder; level?: number }>();
const workspace = useWorkspace();
const open = ref(true);
const move = () => {
  const destination = window.prompt(
    "Move or rename folder to project-relative path",
    props.folder.path,
  );
  if (destination && destination !== props.folder.path)
    void workspace.catalog.moveFolder(props.folder.path, destination);
};
const remove = () => {
  if (
    window.confirm(
      `Delete empty folder “${props.folder.path}”? Non-empty folders are always refused.`,
    )
  )
    void workspace.catalog.deleteFolder(props.folder.path);
};
</script>

<template>
  <div class="browser-folder" role="treeitem" :aria-expanded="open">
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
      <button :aria-label="`Move or rename ${folder.name}`" @click="move">
        Move
      </button>
      <button
        :aria-label="`Delete empty folder ${folder.name}`"
        @click="remove"
      >
        Delete
      </button>
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
