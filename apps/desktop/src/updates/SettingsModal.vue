<script setup lang="ts">
import { PhX } from "@phosphor-icons/vue";
import { computed, onMounted } from "vue";
import { useUpdates } from "./use-updates";

defineEmits<{ close: [] }>();
const updates = useUpdates();
const lastChecked = computed(() =>
  updates.lastCheckedAt.value
    ? new Date(updates.lastCheckedAt.value).toLocaleString()
    : "Not checked yet",
);
onMounted(() => void updates.loadVersion());
</script>

<template>
  <div class="modal-backdrop" @click.self="$emit('close')">
    <section
      class="settings-modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-title"
    >
      <header>
        <div>
          <p class="eyebrow">Pixelate</p>
          <h1 id="settings-title">Settings</h1>
        </div>
        <button
          class="icon-button"
          aria-label="Close settings"
          @click="$emit('close')"
        >
          <PhX />
        </button>
      </header>

      <div class="settings-section">
        <div>
          <h2>Updates</h2>
          <p>Pixelate {{ updates.currentVersion.value || "…" }}</p>
        </div>
        <label class="settings-toggle">
          <span>
            <strong>Check for updates automatically</strong>
            <small>Check when Pixelate opens and every six hours.</small>
          </span>
          <input
            type="checkbox"
            :checked="updates.automaticChecksEnabled.value"
            :disabled="!updates.isMacOS"
            @change="
              updates.setAutomaticChecksEnabled(
                ($event.target as HTMLInputElement).checked,
              )
            "
          />
        </label>

        <div v-if="updates.isMacOS" class="update-status" aria-live="polite">
          <strong v-if="updates.status.value === 'available'">
            Pixelate {{ updates.availableUpdate.value?.version }} is available
          </strong>
          <strong v-else-if="updates.status.value === 'up-to-date'">
            Pixelate is up to date
          </strong>
          <strong v-else-if="updates.status.value === 'checking'">
            Checking for updates…
          </strong>
          <strong v-else-if="updates.status.value === 'installing'">
            Installing update{{
              updates.downloadProgress.value === null
                ? "…"
                : ` ${updates.downloadProgress.value}%`
            }}
          </strong>
          <strong v-else>Updates are ready to check</strong>
          <small>Last checked: {{ lastChecked }}</small>
          <p v-if="updates.error.value" class="settings-error" role="alert">
            {{ updates.error.value }}
          </p>
        </div>
        <p v-else class="update-status">
          Automatic updates are currently available in the macOS release.
        </p>

        <div class="modal-actions">
          <button
            class="quiet settings-button"
            :disabled="
              !updates.isMacOS ||
              updates.status.value === 'checking' ||
              updates.status.value === 'installing'
            "
            @click="updates.checkForUpdates()"
          >
            Check now
          </button>
          <button
            v-if="updates.availableUpdate.value"
            class="primary"
            :disabled="updates.status.value === 'installing'"
            @click="updates.installUpdate"
          >
            Update and restart
          </button>
        </div>
      </div>
    </section>
  </div>
</template>
