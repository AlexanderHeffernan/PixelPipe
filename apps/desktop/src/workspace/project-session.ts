import { computed, onScopeDispose, ref } from "vue";
import * as api from "../api";
import { chooseProjectFolder, confirmDeleteAsset } from "../services/dialogs";
import type { ProjectBrowser } from "../types";

interface ProjectSessionContext {
  selectAsset: (id: string) => Promise<void>;
  clearSelection: () => void;
}

export function createProjectSession(context: ProjectSessionContext) {
  const project = ref<ProjectBrowser>();
  const assetId = ref("");
  const thumbnails = ref<Record<string, string>>({});
  const busy = ref(false);
  const error = ref("");
  const notice = ref("");
  let noticeTimer: ReturnType<typeof setTimeout> | undefined;

  const selectedAsset = computed(() =>
    project.value?.assets.find(({ asset }) => asset.id === assetId.value),
  );

  onScopeDispose(() => {
    if (noticeTimer) clearTimeout(noticeTimer);
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
      if (first) await context.selectAsset(first);
      else context.clearSelection();
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

  async function deleteAsset(id: string) {
    const asset = project.value?.assets.find(
      (entry) => entry.asset.id === id,
    )?.asset;
    if (
      !project.value ||
      !asset ||
      !(await confirmDeleteAsset(id, Boolean(asset.project_path)))
    )
      return;
    await run(async () => {
      await api.deleteAsset(project.value!.project_root, id);
      delete thumbnails.value[id];
      await refresh();
      const next = project.value?.assets[0]?.asset.id;
      if (next) await context.selectAsset(next);
      else {
        assetId.value = "";
        context.clearSelection();
      }
      showNotice(
        asset.project_path
          ? "Removed from Pixelate; project image retained"
          : "Unexported asset and its Pixelate history deleted",
      );
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
    thumbnails,
    busy,
    error,
    notice,
    selectedAsset,
    run,
    showNotice,
    refresh,
    restoreRecentProject,
    chooseProject,
    deleteAsset,
    renameAsset,
  };
}
