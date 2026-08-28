<script setup lang="ts">
import { PhImageSquare, PhWarningCircle } from "@phosphor-icons/vue";
import type { AssetTreeFile } from "../workspace/asset-tree";
import { useWorkspace } from "../workspace/context";

const props = defineProps<{ file: AssetTreeFile; level: number }>();
const workspace = useWorkspace();
const selected = () =>
  props.file.managed
    ? workspace.assetId.value === props.file.managed.asset.id
    : workspace.projectImagePath.value === props.file.path;
const choose = () =>
  props.file.managed
    ? workspace.selectAsset(props.file.managed.asset.id)
    : workspace.selectProjectImage(props.file.path);
const move = () => {
  if (!props.file.managed) return;
  const destination = window.prompt(
    "Move asset to project-relative image path",
    props.file.path,
  );
  if (destination && destination !== props.file.path)
    void workspace.catalog.moveAsset(props.file.managed.asset.id, destination);
};
const relink = () => {
  if (!props.file.managed) return;
  const path = window.prompt(
    "Relink to project-relative image path",
    props.file.path,
  );
  if (path) void workspace.catalog.relink(props.file.managed.asset.id, path);
};
</script>

<template>
  <div
    class="browser-file"
    role="treeitem"
    :aria-current="selected() ? 'page' : undefined"
    :style="{ '--tree-level': level }"
  >
    <button class="browser-file__select" :title="file.path" @click="choose">
      <span class="asset-thumbnail checker">
        <img
          v-if="
            file.managed && workspace.thumbnails.value[file.managed.asset.id]
          "
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
        v-if="file.catalog.status !== 'current'"
        class="asset-status"
        :aria-label="
          file.catalog.status === 'missing'
            ? 'Linked file missing'
            : 'Linked file changed externally'
        "
      />
    </button>
    <div v-if="file.managed" class="browser-file__actions">
      <button
        v-if="file.catalog.status === 'modified'"
        :aria-label="`Import external changes for ${file.name}`"
        title="Import external changes"
        @click="workspace.catalog.updateLinkedSource(file.managed.asset.id)"
      >
        Update
      </button>
      <button
        v-if="file.catalog.status === 'missing'"
        :aria-label="`Relink ${file.name}`"
        @click="relink"
      >
        Relink
      </button>
      <button :aria-label="`Move ${file.name}`" @click="move">Move</button>
      <button
        :aria-label="`Remove ${file.name} from Pixelate`"
        @click="workspace.deleteAsset(file.managed.asset.id)"
      >
        Remove
      </button>
    </div>
  </div>
</template>
