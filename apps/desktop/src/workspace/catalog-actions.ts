import type { Ref } from "vue";
import * as api from "../api";
import type { ProjectBrowser } from "../types";

interface CatalogContext {
  project: Ref<ProjectBrowser | undefined>;
  run: (action: () => Promise<void>) => Promise<void>;
  refresh: () => Promise<void>;
  selectAsset: (id: string) => Promise<void>;
  notice: (message: string) => void;
}

export function createCatalogActions(context: CatalogContext) {
  const act = async (
    message: string,
    action: (root: string) => Promise<unknown>,
  ) => {
    if (!context.project.value) return;
    await context.run(async () => {
      await action(context.project.value!.project_root);
      await context.refresh();
      context.notice(message);
    });
  };

  return {
    createAsset: (id: string, name: string, path?: string) =>
      act("Asset created", async (root) => {
        await api.initializeAsset(root, id, name, path);
        await context.refresh();
        await context.selectAsset(id);
      }),
    adoptReference: (
      path: string,
      id: string,
      name: string,
      destination: string,
    ) =>
      act("Reference imported", async (root) => {
        await api.adoptProjectImage(root, path, id, name, destination);
        await context.refresh();
        await context.selectAsset(id);
      }),
    adoptPixelArt: (path: string, id: string, name: string) =>
      act("Pixel art imported", async (root) => {
        await api.adoptPixelArt(root, path, id, name);
        await context.refresh();
        await context.selectAsset(id);
      }),
    setIgnored: (path: string, ignored: boolean) =>
      act(ignored ? "Project image hidden" : "Project image restored", (root) =>
        api.setProjectImageIgnored(root, path, ignored),
      ),
    updateLinkedSource: (id: string) =>
      act("External image imported as source", (root) =>
        api.updateLinkedSource(root, id),
      ),
    relink: (id: string, path: string) =>
      act("Asset relinked", (root) => api.relinkAsset(root, id, path)),
    createFolder: (path: string) =>
      act("Folder created", (root) => api.createFolder(root, path)),
    moveFolder: (source: string, destination: string) =>
      act("Folder moved", (root) => api.moveFolder(root, source, destination)),
    deleteFolder: (path: string) =>
      act("Empty folder deleted", (root) => api.deleteFolder(root, path)),
    deleteProjectImage: (path: string) =>
      act("Project image deleted", (root) =>
        api.deleteProjectImage(root, path),
      ),
    moveAsset: (id: string, destination: string) =>
      act("Asset moved", (root) => api.moveAsset(root, id, destination)),
  };
}

export const suggestedAssetId = (path: string) =>
  (path.split("/").at(-1) || "asset")
    .replace(/\.[^.]+$/, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "") || "asset";
