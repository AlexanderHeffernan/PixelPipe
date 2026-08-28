const ASSET = "application/x-pixelate-asset";
const FOLDER = "application/x-pixelate-folder";

export const beginAssetDrag = (event: DragEvent, id: string) => {
  event.dataTransfer?.setData(ASSET, id);
  if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
};

export const beginFolderDrag = (event: DragEvent, path: string) => {
  event.dataTransfer?.setData(FOLDER, path);
  if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
};

export const acceptsAssetDrop = (event: DragEvent) =>
  event.dataTransfer?.types.includes(ASSET) ||
  event.dataTransfer?.types.includes(FOLDER);

export const droppedItem = (event: DragEvent) => ({
  asset: event.dataTransfer?.getData(ASSET) || "",
  folder: event.dataTransfer?.getData(FOLDER) || "",
});

export const basename = (path: string) => path.split("/").at(-1) || path;
