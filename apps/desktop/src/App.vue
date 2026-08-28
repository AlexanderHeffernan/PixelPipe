<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import CliInstallPrompt from "./cli-install/CliInstallPrompt.vue";
import AssetWorkspace from "./components/AssetWorkspace.vue";
import AppToast from "./components/AppToast.vue";
import ConversionInspector from "./components/ConversionInspector.vue";
import ProjectSidebar from "./components/ProjectSidebar.vue";
import WelcomeView from "./components/WelcomeView.vue";
import WorkstationTitlebar from "./components/WorkstationTitlebar.vue";
import TerminalDrawer from "./components/TerminalDrawer.vue";
import SettingsModal from "./updates/SettingsModal.vue";
import UpdatePrompt from "./updates/UpdatePrompt.vue";
import { useUpdates } from "./updates/use-updates";
import { createWorkspace, provideWorkspace } from "./workspace/context";

const workspace = createWorkspace();
provideWorkspace(workspace);
const settingsOpen = ref(false);
const updates = useUpdates();
const syncProject = () =>
  void workspace.syncExternalChanges().catch(() => undefined);
onMounted(() => {
  void workspace.restoreRecentProject();
  updates.startAutomaticChecks();
  window.addEventListener("focus", syncProject);
});
onBeforeUnmount(() => {
  updates.stopAutomaticChecks();
  window.removeEventListener("focus", syncProject);
});
</script>

<template>
  <main class="app-shell" :class="{ 'has-project': workspace.project.value }">
    <WorkstationTitlebar @open-settings="settingsOpen = true" />
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
        v-if="workspace.inspectorApplicable.value"
        :class="{ 'is-collapsed': !workspace.rightSidebarOpen.value }"
        :inert="!workspace.rightSidebarOpen.value || undefined"
        :aria-hidden="!workspace.rightSidebarOpen.value || undefined"
      />
    </div>
    <WelcomeView v-else />
    <AppToast
      v-if="
        !workspace.project.value &&
        (workspace.error.value || workspace.notice.value)
      "
      :message="workspace.error.value || workspace.notice.value"
      :error="Boolean(workspace.error.value)"
      @close="workspace.dismissMessage"
    />
    <SettingsModal v-if="settingsOpen" @close="settingsOpen = false" />
    <CliInstallPrompt />
    <UpdatePrompt />
  </main>
</template>
