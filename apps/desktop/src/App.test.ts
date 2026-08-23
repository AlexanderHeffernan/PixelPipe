import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/vue";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App.vue";

const project = {
  project_root: "/game",
  project: { schema: "pixelpipe.project/v1", name: "Fixture Game", preview_scale: 8 },
  assets: [
    {
      asset: { schema: "pixelpipe.asset/v1", id: "signal-flare", kind: "sprite", head: "r000002" },
      revisions: [
        { schema: "pixelpipe.revision/v1", id: "r000001", asset: "signal-flare", created_unix_ms: 1, files: {} },
        { schema: "pixelpipe.revision/v1", id: "r000002", asset: "signal-flare", parent: "r000001", created_unix_ms: 2, files: {} },
      ],
    },
    {
      asset: { schema: "pixelpipe.asset/v1", id: "zombie", kind: "sprite", head: "r000001" },
      revisions: [
        { schema: "pixelpipe.revision/v1", id: "r000001", asset: "zombie", created_unix_ms: 1, files: {} },
      ],
    },
  ],
};

const revision = {
  metadata: {
    project_root: "/game",
    asset: "signal-flare",
    revision: "r000002",
    parent: "r000001",
    inspection: {
      width: 2,
      height: 2,
      pivot: [1, 2],
      visible_bounds: { x: 0, y: 0, width: 2, height: 2 },
      visible_pixels: 3,
      palette: [
        { index: 0, rgba: [0, 0, 0, 0], count: 1 },
        { index: 1, rgba: [220, 40, 20, 255], count: 3 },
      ],
      text_rows: ["-- 01", "01 01"],
    },
    palette_name: "fixture",
    transparent_index: 0,
    validation: {
      schema: "pixelpipe.validation/v1",
      valid: true,
      visual_review: "required",
      checks: [{ name: "dimensions", passed: true, detail: "2x2" }],
    },
  },
  native_png_base64: "aW1hZ2U=",
  preview_png_base64: "cHJldmlldw==",
};

afterEach(() => {
  cleanup();
  clearMocks();
});

describe("desktop review workstation", () => {
  it("opens verified project data, supports keyboard navigation, and records review without mutation", async () => {
    const commands: string[] = [];
    mockIPC((command) => {
      commands.push(command);
      if (command === "browse_project") return project;
      if (command === "load_revision") return revision;
      if (command === "record_review") {
        return {
          schema: "pixelpipe.review/v1",
          asset: "signal-flare",
          revision: "r000002",
          events: [{ sequence: 1, created_unix_ms: 3, actor: "user", actor_kind: "human", decision: "reviewed", note: "" }],
        };
      }
      throw new Error(`unexpected command ${command}`);
    });

    render(App);
    await fireEvent.update(screen.getByLabelText("Project path"), "/game/subdirectory");
    await fireEvent.click(screen.getByRole("button", { name: "Open" }));

    expect(await screen.findByRole("img", { name: /signal-flare r000002 at native size/ })).toHaveAttribute(
      "src",
      "data:image/png;base64,aW1hZ2U=",
    );
    expect(screen.getAllByText("2×2")).toHaveLength(2);
    expect(screen.getByText("Visual review:").parentElement).toHaveTextContent("required");
    await waitFor(() => expect(screen.getByRole("button", { name: "Open" })).toBeEnabled());

    const flare = screen.getByRole("button", { name: "signal-flare, sprite" });
    flare.focus();
    await fireEvent.keyDown(flare.parentElement!, { key: "ArrowDown" });
    expect(screen.getByRole("button", { name: "zombie, sprite" })).toHaveFocus();

    await fireEvent.click(screen.getByRole("button", { name: "Record review" }));
    await waitFor(() => expect(commands).toContain("record_review"));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Head was unchanged"));
    expect(commands).not.toContain("patch_revision");
    expect(commands).not.toContain("remap_revision");
  });

  it("announces command failures and moves focus to the error status", async () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    mockIPC(() => {
      throw new Error("no .pixelpipe/project.toml found");
    });
    render(App);
    await fireEvent.update(screen.getByLabelText("Project path"), "/missing");
    await fireEvent.click(screen.getByRole("button", { name: "Open" }));
    const status = await screen.findByRole("status");
    await waitFor(() => expect(status).toHaveFocus());
    expect(status).toHaveTextContent("no .pixelpipe/project.toml found");
  });
});
