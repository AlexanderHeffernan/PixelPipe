<script setup lang="ts">
import { ref } from "vue";
import type { AssetTreeFile } from "../workspace/asset-tree";
import { useWorkspace } from "../workspace/context";
import PopupMenu from "./PopupMenu.vue";

const props = defineProps<{ file: AssetTreeFile }>();
const emit = defineEmits<{ close: []; rename: [] }>();
const workspace = useWorkspace();
const relinking = ref(false);
const path = ref(props.file.path);

function close() {
  emit("close");
}

function reveal() {
  void workspace.catalog.revealProjectImage(props.file.path);
  close();
}

async function deleteFile() {
  if (
    !window.confirm(
      `Permanently delete “${props.file.path}” from the project? Pixelate history is retained. This cannot be undone.`,
    )
  )
    return;
  await workspace.catalog.deleteProjectImage(props.file.path);
  if (!props.file.managed) workspace.clearProjectImage();
  close();
}
</script>

<template>
  <PopupMenu class="asset-context-menu" @close="close">
    <template v-if="file.managed && !relinking">
      <button
        v-if="
          file.catalog.status === 'current' ||
          file.catalog.status === 'modified'
        "
        role="menuitem"
        @click="reveal"
      >
        Show in Finder
      </button>
      <button role="menuitem" @click="$emit('rename')">Rename</button>
      <button
        v-if="file.catalog.status === 'missing'"
        role="menuitem"
        @click="relinking = true"
      >
        Relink…
      </button>
      <button
        v-if="file.catalog.status === 'modified'"
        role="menuitem"
        @click="
          workspace.catalog.updateLinkedSource(file.managed.asset.id);
          close();
        "
      >
        Import external changes
      </button>
      <button
        role="menuitem"
        class="danger"
        @click="
          workspace.deleteAsset(file.managed.asset.id);
          close();
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
      <button role="menuitem" @click="reveal">Show in Finder</button>
      <button
        role="menuitem"
        @click="
          workspace.catalog.setIgnored(file.path, true);
          close();
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
        workspace.catalog.relink(file.managed!.asset.id, path.trim());
        close();
      "
    >
      <label>Project path<input v-model="path" autofocus /></label>
      <div>
        <button type="button" @click="relinking = false">Back</button>
        <button type="submit">Relink</button>
      </div>
    </form>
  </PopupMenu>
</template>
