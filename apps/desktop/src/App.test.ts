import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/vue";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App.vue";

const eventMock = vi.hoisted(() => ({
  handler: undefined as ((event: { payload: unknown }) => void) | undefined,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_event: string, handler: (event: { payload: unknown }) => void) => {
    eventMock.handler = handler;
    return () => undefined;
  }),
}));

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
  recipes: [],
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
  it("presents a keyboard-accessible draft state and disables revision-only actions", async () => {
    const commands: string[] = [];
    const draftProject = {
      ...project,
      assets: [{
        asset: {
          schema: "pixelpipe.asset/v2",
          id: "new-flare",
          kind: "sprite",
          state: "draft",
          brief: { schema: "pixelpipe.asset-brief/v1", text: "" },
        },
        revisions: [],
      }],
      recipes: [],
    };
    mockIPC((command) => {
      commands.push(command);
      if (command === "browse_project") return draftProject;
      if (command === "browse_agent_runs") return [];
      if (command === "update_asset_brief") return {
        ...draftProject.assets[0].asset,
        state: "awaiting_reference",
      };
      throw new Error(`unexpected command ${command}`);
    });

    render(App);
    await fireEvent.update(screen.getByLabelText("Project path"), "/game");
    await fireEvent.click(screen.getByRole("button", { name: "Open" }));
    expect(await screen.findByRole("heading", { name: "new-flare" })).toBeVisible();
    expect(screen.getByText(/Pre-revision asset · draft/)).toBeVisible();
    expect(screen.getByRole("button", { name: "Critique revision" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Propose refinement" })).toBeDisabled();
    expect(screen.queryByRole("button", { name: "Record review" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Create first revision" })).toBeDisabled();

    const brief = screen.getByLabelText("Project-owned brief");
    await fireEvent.update(brief, "Strict overhead synthetic flare");
    await fireEvent.submit(brief.closest("form")!);
    await waitFor(() => expect(commands).toContain("update_asset_brief"));
    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent("Revision history was unchanged"),
    );
  });

  it("opens verified project data, supports keyboard navigation, and records review without mutation", async () => {
    const commands: string[] = [];
    mockIPC((command) => {
      commands.push(command);
      if (command === "browse_project") return project;
      if (command === "browse_agent_runs") return [];
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

  it("starts agent work asynchronously and keeps a proposal unapplied until explicit revision submission", async () => {
    const commands: string[] = [];
    mockIPC((command) => {
      commands.push(command);
      if (command === "browse_project") return project;
      if (command === "browse_agent_runs") return [];
      if (command === "load_revision") return revision;
      if (command === "start_agent_task") return "task-1";
      throw new Error(`unexpected command ${command}`);
    });
    render(App);
    await fireEvent.update(screen.getByLabelText("Project path"), "/game");
    await fireEvent.click(screen.getByRole("button", { name: "Open" }));
    await screen.findByRole("heading", { name: "Reference and critique workflow" });
    await fireEvent.update(screen.getByLabelText("User profile"), "approved-fixture");
    await fireEvent.update(screen.getByLabelText("Brief or review prompt"), "Tighten the flare tip");
    await fireEvent.click(screen.getByRole("button", { name: "Propose refinement" }));

    await waitFor(() => expect(commands).toContain("start_agent_task"));
    expect(screen.getByRole("button", { name: "Cancel task" })).toBeVisible();
    eventMock.handler?.({
      payload: {
        schema: "pixelpipe.agent-task-event/v1",
        task: "task-1",
        sequence: 3,
        event: {
          type: "completed",
          run: {
            schema: "pixelpipe.agent-run/v1",
            id: "task-1",
            asset: "signal-flare",
            operation: "propose_refinement",
            revision: "r000002",
            profile: "approved-fixture",
            profile_command_sha256: "0".repeat(64),
            prompt: "Tighten the flare tip",
            started_unix_ms: 1,
            duration_ms: 20,
            status: "completed",
            exit_status: 0,
            stdout: "{}",
            stderr: "",
            candidates: [],
            proposal: {
              type: "pixel_patch",
              patch: { schema: "pixelpipe.patch/v1", edits: [{ x: 1, y: 1, index: 1 }] },
            },
          },
        },
      },
    });

    expect(await screen.findByText("Validated, unapplied pixel patch")).toBeVisible();
    expect(commands).not.toContain("patch_revision");
    await fireEvent.click(screen.getByRole("button", { name: "Load into editable form" }));
    expect(screen.getByRole("status")).toHaveTextContent("It has not been applied");
    expect(commands).not.toContain("patch_revision");
  });
});
