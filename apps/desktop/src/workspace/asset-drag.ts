const FILE = "application/x-pixelate-file";
const FOLDER = "application/x-pixelate-folder";

export const beginFileDrag = (
  event: DragEvent,
  path: string,
  asset?: string,
) => {
  event.dataTransfer?.setData(FILE, JSON.stringify({ path, asset }));
  if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
};

export const beginFolderDrag = (event: DragEvent, path: string) => {
  event.dataTransfer?.setData(FOLDER, path);
  if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
};

export const acceptsAssetDrop = (event: DragEvent) =>
  event.dataTransfer?.types.includes(FILE) ||
  event.dataTransfer?.types.includes(FOLDER);

export const droppedItem = (event: DragEvent) => {
  const raw = event.dataTransfer?.getData(FILE);
  let file: { path?: string; asset?: string } = {};
  if (raw) {
    try {
      file = JSON.parse(raw) as typeof file;
    } catch {
      file = {};
    }
  }
  return {
    asset: file.asset || "",
    image: file.path || "",
    folder: event.dataTransfer?.getData(FOLDER) || "",
  };
};

export const basename = (path: string) => path.split("/").at(-1) || path;
