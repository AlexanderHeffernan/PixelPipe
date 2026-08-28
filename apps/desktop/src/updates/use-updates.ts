import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import {
  check,
  type DownloadEvent,
  type Update,
} from "@tauri-apps/plugin-updater";
import { ref, shallowRef } from "vue";

const AUTOMATIC_CHECKS_KEY = "pixelate.updates.automaticChecks";
const CHECK_INTERVAL = 6 * 60 * 60 * 1000;

export type UpdateStatus =
  | "idle"
  | "checking"
  | "up-to-date"
  | "available"
  | "installing"
  | "error";

const currentVersion = ref("");
const availableUpdate = shallowRef<Update | null>(null);
const status = ref<UpdateStatus>("idle");
const error = ref<string | null>(null);
const lastCheckedAt = ref<number | null>(null);
const downloadProgress = ref<number | null>(null);
const automaticChecksEnabled = ref(readAutomaticChecksPreference());
let checkTimer: number | undefined;
let checkPromise: Promise<void> | null = null;

function isSupportedOS() {
  return (
    navigator.userAgent.includes("Mac OS X") ||
    navigator.userAgent.includes("Linux")
  );
}

function readAutomaticChecksPreference() {
  try {
    return localStorage.getItem(AUTOMATIC_CHECKS_KEY) !== "false";
  } catch {
    return true;
  }
}

function message(value: unknown) {
  return value instanceof Error ? value.message : String(value);
}

export function useUpdates() {
  async function loadVersion() {
    if (currentVersion.value) return;
    try {
      currentVersion.value = await getVersion();
    } catch {
      currentVersion.value = "Unknown";
    }
  }

  async function checkForUpdates(silent = false) {
    if (!isSupportedOS() || checkPromise || status.value === "installing")
      return;
    status.value = "checking";
    error.value = null;
    checkPromise = check({ timeout: 15_000 })
      .then((update) => {
        availableUpdate.value = update;
        status.value = update ? "available" : "up-to-date";
      })
      .catch((failure) => {
        if (silent) {
          status.value = availableUpdate.value ? "available" : "idle";
          return;
        }
        status.value = "error";
        error.value = message(failure);
      })
      .finally(() => {
        lastCheckedAt.value = Date.now();
        checkPromise = null;
      });
    await checkPromise;
  }

  async function installUpdate() {
    const update = availableUpdate.value;
    if (!update || status.value === "installing") return;
    status.value = "installing";
    error.value = null;
    downloadProgress.value = 0;
    let total: number | undefined;
    let downloaded = 0;
    try {
      await update.downloadAndInstall((event: DownloadEvent) => {
        if (event.event === "Started") {
          total = event.data.contentLength;
          downloadProgress.value = total ? 0 : null;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          downloadProgress.value = total
            ? Math.min(100, Math.round((downloaded / total) * 100))
            : null;
        } else if (event.event === "Finished") {
          downloadProgress.value = 100;
        }
      });
      await relaunch();
    } catch (failure) {
      status.value = "error";
      error.value = message(failure);
      downloadProgress.value = null;
    }
  }

  function startAutomaticChecks() {
    void loadVersion();
    if (
      checkTimer !== undefined ||
      !automaticChecksEnabled.value ||
      !isSupportedOS()
    )
      return;
    void checkForUpdates(true);
    checkTimer = window.setInterval(
      () => void checkForUpdates(true),
      CHECK_INTERVAL,
    );
  }

  function stopAutomaticChecks() {
    if (checkTimer === undefined) return;
    window.clearInterval(checkTimer);
    checkTimer = undefined;
  }

  function setAutomaticChecksEnabled(enabled: boolean) {
    automaticChecksEnabled.value = enabled;
    try {
      localStorage.setItem(AUTOMATIC_CHECKS_KEY, String(enabled));
    } catch {
      // Preferences must not prevent a manual update check.
    }
    if (enabled) startAutomaticChecks();
    else stopAutomaticChecks();
  }

  return {
    automaticChecksEnabled,
    availableUpdate,
    currentVersion,
    downloadProgress,
    error,
    isSupportedOS: isSupportedOS(),
    lastCheckedAt,
    status,
    checkForUpdates,
    installUpdate,
    loadVersion,
    setAutomaticChecksEnabled,
    startAutomaticChecks,
    stopAutomaticChecks,
  };
}
