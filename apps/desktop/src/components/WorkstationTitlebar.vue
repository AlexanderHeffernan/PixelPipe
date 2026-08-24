<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useWorkspace } from "../workspace/context";

const workspace = useWorkspace();

function startWindowDrag(event: MouseEvent) {
  if (event.button !== 0) return;
  const target = event.target as HTMLElement;
  if (target.closest("button, input, select, textarea, a")) return;
  void getCurrentWindow().startDragging();
}
</script>

<template>
  <header class="titlebar" @mousedown="startWindowDrag">
    <div class="titlebar-leading">
      <button
        v-if="workspace.project.value"
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
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <rect x="2" y="2.5" width="12" height="11" rx="2" />
          <path d="M6 3v10" />
        </svg>
      </button>
      <button
        v-if="workspace.project.value"
        class="project-menu"
        @click="workspace.chooseProject"
      >
        <strong>{{ workspace.project.value.project.name }}</strong>
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <path d="m3 4.5 3 3 3-3" />
        </svg>
      </button>
    </div>

    <div
      v-if="workspace.selectedAsset.value"
      class="mode-switch"
      aria-label="Workspace mode"
    >
      <button
        :aria-pressed="workspace.mode.value === 'convert'"
        :disabled="!workspace.canConvert.value"
        @click="workspace.setMode('convert')"
      >
        Convert
      </button>
      <button
        :aria-pressed="workspace.mode.value === 'edit'"
        :disabled="workspace.busy.value"
        @click="workspace.setMode('edit')"
      >
        {{ workspace.busy.value ? "Preparing…" : "Edit" }}
      </button>
    </div>

    <div v-if="workspace.project.value" class="titlebar-actions">
      <span class="zoom-label">Fit</span>
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
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <rect x="2" y="2.5" width="12" height="11" rx="2" />
          <path d="M10 3v10" />
        </svg>
      </button>
    </div>
  </header>
</template>
