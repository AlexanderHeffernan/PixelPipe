import { computed, inject, provide, ref } from "vue";
import * as api from "../api";
import { chooseProjectFolder, chooseReferenceImage } from "../services/dialogs";
import type {
  ConversionPreviewResponse,
  ProjectBrowser,
  RevisionViewResponse,
} from "../types";
import { createConversionPreview } from "./conversion-preview";

export type WorkspaceMode = "convert" | "edit";
export type AssetSource = "reference" | "agent";

export function createWorkspace() {
  const project = ref<ProjectBrowser>();
  const assetId = ref("");
  const mode = ref<WorkspaceMode>("convert");
  const view = ref<RevisionViewResponse>();
  const preview = ref<ConversionPreviewResponse>();
  const thumbnails = ref<Record<string, string>>({});
  const leftSidebarOpen = ref(true);
  const rightSidebarOpen = ref(true);
  const createAssetOpen = ref(false);
  const busy = ref(false);
  const error = ref("");
  const notice = ref("");

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
  const activeRecipe = computed(() =>
    recipes.value.find(({ id }) => id === recipeId.value),
  );
  const inspection = computed(
    () => preview.value?.inspection ?? view.value?.metadata.inspection,
  );
  const paletteName = computed(
    () =>
      preview.value?.palette_name ?? view.value?.metadata.palette_name ?? "",
  );
  const canvasImage = computed(() => {
    const bytes =
      preview.value?.native_png_base64 ?? view.value?.native_png_base64;
    return bytes ? api.pngDataUrl(bytes) : "";
  });
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
    settings,
    busy: previewBusy,
    error: previewError,
  } = conversion;
  let noticeTimer: ReturnType<typeof setTimeout> | undefined;

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
    if (!project.value) return;
    assetId.value = id;
    preview.value = undefined;
    view.value = undefined;
    const asset = project.value.assets.find(
      ({ asset }) => asset.id === id,
    )?.asset;
    if (asset?.head) {
      if (asset.selected_reference) conversion.chooseDefaultRecipe();
      mode.value = "edit";
      view.value = await api.loadRevision(
        project.value.project_root,
        id,
        asset.head,
      );
      thumbnails.value[id] = api.pngDataUrl(view.value.native_png_base64);
    } else if (asset?.selected_reference) {
      mode.value = "convert";
      conversion.chooseDefaultRecipe();
      await conversion.request();
    }
  }

  async function setMode(next: WorkspaceMode) {
    if (next === "convert" && !canConvert.value) return;
    if (next === "convert") {
      if (!settings.value) conversion.chooseDefaultRecipe();
      mode.value = next;
      await conversion.request();
      return;
    }
    if (next === "edit" && !selectedAsset.value?.asset.head) {
      await commitConversion();
      return;
    }
    preview.value = undefined;
    mode.value = next;
  }

  async function commitConversion() {
    if (!project.value || !recipeId.value || !settings.value) return;
    await run(async () => {
      const result = await api.convertSelectedReference(
        project.value!.project_root,
        assetId.value,
        recipeId.value,
        settings.value,
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
      showNotice("Conversion saved as the editing base");
    });
  }

  async function createAsset(name: string, brief: string, source: AssetSource) {
    const id = slug(name);
    if (!project.value || !id) return;
    await run(async () => {
      await api.initializeAsset(
        project.value!.project_root,
        id,
        "sprite",
        brief.trim() || name.trim(),
      );
      await refresh();
      await selectAsset(id);
      createAssetOpen.value = false;
      showNotice(
        source === "agent"
          ? "Asset ready for your coding agent"
          : "Asset created",
      );
    });
    if (!error.value && source === "reference") await importReference();
  }

  async function importReference() {
    if (!project.value || !assetId.value) return;
    const file = await chooseReferenceImage();
    if (!file) return;
    await run(async () => {
      await api.importReference(
        project.value!.project_root,
        assetId.value,
        file,
      );
      await refresh();
      await selectAsset(assetId.value);
      showNotice("Reference imported");
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
    settings,
    thumbnails,
    leftSidebarOpen,
    rightSidebarOpen,
    createAssetOpen,
    busy,
    previewBusy,
    previewError,
    error,
    notice,
    selectedAsset,
    recipes,
    activeRecipe,
    inspection,
    paletteName,
    canvasImage,
    canConvert,
    chooseProject,
    openPath,
    selectAsset,
    updateSettings: conversion.updateSettings,
    chooseRecipe: conversion.chooseRecipe,
    setMode,
    createAsset,
    importReference,
  };
}

const slug = (value: string) =>
  value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
export type Workspace = ReturnType<typeof createWorkspace>;
const workspaceKey = Symbol("pixelpipe-workspace");
export const provideWorkspace = (workspace: Workspace) =>
  provide(workspaceKey, workspace);
export const useWorkspace = () => {
  const workspace = inject<Workspace>(workspaceKey);
  if (!workspace) throw new Error("PixelPipe workspace is unavailable");
  return workspace;
};
