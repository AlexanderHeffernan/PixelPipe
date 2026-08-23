export type AssetKind = "sprite" | "sheet" | "tile" | "ui";
export type ReviewActorKind = "human" | "agent";
export type ReviewDecision = "reviewed" | "changes_requested" | "accepted";
export type Rgba = [number, number, number, number];

export interface ProjectManifest {
  schema: string;
  name: string;
  preview_scale: number;
  default_palette?: string;
}

export interface AssetManifest {
  schema: string;
  id: string;
  kind: AssetKind;
  head?: string;
  approved?: string;
}

export interface RevisionManifest {
  schema: string;
  id: string;
  asset: string;
  parent?: string;
  created_unix_ms: number;
  files: Record<string, string>;
}

export interface AssetBrowser {
  asset: AssetManifest;
  revisions: RevisionManifest[];
}

export interface ProjectBrowser {
  project_root: string;
  project: ProjectManifest;
  assets: AssetBrowser[];
}

export interface RasterBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface PaletteUsage {
  index: number;
  rgba: Rgba;
  count: number;
}

export interface RasterInspection {
  width: number;
  height: number;
  pivot?: [number, number];
  visible_bounds?: RasterBounds;
  visible_pixels: number;
  palette: PaletteUsage[];
  text_rows: string[];
}

export interface ValidationCheck {
  name: string;
  passed: boolean;
  detail: string;
}

export interface ReviewEvent {
  sequence: number;
  created_unix_ms: number;
  actor: string;
  actor_kind: ReviewActorKind;
  decision: ReviewDecision;
  note: string;
}

export interface ReviewRecord {
  schema: string;
  asset: string;
  revision: string;
  events: ReviewEvent[];
}

export interface RevisionViewMetadata {
  project_root: string;
  asset: string;
  revision: string;
  parent?: string;
  inspection: RasterInspection;
  palette_name: string;
  transparent_index: number;
  validation: {
    schema: string;
    valid: boolean;
    checks: ValidationCheck[];
    visual_review: "required" | "passed";
  };
  review?: ReviewRecord;
}

export interface RevisionViewResponse {
  metadata: RevisionViewMetadata;
  native_png_base64: string;
  preview_png_base64: string;
}

export interface PixelDifference {
  x: number;
  y: number;
  left_index?: number;
  right_index?: number;
  left_rgba?: Rgba;
  right_rgba?: Rgba;
}

export interface RevisionComparisonResponse {
  metadata: {
    project_root: string;
    asset: string;
    left: string;
    right: string;
    diff: {
      left_dimensions: [number, number];
      right_dimensions: [number, number];
      changed_bounds?: RasterBounds;
      changed_pixels: PixelDifference[];
      palette_differences: Array<{ index: number; left?: Rgba; right?: Rgba }>;
    };
    visual_native_sha256: string;
    visual_preview_sha256: string;
  };
  visual_native_png_base64: string;
  visual_preview_png_base64: string;
}

export interface RevisionResult {
  project_root: string;
  asset: string;
  revision: string;
  parent?: string;
  revision_path: string;
  native_sha256: string;
  preview_sha256: string;
  validation: string;
}

export interface PixelEdit {
  x: number;
  y: number;
  index: number;
}

export interface PaletteDraft {
  name: string;
  transparentIndex: number;
  colors: Rgba[];
  indexMap: number[];
}
