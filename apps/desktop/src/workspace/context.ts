import { computed, inject, provide, ref } from "vue";
import * as api from "../api";
import {
  chooseExportFile,
  chooseProjectFolder,
  chooseReferenceImage,
  confirmDeleteAsset,
} from "../services/dialogs";
import type {
  ConversionPreviewResponse,
  ProjectBrowser,
  RevisionViewResponse,
} from "../types";
import { createConversionPreview } from "./conversion-preview";
import { createCompositionPreview } from "./composition-preview";
import { createAssetImport } from "./asset-import";
import { createCanvasLoading } from "./canvas-loading";
import { createPixelEditor, type PixelTool } from "./pixel-editor";
import { createProjectSync, type ExternalAssetChange } from "./project-sync";

export type WorkspaceMode = "convert" | "edit";
export function createWorkspace() {
  const project = ref<ProjectBrowser>();
  const assetId = ref("");
  const mode = ref<WorkspaceMode>("convert");
  const view = ref<RevisionViewResponse>();
  const preview = ref<ConversionPreviewResponse>();
  const thumbnails = ref<Record<string, string>>({});
  const leftSidebarOpen = ref(true);
  const rightSidebarOpen = ref(true);
  const busy = ref(false);
  const importing = ref(false);
  const error = ref("");
  const notice = ref("");
  const assetModes = new Map<string, WorkspaceMode>();

  const selectedAsset = computed(() =>
    project.value?.assets.find(({ asset }) => asset.id === assetId.value),
  );
  const recipes = computed(
    () =>
      project.value?.recipes.filter(
        ({ kind, mode }) =>
          kind === selectedAsset.value?.asset.kind && mode.type === "reference",
      ) ?? [],
  );
  const inspection = computed(
    () => preview.value?.inspection ?? view.value?.metadata.inspection,
  );
  const canvasImage = computed(() => {
    const bytes =
      preview.value?.native_png_base64 ?? view.value?.native_png_base64;
    return bytes ? api.pngDataUrl(bytes) : "";
  });
  const canvasLoading = createCanvasLoading(canvasImage);
  const canConvert = computed(() =>
    Boolean(selectedAsset.value?.asset.selected_reference),
  );
  const conversion = createConversionPreview({
    project,
    assetId,
    preview,
    thumbnails,
    recipes,
  });
  const {
    recipeId,
    colorCount,
    paletteOverrides,
    backgroundAutomatic,
    settings,
    busy: previewBusy,
    error: previewError,
  } = conversion;
  let noticeTimer: ReturnType<typeof setTimeout> | undefined;

  const composition = createCompositionPreview({
    project,
    assetId,
    view,
    preview,
    refresh,
    run,
  });

  function showNotice(message: string) {
    notice.value = message;
    if (noticeTimer) clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => {
      notice.value = "";
    }, 2400);
  }

  async function run(action: () => Promise<void>) {
    busy.value = true;
    error.value = "";
    try {
      await action();
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : String(caught);
    } finally {
      busy.value = false;
    }
  }

  async function openPath(path: string) {
    await run(async () => {
      project.value = await api.openProject(path);
      const first = project.value.assets[0]?.asset.id;
      if (first) await selectAsset(first);
      void loadThumbnails();
      showNotice(`Opened ${project.value.project.name}`);
    });
    if (project.value?.project_root) {
      void api
        .rememberProject(project.value.project_root)
        .catch(() => undefined);
    }
  }

  async function restoreRecentProject() {
    const path = await api.recentProject().catch(() => null);
    if (path) await openPath(path);
  }

  async function chooseProject() {
    const path = await chooseProjectFolder();
    if (path) await openPath(path);
  }

  async function refresh() {
    if (project.value) {
      project.value = await api.browseProject(project.value.project_root);
    }
  }

  async function selectAsset(id: string) {
    await canvasLoading.run("Loading sprite…", () => loadAsset(id));
  }

  async function loadAsset(id: string) {
    if (!project.value) return;
    if (assetId.value) assetModes.set(assetId.value, mode.value);
    editor.resetHistory();
    assetId.value = id;
    const asset = project.value.assets.find(
      ({ asset }) => asset.id === id,
    )?.asset;
    if (asset?.selected_reference && assetModes.get(id) === "convert") {
      mode.value = "convert";
      if (asset.style) conversion.chooseAssetStyle(asset.style);
      else conversion.chooseDefaultRecipe();
      await conversion.request();
      view.value = undefined;
      return;
    }
    if (asset?.head) {
      if (asset.selected_reference) {
        if (asset.style) conversion.chooseAssetStyle(asset.style);
        else conversion.chooseDefaultRecipe();
      }
      mode.value = "edit";
      assetModes.set(id, "edit");
      view.value = await api.loadRevision(
        project.value.project_root,
        id,
        asset.head,
      );
      preview.value = undefined;
      editor.resetHistory(asset.head);
      composition.reset();
      thumbnails.value[id] = api.pngDataUrl(view.value.native_png_base64);
    } else if (asset?.selected_reference) {
      mode.value = "convert";
      assetModes.set(id, "convert");
      conversion.chooseDefaultRecipe();
      await conversion.request();
      view.value = undefined;
    } else {
      preview.value = undefined;
      view.value = undefined;
    }
  }

  async function setMode(next: WorkspaceMode) {
    if (next === "convert" && !canConvert.value) return;
    if (next === "convert") {
      if (!settings.value) conversion.chooseDefaultRecipe();
      mode.value = next;
      assetModes.set(assetId.value, next);
      await conversion.request();
      return;
    }
    if (next === "edit" && mode.value === "convert") {
      await commitConversion();
      return;
    }
    preview.value = undefined;
    mode.value = next;
    assetModes.set(assetId.value, next);
  }

  async function commitConversion() {
    if (!project.value || !recipeId.value || !settings.value) return;
    await canvasLoading.run("Preparing canvas…", () =>
      run(async () => {
        const result = await api.convertSelectedReference(
          project.value!.project_root,
          assetId.value,
          recipeId.value,
          undefined,
          colorCount.value,
          paletteOverrides.value,
          settings.value,
          backgroundAutomatic.value,
          "user",
        );
        await refresh();
        view.value = await api.loadRevision(
          project.value!.project_root,
          assetId.value,
          result.revision,
        );
        preview.value = undefined;
        mode.value = "edit";
        assetModes.set(assetId.value, "edit");
        composition.reset();
        editor.resetHistory(result.revision);
        showNotice("Conversion saved as the editing base");
      }),
    );
  }

  const editor = createPixelEditor({
    project,
    assetId,
    view,
    refresh,
    run,
    notice: showNotice,
  });
  const syncExternalChanges = createProjectSync({
    project,
    assetId,
    view,
    thumbnails,
    selectAsset,
    async refreshSelected(change: ExternalAssetChange) {
      await canvasLoading.run("Refreshing sprite…", async () => {
        if (change.headChanged && change.asset.head && change.loaded) {
          view.value = change.loaded;
          preview.value = undefined;
          mode.value = "edit";
          assetModes.set(change.asset.id, "edit");
          composition.reset();
          editor.resetHistory(change.asset.head);
          return;
        }
        if (change.sourceChanged && change.asset.selected_reference) {
          if (change.asset.style)
            conversion.chooseAssetStyle(change.asset.style);
          else conversion.chooseDefaultRecipe();
          mode.value = "convert";
          assetModes.set(change.asset.id, "convert");
          await conversion.request();
        }
      });
    },
  });
  const canUndo = computed(
    () => composition.dirty.value || editor.canUndo.value,
  );

  async function beginTool(tool: PixelTool) {
    if (mode.value === "convert") await commitConversion();
    if (mode.value === "edit") {
      if (!(await composition.commitIfDirty())) return;
      editor.selectTool(tool);
    }
  }

  async function prepareEditing() {
    return composition.commitIfDirty();
  }

  async function setDrawingColor(hex: string) {
    if (!(await prepareEditing())) return;
    await editor.setDrawingColor(hex);
  }

  async function undo() {
    if (!(await prepareEditing())) return;
    await editor.undo();
  }

  async function redo() {
    if (!(await prepareEditing())) return;
    await editor.redo();
  }

  async function recolorCurrent(index: number, hex: string) {
    if (mode.value === "convert") conversion.recolor(index, hex);
    else {
      if (!(await composition.commitIfDirty())) return;
      await editor.recolor(index, hex);
    }
  }

  async function reconvert() {
    if (!canConvert.value) return;
    await canvasLoading.run("Pixelizing source…", async () => {
      editor.resetHistory();
      if (!settings.value) conversion.chooseDefaultRecipe();
      mode.value = "convert";
      assetModes.set(assetId.value, "convert");
      await conversion.request();
    });
  }

  async function exportCurrent() {
    if (!project.value || !selectedAsset.value?.asset.head) return;
    if (!(await composition.commitIfDirty())) return;
    const destination = await chooseExportFile(assetId.value);
    if (!destination) return;
    await run(async () => {
      const result = await api.exportAssetFile(
        project.value!.project_root,
        assetId.value,
        destination,
        true,
      );
      showNotice(
        `Exported ${result.width}×${result.height} ${result.format.toUpperCase()}`,
      );
    });
  }

  async function replaceSource() {
    if (!project.value || !assetId.value) return;
    const file = await chooseReferenceImage();
    if (!file) return;
    await canvasLoading.run("Updating source…", () =>
      run(async () => {
        await api.importReference(
          project.value!.project_root,
          assetId.value,
          file,
        );
        await refresh();
        const asset = selectedAsset.value?.asset;
        if (asset?.style) conversion.chooseAssetStyle(asset.style);
        else conversion.chooseDefaultRecipe();
        mode.value = "convert";
        assetModes.set(assetId.value, "convert");
        editor.resetHistory();
        await conversion.request();
        showNotice("Source image replaced");
      }),
    );
  }

  const importAssets = createAssetImport({
    project,
    importing,
    run,
    refresh,
    selectAsset,
    notice: showNotice,
  });

  async function deleteAsset(id: string) {
    if (!project.value || !(await confirmDeleteAsset(id))) return;
    await run(async () => {
      await api.deleteAsset(project.value!.project_root, id);
      delete thumbnails.value[id];
      await refresh();
      const next = project.value?.assets[0]?.asset.id;
      if (next) await selectAsset(next);
      else {
        assetId.value = "";
        view.value = undefined;
        preview.value = undefined;
      }
      showNotice("Asset deleted");
    });
  }

  async function renameAsset(id: string, displayName: string) {
    if (!project.value || !displayName.trim()) return;
    await run(async () => {
      await api.renameAsset(
        project.value!.project_root,
        id,
        displayName.trim(),
      );
      await refresh();
      showNotice("Asset renamed");
    });
  }

  async function loadThumbnails() {
    if (!project.value) return;
    const root = project.value.project_root;
    const loaded = await Promise.all(
      project.value.assets
        .filter(({ asset }) => asset.head)
        .map(
          async ({ asset }) =>
            [
              asset.id,
              api.pngDataUrl(
                (await api.loadRevision(root, asset.id, asset.head))
                  .native_png_base64,
              ),
            ] as const,
        ),
    );
    thumbnails.value = { ...thumbnails.value, ...Object.fromEntries(loaded) };
  }

  return {
    project,
    assetId,
    mode,
    view,
    preview,
    recipeId,
    colorCount,
    paletteOverrides,
    backgroundAutomatic,
    settings,
    thumbnails,
    leftSidebarOpen,
    rightSidebarOpen,
    busy,
    importing,
    previewBusy,
    previewError,
    canvasLoading: canvasLoading.active,
    loadingArtwork: canvasLoading.artwork,
    loadingMessage: canvasLoading.message,
    error,
    notice,
    selectedAsset,
    recipes,
    inspection,
    canvasImage,
    canConvert,
    chooseProject,
    syncExternalChanges,
    restoreRecentProject,
    openPath,
    selectAsset,
    updateSettings: conversion.updateSettings,
    setColorCount: conversion.setColorCount,
    setBackgroundAutomatic: conversion.setBackgroundAutomatic,
    setMode,
    beginTool,
    prepareEditing,
    setDrawingColor,
    undo,
    redo,
    canUndo,
    recolorCurrent,
    reconvert,
    exportCurrent,
    editor,
    composition,
    importAssets,
    importReference: replaceSource,
    replaceSource,
    deleteAsset,
    renameAsset,
  };
}
export type Workspace = ReturnType<typeof createWorkspace>;
const workspaceKey = Symbol("pixelpipe-workspace");
export const provideWorkspace = (workspace: Workspace) =>
  provide(workspaceKey, workspace);
export const useWorkspace = () => {
  const workspace = inject<Workspace>(workspaceKey);
  if (!workspace) throw new Error("PixelPipe workspace is unavailable");
  return workspace;
};
