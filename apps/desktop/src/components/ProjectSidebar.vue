<script setup lang="ts">
import { useWorkspace } from "../workspace/context";

const workspace = useWorkspace();
const label = (id: string) =>
  id.replaceAll("-", " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
</script>

<template>
  <aside class="project-sidebar">
    <header class="sidebar-heading">Assets</header>
    <nav class="asset-list" aria-label="Project assets">
      <button
        v-for="entry in workspace.project.value!.assets"
        :key="entry.asset.id"
        :aria-current="
          workspace.assetId.value === entry.asset.id ? 'page' : undefined
        "
        @click="workspace.selectAsset(entry.asset.id)"
      >
        <span class="asset-thumbnail checker">
          <img
            v-if="workspace.thumbnails.value[entry.asset.id]"
            :src="workspace.thumbnails.value[entry.asset.id]"
            alt=""
          />
          <svg v-else viewBox="0 0 24 24" aria-hidden="true">
            <path
              d="M6 4h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2Z"
            />
            <path d="m7 16 3-3 2 2 3-4 3 5" />
          </svg>
        </span>
        <span class="asset-name">{{ label(entry.asset.id) }}</span>
      </button>
      <p v-if="!workspace.project.value!.assets.length" class="sidebar-empty">
        No assets yet
      </p>
    </nav>
    <button
      class="create-asset-button"
      @click="workspace.createAssetOpen.value = true"
    >
      <span aria-hidden="true">＋</span> Create Asset
    </button>
  </aside>
</template>
