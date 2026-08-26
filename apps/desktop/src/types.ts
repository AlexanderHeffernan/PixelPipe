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
  display_name?: string;
  kind: AssetKind;
  state: "draft" | "awaiting_reference" | "selected_reference" | "revisioned";
  brief: { schema: string; text: string };
  selected_reference?: ReferenceSelection;
  head?: string;
  approved?: string;
  style?: AssetStyle;
}

export interface AssetStyle {
  recipe: string;
  palette?: string;
  color_count?: number;
  settings: ConversionSettings;
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
  recipes: ConversionRecipeDocument[];
  palettes: ProjectPalette[];
}

export interface ProjectPalette {
  id: string;
  palette: {
    schema: string;
    name: string;
    transparent_index: number;
    colors: Rgba[];
  };
}

export interface ConversionRecipeDocument {
  schema: string;
  id: string;
  kind: AssetKind;
  palette: string;
  preview_scale: number;
  mode:
    | { type: "reference"; settings: ConversionSettings }
    | { type: "sheet"; settings: SheetSettings };
}

export type BackdropPolicy =
  | { type: "alpha"; alpha_threshold: number }
  | {
      type: "border_connected";
      color: [number, number, number];
      tolerance: number;
      alpha_threshold: number;
    };

export type ColorTreatment = "original" | "warm" | "cool" | "vivid" | "muted";
export interface ColorAdjustments {
  brightness: number;
  contrast: number;
  saturation: number;
  warmth: number;
}

export interface ConversionSettings {
  width: number;
  height: number;
  color_treatment?: ColorTreatment;
  color_adjustments?: ColorAdjustments;
  margin: number;
  subject_scale_percent: number;
  offset_x: number;
  offset_y: number;
  coverage_percent: number;
  backdrop: BackdropPolicy;
  registration: "top" | "center" | "bottom";
  components: { min: number; max: number };
}

export interface CanvasSettings {
  width: number;
  height: number;
  scale_percent: number;
  offset_x: number;
  offset_y: number;
}

export interface SheetSettings {
  columns: number;
  rows: number;
  frame: ConversionSettings;
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
  palette: {
    schema: string;
    name: string;
    transparent_index: number;
    colors: Rgba[];
  };
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

export interface ConversionPreviewResponse {
  inspection: RasterInspection;
  palette_name: string;
  native_png_base64: string;
  background_removed?: boolean;
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

export interface PaletteColorOverride {
  index: number;
  rgba: Rgba;
}

export type AgentOperation =
  | "generate_references"
  | "critique_asset"
  | "propose_refinement";
export type AgentRunStatus = "completed" | "failed" | "cancelled";

export interface AgentCandidate {
  id: string;
  sha256: string;
  width: number;
  height: number;
  png: string;
}

export type AgentProposal =
  | { type: "pixel_patch"; patch: { schema: string; edits: PixelEdit[] } }
  | {
      type: "palette_remap";
      remap: {
        schema: string;
        palette: {
          schema: string;
          name: string;
          transparent_index: number;
          colors: Rgba[];
        };
        index_map: number[];
      };
    };

export interface AgentRunRecord {
  schema: string;
  id: string;
  asset: string;
  operation: AgentOperation;
  revision?: string;
  profile: string;
  profile_command_sha256: string;
  prompt: string;
  started_unix_ms: number;
  duration_ms: number;
  status: AgentRunStatus;
  exit_status?: number;
  adapter?: {
    adapter: string;
    provider?: string;
    model?: string;
    capabilities: AgentOperation[];
  };
  stdout: string;
  stderr: string;
  error?: string;
  candidates: AgentCandidate[];
  critique?: string;
  proposal?: AgentProposal;
}

export interface AgentTaskEvent {
  schema: string;
  task: string;
  sequence: number;
  event:
    | { type: "started"; operation: AgentOperation }
    | { type: "progress"; stage: string; message: string }
    | { type: "log"; stream: string; message: string }
    | { type: "candidate_ready"; candidate: AgentCandidate }
    | { type: "completed"; run: AgentRunRecord }
    | { type: "failed"; run?: AgentRunRecord; error: string }
    | { type: "cancelled"; run: AgentRunRecord };
}

export interface ReferenceSelection {
  schema: string;
  asset: string;
  run: string;
  candidate: string;
  sha256: string;
  selected_unix_ms: number;
}

export interface AgentConnector {
  id: "amp" | "codex";
  name: string;
  installed: boolean;
  approved: boolean;
}

export interface ExportResult {
  asset: string;
  revision: string;
  png: string;
  metadata: string;
}

export interface ExportFileResult {
  asset: string;
  revision: string;
  file: string;
  format: "png" | "webp";
  width: number;
  height: number;
}
