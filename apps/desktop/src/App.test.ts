import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App.vue";
import * as api from "./api";

const dialog = vi.hoisted(() => ({
  project: "/game",
  reference: undefined as string | undefined,
}));
vi.mock("./services/dialogs", () => ({
  chooseProjectFolder: vi.fn(async () => dialog.project),
  chooseReferenceImage: vi.fn(async () => dialog.reference),
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

const brief = {
  schema: "pixelpipe.asset-brief/v1",
  text: "Strict overhead field medic",
};
const draftProject = {
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
        state: "awaiting_reference" as const,
        brief,
      },
      revisions: [],
    },
  ],
  recipes: [
    {
      schema: "pixelpipe.conversion-recipe/v1",
      id: "sprite-32",
      kind: "sprite" as const,
      palette: "starter",
      preview_scale: 8,
      mode: { type: "reference" as const, settings: {} },
    },
  ],
};

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  dialog.reference = undefined;
});

function mockProject(project = draftProject) {
  vi.spyOn(api, "openProject").mockResolvedValue(project);
  vi.spyOn(api, "detectAgentConnectors").mockResolvedValue([
    { id: "amp", name: "Amp", installed: true, approved: false },
    { id: "codex", name: "Codex", installed: false, approved: false },
  ]);
  vi.spyOn(api, "browseAgentRuns").mockResolvedValue([]);
}

describe("opinionated sprite workflow", () => {
  it("opens a game folder without asking for paths or JSON", async () => {
    mockProject();
    render(App);
    expect(
      screen.getByRole("heading", { name: /Make game-ready sprites/ }),
    ).toBeVisible();
    await fireEvent.click(
      screen.getByRole("button", { name: "Open Project Folder…" }),
    );
    expect(await screen.findByText("Fixture Game")).toBeVisible();
    expect(
      screen.getByRole("heading", { name: "Generate smooth references" }),
    ).toBeVisible();
    expect(api.openProject).toHaveBeenCalledWith("/game");
  });

  it("imports a PNG and advances to deterministic conversion", async () => {
    mockProject();
    dialog.reference = "/game/medic.png";
    vi.spyOn(api, "importReference").mockResolvedValue({
      schema: "pixelpipe.reference-selection/v1",
      asset: "field-medic",
      run: "import",
      candidate: "medic",
      sha256: "0".repeat(64),
      selected_unix_ms: 1,
    });
    vi.spyOn(api, "browseProject").mockResolvedValue({
      ...draftProject,
      assets: [
        {
          ...draftProject.assets[0],
          asset: {
            ...draftProject.assets[0].asset,
            state: "selected_reference",
            selected_reference: {
              schema: "pixelpipe.reference-selection/v1",
              asset: "field-medic",
              run: "import",
              candidate: "medic",
              sha256: "0".repeat(64),
              selected_unix_ms: 1,
            },
          },
        },
      ],
    });
    render(App);
    await fireEvent.click(
      screen.getByRole("button", { name: "Open Project Folder…" }),
    );
    await fireEvent.click(
      await screen.findByRole("button", { name: "Import PNG…" }),
    );
    expect(
      await screen.findByRole("heading", { name: "Choose the sprite size" }),
    ).toBeVisible();
    expect(api.importReference).toHaveBeenCalledWith(
      "/game",
      "field-medic",
      "/game/medic.png",
    );
  });

  it("requires explicit consent before using an installed agent", async () => {
    mockProject();
    vi.spyOn(api, "approveAgentConnector").mockResolvedValue({
      id: "amp",
      name: "Amp",
      installed: true,
      approved: true,
    });
    render(App);
    await fireEvent.click(
      screen.getByRole("button", { name: "Open Project Folder…" }),
    );
    await fireEvent.click(
      await screen.findByRole("button", { name: "Connect Amp" }),
    );
    await waitFor(() =>
      expect(api.approveAgentConnector).toHaveBeenCalledWith("amp"),
    );
  });
});
