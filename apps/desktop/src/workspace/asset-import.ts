import type { Ref } from "vue";
import * as api from "../api";
import { chooseReferenceImages } from "../services/dialogs";
import type { ProjectBrowser } from "../types";

interface AssetImportContext {
  project: Ref<ProjectBrowser | undefined>;
  importing: Ref<boolean>;
  run: (action: () => Promise<void>) => Promise<void>;
  refresh: () => Promise<void>;
  selectAsset: (id: string) => Promise<void>;
  notice: (message: string) => void;
}

export function createAssetImport(context: AssetImportContext) {
  return async function importAssets() {
    if (!context.project.value) return;
    const files = await chooseReferenceImages();
    if (!files.length) return;
    context.importing.value = true;
    try {
      await context.run(async () => {
        let selected = "";
        const existing = context.project.value!.assets.map(
          ({ asset }) => asset.id,
        );
        for (const file of files) {
          const name = fileName(file);
          const id = availableAssetId(name, existing);
          await api.initializeAsset(
            context.project.value!.project_root,
            id,
            "sprite",
            name,
          );
          try {
            await api.importReference(
              context.project.value!.project_root,
              id,
              file,
            );
          } catch (caught) {
            await api
              .deleteAsset(context.project.value!.project_root, id)
              .catch(() => undefined);
            throw caught;
          }
          existing.push(id);
          selected = id;
        }
        await context.refresh();
        if (selected) await context.selectAsset(selected);
        context.notice(
          files.length === 1
            ? "Asset imported"
            : `${files.length} assets imported`,
        );
      });
    } finally {
      context.importing.value = false;
    }
  };
}

const slug = (value: string) =>
  value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
const fileName = (path: string) =>
  path
    .split(/[\\/]/)
    .at(-1)
    ?.replace(/\.[^.]+$/, "") || "Imported Asset";
function availableAssetId(name: string, existing: string[]) {
  const base = slug(name) || "imported-asset";
  let id = base;
  let suffix = 2;
  while (existing.includes(id)) id = `${base}-${suffix++}`;
  return id;
}
