import { confirm, open } from "@tauri-apps/plugin-dialog";

export async function chooseProjectFolder(): Promise<string | undefined> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Open a game project folder",
  });
  return typeof selected === "string" ? selected : undefined;
}

export async function chooseReferenceImage(): Promise<string | undefined> {
  const selected = await open({
    multiple: false,
    title: "Choose a smooth reference image",
    filters: [{ name: "PNG image", extensions: ["png"] }],
  });
  return typeof selected === "string" ? selected : undefined;
}

export async function chooseExportFolder(): Promise<string | undefined> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Export game-ready sprite",
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

export const confirmExport = (asset: string) =>
  confirm(
    `Export ${asset}.png and ${asset}.json? Existing files with those names will be replaced.`,
    {
      title: "Export Sprite",
      kind: "info",
      okLabel: "Export",
      cancelLabel: "Cancel",
    },
  );
