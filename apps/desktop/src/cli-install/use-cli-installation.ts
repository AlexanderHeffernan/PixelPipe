import { computed, ref } from "vue";
import {
  cliInstallationStatus,
  installCli,
  uninstallCli,
  type CliInstallStatus,
} from "../api";

const status = ref<CliInstallStatus | null>(null);
const busy = ref(false);
const error = ref<string | null>(null);
const dismissed = ref(false);

function message(value: unknown) {
  return value instanceof Error ? value.message : String(value);
}

export function useCliInstallation() {
  const promptVisible = computed(
    () =>
      !dismissed.value &&
      (status.value?.state === "not_installed" ||
        status.value?.state === "needs_repair"),
  );

  async function loadStatus() {
    try {
      status.value = await cliInstallationStatus();
      error.value = null;
    } catch (failure) {
      error.value = message(failure);
    }
  }

  async function install() {
    if (busy.value) return;
    busy.value = true;
    error.value = null;
    try {
      status.value = await installCli();
      dismissed.value = false;
    } catch (failure) {
      error.value = message(failure);
    } finally {
      busy.value = false;
    }
  }

  async function uninstall() {
    if (busy.value) return;
    busy.value = true;
    error.value = null;
    try {
      status.value = await uninstallCli();
      dismiss();
    } catch (failure) {
      error.value = message(failure);
    } finally {
      busy.value = false;
    }
  }

  function dismiss() {
    dismissed.value = true;
  }

  return {
    busy,
    dismissed,
    error,
    promptVisible,
    status,
    dismiss,
    install,
    loadStatus,
    uninstall,
  };
}
