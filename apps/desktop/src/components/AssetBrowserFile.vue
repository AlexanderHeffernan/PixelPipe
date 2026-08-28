<script setup lang="ts">
import { PhImageSquare, PhWarningCircle } from "@phosphor-icons/vue";
import { ref } from "vue";
import type { AssetTreeFile } from "../workspace/asset-tree";
import { useWorkspace } from "../workspace/context";

const props = defineProps<{ file: AssetTreeFile; level: number }>();
const workspace = useWorkspace();
const menuOpen = ref(false);
const action = ref<"move" | "relink" | "rename" | "">("");
const value = ref("");
const selected = () =>
  props.file.managed
    ? workspace.assetId.value === props.file.managed.asset.id
    : workspace.projectImagePath.value === props.file.path;
const choose = () =>
  props.file.managed
    ? workspace.selectAsset(props.file.managed.asset.id)
    : workspace.selectProjectImage(props.file.path);

function openMenu() {
  menuOpen.value = true;
  action.value = "";
}
function begin(next: "move" | "relink" | "rename") {
  action.value = next;
  value.value = next === "rename" ? props.file.name : props.file.path;
}
async function submit() {
  const managed = props.file.managed;
  if (!managed || !value.value.trim()) return;
  if (action.value === "move")
    await workspace.catalog.moveAsset(managed.asset.id, value.value.trim());
  else if (action.value === "relink")
    await workspace.catalog.relink(managed.asset.id, value.value.trim());
  else await workspace.renameAsset(managed.asset.id, value.value.trim());
  menuOpen.value = false;
}
</script>

<template>
  <div
    class="browser-file"
    :class="{ 'is-project-file': !file.managed }"
    role="treeitem"
    :aria-current="selected() ? 'page' : undefined"
    :style="{ '--tree-level': level }"
    @contextmenu.prevent="openMenu"
    @keydown.shift.f10.prevent="openMenu"
  >
    <button class="browser-file__select" :title="file.path" @click="choose">
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
        <span class="asset-path">{{ file.path }}</span>
      </span>
      <span
        class="asset-kind"
        :class="file.managed ? 'is-managed' : 'is-project'"
        :aria-label="file.managed ? 'Pixelate managed' : 'Project image'"
        role="img"
      />
      <PhWarningCircle
        v-if="file.managed && file.catalog.status !== 'current'"
        class="asset-status"
        :aria-label="
          file.catalog.status === 'missing'
            ? 'Linked file missing'
            : file.catalog.status === 'modified'
              ? 'Linked file changed externally'
              : 'Not yet exported'
        "
      />
    </button>
    <div
      v-if="menuOpen"
      class="asset-context-menu"
      role="menu"
      @mouseleave="menuOpen = false"
    >
      <template v-if="file.managed && !action">
        <button role="menuitem" @click="begin('rename')">Rename</button>
        <button role="menuitem" @click="begin('move')">Move…</button>
        <button
          v-if="file.catalog.status === 'missing'"
          role="menuitem"
          @click="begin('relink')"
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
      </template>
      <button
        v-else-if="!file.managed"
        role="menuitem"
        @click="
          workspace.catalog.setIgnored(file.path, true);
          menuOpen = false;
        "
      >
        Hide from Assets
      </button>
      <form v-else @submit.prevent="submit">
        <label
          >{{ action === "rename" ? "Display name" : "Project path"
          }}<input v-model="value" autofocus
        /></label>
        <div>
          <button type="button" @click="action = ''">Back</button
          ><button type="submit">Save</button>
        </div>
      </form>
    </div>
  </div>
</template>
