import { computed, inject, onMounted, onUnmounted, provide, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import * as api from "../api";
import {
  chooseExportFolder,
  chooseProjectFolder,
  chooseReferenceImage,
  confirmAgentConnector,
  confirmExport,
} from "../services/dialogs";
import type {
  AgentConnector,
  AgentRunRecord,
  AgentTaskEvent,
  ProjectBrowser,
  RevisionViewResponse,
} from "../types";

export type WorkspaceStage =
  | "brief"
  | "reference"
  | "pixelize"
  | "review"
  | "export";

export function createWorkspace() {
  const project = ref<ProjectBrowser>();
  const assetId = ref("");
  const stage = ref<WorkspaceStage>("brief");
  const view = ref<RevisionViewResponse>();
  const connectors = ref<AgentConnector[]>([]);
  const activeRun = ref<AgentRunRecord>();
  const candidateImages = ref<Record<string, string>>({});
  const busy = ref(false);
  const agentBusy = ref(false);
  const agentTask = ref("");
  const creatingAsset = ref(false);
  const error = ref("");
  const notice = ref("");
  let unlisten: UnlistenFn | undefined;

  const selectedAsset = computed(() =>
    project.value?.assets.find(({ asset }) => asset.id === assetId.value),
  );
  const selectedReference = computed(
    () => selectedAsset.value?.asset.selected_reference,
  );
  const recipes = computed(
    () =>
      project.value?.recipes.filter(
        ({ kind }) => kind === selectedAsset.value?.asset.kind,
      ) ?? [],
  );

  async function run(action: () => Promise<void>) {
    busy.value = true;
    error.value = "";
    try {
      await action();
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : String(caught);
    } finally {
      busy.value = false;
    }
  }

  async function refresh() {
    if (!project.value) return;
    project.value = await api.browseProject(project.value.project_root);
  }

  async function openPath(path: string) {
    await run(async () => {
      project.value = await api.openProject(path);
      connectors.value = await api.detectAgentConnectors();
      const first = project.value.assets[0]?.asset.id;
      if (first) await selectAsset(first);
      notice.value = `Opened ${project.value.project.name}`;
    });
  }

  async function chooseProject() {
    const path = await chooseProjectFolder();
    if (path) await openPath(path);
  }

  async function selectAsset(id: string) {
    assetId.value = id;
    activeRun.value = undefined;
    candidateImages.value = {};
    const asset = project.value?.assets.find(
      ({ asset }) => asset.id === id,
    )?.asset;
    if (asset?.head) {
      view.value = await api.loadRevision(
        project.value!.project_root,
        id,
        asset.head,
      );
      stage.value = "review";
    } else {
      view.value = undefined;
      stage.value = asset?.selected_reference
        ? "pixelize"
        : asset?.brief.text
          ? "reference"
          : "brief";
    }
    const runs = await api.browseAgentRuns(project.value!.project_root, id);
    activeRun.value = runs.at(-1);
    if (activeRun.value) await loadCandidates(activeRun.value);
  }

  async function createAsset(name: string, brief: string) {
    const id = name
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "");
    if (!project.value || !id) return;
    await run(async () => {
      await api.initializeAsset(
        project.value!.project_root,
        id,
        "sprite",
        brief.trim(),
      );
      await refresh();
      await selectAsset(id);
      notice.value = `Created ${name.trim()}`;
    });
  }

  async function saveBrief(brief: string) {
    if (!project.value || !assetId.value) return;
    await run(async () => {
      await api.updateAssetBrief(
        project.value!.project_root,
        assetId.value,
        brief.trim(),
      );
      await refresh();
      stage.value = "reference";
      notice.value = "Brief saved";
    });
  }

  async function connect(id: string) {
    const name = id === "amp" ? "Amp" : "Codex";
    if (!(await confirmAgentConnector(name))) return;
    await run(async () => {
      await api.approveAgentConnector(id);
      connectors.value = await api.detectAgentConnectors();
      notice.value = `${name} connected`;
    });
  }

  async function importReference() {
    if (!project.value) return;
    const file = await chooseReferenceImage();
    if (!file) return;
    await run(async () => {
      await api.importReference(
        project.value!.project_root,
        assetId.value,
        file,
      );
      await refresh();
      stage.value = "pixelize";
      notice.value = "Reference selected";
    });
  }

  async function generate(connector: string) {
    if (!project.value || !selectedAsset.value) return;
    error.value = "";
    agentBusy.value = true;
    try {
      agentTask.value = await api.startAgentTask(
        project.value.project_root,
        assetId.value,
        connector,
        "generate_references",
        selectedAsset.value.asset.brief.text,
      );
    } catch (caught) {
      agentBusy.value = false;
      error.value = caught instanceof Error ? caught.message : String(caught);
    }
  }

  async function cancelGeneration() {
    if (!agentTask.value) return;
    await api.cancelAgentTask(agentTask.value);
    notice.value = "Cancelling generation…";
  }

  async function selectCandidate(runId: string, candidate: string) {
    await run(async () => {
      await api.selectAgentCandidate(
        project.value!.project_root,
        assetId.value,
        runId,
        candidate,
      );
      await refresh();
      stage.value = "pixelize";
      notice.value = "Reference selected";
    });
  }

  async function pixelize(recipe: string) {
    await run(async () => {
      const result = await api.convertSelectedReference(
        project.value!.project_root,
        assetId.value,
        recipe,
        "user",
      );
      await refresh();
      view.value = await api.loadRevision(
        project.value!.project_root,
        assetId.value,
        result.revision,
      );
      stage.value = "review";
      notice.value = "Sprite created";
    });
  }

  async function exportSprite() {
    const destination = await chooseExportFolder();
    if (!destination) return;
    if (!(await confirmExport(assetId.value))) return;
    await run(async () => {
      const result = await api.exportAsset(
        project.value!.project_root,
        assetId.value,
        destination,
        true,
      );
      notice.value = `Exported ${result.png}`;
    });
  }

  async function loadCandidates(run: AgentRunRecord) {
    const images: Record<string, string> = {};
    for (const candidate of run.candidates) {
      const loaded = await api.loadAgentCandidate(
        project.value!.project_root,
        run.id,
        candidate.id,
      );
      images[candidate.id] = api.pngDataUrl(loaded.png_base64);
    }
    candidateImages.value = images;
  }

  async function handleAgent(event: AgentTaskEvent) {
    if (agentTask.value && event.task !== agentTask.value) return;
    if (event.event.type === "completed") {
      agentBusy.value = false;
      agentTask.value = "";
      activeRun.value = event.event.run;
      await loadCandidates(event.event.run);
      notice.value = "References ready—choose one";
    } else if (
      event.event.type === "failed" ||
      event.event.type === "cancelled"
    ) {
      agentBusy.value = false;
      agentTask.value = "";
      error.value =
        event.event.type === "failed"
          ? event.event.error
          : "Generation cancelled";
    }
  }

  onMounted(async () => {
    unlisten = await listen<AgentTaskEvent>(
      "pixelpipe://agent-task",
      ({ payload }) => void handleAgent(payload),
    );
  });
  onUnmounted(() => unlisten?.());

  return {
    project,
    assetId,
    stage,
    view,
    connectors,
    activeRun,
    candidateImages,
    busy,
    agentBusy,
    creatingAsset,
    error,
    notice,
    selectedAsset,
    selectedReference,
    recipes,
    chooseProject,
    openPath,
    selectAsset,
    createAsset,
    saveBrief,
    connect,
    importReference,
    generate,
    cancelGeneration,
    selectCandidate,
    pixelize,
    exportSprite,
  };
}

export type Workspace = ReturnType<typeof createWorkspace>;
const workspaceKey = Symbol("pixelpipe-workspace");
export const provideWorkspace = (workspace: Workspace) =>
  provide(workspaceKey, workspace);
export const useWorkspace = () => {
  const workspace = inject<Workspace>(workspaceKey);
  if (!workspace) throw new Error("PixelPipe workspace is unavailable");
  return workspace;
};
