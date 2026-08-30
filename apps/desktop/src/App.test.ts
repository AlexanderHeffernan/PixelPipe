import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/vue";
import { readFileSync } from "node:fs";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App.vue";
import * as api from "./api";
import { preview, project, revisionView, settings } from "./test-fixtures";

const timelineCss = readFileSync("src/styles/timeline.css", "utf8");
const workspaceCss = readFileSync("src/styles/workspace.css", "utf8");

const dialogs = vi.hoisted(() => ({
  project: "/game",
  references: ["/tmp/New Hero.jpg"],
  confirmAnimationReplacement: vi.fn(async () => true),
}));
const tauriWindow = vi.hoisted(() => ({
  isFullscreen: vi.fn(async () => false),
  onResized: vi.fn(async () => () => {}),
}));
vi.mock("./services/dialogs", () => ({
  chooseProjectFolder: vi.fn(async () => dialogs.project),
  chooseReferenceImage: vi.fn(async () => "/tmp/source.png"),
  chooseFrameImage: vi.fn(async () => "/tmp/next-pose.png"),
  chooseReferenceImages: vi.fn(async () => dialogs.references),
  chooseExportFile: vi.fn(async () => "/exports/custom-medic.webp"),
  confirmDeleteAsset: vi.fn(async () => true),
  confirmReplaceAnimationWithPixelization: dialogs.confirmAnimationReplacement,
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
const localStorageDescriptor = Object.getOwnPropertyDescriptor(
  window,
  "localStorage",
);

beforeEach(() => {
  dialogs.confirmAnimationReplacement.mockReset();
  dialogs.confirmAnimationReplacement.mockResolvedValue(true);
  tauriWindow.isFullscreen.mockClear();
  tauriWindow.onResized.mockClear();
  vi.spyOn(api, "recentProject").mockResolvedValue(null);
  vi.spyOn(api, "rememberProject").mockResolvedValue();
  vi.spyOn(api, "cliInstallationStatus").mockResolvedValue({
    state: "installed",
    command: "/usr/local/bin/pixelate",
    managed: true,
  });
  vi.spyOn(api, "openProject").mockResolvedValue(structuredClone(project));
  vi.spyOn(api, "browseProject").mockResolvedValue(structuredClone(project));
  vi.spyOn(api, "previewSelectedReference").mockResolvedValue(preview);
  vi.spyOn(api, "loadRevision").mockResolvedValue(revisionView);
  vi.spyOn(api, "convertSelectedReference").mockResolvedValue(commit);
  vi.spyOn(api, "mutateFrames").mockResolvedValue(commit);
  vi.spyOn(api, "mutateRig").mockResolvedValue(commit);
  vi.spyOn(api, "bakeRig").mockResolvedValue(commit);
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
  if (localStorageDescriptor)
    Object.defineProperty(window, "localStorage", localStorageDescriptor);
});

async function openWorkstation() {
  render(App);
  await fireEvent.click(screen.getByRole("button", { name: "Open Project…" }));
  await screen.findByRole("navigation", { name: "Project assets" });
  await screen.findByRole("img", { name: "field-medic pixel art" });
}

async function enterCanvas() {
  await fireEvent.click(
    screen.getByRole("button", { name: /Continue to Canvas/ }),
  );
  await screen.findByText(/Canvas & Touch Up|Rig Motion/);
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

  it("keeps the timeline absent until a one-frame asset becomes an animation", async () => {
    let imported = false;
    const animated = structuredClone(revisionView);
    animated.metadata.frames.push({ id: "frame-0002", duration_ms: 100 });
    vi.mocked(api.mutateFrames).mockImplementation(async () => {
      imported = true;
      return commit;
    });
    vi.mocked(api.loadRevision).mockImplementation(async () =>
      structuredClone(imported ? animated : revisionView),
    );
    await openWorkstation();
    await enterCanvas();
    expect(
      screen.queryByRole("region", { name: "Frame timeline" }),
    ).not.toBeInTheDocument();
    const add = screen.getByRole("button", {
      name: "Add frame to create animation",
    });
    expect(add).toBeVisible();
    await fireEvent.click(add);
    expect(api.mutateFrames).toHaveBeenCalledWith(
      "/game",
      "field-medic",
      "r000001",
      {
        type: "import_frame",
        file: "/tmp/next-pose.png",
        position: 1,
      },
      "user",
    );
    expect(
      await screen.findByRole("button", { name: "Play animation" }),
    ).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "Open frame timeline" }),
    ).not.toBeInTheDocument();
  });

  it("opens, edits, renames, and drag-reorders stable animation frames", async () => {
    const animated = structuredClone(revisionView);
    animated.metadata.frames = [
      { id: "idle-a", duration_ms: 80 },
      { id: "idle-b", duration_ms: 120 },
      { id: "idle-c", duration_ms: 160 },
    ];
    vi.mocked(api.loadRevision).mockImplementation(
      async (_root, _asset, revision, frame) => ({
        ...structuredClone(animated),
        metadata: {
          ...structuredClone(animated.metadata),
          revision: revision ?? "r000001",
          selected_frame_id: frame ?? "idle-a",
        },
        native_png_base64: frame ?? "idle-a",
      }),
    );
    await openWorkstation();
    await enterCanvas();
    expect(
      screen.queryByRole("list", { name: "Ordered frames" }),
    ).not.toBeInTheDocument();
    await fireEvent.click(
      screen.getByRole("button", { name: "Open frame timeline" }),
    );

    const second = screen.getByRole("button", {
      name: "Frame 2, 120 milliseconds",
    });
    await fireEvent.click(second);
    await waitFor(() =>
      expect(api.loadRevision).toHaveBeenCalledWith(
        "/game",
        "field-medic",
        "r000001",
        "idle-b",
      ),
    );
    const resize = screen.getByRole("separator", {
      name: "Resize frame timeline",
    });
    for (let step = 0; step < 8; step += 1)
      await fireEvent.keyDown(resize, { key: "ArrowUp" });
    expect(
      screen.queryByText("Shift + ←/→ reorders the focused frame"),
    ).not.toBeInTheDocument();

    const duration = screen.getByLabelText(
      "Animation frame duration in milliseconds",
    );
    await fireEvent.update(duration, "140");
    await fireEvent.change(duration);
    await waitFor(() =>
      expect(api.mutateFrames).toHaveBeenCalledWith(
        "/game",
        "field-medic",
        "r000001",
        { type: "set_all_durations", duration_ms: 140 },
        "user",
      ),
    );

    const secondItem = second.closest("li")!;
    await fireEvent.contextMenu(secondItem);
    expect(screen.getByRole("menuitem", { name: "Rename" })).toBeVisible();
    await fireEvent.pointerDown(screen.getByText("Frame duration"));
    expect(
      screen.queryByRole("menuitem", { name: "Rename" }),
    ).not.toBeInTheDocument();
    await fireEvent.contextMenu(secondItem);
    await fireEvent.click(screen.getByRole("menuitem", { name: "Rename" }));
    const name = screen.getByRole("textbox", { name: "Frame name" });
    await fireEvent.update(name, "Passing pose");
    await fireEvent.keyDown(name, { key: "Enter" });
    await waitFor(() =>
      expect(api.mutateFrames).toHaveBeenCalledWith(
        "/game",
        "field-medic",
        "r000001",
        { type: "rename", frame_id: "idle-b", name: "Passing pose" },
        "user",
      ),
    );

    const third = screen
      .getByRole("button", { name: "Frame 3, 160 milliseconds" })
      .closest("li")!;
    const items = screen.getAllByRole("listitem");
    items.forEach((item, index) =>
      vi.spyOn(item, "getBoundingClientRect").mockReturnValue({
        left: index * 100,
        right: index * 100 + 100,
        top: 0,
        bottom: 100,
        width: 100,
        height: 100,
        x: index * 100,
        y: 0,
        toJSON: () => ({}),
      }),
    );
    await fireEvent.pointerDown(secondItem, {
      button: 0,
      clientX: 150,
      clientY: 20,
    });
    await fireEvent.pointerMove(window, { clientX: 290, clientY: 20 });
    expect(third).toHaveClass("drop-after");
    await fireEvent.pointerUp(window, { clientX: 290, clientY: 20 });
    await waitFor(() =>
      expect(api.mutateFrames).toHaveBeenCalledWith(
        "/game",
        "field-medic",
        "r000001",
        { type: "reorder", frame_id: "idle-b", position: 2 },
        "user",
      ),
    );
  });

  it("edits a generic rig over the pixels and hides automatic frames", async () => {
    let baked = false;
    const rigged = structuredClone(revisionView);
    rigged.metadata.frames = [
      { id: "pose-a", name: "Contact", duration_ms: 80 },
      { id: "__generated-0001", duration_ms: 80 },
      { id: "pose-b", name: "Passing", duration_ms: 80 },
    ];
    rigged.metadata.selected_frame_id = "pose-a";
    rigged.metadata.rig = {
      parts: [
        { id: "left-part", width: 4, height: 8, pivot: [2, 1] },
        { id: "right-part", width: 4, height: 8, pivot: [2, 1] },
      ],
      nodes: [{ id: "left" }, { id: "right", parent_id: "left" }],
      poses: [
        {
          id: "pose-a",
          name: "Contact",
          nodes: [
            {
              node_id: "left",
              part_id: "left-part",
              x_millis: 10000,
              y_millis: 12000,
              rotation_millidegrees: 0,
              scale_x_millis: 1000,
              scale_y_millis: 1000,
              depth: 0,
              visible: true,
            },
            {
              node_id: "right",
              part_id: "right-part",
              x_millis: 4000,
              y_millis: 0,
              rotation_millidegrees: 0,
              scale_x_millis: 1000,
              scale_y_millis: 1000,
              depth: 1,
              visible: true,
            },
          ],
        },
        {
          id: "pose-b",
          name: "Passing",
          nodes: [
            {
              node_id: "left",
              part_id: "left-part",
              x_millis: 14000,
              y_millis: 12000,
              rotation_millidegrees: 15000,
              scale_x_millis: 1000,
              scale_y_millis: 1000,
              depth: 0,
              visible: true,
            },
            {
              node_id: "right",
              part_id: "right-part",
              x_millis: 4000,
              y_millis: 0,
              rotation_millidegrees: -15000,
              scale_x_millis: 1000,
              scale_y_millis: 1000,
              depth: 1,
              visible: true,
            },
          ],
        },
      ],
      frame_duration_ms: 80,
      interpolation: { inbetweens: 1, looped: false },
    };
    rigged.rig_part_pngs = {
      "left-part": "left-png",
      "right-part": "right-png",
    };
    vi.mocked(api.bakeRig).mockImplementation(async () => {
      baked = true;
      return commit;
    });
    vi.mocked(api.loadRevision).mockImplementation(
      async (_root, _asset, revision, frame) => {
        const loaded = structuredClone(baked ? revisionView : rigged);
        loaded.metadata.revision = revision ?? "r000001";
        loaded.metadata.selected_frame_id =
          frame ?? loaded.metadata.frames[0].id;
        return loaded;
      },
    );

    await openWorkstation();
    await enterCanvas();
    expect(screen.getByLabelText("Editable pixel rig")).toBeVisible();
    expect(
      screen.getAllByRole("button", { name: /Adjust rig joint/ }),
    ).toHaveLength(2);
    expect(document.querySelectorAll(".rig-overlay line")).toHaveLength(1);
    expect(document.querySelectorAll(".rig-artwork image")).toHaveLength(2);
    expect(
      screen.queryByRole("button", { name: "Pencil" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText("Select a joint on the canvas to edit its sprite."),
    ).toBeVisible();

    await fireEvent.pointerDown(
      screen.getByRole("button", { name: "Adjust rig joint left" }),
      { button: 0, pointerId: 1 },
    );
    expect(document.querySelector(".rig-selection")).toBeInTheDocument();
    expect(
      screen.getByLabelText("Sprite assigned to selected joint"),
    ).toHaveValue("left-part");
    await fireEvent.pointerCancel(screen.getByLabelText("Editable pixel rig"));
    const rotation = screen.getByLabelText(
      "Selected sprite rotation in degrees",
    );
    await fireEvent.update(rotation, "25");
    await fireEvent.change(rotation);
    await waitFor(() =>
      expect(api.mutateRig).toHaveBeenCalledWith(
        "/game",
        "field-medic",
        "r000001",
        {
          type: "update_node",
          pose_id: "pose-a",
          node_id: "left",
          rotation_millidegrees: 25000,
        },
        "user",
      ),
    );
    await fireEvent.keyDown(
      screen.getByRole("button", { name: "Adjust rig joint left" }),
      { key: "ArrowRight" },
    );
    await waitFor(() =>
      expect(api.mutateRig).toHaveBeenCalledWith(
        "/game",
        "field-medic",
        "r000001",
        expect.objectContaining({
          type: "update_node",
          pose_id: "pose-a",
          node_id: "left",
          x_millis: 11000,
          y_millis: 12000,
        }),
        "user",
      ),
    );

    vi.mocked(api.mutateRig).mockClear();
    const overlay = screen.getByLabelText("Editable pixel rig");
    vi.spyOn(overlay, "getBoundingClientRect").mockReturnValue({
      x: 0,
      y: 0,
      left: 0,
      top: 0,
      right: 100,
      bottom: 100,
      width: 100,
      height: 100,
      toJSON: () => ({}),
    });
    await fireEvent.pointerDown(
      screen.getByRole("button", { name: "Adjust rig joint right" }),
      { button: 0, pointerId: 2, clientX: 40, clientY: 40 },
    );
    await fireEvent.pointerMove(overlay, {
      pointerId: 2,
      clientX: 50,
      clientY: 50,
    });
    expect(api.mutateRig).not.toHaveBeenCalled();
    expect(document.querySelector(".rig-reach")).toBeInTheDocument();
    await fireEvent.pointerUp(overlay, {
      pointerId: 2,
      clientX: 50,
      clientY: 50,
    });
    await waitFor(() => expect(api.mutateRig).toHaveBeenCalledTimes(1));
    await fireEvent.pointerDown(overlay, {
      button: 0,
      pointerId: 3,
      clientX: 2,
      clientY: 2,
    });
    expect(
      screen.getByText("Select a joint on the canvas to edit its sprite."),
    ).toBeVisible();

    await fireEvent.click(
      screen.getByRole("button", { name: "Open frame timeline" }),
    );
    expect(screen.getAllByRole("listitem")).toHaveLength(2);
    expect(screen.getByText(/1 automatic/)).toBeVisible();
    expect(screen.queryByText("__generated-0001")).not.toBeInTheDocument();

    await fireEvent.click(
      screen.getByRole("button", { name: "Proceed to Touch Ups" }),
    );
    await waitFor(() =>
      expect(api.bakeRig).toHaveBeenCalledWith(
        "/game",
        "field-medic",
        "r000001",
        "user",
      ),
    );
    expect(await screen.findByRole("button", { name: "Pencil" })).toBeVisible();
    expect(workspaceCss).toContain(".rig-overlay g:focus-visible");
  });

  it("persists and drag-resizes the open timeline outside project data", async () => {
    const getItem = vi.fn(() => "240");
    const setItem = vi.fn();
    Object.defineProperty(window, "localStorage", {
      configurable: true,
      value: { getItem, setItem },
    });
    const animated = structuredClone(revisionView);
    animated.metadata.frames.push({ id: "idle-b", duration_ms: 120 });
    vi.mocked(api.loadRevision).mockResolvedValue(animated);

    await openWorkstation();
    await enterCanvas();
    await fireEvent.click(
      screen.getByRole("button", { name: "Open frame timeline" }),
    );
    expect(getItem).toHaveBeenCalledWith("pixelate.timeline-height");
    const resize = screen.getByRole("separator", {
      name: "Resize frame timeline",
    });
    expect(resize).toHaveAttribute("aria-valuenow", "240");
    await fireEvent.pointerDown(resize, { button: 0, clientY: 300 });
    await fireEvent.pointerMove(window, { clientY: 250 });
    await fireEvent.pointerUp(window);
    expect(resize).toHaveAttribute("aria-valuenow", "290");
    await waitFor(() =>
      expect(setItem).toHaveBeenCalledWith("pixelate.timeline-height", "290"),
    );
    for (let step = 0; step < 20; step += 1)
      await fireEvent.keyDown(resize, { key: "ArrowDown" });
    expect(resize).toHaveAttribute("aria-valuenow", "94");
    expect(screen.getByRole("region", { name: "Frame timeline" })).toHaveClass(
      "is-minimal",
    );
    expect(timelineCss).toContain("height: 16px");
    await fireEvent.click(
      screen.getByRole("button", { name: "Close frame timeline" }),
    );
    expect(
      screen.getByRole("button", { name: "Open frame timeline" }),
    ).toBeVisible();
  });

  it("keeps long sequences in one accessible horizontal strip", async () => {
    const animated = structuredClone(revisionView);
    animated.metadata.frames = Array.from({ length: 20 }, (_, index) => ({
      id: `frame-${index + 1}`,
      duration_ms: 80 + index,
    }));
    vi.mocked(api.loadRevision).mockResolvedValue(animated);

    await openWorkstation();
    await enterCanvas();
    await fireEvent.click(
      screen.getByRole("button", { name: "Open frame timeline" }),
    );
    const strip = screen.getByRole("list", { name: "Ordered frames" });
    expect(strip).toHaveClass("frame-strip");
    expect(screen.getAllByRole("listitem")).toHaveLength(20);
    expect(
      screen.getByRole("region", { name: "Frame timeline" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "More frame actions" }),
    ).toBeInTheDocument();
    expect(timelineCss).toContain("overflow-x: auto");
    expect(timelineCss).toContain("@media (max-width: 760px)");
    expect(timelineCss).toContain("prefers-reduced-motion: reduce");
    expect(screen.queryByText(/ ms$/)).not.toBeInTheDocument();
  });

  it("plays the selected animation in its asset thumbnail", async () => {
    vi.useFakeTimers();
    const animated = structuredClone(revisionView);
    animated.metadata.frames = [
      { id: "a", duration_ms: 10 },
      { id: "b", duration_ms: 10 },
    ];
    vi.mocked(api.loadRevision).mockImplementation(
      async (_root, _asset, revision, frame) => ({
        ...structuredClone(animated),
        metadata: {
          ...structuredClone(animated.metadata),
          revision: revision ?? "r000001",
          selected_frame_id: frame ?? "a",
        },
        native_png_base64: frame ?? "a",
      }),
    );
    await openWorkstation();
    await enterCanvas();
    const thumbnail = document.querySelector<HTMLImageElement>(
      ".asset-thumbnail img",
    )!;
    await waitFor(() => expect(thumbnail.src).toContain("base64,a"));
    await vi.advanceTimersByTimeAsync(11);
    await waitFor(() => expect(thumbnail.src).toContain("base64,b"));
  });

  it("never carries an animated thumbnail into the next selected asset", async () => {
    const twoAssets = structuredClone(project);
    twoAssets.assets.push({
      asset: {
        ...structuredClone(project.assets[0].asset),
        id: "scout",
        display_name: "Scout",
        head: "r000002",
      },
      revisions: [],
    });
    vi.mocked(api.openProject).mockResolvedValue(twoAssets);
    vi.mocked(api.browseProject).mockResolvedValue(twoAssets);
    const animated = structuredClone(revisionView);
    animated.metadata.frames = [
      { id: "a", duration_ms: 1000 },
      { id: "b", duration_ms: 1000 },
    ];
    vi.mocked(api.loadRevision).mockImplementation(
      async (_root, asset, revision, frame) => {
        const loaded = structuredClone(
          asset === "scout" ? revisionView : animated,
        );
        loaded.metadata.asset = asset;
        loaded.metadata.revision = revision ?? "r000001";
        loaded.metadata.selected_frame_id =
          frame ?? loaded.metadata.frames[0].id;
        loaded.native_png_base64 = asset === "scout" ? "scout" : (frame ?? "a");
        return loaded;
      },
    );
    await openWorkstation();
    await enterCanvas();
    await waitFor(() =>
      expect(
        document.querySelector<HTMLImageElement>(
          ".asset-row[aria-current='page'] .asset-thumbnail img",
        )?.src,
      ).toContain("base64,a"),
    );
    await fireEvent.click(screen.getByRole("button", { name: "Scout" }));
    await screen.findByRole("img", { name: "scout pixel art" });
    await waitFor(() =>
      expect(
        document.querySelector<HTMLImageElement>(
          ".asset-row[aria-current='page'] .asset-thumbnail img",
        )?.src,
      ).toContain("base64,scout"),
    );
  });

  it("plays stored frame durations, stops at loop-off end, and pauses before editing", async () => {
    vi.useFakeTimers();
    const animated = structuredClone(revisionView);
    animated.metadata.frames = [
      { id: "a", duration_ms: 10 },
      { id: "b", duration_ms: 10 },
    ];
    vi.mocked(api.loadRevision).mockImplementation(
      async (_root, _asset, revision, frame) => ({
        ...structuredClone(animated),
        metadata: {
          ...structuredClone(animated.metadata),
          revision: revision ?? "r000001",
          selected_frame_id: frame ?? "a",
        },
      }),
    );
    await openWorkstation();
    await enterCanvas();
    await fireEvent.click(
      screen.getByRole("button", { name: "Open frame timeline" }),
    );
    await vi.advanceTimersByTimeAsync(0);
    vi.mocked(api.loadRevision).mockClear();
    await fireEvent.click(
      screen.getByRole("button", { name: "Play animation" }),
    );
    await vi.advanceTimersByTimeAsync(25);
    expect(api.loadRevision).not.toHaveBeenCalled();
    await fireEvent.click(
      screen.getByRole("button", { name: "Pause animation" }),
    );
    await fireEvent.click(screen.getByRole("button", { name: "Loop" }));
    await fireEvent.click(
      screen.getByRole("button", { name: "Play animation" }),
    );
    await vi.advanceTimersByTimeAsync(25);
    expect(
      screen.getByRole("button", { name: "Play animation" }),
    ).toBeVisible();

    await fireEvent.click(
      screen.getByRole("button", { name: "Play animation" }),
    );
    expect(
      screen.getByRole("button", { name: "Pause animation" }),
    ).toBeVisible();
    await fireEvent.pointerDown(screen.getByLabelText("Sprite canvas"), {
      button: 0,
      clientX: 1,
      clientY: 1,
      pointerId: 1,
    });
    expect(
      screen.getByRole("button", { name: "Play animation" }),
    ).toBeVisible();
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

  it("does not silently replace an animation when returning to Pixelize", async () => {
    const animated = structuredClone(revisionView);
    animated.metadata.frames.push({ id: "pose-b", duration_ms: 120 });
    vi.mocked(api.loadRevision).mockResolvedValue(animated);
    dialogs.confirmAnimationReplacement.mockResolvedValue(false);

    await openWorkstation();
    await enterCanvas();
    await fireEvent.click(
      screen.getByRole("button", { name: /Back to Pixelize/ }),
    );

    expect(dialogs.confirmAnimationReplacement).toHaveBeenCalledWith(2);
    expect(screen.getByText("Canvas & Touch Up")).toBeVisible();
    expect(screen.queryByText("Pixelize")).not.toBeInTheDocument();
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
