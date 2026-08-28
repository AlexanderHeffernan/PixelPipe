const FILE = "application/x-pixelate-file";
const FOLDER = "application/x-pixelate-folder";
const TEXT = "text/plain";
const PREFIX = "pixelate-drag:";

type DraggedItem = { path?: string; asset?: string; folder?: string };

function textPayload(item: DraggedItem) {
  return `${PREFIX}${JSON.stringify(item)}`;
}

export const beginFileDrag = (
  event: DragEvent,
  path: string,
  asset?: string,
) => {
  const transfer = event.dataTransfer;
  if (!transfer) return;
  try {
    transfer.setData(FILE, JSON.stringify({ path, asset }));
  } catch {
    // Older WebKit may reject custom MIME types; text/plain remains internal.
  }
  transfer.setData(TEXT, textPayload({ path, asset }));
  transfer.effectAllowed = "move";
};

export const beginFolderDrag = (event: DragEvent, path: string) => {
  const transfer = event.dataTransfer;
  if (!transfer) return;
  try {
    transfer.setData(FOLDER, path);
  } catch {
    // Older WebKit may reject custom MIME types; text/plain remains internal.
  }
  transfer.setData(TEXT, textPayload({ folder: path }));
  transfer.effectAllowed = "move";
};

export const acceptsAssetDrop = (event: DragEvent) => {
  const transfer = event.dataTransfer;
  return (
    hasType(transfer?.types, FILE) ||
    hasType(transfer?.types, FOLDER) ||
    (hasType(transfer?.types, TEXT) &&
      transfer?.getData(TEXT).startsWith(PREFIX))
  );
};

function hasType(types: readonly string[] | undefined, type: string) {
  const legacy = types as unknown as { contains?: (value: string) => boolean };
  return Array.from(types ?? []).includes(type) || legacy?.contains?.(type);
}

export const droppedItem = (event: DragEvent) => {
  const transfer = event.dataTransfer;
  const fallback = transfer?.getData(TEXT) || "";
  const rawFile = transfer?.getData(FILE);
  const rawFolder = transfer?.getData(FOLDER);
  const item = parseItem(
    rawFile ||
      rawFolder ||
      (fallback.startsWith(PREFIX) ? fallback.slice(PREFIX.length) : ""),
    Boolean(rawFolder),
  );
  return {
    asset: item.asset || "",
    image: item.path || "",
    folder: item.folder || "",
  };
};

function parseItem(raw: string, folder: boolean): DraggedItem {
  if (!raw) return {};
  if (folder) return { folder: raw };
  try {
    return JSON.parse(raw) as DraggedItem;
  } catch {
    return {};
  }
}

export const basename = (path: string) => path.split("/").at(-1) || path;
