import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App.vue";
import * as api from "./api";
import { preview, project, revisionView, settings } from "./test-fixtures";

const dialogs = vi.hoisted(() => ({
  project: "/game",
  reference: "/tmp/source.png",
}));
const tauriWindow = vi.hoisted(() => ({
  startDragging: vi.fn(async () => {}),
}));
vi.mock("./services/dialogs", () => ({
  chooseProjectFolder: vi.fn(async () => dialogs.project),
  chooseReferenceImage: vi.fn(async () => dialogs.reference),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => tauriWindow,
}));
vi.mock("./api", async (original) => ({
  ...(await original<typeof import("./api")>()),
}));

beforeEach(() => {
  tauriWindow.startDragging.mockClear();
  vi.spyOn(api, "openProject").mockResolvedValue(structuredClone(project));
  vi.spyOn(api, "browseProject").mockResolvedValue(structuredClone(project));
  vi.spyOn(api, "previewSelectedReference").mockResolvedValue(preview);
  vi.spyOn(api, "loadRevision").mockResolvedValue(revisionView);
  vi.spyOn(api, "convertSelectedReference");
  vi.spyOn(api, "initializeAsset").mockResolvedValue(project.assets[0].asset);
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.useRealTimers();
});

async function openWorkstation() {
  render(App);
  await fireEvent.click(
    screen.getByRole("button", { name: "Open Project Folder…" }),
  );
  await screen.findByRole("navigation", { name: "Project assets" });
  await screen.findByRole("img", { name: "field-medic pixel art" });
}

describe("deterministic workstation", () => {
  it("opens a project into the focused sprite workspace", async () => {
    await openWorkstation();
    expect(screen.getByText("Fixture Game")).toBeVisible();
    expect(screen.getByRole("button", { name: "Field Medic" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(
      screen.getByRole("img", { name: "field-medic pixel art" }),
    ).toHaveAttribute("src", "data:image/png;base64,preview-native");
    expect(api.previewSelectedReference).toHaveBeenCalledWith(
      "/game",
      "field-medic",
      "sprite-32",
      settings,
    );
  });

  it("previews control changes live without creating a revision", async () => {
    vi.useFakeTimers();
    await openWorkstation();
    vi.mocked(api.previewSelectedReference).mockClear();
    await fireEvent.click(screen.getByRole("button", { name: "64" }));
    await vi.advanceTimersByTimeAsync(100);
    await Promise.resolve();
    expect(api.previewSelectedReference).toHaveBeenCalledWith(
      "/game",
      "field-medic",
      "sprite-32",
      expect.objectContaining({ width: 64, height: 64 }),
    );
    expect(api.convertSelectedReference).not.toHaveBeenCalled();
  });

  it("supports linked and independent custom output dimensions", async () => {
    vi.useFakeTimers();
    await openWorkstation();
    vi.mocked(api.previewSelectedReference).mockClear();

    const width = screen.getByRole("spinbutton", { name: "Width" });
    await fireEvent.update(width, "40");
    await fireEvent.change(width);
    await vi.advanceTimersByTimeAsync(60);
    expect(api.previewSelectedReference).toHaveBeenLastCalledWith(
      "/game",
      "field-medic",
      "sprite-32",
      expect.objectContaining({ width: 40, height: 40 }),
    );

    await fireEvent.click(screen.getByRole("button", { name: "Linked" }));
    const height = screen.getByRole("spinbutton", { name: "Height" });
    await fireEvent.update(height, "24");
    await fireEvent.change(height);
    await vi.advanceTimersByTimeAsync(60);
    expect(api.previewSelectedReference).toHaveBeenLastCalledWith(
      "/game",
      "field-medic",
      "sprite-32",
      expect.objectContaining({ width: 40, height: 24 }),
    );
  });

  it("exposes deterministic backdrop controls with live help", async () => {
    await openWorkstation();
    expect(screen.getByRole("combobox", { name: "Background" })).toHaveValue(
      "border_connected",
    );
    expect(
      screen.getByRole("slider", { name: "Colour tolerance" }),
    ).toHaveValue("28");
    await fireEvent.update(
      screen.getByRole("slider", { name: "Colour tolerance" }),
      "51",
    );

    await fireEvent.update(
      screen.getByRole("combobox", { name: "Background" }),
      "alpha",
    );
    expect(
      screen.queryByRole("slider", { name: "Colour tolerance" }),
    ).not.toBeInTheDocument();
    expect(screen.getByText(/source image’s transparency/)).toBeVisible();

    await fireEvent.update(
      screen.getByRole("combobox", { name: "Background" }),
      "border_connected",
    );
    expect(
      screen.getByRole("slider", { name: "Colour tolerance" }),
    ).toHaveValue("51");
  });

  it("keeps the last valid image visible when a preview is rejected", async () => {
    vi.useFakeTimers();
    await openWorkstation();
    vi.mocked(api.previewSelectedReference).mockRejectedValueOnce(
      new Error("too many disconnected shapes"),
    );

    await fireEvent.update(
      screen.getByRole("slider", { name: "Shape coverage" }),
      "50",
    );
    await vi.advanceTimersByTimeAsync(100);
    await Promise.resolve();

    expect(screen.getByRole("alert")).toHaveTextContent(
      "too many disconnected shapes",
    );
    expect(
      screen.getByRole("img", { name: "field-medic pixel art" }),
    ).toHaveAttribute("src", "data:image/png;base64,preview-native");
  });

  it("creates one immutable editing base only when entering Edit", async () => {
    vi.mocked(api.convertSelectedReference).mockResolvedValue({
      project_root: "/game",
      asset: "field-medic",
      revision: "r000001",
      revision_path: "/game/.pixelpipe/assets/field-medic/revisions/r000001",
      native_sha256: "1".repeat(64),
      preview_sha256: "2".repeat(64),
      validation: "valid_visual_review_required",
    });
    vi.mocked(api.browseProject).mockResolvedValue({
      ...project,
      assets: [
        {
          ...project.assets[0],
          asset: {
            ...project.assets[0].asset,
            state: "revisioned",
            head: "r000001",
          },
        },
      ],
    });
    await openWorkstation();
    await fireEvent.click(await screen.findByRole("button", { name: "Edit" }));
    await waitFor(() =>
      expect(api.convertSelectedReference).toHaveBeenCalledTimes(1),
    );
    expect(api.convertSelectedReference).toHaveBeenCalledWith(
      "/game",
      "field-medic",
      "sprite-32",
      settings,
      "user",
    );
    expect(await screen.findByText("Pixel Editing")).toBeVisible();
  });

  it("exposes accessible independent sidebar toggles", async () => {
    await openWorkstation();
    await fireEvent.click(
      screen.getByRole("button", { name: "Hide asset sidebar" }),
    );
    await fireEvent.click(
      screen.getByRole("button", { name: "Hide inspector" }),
    );
    expect(
      screen.getByRole("button", { name: "Show asset sidebar" }),
    ).toHaveAttribute("aria-pressed", "false");
    expect(
      screen.getByRole("button", { name: "Show inspector" }),
    ).toHaveAttribute("aria-pressed", "false");
    expect(document.querySelector(".project-sidebar")).toHaveAttribute(
      "aria-hidden",
      "true",
    );
    expect(document.querySelector(".conversion-inspector")).toHaveClass(
      "is-collapsed",
    );
  });

  it("drags the native window only from non-interactive titlebar space", async () => {
    await openWorkstation();
    const titlebar = document.querySelector(".titlebar");
    expect(titlebar).not.toBeNull();
    await fireEvent.mouseDown(titlebar!);
    expect(tauriWindow.startDragging).toHaveBeenCalledTimes(1);

    await fireEvent.mouseDown(
      screen.getByRole("button", { name: "Hide inspector" }),
    );
    expect(tauriWindow.startDragging).toHaveBeenCalledTimes(1);
  });

  it("creates a coding-agent-ready asset without requiring setup fields", async () => {
    await openWorkstation();
    await fireEvent.click(screen.getByRole("button", { name: "Create Asset" }));
    const dialog = screen.getByRole("dialog", { name: "Create Asset" });
    const controls = within(dialog);
    const name = controls.getByRole("textbox", { name: "Name" });
    expect(name).toHaveFocus();
    await fireEvent.update(name, "Health Potion");
    await fireEvent.click(
      controls.getByRole("radio", { name: /Use my coding agent/ }),
    );
    await fireEvent.click(
      controls.getByRole("button", { name: "Create Asset" }),
    );
    await waitFor(() =>
      expect(api.initializeAsset).toHaveBeenCalledWith(
        "/game",
        "health-potion",
        "sprite",
        "Health Potion",
      ),
    );
    await waitFor(() =>
      expect(
        screen.queryByRole("dialog", { name: "Create Asset" }),
      ).not.toBeInTheDocument(),
    );
  });
});
