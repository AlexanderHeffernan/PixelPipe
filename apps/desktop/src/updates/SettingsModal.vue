<script setup lang="ts">
import { PhX } from "@phosphor-icons/vue";
import { computed, onMounted } from "vue";
import { useCliInstallation } from "../cli-install/use-cli-installation";
import { useUpdates } from "./use-updates";

defineEmits<{ close: [] }>();
const updates = useUpdates();
const cli = useCliInstallation();
const lastChecked = computed(() =>
  updates.lastCheckedAt.value
    ? new Date(updates.lastCheckedAt.value).toLocaleString()
    : "Not checked yet",
);
onMounted(() => {
  void updates.loadVersion();
  void cli.loadStatus();
});
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

      <div class="settings-content">
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
              :disabled="!updates.isSupportedOS"
              @change="
                updates.setAutomaticChecksEnabled(
                  ($event.target as HTMLInputElement).checked,
                )
              "
            />
          </label>

          <div
            v-if="updates.isSupportedOS"
            class="update-status"
            aria-live="polite"
          >
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
            Automatic updates are available in the macOS and Linux releases.
          </p>

          <div class="modal-actions">
            <button
              class="quiet settings-button"
              :disabled="
                !updates.isSupportedOS ||
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

        <div class="settings-section">
          <div>
            <h2>Command line</h2>
            <p>Run <code>pixelate</code> from any terminal or coding agent.</p>
          </div>
          <div class="update-status" aria-live="polite">
            <strong v-if="cli.status.value?.state === 'installed'">
              Command installed
            </strong>
            <strong v-else-if="cli.status.value?.state === 'needs_repair'">
              Command needs repair
            </strong>
            <strong v-else-if="cli.status.value?.state === 'conflict'">
              Another command already exists
            </strong>
            <strong v-else-if="cli.status.value?.state === 'unavailable'">
              Command installation unavailable
            </strong>
            <strong v-else>Command not installed</strong>
            <small>{{
              cli.status.value?.command || "/usr/local/bin/pixelate"
            }}</small>
            <p v-if="cli.status.value?.state === 'conflict'">
              Pixelate will not overwrite the existing file.
            </p>
            <p v-if="cli.error.value" class="settings-error" role="alert">
              {{ cli.error.value }}
            </p>
          </div>
          <div class="modal-actions">
            <button
              v-if="
                cli.status.value?.state === 'installed' &&
                cli.status.value.managed
              "
              class="quiet settings-button"
              :disabled="cli.busy.value"
              @click="cli.uninstall"
            >
              Remove command
            </button>
            <button
              v-else-if="
                cli.status.value?.state === 'not_installed' ||
                cli.status.value?.state === 'needs_repair'
              "
              class="primary"
              :disabled="cli.busy.value"
              @click="cli.install"
            >
              {{
                cli.busy.value
                  ? "Working…"
                  : cli.status.value?.state === "needs_repair"
                    ? "Repair command"
                    : "Install command"
              }}
            </button>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
