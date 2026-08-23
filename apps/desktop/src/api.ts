import { invoke } from "@tauri-apps/api/core";
import type {
  PaletteDraft,
  PixelEdit,
  ProjectBrowser,
  RevisionComparisonResponse,
  RevisionResult,
  RevisionViewResponse,
  ReviewActorKind,
  ReviewDecision,
  ReviewRecord,
} from "./types";

export const pngDataUrl = (base64: string): string => `data:image/png;base64,${base64}`;

export const browseProject = (start: string) =>
  invoke<ProjectBrowser>("browse_project", { request: { start } });

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
    request: { start, asset, revision, actor, actor_kind: actorKind, decision, note },
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
