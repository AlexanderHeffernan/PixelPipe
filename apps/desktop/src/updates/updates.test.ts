import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import SettingsModal from "./SettingsModal.vue";
import UpdatePrompt from "./UpdatePrompt.vue";
import { useUpdates } from "./use-updates";
import CliInstallPrompt from "../cli-install/CliInstallPrompt.vue";
import { useCliInstallation } from "../cli-install/use-cli-installation";

const tauri = vi.hoisted(() => ({
  check: vi.fn(),
  relaunch: vi.fn(async () => undefined),
  version: vi.fn(async () => "1.2.3"),
  install: vi.fn(async () => undefined),
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/app", () => ({ getVersion: tauri.version }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: tauri.relaunch }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: tauri.check }));

beforeEach(() => {
  vi.stubGlobal("navigator", { userAgent: "Mac OS X" });
  const updates = useUpdates();
  updates.stopAutomaticChecks();
  updates.availableUpdate.value = null;
  updates.currentVersion.value = "";
  updates.downloadProgress.value = null;
  updates.error.value = null;
  updates.lastCheckedAt.value = null;
  updates.status.value = "idle";
  const cli = useCliInstallation();
  cli.status.value = null;
  cli.busy.value = false;
  cli.error.value = null;
  cli.dismissed.value = false;
  tauri.check.mockReset();
  tauri.check.mockResolvedValue({
    version: "1.3.0",
    body: "Faster exports",
    downloadAndInstall: tauri.install,
  });
  tauri.install.mockClear();
  tauri.relaunch.mockClear();
  tauri.invoke.mockReset();
  tauri.invoke.mockImplementation(async (command: string) => {
    if (command === "cli_installation_status") {
      return {
        state: "not_installed",
        command: "/usr/local/bin/pixelate",
      };
    }
    if (command === "install_cli") {
      return { state: "installed", command: "/usr/local/bin/pixelate" };
    }
    if (command === "uninstall_cli") {
      return { state: "not_installed", command: "/usr/local/bin/pixelate" };
    }
    throw new Error(`Unexpected command: ${command}`);
  });
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("desktop updates", () => {
  it("offers to install the command on first launch", async () => {
    render(CliInstallPrompt);

    expect(await screen.findByRole("alertdialog")).toHaveTextContent(
      "Use Pixelate from any terminal",
    );
    await fireEvent.click(
      screen.getByRole("button", { name: "Install command" }),
    );
    await waitFor(() =>
      expect(tauri.invoke).toHaveBeenCalledWith("install_cli"),
    );
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("prompts after a startup-style check and allows deferring the update", async () => {
    render(UpdatePrompt);
    await useUpdates().checkForUpdates(true);

    expect(await screen.findByRole("alertdialog")).toHaveTextContent(
      "Pixelate 1.3.0 is ready",
    );
    await fireEvent.click(screen.getByRole("button", { name: "Later" }));
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("shows version and update controls and installs only on request", async () => {
    const updates = useUpdates();
    await updates.checkForUpdates();
    render(SettingsModal);

    expect(await screen.findByText("Pixelate 1.2.3")).toBeVisible();
    expect(screen.getByText("Pixelate 1.3.0 is available")).toBeVisible();
    expect(await screen.findByText("Command not installed")).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Install command" }),
    ).toBeVisible();
    await fireEvent.click(
      screen.getByRole("button", { name: "Update and restart" }),
    );
    await waitFor(() => expect(tauri.install).toHaveBeenCalledOnce());
    expect(tauri.relaunch).toHaveBeenCalledOnce();
  });

  it("shows and removes an installed command from settings", async () => {
    tauri.invoke.mockImplementation(async (command: string) => {
      if (command === "cli_installation_status") {
        return { state: "installed", command: "/usr/local/bin/pixelate" };
      }
      if (command === "uninstall_cli") {
        return { state: "not_installed", command: "/usr/local/bin/pixelate" };
      }
      throw new Error(`Unexpected command: ${command}`);
    });
    render(SettingsModal);

    expect(await screen.findByText("Command installed")).toBeVisible();
    await fireEvent.click(
      screen.getByRole("button", { name: "Remove command" }),
    );
    await waitFor(() =>
      expect(tauri.invoke).toHaveBeenCalledWith("uninstall_cli"),
    );
    expect(await screen.findByText("Command not installed")).toBeVisible();
  });
});
