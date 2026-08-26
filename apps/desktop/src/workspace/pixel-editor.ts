import { computed, ref, type Ref } from "vue";
import * as api from "../api";
import type { ProjectBrowser, RevisionViewResponse } from "../types";

export type PixelTool = "eyedropper" | "pencil" | "eraser" | "fill";

interface EditorContext {
  project: Ref<ProjectBrowser | undefined>;
  assetId: Ref<string>;
  view: Ref<RevisionViewResponse | undefined>;
  refresh: () => Promise<void>;
  run: (action: () => Promise<void>) => Promise<void>;
  notice: (message: string) => void;
}

export function createPixelEditor(context: EditorContext) {
  const tool = ref<PixelTool>("pencil");
  const selectedIndex = ref(1);
  const brushSize = ref(1);
  const cursor = ref({ x: 0, y: 0 });
  const drawing = ref(false);
  const pendingEdits = ref<{ x: number; y: number; index: number }[]>([]);
  const redoStack = ref<string[]>([]);
  const historyBoundary = ref<string>();
  const stroke = new Map<string, { x: number; y: number; index: number }>();
  let lastPoint: { x: number; y: number } | undefined;

  const inspection = computed(() => context.view.value?.metadata.inspection);
  const canUndo = computed(
    () =>
      Boolean(context.view.value?.metadata.parent) &&
      context.view.value?.metadata.revision !== historyBoundary.value,
  );
  const canRedo = computed(() => redoStack.value.length > 0);
  const drawingColor = computed(() => {
    const color =
      context.view.value?.metadata.palette.colors[selectedIndex.value];
    return color ? rgbaHex(color) : "#000000";
  });

  function indexAt(x: number, y: number) {
    const token = inspection.value?.text_rows[y]?.split(" ")[x];
    return token === undefined || token === "--"
      ? (context.view.value?.metadata.transparent_index ?? 0)
      : Number.parseInt(token, 16);
  }

  function selectTool(next: PixelTool) {
    tool.value = next;
  }

  function point(x: number, y: number) {
    cursor.value = { x, y };
    if (tool.value === "eyedropper") {
      selectedIndex.value = indexAt(x, y);
      tool.value = "pencil";
      return;
    }
    if (tool.value === "fill") {
      void fill(x, y);
      return;
    }
    if (tool.value !== "pencil" && tool.value !== "eraser") return;
    drawing.value = true;
    addStrokePoint(x, y);
    lastPoint = { x, y };
  }

  function drag(x: number, y: number) {
    cursor.value = { x, y };
    if (drawing.value && (tool.value === "pencil" || tool.value === "eraser")) {
      addStrokeLine(lastPoint ?? { x, y }, { x, y });
      lastPoint = { x, y };
    }
  }

  function addStrokeLine(
    from: { x: number; y: number },
    to: { x: number; y: number },
  ) {
    let { x, y } = from;
    const dx = Math.abs(to.x - x);
    const dy = -Math.abs(to.y - y);
    const stepX = x < to.x ? 1 : -1;
    const stepY = y < to.y ? 1 : -1;
    let error = dx + dy;
    while (true) {
      addStrokePoint(x, y, false);
      if (x === to.x && y === to.y) break;
      const doubled = error * 2;
      if (doubled >= dy) {
        error += dy;
        x += stepX;
      }
      if (doubled <= dx) {
        error += dx;
        y += stepY;
      }
    }
    pendingEdits.value = [...stroke.values()];
  }

  function addStrokePoint(x: number, y: number, updatePreview = true) {
    const width = inspection.value?.width ?? 0;
    const height = inspection.value?.height ?? 0;
    const start = -Math.floor((brushSize.value - 1) / 2);
    const index =
      tool.value === "eraser"
        ? (context.view.value?.metadata.transparent_index ?? 0)
        : selectedIndex.value;
    for (let offsetY = start; offsetY < start + brushSize.value; offsetY += 1) {
      for (
        let offsetX = start;
        offsetX < start + brushSize.value;
        offsetX += 1
      ) {
        const editX = x + offsetX;
        const editY = y + offsetY;
        if (editX < 0 || editY < 0 || editX >= width || editY >= height)
          continue;
        stroke.set(`${editX}:${editY}`, { x: editX, y: editY, index });
      }
    }
    if (updatePreview) pendingEdits.value = [...stroke.values()];
  }

  async function finishStroke() {
    if (!drawing.value) return;
    drawing.value = false;
    lastPoint = undefined;
    const edits = [...stroke.values()];
    stroke.clear();
    if (!edits.length) return;
    try {
      await mutate((root, asset, parent) =>
        api.patchRevision(root, asset, parent, edits, "user"),
      );
    } finally {
      pendingEdits.value = [];
    }
  }

  async function fill(x: number, y: number) {
    await mutate((root, asset, parent) =>
      api.fillRevision(root, asset, parent, x, y, selectedIndex.value, "user"),
    );
  }

  async function recolor(index: number, hex: string) {
    const palette = context.view.value?.metadata.palette;
    if (!palette || index === palette.transparent_index) return;
    const colors = palette.colors.map((color) => [...color] as typeof color);
    colors[index] = [
      Number.parseInt(hex.slice(1, 3), 16),
      Number.parseInt(hex.slice(3, 5), 16),
      Number.parseInt(hex.slice(5, 7), 16),
      colors[index][3],
    ];
    await mutate((root, asset, parent) =>
      api.remapRevision(
        root,
        asset,
        parent,
        {
          name: palette.name,
          transparentIndex: palette.transparent_index,
          colors,
          indexMap: colors.map((_, colorIndex) => colorIndex),
        },
        "user",
      ),
    );
  }

  async function setDrawingColor(hex: string) {
    const palette = context.view.value?.metadata.palette;
    if (!palette) return;
    const rgb = parseHex(hex);
    const existing = palette.colors.findIndex(
      (color) =>
        color[0] === rgb[0] && color[1] === rgb[1] && color[2] === rgb[2],
    );
    if (existing >= 0) {
      selectedIndex.value = existing;
      return;
    }
    if (palette.colors.length >= 256) return;
    const colors = [
      ...palette.colors.map((color) => [...color] as typeof color),
      [...rgb, 255] as [number, number, number, number],
    ];
    await mutate((root, asset, parent) =>
      api.remapRevision(
        root,
        asset,
        parent,
        {
          name: palette.name,
          transparentIndex: palette.transparent_index,
          colors,
          indexMap: palette.colors.map((_, colorIndex) => colorIndex),
        },
        "user",
      ),
    );
    selectedIndex.value = colors.length - 1;
  }

  async function mutate(
    operation: (
      root: string,
      asset: string,
      parent: string,
    ) => Promise<unknown>,
  ) {
    const root = context.project.value?.project_root;
    const parent = context.view.value?.metadata.revision;
    if (!root || !parent) return;
    await context.run(async () => {
      redoStack.value = [];
      await operation(root, context.assetId.value, parent);
      await context.refresh();
      await loadHead();
      context.notice("Saved as a new revision");
    });
  }

  async function moveHistory(revision: string | undefined) {
    const root = context.project.value?.project_root;
    if (!root || !revision) return;
    await context.run(async () => {
      await api.setAssetHead(root, context.assetId.value, revision);
      await context.refresh();
      context.view.value = await api.loadRevision(
        root,
        context.assetId.value,
        revision,
      );
    });
  }

  async function loadHead() {
    const root = context.project.value?.project_root;
    const head = context.project.value?.assets.find(
      ({ asset }) => asset.id === context.assetId.value,
    )?.asset.head;
    if (root && head) {
      context.view.value = await api.loadRevision(
        root,
        context.assetId.value,
        head,
      );
    }
  }

  function moveCursor(dx: number, dy: number) {
    const width = inspection.value?.width ?? 1;
    const height = inspection.value?.height ?? 1;
    cursor.value = {
      x: Math.max(0, Math.min(width - 1, cursor.value.x + dx)),
      y: Math.max(0, Math.min(height - 1, cursor.value.y + dy)),
    };
  }

  async function undo() {
    const current = context.view.value?.metadata.revision;
    const parent = context.view.value?.metadata.parent;
    if (!current || !parent || current === historyBoundary.value) return;
    redoStack.value = [...redoStack.value, current];
    await moveHistory(parent);
  }

  async function redo() {
    const next = redoStack.value.at(-1);
    if (!next) return;
    redoStack.value = redoStack.value.slice(0, -1);
    await moveHistory(next);
  }

  function resetHistory(boundary?: string) {
    historyBoundary.value = boundary;
    redoStack.value = [];
    stroke.clear();
    drawing.value = false;
    lastPoint = undefined;
    pendingEdits.value = [];
  }

  return {
    tool,
    selectedIndex,
    brushSize,
    cursor,
    pendingEdits,
    canUndo,
    canRedo,
    drawingColor,
    selectTool,
    point,
    drag,
    finishStroke,
    recolor,
    setDrawingColor,
    undo,
    redo,
    resetHistory,
    moveCursor,
  };
}

function parseHex(hex: string): [number, number, number] {
  return [
    Number.parseInt(hex.slice(1, 3), 16),
    Number.parseInt(hex.slice(3, 5), 16),
    Number.parseInt(hex.slice(5, 7), 16),
  ];
}

function rgbaHex(rgba: readonly number[]) {
  return `#${rgba
    .slice(0, 3)
    .map((value) => value.toString(16).padStart(2, "0"))
    .join("")}`;
}
