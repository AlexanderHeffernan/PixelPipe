import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App.vue";
import * as api from "./api";
import { preview, project, revisionView, settings } from "./test-fixtures";

const dialogs = vi.hoisted(() => ({
  project: "/game",
  references: ["/tmp/New Hero.jpg"],
}));
const tauriWindow = vi.hoisted(() => ({
  isFullscreen: vi.fn(async () => false),
  onResized: vi.fn(async () => () => {}),
}));
vi.mock("./services/dialogs", () => ({
  chooseProjectFolder: vi.fn(async () => dialogs.project),
  chooseReferenceImage: vi.fn(async () => "/tmp/source.png"),
  chooseReferenceImages: vi.fn(async () => dialogs.references),
  chooseExportFile: vi.fn(async () => "/exports/custom-medic.webp"),
  confirmDeleteAsset: vi.fn(async () => true),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => tauriWindow,
}));
vi.mock("./api", async (original) => ({
  ...(await original<typeof import("./api")>()),
}));

const pixelizeSettings = {
  ...settings,
  margin: 0,
  subject_scale_percent: 100,
  offset_x: 0,
  offset_y: 0,
  registration: "center" as const,
};
const commit = {
  project_root: "/game",
  asset: "field-medic",
  revision: "r000001",
  revision_path: "/game/.pixelate/assets/field-medic/revisions/r000001",
  native_sha256: "1".repeat(64),
};

beforeEach(() => {
  tauriWindow.isFullscreen.mockClear();
  tauriWindow.onResized.mockClear();
  vi.spyOn(api, "recentProject").mockResolvedValue(null);
  vi.spyOn(api, "rememberProject").mockResolvedValue();
  vi.spyOn(api, "openProject").mockResolvedValue(structuredClone(project));
  vi.spyOn(api, "browseProject").mockResolvedValue(structuredClone(project));
  vi.spyOn(api, "previewSelectedReference").mockResolvedValue(preview);
  vi.spyOn(api, "loadRevision").mockResolvedValue(revisionView);
  vi.spyOn(api, "convertSelectedReference").mockResolvedValue(commit);
  vi.spyOn(api, "previewComposition").mockResolvedValue(preview);
  vi.spyOn(api, "commitComposition").mockResolvedValue(commit);
  vi.spyOn(api, "initializeAsset").mockResolvedValue(project.assets[0].asset);
  vi.spyOn(api, "importReference").mockResolvedValue({} as never);
  vi.spyOn(api, "deleteAsset").mockResolvedValue();
  vi.spyOn(api, "renameAsset").mockResolvedValue({} as never);
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

async function enterCanvas() {
  await fireEvent.click(
    screen.getByRole("button", { name: /Continue to Canvas/ }),
  );
  await screen.findByText("Canvas & Touch Up");
}

describe("deterministic workstation", () => {
  it("opens in a focused pixelization phase without editing or framing controls", async () => {
    await openWorkstation();
    expect(screen.getByText("Pixelize")).toBeVisible();
    expect(screen.getByText("Sprite resolution")).toBeVisible();
    expect(screen.getByText("Colour detail")).toBeVisible();
    expect(screen.getByRole("button", { name: "32" })).toBeVisible();
    expect(screen.getByRole("button", { name: "256" })).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "Pencil" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/Replace colour/)).not.toBeInTheDocument();
    expect(screen.getByLabelText("Sprite canvas")).not.toHaveClass("checker");
    expect(api.previewSelectedReference).toHaveBeenCalledWith(
      "/game",
      "field-medic",
      16,
      [],
      pixelizeSettings,
      true,
    );
  });

  it("derives a selectable number of colours from the source", async () => {
    vi.useFakeTimers();
    await openWorkstation();
    vi.mocked(api.previewSelectedReference).mockClear();
    await fireEvent.click(screen.getByRole("button", { name: "8" }));
    await vi.advanceTimersByTimeAsync(100);
    expect(api.previewSelectedReference).toHaveBeenLastCalledWith(
      "/game",
      "field-medic",
      8,
      [],
      pixelizeSettings,
      true,
    );
    await fireEvent.click(screen.getByRole("button", { name: "Vivid" }));
    await vi.advanceTimersByTimeAsync(100);
    const vividSettings = {
      ...pixelizeSettings,
      color_treatment: "original" as const,
      color_adjustments: {
        brightness: 0,
        contrast: 10,
        saturation: 35,
        warmth: 0,
      },
    };
    expect(api.previewSelectedReference).toHaveBeenLastCalledWith(
      "/game",
      "field-medic",
      8,
      [],
      vividSettings,
      true,
    );
    await fireEvent.click(
      screen.getByRole("button", { name: "Custom colour detail" }),
    );
    const count = screen.getByRole("spinbutton", { name: "Maximum colours" });
    await fireEvent.update(count, "7");
    await fireEvent.change(count);
    await vi.advanceTimersByTimeAsync(100);
    expect(api.previewSelectedReference).toHaveBeenLastCalledWith(
      "/game",
      "field-medic",
      7,
      [],
      vividSettings,
      true,
    );
    expect(screen.getByLabelText("Saturation exact value")).toHaveValue(35);
    await fireEvent.update(
      screen.getByLabelText("Saturation exact value"),
      "34",
    );
    await fireEvent.change(screen.getByLabelText("Saturation exact value"));
    expect(screen.getByRole("button", { name: "Custom" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("detects the background automatically with a clear manual override", async () => {
    vi.useFakeTimers();
    await openWorkstation();
    expect(screen.getByRole("button", { name: "Automatic" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(
      screen.getByRole("slider", { name: "Background range" }),
    ).toHaveValue("28");
    expect(
      screen.queryByLabelText("Background colour picker"),
    ).not.toBeInTheDocument();

    await fireEvent.click(screen.getByRole("button", { name: "Pick colour" }));
    const picker = screen.getByLabelText("Background colour picker");
    await fireEvent.update(picker, "#12ab34");
    await fireEvent.change(picker);
    await vi.advanceTimersByTimeAsync(110);

    expect(api.previewSelectedReference).toHaveBeenLastCalledWith(
      "/game",
      "field-medic",
      16,
      [],
      {
        ...pixelizeSettings,
        backdrop: {
          ...pixelizeSettings.backdrop,
          color: [18, 171, 52],
        },
      },
      false,
    );

    await fireEvent.click(
      screen.getByRole("button", { name: "No background" }),
    );
    await vi.advanceTimersByTimeAsync(110);
    expect(api.previewSelectedReference).toHaveBeenLastCalledWith(
      "/game",
      "field-medic",
      16,
      [],
      {
        ...pixelizeSettings,
        backdrop: { type: "alpha", alpha_threshold: 8 },
      },
      false,
    );
  });

  it("shows custom resolution only when requested and retains a failed preview", async () => {
    vi.useFakeTimers();
    await openWorkstation();
    expect(
      screen.queryByLabelText("Custom resolution value"),
    ).not.toBeInTheDocument();
    await fireEvent.click(
      screen.getByRole("button", { name: "Custom resolution" }),
    );
    expect(screen.getByLabelText("Custom resolution value")).toBeVisible();
    vi.mocked(api.previewSelectedReference).mockRejectedValueOnce(
      new Error("shape is too fragmented"),
    );
    await fireEvent.click(screen.getByRole("button", { name: "64" }));
    await vi.advanceTimersByTimeAsync(100);
    await Promise.resolve();
    expect(screen.getByRole("alert")).toHaveTextContent(
      "shape is too fragmented",
    );
    expect(
      screen.getByRole("img", { name: "field-medic pixel art" }),
    ).toHaveAttribute("src", "data:image/png;base64,preview-native");
  });

  it("checkpoints conversion only when continuing to the canvas phase", async () => {
    await openWorkstation();
    expect(api.convertSelectedReference).not.toHaveBeenCalled();
    await enterCanvas();
    expect(api.convertSelectedReference).toHaveBeenCalledWith(
      "/game",
      "field-medic",
      16,
      [],
      pixelizeSettings,
      true,
      "user",
    );
    expect(screen.getByRole("button", { name: "Pencil" })).toBeVisible();
    expect(screen.getByLabelText("Drawing colour")).toHaveValue("#262c3e");
    expect(screen.getByLabelText("Sprite canvas")).toHaveClass("checker");
  });

  it("replaces a source in Pixelize while keeping the last sprite visible", async () => {
    await openWorkstation();
    let finishPreview: ((result: typeof preview) => void) | undefined;
    vi.mocked(api.previewSelectedReference).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finishPreview = resolve;
        }),
    );

    void fireEvent.click(
      screen.getByRole("button", { name: /Replace Source Image/ }),
    );

    expect(await screen.findByText("Updating source…")).toBeVisible();
    expect(
      screen.getByRole("img", { name: "field-medic pixel art" }),
    ).toHaveAttribute("src", "data:image/png;base64,preview-native");
    expect(api.importReference).toHaveBeenCalledWith(
      "/game",
      "field-medic",
      "/tmp/source.png",
    );

    await waitFor(() =>
      expect(api.previewSelectedReference).toHaveBeenCalledTimes(2),
    );
    finishPreview!(preview);
    await waitFor(() =>
      expect(screen.queryByText("Updating source…")).not.toBeInTheDocument(),
    );
    expect(screen.getByText("Pixelize")).toBeVisible();
  });

  it("shows the precise pixel cursor only while hovering the canvas", async () => {
    await openWorkstation();
    await enterCanvas();
    const canvas = screen.getByLabelText("Sprite canvas");
    expect(document.querySelector(".grid-cursor")).not.toBeInTheDocument();
    await fireEvent.pointerEnter(canvas);
    expect(document.querySelector(".grid-cursor")).toBeInTheDocument();
    await fireEvent.pointerLeave(canvas);
    expect(document.querySelector(".grid-cursor")).not.toBeInTheDocument();
  });

  it("previews canvas placement without reconverting the source", async () => {
    vi.useFakeTimers();
    await openWorkstation();
    await enterCanvas();
    vi.mocked(api.previewComposition).mockClear();
    vi.mocked(api.convertSelectedReference).mockClear();
    await fireEvent.update(
      screen.getByLabelText("Canvas horizontal position"),
      "9",
    );
    await vi.advanceTimersByTimeAsync(30);
    expect(api.previewComposition).toHaveBeenCalledWith(
      "/game",
      "field-medic",
      "r000001",
      { width: 32, height: 32, scale_percent: 100, offset_x: 9, offset_y: 0 },
    );
    expect(api.convertSelectedReference).not.toHaveBeenCalled();
  });

  it("replaces palette colours as immutable editing revisions", async () => {
    vi.spyOn(api, "remapRevision").mockResolvedValue({} as never);
    await openWorkstation();
    await enterCanvas();
    const colour = screen.getByLabelText("Replace colour 1");
    await fireEvent.update(colour, "#ff0080");
    await fireEvent.change(colour);
    await waitFor(() => expect(api.remapRevision).toHaveBeenCalledTimes(1));
    expect(screen.getByText("Canvas & Touch Up")).toBeVisible();
  });

  it("returns from canvas with a clear destructive boundary and exports there", async () => {
    vi.mocked(api.browseProject).mockResolvedValue({
      ...structuredClone(project),
      assets: project.assets.map((entry) => ({
        ...entry,
        asset: { ...entry.asset, head: "r000001" },
      })),
    });
    vi.spyOn(api, "exportAssetFile").mockResolvedValue({
      asset: "field-medic",
      revision: "r000001",
      file: "/exports/custom-medic.webp",
      format: "webp",
      width: 32,
      height: 32,
    });
    await openWorkstation();
    await enterCanvas();
    expect(screen.queryByLabelText("Artwork scale")).not.toBeInTheDocument();
    const warning = screen.getByLabelText(/do not carry forward/);
    expect(warning).toBeInTheDocument();
    await fireEvent.mouseEnter(warning);
    expect(screen.getByRole("tooltip")).toHaveTextContent(
      "canvas changes remain in history",
    );
    await fireEvent.click(
      screen.getByRole("button", { name: /Export Sprite/ }),
    );
    await waitFor(() =>
      expect(api.exportAssetFile).toHaveBeenCalledWith(
        "/game",
        "field-medic",
        "/exports/custom-medic.webp",
        true,
      ),
    );
    expect(await screen.findByText("Exported 32×32 WEBP")).toBeVisible();
    await fireEvent.click(
      screen.getByRole("button", { name: /Back to Pixelize/ }),
    );
    await screen.findByText("Pixelize");
  });

  it("restores the recent project on launch", async () => {
    vi.mocked(api.recentProject).mockResolvedValue("/game");
    render(App);
    await screen.findByRole("navigation", { name: "Project assets" });
    expect(api.openProject).toHaveBeenCalledWith("/game");
  });

  it("discovers assets created externally by the project terminal", async () => {
    const external = structuredClone(project);
    external.assets.push({
      asset: {
        ...structuredClone(project.assets[0].asset),
        id: "wasteland-bush",
        display_name: "Wasteland Bush",
        head: "r000001",
      },
      revisions: [],
    });
    await openWorkstation();
    vi.mocked(api.browseProject).mockResolvedValueOnce(external);

    window.dispatchEvent(new Event("focus"));

    expect(
      await screen.findByRole("button", { name: "Wasteland Bush" }),
    ).toBeVisible();
    expect(api.loadRevision).toHaveBeenCalledWith(
      "/game",
      "wasteland-bush",
      "r000001",
    );
  });

  it("shows an externally replaced selected sprite without reopening it", async () => {
    await openWorkstation();
    const external = structuredClone(project);
    external.assets[0].asset.selected_reference!.sha256 = "9".repeat(64);
    external.assets[0].asset.head = "r000002";
    vi.mocked(api.browseProject).mockResolvedValueOnce(external);
    vi.mocked(api.loadRevision).mockResolvedValueOnce({
      ...revisionView,
      metadata: { ...revisionView.metadata, revision: "r000002" },
      native_png_base64: "externally-updated",
    });

    window.dispatchEvent(new Event("focus"));

    await waitFor(() =>
      expect(
        screen.getByRole("img", { name: "field-medic pixel art" }),
      ).toHaveAttribute("src", "data:image/png;base64,externally-updated"),
    );
    expect(screen.getByText("Canvas & Touch Up")).toBeVisible();
  });

  it("imports images directly with a derived unique asset name", async () => {
    let finishImport:
      | ((asset: (typeof project.assets)[0]["asset"]) => void)
      | undefined;
    vi.mocked(api.initializeAsset).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finishImport = resolve;
        }),
    );
    await openWorkstation();
    void fireEvent.click(screen.getByRole("button", { name: "Import Asset" }));
    expect(await screen.findByText("Importing…")).toBeVisible();
    finishImport?.(project.assets[0].asset);
    await waitFor(() =>
      expect(api.initializeAsset).toHaveBeenCalledWith(
        "/game",
        "new-hero",
        "New Hero",
      ),
    );
    expect(api.importReference).toHaveBeenCalledWith(
      "/game",
      "new-hero",
      "/tmp/New Hero.jpg",
    );
    await waitFor(() =>
      expect(screen.queryByText("Importing…")).not.toBeInTheDocument(),
    );
  });

  it("renames an asset without changing its stable identity", async () => {
    await openWorkstation();
    await fireEvent.click(
      screen.getByRole("button", { name: "Rename Field Medic" }),
    );
    expect(
      screen.getByRole("button", { name: "Close rename for Field Medic" }),
    ).toBeVisible();
    const name = screen.getByRole("textbox", { name: "Asset name" });
    await fireEvent.update(name, "Lead Healer");
    await fireEvent.keyDown(name, { key: "Enter" });
    await waitFor(() =>
      expect(api.renameAsset).toHaveBeenCalledWith(
        "/game",
        "field-medic",
        "Lead Healer",
      ),
    );
  });

  it("returns to the conversion phase last used for each asset", async () => {
    const withHeads = structuredClone(project);
    withHeads.assets[0].asset.head = "r000001";
    withHeads.assets.push({
      asset: {
        ...structuredClone(withHeads.assets[0].asset),
        id: "forest-scout",
        display_name: "Forest Scout",
      },
      revisions: structuredClone(withHeads.assets[0].revisions),
    });
    vi.mocked(api.openProject).mockResolvedValue(withHeads);
    vi.mocked(api.browseProject).mockResolvedValue(withHeads);

    await openWorkstation();
    await fireEvent.click(
      screen.getByRole("button", { name: /Back to Pixelize/ }),
    );
    await fireEvent.click(screen.getByRole("button", { name: "Forest Scout" }));
    await screen.findByText("Canvas & Touch Up");
    await fireEvent.click(screen.getByRole("button", { name: "Field Medic" }));
    await screen.findByText("Pixelize");
    expect(
      screen.queryByRole("button", { name: "Pencil" }),
    ).not.toBeInTheDocument();
  });

  it("keeps both sidebars independently accessible and draggable chrome safe", async () => {
    await openWorkstation();
    const assetResize = screen.getByRole("separator", {
      name: "Resize asset sidebar",
    });
    const inspectorResize = screen.getByRole("separator", {
      name: "Resize inspector",
    });
    await fireEvent.pointerDown(assetResize, { button: 0 });
    await fireEvent.pointerMove(window, { clientX: 320 });
    await fireEvent.pointerUp(window);
    expect(document.querySelector(".project-sidebar")).toHaveStyle(
      "--sidebar-width: 320px",
    );
    await fireEvent.keyDown(inspectorResize, { key: "ArrowLeft" });
    expect(document.querySelector(".conversion-inspector")).toHaveStyle(
      "--sidebar-width: 336px",
    );
    expect(document.querySelector(".window-drag-region")).toHaveAttribute(
      "data-tauri-drag-region",
    );
    await fireEvent.click(
      screen.getByRole("button", { name: "Hide asset sidebar" }),
    );
    expect(
      screen.queryByRole("button", { name: "Fixture Game" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Show asset sidebar" }),
    ).toBeVisible();
    await fireEvent.click(
      screen.getByRole("button", { name: "Hide inspector" }),
    );
    expect(document.querySelector(".project-sidebar")).toHaveAttribute(
      "aria-hidden",
      "true",
    );
    expect(document.querySelector(".conversion-inspector")).toHaveAttribute(
      "aria-hidden",
      "true",
    );
    await fireEvent.click(
      screen.getByRole("button", { name: "Show inspector" }),
    );
    expect(document.querySelector(".conversion-inspector")).not.toHaveAttribute(
      "aria-hidden",
    );
  });
});
