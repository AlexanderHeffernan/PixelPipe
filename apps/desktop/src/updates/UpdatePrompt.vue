<script setup lang="ts">
import { ref } from "vue";
import { useUpdates } from "./use-updates";

const updates = useUpdates();
const dismissedVersion = ref<string>();
</script>

<template>
  <div
    v-if="
      updates.availableUpdate.value &&
      dismissedVersion !== updates.availableUpdate.value.version
    "
    class="modal-backdrop"
  >
    <section
      class="update-prompt"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="update-prompt-title"
    >
      <p class="eyebrow">Update available</p>
      <h1 id="update-prompt-title">
        Pixelate {{ updates.availableUpdate.value.version }} is ready
      </h1>
      <p>
        Install the latest version now? Pixelate will restart when the update is
        complete.
      </p>
      <p v-if="updates.error.value" class="settings-error" role="alert">
        {{ updates.error.value }}
      </p>
      <div class="modal-actions">
        <button
          class="quiet"
          :disabled="updates.status.value === 'installing'"
          @click="dismissedVersion = updates.availableUpdate.value?.version"
        >
          Later
        </button>
        <button
          class="primary"
          :disabled="updates.status.value === 'installing'"
          @click="updates.installUpdate"
        >
          {{
            updates.status.value === "installing"
              ? `Installing${updates.downloadProgress.value === null ? "…" : ` ${updates.downloadProgress.value}%`}`
              : "Update and restart"
          }}
        </button>
      </div>
    </section>
  </div>
</template>
