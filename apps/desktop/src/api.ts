import { invoke } from "@tauri-apps/api/core";
import type {
  AssetManifest,
  CanvasSettings,
  ConversionPreviewResponse,
  ConversionSettings,
  PaletteColorOverride,
  PaletteDraft,
  PixelEdit,
  ProjectBrowser,
  ProjectManifest,
  RevisionResult,
  RevisionViewResponse,
  ReferenceSelection,
  ExportResult,
  ExportFileResult,
} from "./types";

export const pngDataUrl = (base64: string): string =>
  `data:image/png;base64,${base64}`;

export const browseProject = (start: string) =>
  invoke<ProjectBrowser>("browse_project", { request: { start } });

export const openProject = (start: string) =>
  invoke<ProjectBrowser>("open_project", { request: { start } });

export const initializeAsset = (
  start: string,
  asset: string,
  brief: string,
  projectPath?: string,
) =>
  invoke<AssetManifest>("initialize_asset", {
    request: { start, asset, brief, project_path: projectPath ?? null },
  });

export const deleteAsset = (start: string, asset: string) =>
  invoke<void>("delete_asset", { request: { start, asset } });

export const updateAssetBrief = (start: string, asset: string, brief: string) =>
  invoke<AssetManifest>("update_asset_brief", {
    request: { start, asset, brief },
  });

export const renameAsset = (
  start: string,
  asset: string,
  displayName: string,
) =>
  invoke<AssetManifest>("rename_asset", {
    request: { start, asset, display_name: displayName },
  });

export const adoptProjectImage = (
  start: string,
  path: string,
  asset: string,
  brief: string,
  destination: string,
) =>
  invoke<AssetManifest>("adopt_project_image", {
    request: { start, path, asset, brief, destination },
  });

export const adoptPixelArt = (
  start: string,
  path: string,
  asset: string,
  brief: string,
) =>
  invoke<RevisionResult>("adopt_pixel_art", {
    request: { start, path, asset, brief, actor: "user" },
  });

export const setProjectImageIgnored = (
  start: string,
  path: string,
  ignored: boolean,
) =>
  invoke<ProjectManifest>("set_project_image_ignored", {
    request: { start, path, ignored },
  });

export const relinkAsset = (start: string, asset: string, path: string) =>
  invoke<AssetManifest>("relink_asset", {
    request: { start, asset, path },
  });

export const updateLinkedSource = (start: string, asset: string) =>
  invoke<AssetManifest>("update_linked_source", {
    request: { start, asset },
  });

export const createFolder = (start: string, path: string) =>
  invoke<void>("create_folder", { request: { start, path } });

export const moveFolder = (
  start: string,
  source: string,
  destination: string,
) =>
  invoke<AssetManifest[]>("move_folder", {
    request: { start, source, destination },
  });

export const deleteFolder = (start: string, path: string) =>
  invoke<void>("delete_folder", { request: { start, path } });

export const moveAsset = (start: string, asset: string, destination: string) =>
  invoke<AssetManifest>("move_asset", {
    request: { start, asset, destination },
  });

export const loadProjectImage = (start: string, path: string) =>
  invoke<string>("load_project_image", { request: { start, path } });

export const importReference = (start: string, asset: string, file: string) =>
  invoke<ReferenceSelection>("import_reference", {
    request: { start, asset, file },
  });

export const exportAsset = (
  start: string,
  asset: string,
  destination: string,
  overwrite: boolean,
) =>
  invoke<ExportResult>("export_asset", {
    request: { start, asset, destination, overwrite },
  });

export const exportAssetFile = (
  start: string,
  asset: string,
  destination: string,
  overwrite: boolean,
) =>
  invoke<ExportFileResult>("export_asset_file", {
    request: { start, asset, destination, overwrite },
  });

export const convertSelectedReference = (
  start: string,
  asset: string,
  colorCount: number | undefined,
  paletteOverrides: PaletteColorOverride[],
  settings: ConversionSettings | undefined,
  autoBackground: boolean,
  actor: string,
) =>
  invoke<RevisionResult>("convert_selected_reference", {
    request: {
      start,
      asset,
      color_count: colorCount ?? null,
      palette_overrides: paletteOverrides,
      settings: settings ?? null,
      auto_background: autoBackground,
      actor,
    },
  });

export const previewSelectedReference = (
  start: string,
  asset: string,
  colorCount: number | undefined,
  paletteOverrides: PaletteColorOverride[],
  settings: ConversionSettings | undefined,
  autoBackground: boolean,
) =>
  invoke<ConversionPreviewResponse>("preview_selected_reference", {
    request: {
      start,
      asset,
      color_count: colorCount ?? null,
      palette_overrides: paletteOverrides,
      settings: settings ?? null,
      auto_background: autoBackground,
    },
  });

export const previewComposition = (
  start: string,
  asset: string,
  parent: string,
  settings: CanvasSettings,
) =>
  invoke<ConversionPreviewResponse>("preview_composition", {
    request: { start, asset, parent, settings },
  });

export const commitComposition = (
  start: string,
  asset: string,
  parent: string,
  settings: CanvasSettings,
  actor: string,
) =>
  invoke<RevisionResult>("commit_composition", {
    request: { start, asset, parent, settings, actor },
  });

export const recentProject = () => invoke<string | null>("recent_project");

export const rememberProject = (path: string) =>
  invoke<void>("remember_project", { path });

export type CliInstallState =
  | "installed"
  | "not_installed"
  | "needs_repair"
  | "conflict"
  | "unavailable";

export interface CliInstallStatus {
  state: CliInstallState;
  command: string;
  managed: boolean;
}

export const cliInstallationStatus = () =>
  invoke<CliInstallStatus>("cli_installation_status");

export const installCli = () => invoke<CliInstallStatus>("install_cli");

export const uninstallCli = () => invoke<CliInstallStatus>("uninstall_cli");

export const startTerminal = (
  session: string,
  cwd: string,
  cols: number,
  rows: number,
) => invoke<void>("start_terminal", { session, cwd, cols, rows });

export const writeTerminal = (session: string, data: string) =>
  invoke<void>("write_terminal", { session, data });

export const resizeTerminal = (session: string, cols: number, rows: number) =>
  invoke<void>("resize_terminal", { session, cols, rows });

export const closeTerminal = (session: string) =>
  invoke<void>("close_terminal", { session });

export const loadRevision = (start: string, asset: string, revision?: string) =>
  invoke<RevisionViewResponse>("load_revision", {
    request: { start, asset, revision: revision ?? null },
  });

export const patchRevision = (
  start: string,
  asset: string,
  parent: string,
  edits: PixelEdit[],
  actor: string,
) =>
  invoke<RevisionResult>("patch_revision", {
    request: {
      start,
      asset,
      parent,
      patch: { schema: "pixelate.patch/v1", edits },
      brief: null,
      actor,
    },
  });

export const fillRevision = (
  start: string,
  asset: string,
  parent: string,
  x: number,
  y: number,
  index: number,
  actor: string,
) =>
  invoke<RevisionResult>("fill_revision", {
    request: { start, asset, parent, x, y, index, actor },
  });

export const setAssetHead = (start: string, asset: string, revision: string) =>
  invoke<AssetManifest>("set_asset_head", {
    request: { start, asset, revision },
  });

export const remapRevision = (
  start: string,
  asset: string,
  parent: string,
  draft: PaletteDraft,
  actor: string,
) =>
  invoke<RevisionResult>("remap_revision", {
    request: {
      start,
      asset,
      parent,
      remap: {
        schema: "pixelate.palette-remap/v1",
        palette: {
          schema: "pixelate.palette/v1",
          name: draft.name,
          transparent_index: draft.transparentIndex,
          colors: draft.colors,
        },
        index_map: draft.indexMap,
      },
      brief: null,
      actor,
    },
  });
