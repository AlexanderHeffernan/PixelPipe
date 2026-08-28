import { computed, type Ref } from "vue";
import type { AssetBrowser, CatalogEntry, ProjectBrowser } from "../types";

export interface AssetTreeFile {
  kind: "file";
  path: string;
  name: string;
  catalog: CatalogEntry;
  managed?: AssetBrowser;
}

export interface AssetTreeFolder {
  kind: "folder";
  path: string;
  name: string;
  folders: AssetTreeFolder[];
  files: AssetTreeFile[];
}

export function useAssetTree(
  project: Ref<ProjectBrowser | undefined>,
  query: Ref<string>,
) {
  const drafts = computed(() => {
    const needle = query.value.trim().toLowerCase();
    return (project.value?.assets || []).filter(({ asset }) => {
      if (asset.project_path) return false;
      const name = asset.display_name || displayName(asset.id);
      return (
        !needle ||
        name.toLowerCase().includes(needle) ||
        asset.id.includes(needle)
      );
    });
  });
  const tree = computed(() => {
    const root: AssetTreeFolder = {
      kind: "folder",
      path: "",
      name: "",
      folders: [],
      files: [],
    };
    const assets = new Map(
      (project.value?.assets || []).map((entry) => [entry.asset.id, entry]),
    );
    const needle = query.value.trim().toLowerCase();
    for (const catalog of project.value?.catalog || []) {
      const managed = catalog.asset_id
        ? assets.get(catalog.asset_id)
        : undefined;
      const name = catalog.path.split("/").at(-1) || catalog.path;
      const display =
        managed?.asset.display_name || name.replace(/\.[^.]+$/, "");
      if (
        needle &&
        !display.toLowerCase().includes(needle) &&
        !catalog.path.toLowerCase().includes(needle)
      )
        continue;
      const parts = catalog.path.split("/");
      parts.pop();
      let folder = root;
      for (const part of parts) {
        const path = folder.path ? `${folder.path}/${part}` : part;
        let child = folder.folders.find((entry) => entry.name === part);
        if (!child) {
          child = { kind: "folder", path, name: part, folders: [], files: [] };
          folder.folders.push(child);
        }
        folder = child;
      }
      folder.files.push({
        kind: "file",
        path: catalog.path,
        name: display,
        catalog,
        managed,
      });
    }
    sortFolder(root);
    return root;
  });
  return {
    folders: computed(() => tree.value.folders),
    rootFiles: computed(() => tree.value.files),
    drafts,
  };
}

const displayName = (id: string) =>
  id.replaceAll("-", " ").replace(/\b\w/g, (letter) => letter.toUpperCase());

function sortFolder(folder: AssetTreeFolder) {
  folder.folders.sort((a, b) => a.name.localeCompare(b.name));
  folder.files.sort((a, b) => a.name.localeCompare(b.name));
  folder.folders.forEach(sortFolder);
}
