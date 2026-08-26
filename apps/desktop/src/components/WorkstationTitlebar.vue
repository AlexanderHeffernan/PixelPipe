<script setup lang="ts">
import { PhCaretDown, PhSidebarSimple } from "@phosphor-icons/vue";
import { computed } from "vue";
import { useWorkspace } from "../workspace/context";
import { useWindowFullscreen } from "../workspace/window-fullscreen";

const workspace = useWorkspace();
const fullscreen = useWindowFullscreen();
const leadingInset = computed(() => (fullscreen.value ? 12 : 82));
</script>

<template>
  <header class="window-drag-region" data-tauri-drag-region></header>
  <div
    v-if="workspace.project.value"
    class="window-controls window-controls--leading"
    :style="{ left: `${leadingInset}px` }"
  >
    <button
      class="icon-button sidebar-toggle"
      :aria-label="
        workspace.leftSidebarOpen.value
          ? 'Hide asset sidebar'
          : 'Show asset sidebar'
      "
      :aria-pressed="workspace.leftSidebarOpen.value"
      @click="
        workspace.leftSidebarOpen.value = !workspace.leftSidebarOpen.value
      "
    >
      <PhSidebarSimple />
    </button>
    <button
      v-if="workspace.leftSidebarOpen.value"
      class="project-menu"
      @click="workspace.chooseProject"
    >
      <strong>{{ workspace.project.value.project.name }}</strong>
      <PhCaretDown />
    </button>
  </div>
  <div
    v-if="workspace.project.value"
    class="window-controls window-controls--trailing"
  >
    <button
      class="icon-button"
      :aria-label="
        workspace.rightSidebarOpen.value ? 'Hide inspector' : 'Show inspector'
      "
      :aria-pressed="workspace.rightSidebarOpen.value"
      @click="
        workspace.rightSidebarOpen.value = !workspace.rightSidebarOpen.value
      "
    >
      <PhSidebarSimple class="right-sidebar-icon" />
    </button>
  </div>
</template>
