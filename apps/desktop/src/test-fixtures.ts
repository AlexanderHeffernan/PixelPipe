import type {
  ConversionSettings,
  ProjectBrowser,
  RevisionViewResponse,
} from "./types";

export const settings: ConversionSettings = {
  width: 32,
  height: 32,
  margin: 1,
  subject_scale_percent: 100,
  offset_x: 0,
  offset_y: 0,
  coverage_percent: 35,
  backdrop: {
    type: "border_connected",
    color: [255, 255, 255],
    tolerance: 28,
    alpha_threshold: 8,
  },
  registration: "center",
  components: { min: 1, max: 8 },
};

export const reference = {
  sha256: "0".repeat(64),
};

export const project: ProjectBrowser = {
  project_root: "/game",
  project: {
    schema: "pixelate.project/v1",
    name: "Fixture Game",
  },
  assets: [
    {
      asset: {
        schema: "pixelate.asset/v2",
        id: "field-medic",
        brief: {
          schema: "pixelate.asset-brief/v1",
          text: "Strict overhead field medic",
        },
        selected_reference: reference,
      },
      revisions: [],
    },
  ],
  catalog: [],
  pixelization: { color_count: 16, settings },
};

export const revisionView: RevisionViewResponse = {
  metadata: {
    project_root: "/game",
    asset: "field-medic",
    revision: "r000001",
    inspection: {
      width: 32,
      height: 32,
      visible_pixels: 100,
      palette: [{ index: 1, rgba: [38, 44, 62, 255], count: 100 }],
      text_rows: [],
    },
    palette: {
      schema: "pixelate.palette/v1",
      name: "starter",
      transparent_index: 0,
      colors: [
        [0, 0, 0, 0],
        [38, 44, 62, 255],
      ],
    },
    transparent_index: 0,
    validation: {
      schema: "pixelate.validation/v1",
      valid: true,
      checks: [],
    },
  },
  native_png_base64: "native",
};

export const preview = {
  inspection: revisionView.metadata.inspection,
  palette_name: "starter",
  native_png_base64: "preview-native",
  background_removed: true,
};
