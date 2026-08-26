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
        extensions: [
          "png",
          "jpg",
          "jpeg",
          "webp",
          "gif",
          "bmp",
          "tif",
          "tiff",
          "ico",
          "pnm",
        ],
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

export const confirmAgentConnector = (name: string) =>
  confirm(
    `PixelPipe will run your installed ${name} CLI for generation and critique. The executable is stored only in your user settings and is never selected by a project. Continue?`,
    {
      title: `Connect ${name}`,
      kind: "warning",
      okLabel: `Connect ${name}`,
      cancelLabel: "Cancel",
    },
  );

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
