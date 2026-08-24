<script setup lang="ts">
import AssetWorkspace from "./components/AssetWorkspace.vue";
import ConversionInspector from "./components/ConversionInspector.vue";
import CreateAssetDialog from "./components/CreateAssetDialog.vue";
import ProjectSidebar from "./components/ProjectSidebar.vue";
import WelcomeView from "./components/WelcomeView.vue";
import WorkstationTitlebar from "./components/WorkstationTitlebar.vue";
import { createWorkspace, provideWorkspace } from "./workspace/context";

const workspace = createWorkspace();
provideWorkspace(workspace);
</script>

<template>
  <main class="app-shell" :class="{ 'has-project': workspace.project.value }">
    <WorkstationTitlebar
      :inert="workspace.createAssetOpen.value || undefined"
    />
    <div
      v-if="workspace.project.value"
      class="workstation-body"
      :inert="workspace.createAssetOpen.value || undefined"
    >
      <ProjectSidebar v-if="workspace.leftSidebarOpen.value" />
      <AssetWorkspace />
      <ConversionInspector v-if="workspace.rightSidebarOpen.value" />
    </div>
    <WelcomeView v-else />
    <div
      v-if="workspace.error.value || workspace.notice.value"
      class="toast"
      :class="{ error: workspace.error.value }"
      :role="workspace.error.value ? 'alert' : 'status'"
    >
      {{ workspace.error.value || workspace.notice.value }}
    </div>
    <CreateAssetDialog v-if="workspace.createAssetOpen.value" />
  </main>
</template>
