import { ref } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as api from "../api";
import { project, revisionView } from "../test-fixtures";
import { createPixelEditor } from "./pixel-editor";

afterEach(() => vi.restoreAllMocks());

function setup() {
  const browser = structuredClone(project);
  browser.assets[0].asset.head = "r000001";
  browser.assets[0].asset.state = "revisioned";
  browser.assets[0].revisions = [
    {
      schema: "pixelpipe.revision/v1",
      id: "r000001",
      asset: "field-medic",
      created_unix_ms: 1,
      files: {},
    },
    {
      schema: "pixelpipe.revision/v1",
      id: "r000002",
      asset: "field-medic",
      parent: "r000001",
      created_unix_ms: 2,
      files: {},
    },
  ];
  const context = {
    project: ref(browser),
    assetId: ref("field-medic"),
    view: ref(structuredClone(revisionView)),
    refresh: vi.fn(async () => {}),
    run: vi.fn(async (action: () => Promise<void>) => action()),
    notice: vi.fn(),
  };
  vi.spyOn(api, "loadRevision").mockResolvedValue(revisionView);
  return { context, editor: createPixelEditor(context) };
}

describe("pixel editor", () => {
  it("batches one pencil gesture into one immutable patch", async () => {
    const patch = vi.spyOn(api, "patchRevision").mockResolvedValue({} as never);
    const { editor } = setup();

    editor.point(2, 3);
    editor.drag(3, 3);
    editor.drag(2, 3);
    await editor.finishStroke();

    expect(patch).toHaveBeenCalledTimes(1);
    expect(patch).toHaveBeenCalledWith(
      "/game",
      "field-medic",
      "r000001",
      [
        { x: 2, y: 3, index: 1 },
        { x: 3, y: 3, index: 1 },
      ],
      "user",
    );
  });

  it("interpolates fast pointer movement without leaving pixel gaps", async () => {
    const patch = vi.spyOn(api, "patchRevision").mockResolvedValue({} as never);
    const { editor } = setup();

    editor.point(2, 3);
    editor.drag(6, 3);
    await editor.finishStroke();

    expect(patch.mock.calls[0]?.[3]).toEqual([
      { x: 2, y: 3, index: 1 },
      { x: 3, y: 3, index: 1 },
      { x: 4, y: 3, index: 1 },
      { x: 5, y: 3, index: 1 },
      { x: 6, y: 3, index: 1 },
    ]);
  });

  it("previews a sized eraser immediately and commits one transparent patch", async () => {
    const patch = vi.spyOn(api, "patchRevision").mockResolvedValue({} as never);
    const { editor } = setup();
    editor.brushSize.value = 2;
    editor.selectTool("eraser");

    editor.point(2, 3);
    expect(editor.pendingEdits.value).toEqual([
      { x: 2, y: 3, index: 0 },
      { x: 3, y: 3, index: 0 },
      { x: 2, y: 4, index: 0 },
      { x: 3, y: 4, index: 0 },
    ]);
    await editor.finishStroke();

    expect(patch).toHaveBeenCalledTimes(1);
    expect(patch.mock.calls[0]?.[3]).toHaveLength(4);
    expect(editor.pendingEdits.value).toEqual([]);
  });

  it("recolours one palette entry in one immutable remap", async () => {
    const remap = vi.spyOn(api, "remapRevision").mockResolvedValue({} as never);
    const { editor } = setup();

    await editor.recolor(1, "#ff0080");

    expect(remap).toHaveBeenCalledTimes(1);
    expect(remap).toHaveBeenCalledWith(
      "/game",
      "field-medic",
      "r000001",
      {
        name: "starter",
        transparentIndex: 0,
        colors: [
          [0, 0, 0, 0],
          [255, 0, 128, 255],
        ],
        indexMap: [0, 1],
      },
      "user",
    );
  });

  it("selects an existing drawing colour or appends a reusable palette entry", async () => {
    const remap = vi.spyOn(api, "remapRevision").mockResolvedValue({} as never);
    const { editor } = setup();

    await editor.setDrawingColor("#262c3e");
    expect(editor.selectedIndex.value).toBe(1);
    expect(remap).not.toHaveBeenCalled();

    await editor.setDrawingColor("#ff0080");
    expect(remap).toHaveBeenCalledWith(
      "/game",
      "field-medic",
      "r000001",
      {
        name: "starter",
        transparentIndex: 0,
        colors: [
          [0, 0, 0, 0],
          [38, 44, 62, 255],
          [255, 0, 128, 255],
        ],
        indexMap: [0, 1],
      },
      "user",
    );
    expect(editor.selectedIndex.value).toBe(2);
  });

  it("resolves fill in the Rust use case and moves immutable history", async () => {
    const fill = vi.spyOn(api, "fillRevision").mockResolvedValue({} as never);
    const head = vi.spyOn(api, "setAssetHead").mockResolvedValue({} as never);
    const { context, editor } = setup();

    editor.selectTool("fill");
    editor.selectedIndex.value = 2;
    editor.point(4, 5);
    await vi.waitFor(() => expect(fill).toHaveBeenCalledTimes(1));
    context.view.value = {
      ...revisionView,
      metadata: {
        ...revisionView.metadata,
        revision: "r000002",
        parent: "r000001",
      },
    };
    await editor.undo();
    expect(head).toHaveBeenLastCalledWith("/game", "field-medic", "r000001");

    context.view.value = {
      ...revisionView,
      metadata: { ...revisionView.metadata, revision: "r000001" },
    };
    await editor.redo();
    expect(head).toHaveBeenLastCalledWith("/game", "field-medic", "r000002");
  });

  it("does not undo past the canvas editing boundary", async () => {
    const head = vi.spyOn(api, "setAssetHead").mockResolvedValue({} as never);
    const { context, editor } = setup();
    context.view.value = {
      ...revisionView,
      metadata: {
        ...revisionView.metadata,
        revision: "r000002",
        parent: "r000001",
      },
    };
    editor.resetHistory("r000002");

    expect(editor.canUndo.value).toBe(false);
    await editor.undo();
    expect(head).not.toHaveBeenCalled();
  });
});
