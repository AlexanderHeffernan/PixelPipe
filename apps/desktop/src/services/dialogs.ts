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

export async function chooseFrameImage(): Promise<string | undefined> {
  const selected = await open({
    multiple: false,
    title: "Add a reference image or pixel art frame",
    filters: [
      {
        name: "Reference or pixel art image",
        extensions: ["png", "jpg", "jpeg", "webp"],
      },
    ],
  });
  return typeof selected === "string" ? selected : undefined;
}

export async function chooseExportFile(
  asset: string,
): Promise<string | undefined> {
  const selected = await save({
    title: "Export sprite at native resolution",
    defaultPath: `${asset}.png`,
    filters: [
      { name: "PNG image", extensions: ["png"] },
      { name: "Lossless WebP image", extensions: ["webp"] },
    ],
  });
  return typeof selected === "string" ? selected : undefined;
}

export const confirmDeleteAsset = (asset: string) =>
  confirm(
    `Delete “${asset}” and all of its revision history? This cannot be undone.`,
    {
      title: "Delete Asset",
      kind: "warning",
      okLabel: "Delete",
      cancelLabel: "Cancel",
    },
  );

export const confirmReplaceAnimationWithPixelization = (frames: number) =>
  confirm(
    `Re-pixelizing starts a new one-frame version and replaces the current ${frames}-frame animation as the active revision. The animation remains in revision history, but it will no longer be the version shown or exported. Continue?`,
    {
      title: "Return to Pixelize?",
      kind: "warning",
      okLabel: "Return to Pixelize",
      cancelLabel: "Keep Animation",
    },
  );
