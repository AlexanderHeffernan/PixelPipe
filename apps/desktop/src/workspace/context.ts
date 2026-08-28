import { computed, inject, provide, ref } from "vue";
import * as api from "../api";
import type { ConversionPreviewResponse, RevisionViewResponse } from "../types";
import { createAssetActions } from "./asset-actions";
import { createConversionPreview } from "./conversion-preview";
import { createCompositionPreview } from "./composition-preview";
import { createAssetImport } from "./asset-import";
import { createCanvasLoading } from "./canvas-loading";
import { createCatalogActions } from "./catalog-actions";
import { createPixelEditor } from "./pixel-editor";
import { createProjectSession } from "./project-session";
import { createProjectSync, type ExternalAssetChange } from "./project-sync";

type WorkspaceMode = "convert" | "edit";
export function createWorkspace() {
  const mode = ref<WorkspaceMode>("convert");
  const view = ref<RevisionViewResponse>();
  const preview = ref<ConversionPreviewResponse>();
  const leftSidebarOpen = ref(true);
  const rightSidebarOpen = ref(true);
  const importing = ref(false);
  const projectImagePath = ref("");
  const projectImagePreview = ref("");
  const projectImagePixelArtImportable = ref(false);
  const artworkError = ref("");
  const assetModes = new Map<string, WorkspaceMode>();
  const session = createProjectSession({
    selectAsset,
    clearSelection() {
      view.value = undefined;
      preview.value = undefined;
      projectImagePath.value = "";
      projectImagePreview.value = "";
      projectImagePixelArtImportable.value = false;
      artworkError.value = "";
      rightSidebarOpen.value = false;
    },
  });
  const {
    project,
    assetId,
    thumbnails,
    busy,
    error,
    notice,
    selectedAsset,
    run,
    showNotice,
    dismissMessage,
    refresh,
  } = session;

  const inspection = computed(
    () => preview.value?.inspection ?? view.value?.metadata.inspection,
  );
  const canvasImage = computed(() => {
    const bytes =
      preview.value?.native_png_base64 ?? view.value?.native_png_base64;
    return bytes ? api.pngDataUrl(bytes) : "";
  });
  const inspectorApplicable = computed(() =>
    Boolean(view.value || preview.value),
  );
  const canvasLoading = createCanvasLoading(canvasImage);
  const canConvert = computed(() =>
    Boolean(selectedAsset.value?.asset.selected_reference),
  );
  const conversion = createConversionPreview({
    project,
    assetId,
    preview,
    thumbnails,
  });
  const {
    colorCount,
    paletteOverrides,
    backgroundAutomatic,
    settings,
    busy: previewBusy,
    error: previewError,
  } = conversion;

  const composition = createCompositionPreview({
    project,
    assetId,
    view,
    preview,
    refresh,
    run,
  });

  async function selectAsset(id: string) {
    artworkError.value = "";
    projectImagePath.value = "";
    projectImagePreview.value = "";
    projectImagePixelArtImportable.value = false;
    rightSidebarOpen.value = false;
    await canvasLoading.run("Loading sprite…", async () => {
      view.value = undefined;
      preview.value = undefined;
      try {
        await loadAsset(id);
        if (
          selectedAsset.value?.asset.selected_reference &&
          !view.value &&
          !preview.value &&
          previewError.value
        ) {
          throw new Error(previewError.value);
        }
        rightSidebarOpen.value = Boolean(view.value || preview.value);
      } catch (caught) {
        view.value = undefined;
        preview.value = undefined;
        rightSidebarOpen.value = false;
        artworkError.value =
          caught instanceof Error ? caught.message : String(caught);
      }
    });
  }

  async function selectProjectImage(path: string) {
    if (!project.value) return;
    if (projectImagePath.value === path) {
      clearProjectImage();
      return;
    }
    assetId.value = "";
    view.value = undefined;
    preview.value = undefined;
    artworkError.value = "";
    rightSidebarOpen.value = false;
    projectImagePath.value = path;
    projectImagePreview.value = "";
    projectImagePixelArtImportable.value = false;
    await canvasLoading.run("Loading image…", async () => {
      try {
        const image = await api.loadProjectImage(
          project.value!.project_root,
          path,
        );
        projectImagePreview.value = image.data_url;
        projectImagePixelArtImportable.value = image.pixel_art_importable;
      } catch (caught) {
        projectImagePreview.value = "";
        artworkError.value =
          caught instanceof Error ? caught.message : String(caught);
      }
    });
  }

  function clearProjectImage() {
    projectImagePath.value = "";
    projectImagePreview.value = "";
    projectImagePixelArtImportable.value = false;
    artworkError.value = "";
  }

  function clearAsset() {
    assetId.value = "";
    view.value = undefined;
    preview.value = undefined;
    artworkError.value = "";
    rightSidebarOpen.value = false;
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
      else conversion.chooseDefaults();
      await conversion.request();
      view.value = undefined;
      return;
    }
    if (asset?.head) {
      if (asset.selected_reference) {
        if (asset.style) conversion.chooseAssetStyle(asset.style);
        else conversion.chooseDefaults();
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
      conversion.chooseDefaults();
      await conversion.request();
      view.value = undefined;
    } else {
      preview.value = undefined;
      view.value = undefined;
    }
  }

  async function commitConversion() {
    if (!project.value || !settings.value) return;
    await canvasLoading.run("Preparing canvas…", () =>
      run(async () => {
        const result = await api.convertSelectedReference(
          project.value!.project_root,
          assetId.value,
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
  const assetActions = createAssetActions({
    project,
    assetId,
    selectedAsset,
    mode,
    assetModes,
    preview,
    canConvert,
    conversion,
    composition,
    editor,
    canvasLoading,
    run,
    refresh,
    notice: showNotice,
    commitConversion,
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
          else conversion.chooseDefaults();
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

  const importAssets = createAssetImport({
    project,
    importing,
    run,
    refresh,
    selectAsset,
    notice: showNotice,
  });
  const catalog = createCatalogActions({
    project,
    run,
    refresh,
    selectAsset,
    notice: showNotice,
  });

  return {
    project,
    assetId,
    mode,
    view,
    preview,
    colorCount,
    backgroundAutomatic,
    settings,
    thumbnails,
    leftSidebarOpen,
    rightSidebarOpen,
    busy,
    importing,
    projectImagePath,
    projectImagePreview,
    projectImagePixelArtImportable,
    artworkError,
    previewBusy,
    previewError,
    canvasLoading: canvasLoading.active,
    loadingArtwork: canvasLoading.artwork,
    loadingMessage: canvasLoading.message,
    error,
    notice,
    dismissMessage,
    selectedAsset,
    inspection,
    inspectorApplicable,
    canvasImage,
    canConvert,
    chooseProject: session.chooseProject,
    syncExternalChanges,
    restoreRecentProject: session.restoreRecentProject,
    selectAsset,
    selectProjectImage,
    clearProjectImage,
    clearAsset,
    updateSettings: conversion.updateSettings,
    setColorCount: conversion.setColorCount,
    setBackgroundAutomatic: conversion.setBackgroundAutomatic,
    setMode: assetActions.setMode,
    beginTool: assetActions.beginTool,
    prepareEditing: assetActions.prepareEditing,
    setDrawingColor: assetActions.setDrawingColor,
    undo: assetActions.undo,
    redo: assetActions.redo,
    canUndo,
    recolorCurrent: assetActions.recolorCurrent,
    reconvert: assetActions.reconvert,
    exportCurrent: assetActions.exportCurrent,
    editor,
    composition,
    importAssets: importAssets.references,
    importPixelArt: importAssets.pixelArt,
    importReference: assetActions.replaceSource,
    replaceSource: assetActions.replaceSource,
    deleteAsset: session.deleteAsset,
    renameAsset: session.renameAsset,
    catalog,
  };
}
type Workspace = ReturnType<typeof createWorkspace>;
const workspaceKey = Symbol("pixelate-workspace");
export const provideWorkspace = (workspace: Workspace) =>
  provide(workspaceKey, workspace);
export const useWorkspace = () => {
  const workspace = inject<Workspace>(workspaceKey);
  if (!workspace) throw new Error("Pixelate workspace is unavailable");
  return workspace;
};
