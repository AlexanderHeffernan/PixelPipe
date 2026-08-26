import { invoke } from "@tauri-apps/api/core";
import type {
  AssetKind,
  AssetManifest,
  CanvasSettings,
  ConversionPreviewResponse,
  ConversionSettings,
  PaletteColorOverride,
  PaletteDraft,
  PixelEdit,
  ProjectBrowser,
  RevisionComparisonResponse,
  RevisionResult,
  RevisionViewResponse,
  ReviewActorKind,
  ReviewDecision,
  ReviewRecord,
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
  kind: AssetKind,
  brief: string,
) =>
  invoke<AssetManifest>("initialize_asset", {
    request: { start, asset, kind, brief },
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
  recipe: string,
  palette: string | undefined,
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
      recipe,
      palette: palette ?? null,
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
  recipe: string,
  palette: string | undefined,
  colorCount: number | undefined,
  paletteOverrides: PaletteColorOverride[],
  settings: ConversionSettings | undefined,
  autoBackground: boolean,
) =>
  invoke<ConversionPreviewResponse>("preview_selected_reference", {
    request: {
      start,
      asset,
      recipe,
      palette: palette ?? null,
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

export const storeProjectPalette = (start: string, id: string, file: string) =>
  invoke<unknown>("store_project_palette", { request: { start, id, file } });

export const storeProjectRecipe = (start: string, file: string) =>
  invoke<unknown>("store_project_recipe", { request: { start, file } });

export const loadRevision = (start: string, asset: string, revision?: string) =>
  invoke<RevisionViewResponse>("load_revision", {
    request: { start, asset, revision: revision ?? null },
  });

export const compareRevisions = (
  start: string,
  asset: string,
  left: string,
  right: string,
) =>
  invoke<RevisionComparisonResponse>("compare_revisions", {
    request: { start, asset, left, right, preview_scale: null },
  });

export const recordReview = (
  start: string,
  asset: string,
  revision: string,
  actor: string,
  actorKind: ReviewActorKind,
  decision: ReviewDecision,
  note: string,
) =>
  invoke<ReviewRecord>("record_review", {
    request: {
      start,
      asset,
      revision,
      actor,
      actor_kind: actorKind,
      decision,
      note,
    },
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
      patch: { schema: "pixelpipe.patch/v1", edits },
      brief: null,
      preview_scale: null,
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
        schema: "pixelpipe.palette-remap/v1",
        palette: {
          schema: "pixelpipe.palette/v1",
          name: draft.name,
          transparent_index: draft.transparentIndex,
          colors: draft.colors,
        },
        index_map: draft.indexMap,
      },
      brief: null,
      preview_scale: null,
      actor,
    },
  });
