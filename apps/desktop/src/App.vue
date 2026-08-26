<script setup lang="ts">
import { onBeforeUnmount, onMounted } from "vue";
import AssetWorkspace from "./components/AssetWorkspace.vue";
import ConversionInspector from "./components/ConversionInspector.vue";
import ProjectSidebar from "./components/ProjectSidebar.vue";
import WelcomeView from "./components/WelcomeView.vue";
import WorkstationTitlebar from "./components/WorkstationTitlebar.vue";
import TerminalDrawer from "./components/TerminalDrawer.vue";
import { createWorkspace, provideWorkspace } from "./workspace/context";

const workspace = createWorkspace();
provideWorkspace(workspace);
const syncProject = () =>
  void workspace.syncExternalChanges().catch(() => undefined);
onMounted(() => {
  void workspace.restoreRecentProject();
  window.addEventListener("focus", syncProject);
});
onBeforeUnmount(() => window.removeEventListener("focus", syncProject));
</script>

<template>
  <main class="app-shell" :class="{ 'has-project': workspace.project.value }">
    <WorkstationTitlebar />
    <div v-if="workspace.project.value" class="workstation-body">
      <ProjectSidebar
        :class="{ 'is-collapsed': !workspace.leftSidebarOpen.value }"
        :inert="!workspace.leftSidebarOpen.value || undefined"
        :aria-hidden="!workspace.leftSidebarOpen.value || undefined"
      />
      <div class="workspace-stack">
        <AssetWorkspace />
        <TerminalDrawer />
      </div>
      <ConversionInspector
        :class="{ 'is-collapsed': !workspace.rightSidebarOpen.value }"
        :inert="!workspace.rightSidebarOpen.value || undefined"
        :aria-hidden="!workspace.rightSidebarOpen.value || undefined"
      />
    </div>
    <WelcomeView v-else />
    <div
      v-if="
        !workspace.project.value &&
        (workspace.error.value || workspace.notice.value)
      "
      class="toast"
      :class="{ error: workspace.error.value }"
      :role="workspace.error.value ? 'alert' : 'status'"
    >
      {{ workspace.error.value || workspace.notice.value }}
    </div>
  </main>
</template>
