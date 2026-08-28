import { confirm, open, save } from "@tauri-apps/plugin-dialog";

export async function chooseProjectFolder(): Promise<string | undefined> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Open a game project folder",
  });
  return typeof selected === "string" ? selected : undefined;
}

export async function chooseReferenceImages(): Promise<string[]> {
  const selected = await open({
    multiple: true,
    title: "Import artwork",
    filters: [
      {
        name: "Images",
        extensions: ["png", "jpg", "jpeg", "webp"],
      },
    ],
  });
  if (Array.isArray(selected)) return selected;
  return typeof selected === "string" ? [selected] : [];
}

export async function chooseReferenceImage(): Promise<string | undefined> {
  return (await chooseReferenceImages())[0];
}

export async function chooseExportFile(
  defaultPath: string,
): Promise<string | undefined> {
  const selected = await save({
    title: "Export sprite at native resolution",
    defaultPath,
    filters: [
      { name: "PNG image", extensions: ["png"] },
      { name: "Lossless WebP image", extensions: ["webp"] },
    ],
  });
  return typeof selected === "string" ? selected : undefined;
}

export const confirmDeleteAsset = (asset: string, linked = false) =>
  confirm(
    linked
      ? `Remove “${asset}” from Pixelate and delete its revision history? The linked project image will remain untouched.`
      : `Delete unexported asset “${asset}” and all of its Pixelate revision history? No project image exists. This cannot be undone.`,
    {
      title: linked ? "Remove from Pixelate" : "Delete unexported asset",
      kind: "warning",
      okLabel: linked ? "Remove" : "Delete asset",
      cancelLabel: "Cancel",
    },
  );
