import { invoke } from "@tauri-apps/api/core";
import type {
  AgentOperation,
  AgentConnector,
  AssetKind,
  AssetManifest,
  AgentRunRecord,
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

export const updateAssetBrief = (start: string, asset: string, brief: string) =>
  invoke<AssetManifest>("update_asset_brief", {
    request: { start, asset, brief },
  });

export const importReference = (start: string, asset: string, file: string) =>
  invoke<ReferenceSelection>("import_reference", {
    request: { start, asset, file },
  });

export const detectAgentConnectors = () =>
  invoke<AgentConnector[]>("detect_agent_connectors");

export const approveAgentConnector = (id: string) =>
  invoke<AgentConnector>("approve_agent_connector", { request: { id } });

export const exportAsset = (
  start: string,
  asset: string,
  destination: string,
  overwrite: boolean,
) =>
  invoke<ExportResult>("export_asset", {
    request: { start, asset, destination, overwrite },
  });

export const convertSelectedReference = (
  start: string,
  asset: string,
  recipe: string,
  actor: string,
) =>
  invoke<RevisionResult>("convert_selected_reference", {
    request: { start, asset, recipe, actor },
  });

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

export const startAgentTask = (
  start: string,
  asset: string,
  profile: string,
  operation: AgentOperation,
  prompt: string,
  revision?: string,
) =>
  invoke<string>("start_agent_task", {
    request: {
      start,
      asset,
      profile,
      operation,
      revision: revision ?? null,
      prompt,
    },
  });

export const cancelAgentTask = (task: string) =>
  invoke<void>("cancel_agent_task", { task });

export const browseAgentRuns = (start: string, asset: string) =>
  invoke<AgentRunRecord[]>("browse_agent_runs", { request: { start, asset } });

export const loadAgentCandidate = (
  start: string,
  run: string,
  candidate: string,
) =>
  invoke<{ png_base64: string }>("load_agent_candidate", {
    request: { start, run, candidate },
  });

export const selectAgentCandidate = (
  start: string,
  asset: string,
  run: string,
  candidate: string,
) =>
  invoke<ReferenceSelection>("select_agent_candidate", {
    request: { start, asset, run, candidate },
  });
