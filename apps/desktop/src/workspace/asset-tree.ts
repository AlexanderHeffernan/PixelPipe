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
  hasManagedAssets: boolean;
  isPriority: boolean;
  folders: AssetTreeFolder[];
  files: AssetTreeFile[];
}

export function useAssetTree(
  project: Ref<ProjectBrowser | undefined>,
  query: Ref<string>,
  promotedFolders?: Ref<ReadonlySet<string>>,
) {
  const tree = computed(() => {
    const root: AssetTreeFolder = {
      kind: "folder",
      path: "",
      name: "",
      hasManagedAssets: false,
      isPriority: false,
      folders: [],
      files: [],
    };
    const assets = new Map(
      (project.value?.assets || []).map((entry) => [entry.asset.id, entry]),
    );
    const catalogedAssets = new Set<string>();
    const needle = query.value.trim().toLowerCase();
    for (const path of project.value?.folders || []) {
      if (needle && !path.toLowerCase().includes(needle)) continue;
      addFolder(root, path);
    }
    for (const catalog of project.value?.catalog || []) {
      const managed = catalog.asset_id
        ? assets.get(catalog.asset_id)
        : undefined;
      if (catalog.asset_id) catalogedAssets.add(catalog.asset_id);
      const name = catalog.path.split("/").at(-1) || catalog.path;
      const display = managed?.asset.display_name || name;
      if (
        needle &&
        !display.toLowerCase().includes(needle) &&
        !catalog.path.toLowerCase().includes(needle)
      )
        continue;
      const parts = catalog.path.split("/");
      parts.pop();
      const folder = addFolder(root, parts.join("/"));
      folder.files.push({
        kind: "file",
        path: catalog.path,
        name: display,
        catalog,
        managed,
      });
    }
    for (const [assetId, managed] of assets) {
      if (catalogedAssets.has(assetId)) continue;
      const path = managed.asset.project_path || `${assetId}.png`;
      const display = managed.asset.display_name || displayName(assetId);
      if (
        needle &&
        !display.toLowerCase().includes(needle) &&
        !path.toLowerCase().includes(needle)
      )
        continue;
      root.files.push({
        kind: "file",
        path,
        name: display,
        catalog: { path, asset_id: assetId, status: "unexported" },
        managed,
      });
    }
    sortFolder(root, promotedFolders?.value ?? new Set());
    return root;
  });
  return {
    managedFolders: computed(() =>
      tree.value.folders.filter((folder) => folder.isPriority),
    ),
    unmanagedFolders: computed(() =>
      tree.value.folders.filter((folder) => !folder.isPriority),
    ),
    managedRootFiles: computed(() =>
      tree.value.files.filter((file) => file.managed),
    ),
    projectRootFiles: computed(() =>
      tree.value.files.filter((file) => !file.managed),
    ),
  };
}

function addFolder(root: AssetTreeFolder, path: string) {
  let folder = root;
  for (const part of path.split("/").filter(Boolean)) {
    const childPath = folder.path ? `${folder.path}/${part}` : part;
    let child = folder.folders.find((entry) => entry.name === part);
    if (!child) {
      child = {
        kind: "folder",
        path: childPath,
        name: part,
        hasManagedAssets: false,
        isPriority: false,
        folders: [],
        files: [],
      };
      folder.folders.push(child);
    }
    folder = child;
  }
  return folder;
}

const displayName = (id: string) =>
  id.replaceAll("-", " ").replace(/\b\w/g, (letter) => letter.toUpperCase());

function sortFolder(folder: AssetTreeFolder, promoted: ReadonlySet<string>) {
  folder.folders.forEach((child) => sortFolder(child, promoted));
  folder.hasManagedAssets =
    folder.files.some((file) => Boolean(file.managed)) ||
    folder.folders.some((child) => child.hasManagedAssets);
  folder.isPriority =
    folder.hasManagedAssets ||
    promoted.has(folder.path) ||
    folder.folders.some((child) => child.isPriority);
  folder.folders.sort((a, b) => {
    if (a.isPriority !== b.isPriority) return a.isPriority ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
  folder.files.sort((a, b) => {
    if (Boolean(a.managed) !== Boolean(b.managed)) return a.managed ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
}
