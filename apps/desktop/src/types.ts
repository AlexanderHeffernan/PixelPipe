export type Rgba = [number, number, number, number];

export interface ProjectManifest {
  schema: string;
  name: string;
  ignored_project_images: string[];
}

export interface AssetManifest {
  schema: string;
  id: string;
  display_name?: string;
  project_path?: string;
  project_file_sha256?: string;
  brief: { schema: string; text: string };
  selected_reference?: ReferenceSelection;
  head?: string;
  style?: AssetStyle;
}

export interface AssetStyle {
  color_count: number;
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
  catalog: CatalogEntry[];
  pixelization: PixelizationDefaults;
}

export interface CatalogEntry {
  path: string;
  asset_id?: string;
  status: "current" | "modified" | "missing" | "unexported";
}

export interface PixelizationDefaults {
  color_count: number;
  settings: ConversionSettings;
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
  transparent_index: number;
  validation: {
    schema: string;
    valid: boolean;
    checks: ValidationCheck[];
  };
}

export interface RevisionViewResponse {
  metadata: RevisionViewMetadata;
  native_png_base64: string;
}

export interface ConversionPreviewResponse {
  inspection: RasterInspection;
  palette_name: string;
  native_png_base64: string;
  background_removed?: boolean;
}

export interface RevisionResult {
  project_root: string;
  asset: string;
  revision: string;
  parent?: string;
  revision_path: string;
  native_sha256: string;
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

export interface ReferenceSelection {
  sha256: string;
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
