<script setup lang="ts">
import { onMounted } from "vue";
import { useCliInstallation } from "./use-cli-installation";

const cli = useCliInstallation();
onMounted(() => void cli.loadStatus());
</script>

<template>
  <div v-if="cli.promptVisible.value" class="modal-backdrop">
    <section
      class="update-prompt"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="cli-install-title"
    >
      <p class="eyebrow">Command line access</p>
      <h1 id="cli-install-title">Use Pixelate from any terminal</h1>
      <p>
        Install the <code>pixelate</code> command in
        <code>{{ cli.status.value?.command }}</code
        >. Your system may ask for authorization.
      </p>
      <p v-if="cli.error.value" class="settings-error" role="alert">
        {{ cli.error.value }}
      </p>
      <div class="modal-actions">
        <button class="quiet" :disabled="cli.busy.value" @click="cli.dismiss">
          Not now
        </button>
        <button class="primary" :disabled="cli.busy.value" @click="cli.install">
          {{ cli.busy.value ? "Installing…" : "Install command" }}
        </button>
      </div>
    </section>
  </div>
</template>
