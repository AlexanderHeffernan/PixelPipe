<script setup lang="ts">
import AssetWorkspace from "./components/AssetWorkspace.vue";
import ProjectSidebar from "./components/ProjectSidebar.vue";
import WelcomeView from "./components/WelcomeView.vue";
import { createWorkspace, provideWorkspace } from "./workspace/context";

const workspace = createWorkspace();
provideWorkspace(workspace);
</script>

<template>
  <main class="app-shell">
    <ProjectSidebar v-if="workspace.project.value" />
    <section class="workspace-surface">
      <header class="window-header">
        <div class="brand-mark" aria-hidden="true">
          <span></span><span></span><span></span><span></span>
        </div>
        <strong>PixelPipe</strong>
        <span v-if="workspace.project.value" class="project-path">{{
          workspace.project.value.project_root
        }}</span>
        <button
          v-if="workspace.project.value"
          class="quiet"
          @click="workspace.chooseProject"
        >
          Open Folder…
        </button>
      </header>
      <div
        class="message"
        :class="{
          error: workspace.error.value,
          empty: !workspace.error.value && !workspace.notice.value,
        }"
        :role="workspace.error.value ? 'alert' : 'status'"
      >
        {{ workspace.error.value || workspace.notice.value }}
      </div>
      <WelcomeView v-if="!workspace.project.value" />
      <AssetWorkspace v-else />
    </section>
  </main>
</template>
