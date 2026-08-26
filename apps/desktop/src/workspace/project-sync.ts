import type { Ref } from "vue";
import * as api from "../api";
import type {
  AssetManifest,
  ProjectBrowser,
  RevisionViewResponse,
} from "../types";

export interface ExternalAssetChange {
  asset: AssetManifest;
  headChanged: boolean;
  sourceChanged: boolean;
  loaded?: RevisionViewResponse;
}

interface ProjectSyncContext {
  project: Ref<ProjectBrowser | undefined>;
  assetId: Ref<string>;
  view: Ref<RevisionViewResponse | undefined>;
  thumbnails: Ref<Record<string, string>>;
  selectAsset: (id: string) => Promise<void>;
  refreshSelected: (change: ExternalAssetChange) => Promise<void>;
}

export function createProjectSync(context: ProjectSyncContext) {
  let syncing = false;

  return async function syncExternalChanges() {
    const current = context.project.value;
    if (!current || syncing) return;
    syncing = true;
    try {
      const previousHeads = new Map(
        current.assets.map(({ asset }) => [asset.id, asset.head]),
      );
      const previousSources = new Map(
        current.assets.map(({ asset }) => [
          asset.id,
          asset.selected_reference?.sha256,
        ]),
      );
      const next = await api.browseProject(current.project_root);
      context.project.value = next;

      const knownIds = new Set(next.assets.map(({ asset }) => asset.id));
      for (const id of Object.keys(context.thumbnails.value)) {
        if (!knownIds.has(id)) delete context.thumbnails.value[id];
      }

      for (const { asset } of next.assets) {
        const headChanged = previousHeads.get(asset.id) !== asset.head;
        const sourceChanged =
          previousSources.get(asset.id) !== asset.selected_reference?.sha256;
        if (!headChanged && !sourceChanged) continue;
        let loaded: RevisionViewResponse | undefined;
        if (asset.head && headChanged) {
          loaded = await api.loadRevision(
            next.project_root,
            asset.id,
            asset.head,
          );
          context.thumbnails.value[asset.id] = api.pngDataUrl(
            loaded.native_png_base64,
          );
        }
        if (asset.id === context.assetId.value) {
          await context.refreshSelected({
            asset,
            headChanged,
            sourceChanged,
            loaded,
          });
        }
      }

      if (
        context.assetId.value &&
        !next.assets.some(({ asset }) => asset.id === context.assetId.value)
      ) {
        const first = next.assets[0]?.asset.id;
        if (first) await context.selectAsset(first);
        else {
          context.assetId.value = "";
          context.view.value = undefined;
        }
      }
    } finally {
      syncing = false;
    }
  };
}
