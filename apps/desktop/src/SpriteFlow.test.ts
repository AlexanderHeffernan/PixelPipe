import { cleanup, fireEvent, render, screen } from "@testing-library/vue";
import { afterEach, expect, it, vi } from "vitest";
import App from "./App.vue";
import * as api from "./api";

vi.mock("./services/dialogs", () => ({
  chooseProjectFolder: vi.fn(async () => "/game"),
  chooseReferenceImage: vi.fn(async () => undefined),
  chooseExportFolder: vi.fn(async () => "/game/assets"),
  confirmAgentConnector: vi.fn(async () => true),
  confirmExport: vi.fn(async () => true),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => undefined),
}));
vi.mock("./api", async (original) => ({
  ...(await original<typeof import("./api")>()),
}));

const reference = {
  schema: "pixelpipe.reference-selection/v1",
  asset: "field-medic",
  run: "import",
  candidate: "local-file",
  sha256: "0".repeat(64),
  selected_unix_ms: 1,
};
const project = {
  project_root: "/game",
  project: {
    schema: "pixelpipe.project/v1",
    name: "Fixture Game",
    preview_scale: 8,
  },
  assets: [
    {
      asset: {
        schema: "pixelpipe.asset/v2",
        id: "field-medic",
        kind: "sprite" as const,
        state: "selected_reference" as const,
        brief: {
          schema: "pixelpipe.asset-brief/v1",
          text: "Strict overhead field medic",
        },
        selected_reference: reference,
      },
      revisions: [],
    },
  ],
  recipes: [16, 32, 64].map((size) => ({
    schema: "pixelpipe.conversion-recipe/v1",
    id: `sprite-${size}`,
    kind: "sprite" as const,
    palette: "starter",
    preview_scale: 8,
    mode: { type: "reference" as const, settings: {} },
  })),
};
const revisioned = {
  ...project,
  assets: [
    {
      asset: {
        ...project.assets[0].asset,
        state: "revisioned" as const,
        head: "r000001",
      },
      revisions: [],
    },
  ],
};

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

it("pixelizes, reviews, and exports through the visible workflow", async () => {
  vi.spyOn(api, "openProject").mockResolvedValue(project);
  vi.spyOn(api, "detectAgentConnectors").mockResolvedValue([]);
  vi.spyOn(api, "browseAgentRuns").mockResolvedValue([]);
  vi.spyOn(api, "convertSelectedReference").mockResolvedValue({
    project_root: "/game",
    asset: "field-medic",
    revision: "r000001",
    revision_path: "/game/.pixelpipe/assets/field-medic/revisions/r000001",
    native_sha256: "1".repeat(64),
    preview_sha256: "2".repeat(64),
    validation: "valid_visual_review_required",
  });
  vi.spyOn(api, "browseProject").mockResolvedValue(revisioned);
  vi.spyOn(api, "loadRevision").mockResolvedValue({
    metadata: {
      project_root: "/game",
      asset: "field-medic",
      revision: "r000001",
      inspection: {
        width: 32,
        height: 32,
        visible_pixels: 100,
        palette: [{ index: 1, rgba: [1, 2, 3, 255], count: 100 }],
        text_rows: [],
      },
      palette_name: "starter",
      transparent_index: 0,
      validation: {
        schema: "pixelpipe.validation/v1",
        valid: true,
        checks: [],
        visual_review: "required",
      },
    },
    native_png_base64: "png",
    preview_png_base64: "preview",
  });
  vi.spyOn(api, "exportAsset").mockResolvedValue({
    asset: "field-medic",
    revision: "r000001",
    png: "/game/assets/field-medic.png",
    metadata: "/game/assets/field-medic.json",
  });

  render(App);
  await fireEvent.click(
    screen.getByRole("button", { name: "Open Project Folder…" }),
  );
  await fireEvent.click(
    await screen.findByRole("button", { name: "Create Sprite" }),
  );
  expect(
    await screen.findByRole("heading", {
      name: "Does it read at native size?",
    }),
  ).toBeVisible();
  await fireEvent.click(screen.getByRole("button", { name: "Export Sprite" }));
  await fireEvent.click(
    screen.getByRole("button", { name: "Choose Export Folder…" }),
  );
  expect(api.convertSelectedReference).toHaveBeenCalledWith(
    "/game",
    "field-medic",
    "sprite-32",
    "user",
  );
  expect(api.exportAsset).toHaveBeenCalledWith(
    "/game",
    "field-medic",
    "/game/assets",
    true,
  );
});
