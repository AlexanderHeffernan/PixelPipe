import { ref, type Ref } from "vue";
import * as api from "../api";
import type {
  CanvasSettings,
  ConversionPreviewResponse,
  ProjectBrowser,
  RevisionViewResponse,
} from "../types";

interface CompositionContext {
  project: Ref<ProjectBrowser | undefined>;
  assetId: Ref<string>;
  view: Ref<RevisionViewResponse | undefined>;
  preview: Ref<ConversionPreviewResponse | undefined>;
  refresh: () => Promise<void>;
  run: (action: () => Promise<void>) => Promise<void>;
}

export function createCompositionPreview(context: CompositionContext) {
  const settings = ref<CanvasSettings>();
  const dirty = ref(false);
  const busy = ref(false);
  const error = ref("");
  let timer: ReturnType<typeof setTimeout> | undefined;
  let sequence = 0;

  function reset() {
    const inspection = context.view.value?.metadata.inspection;
    settings.value = inspection
      ? {
          width: inspection.width,
          height: inspection.height,
          scale_percent: 100,
          offset_x: 0,
          offset_y: 0,
        }
      : undefined;
    context.preview.value = undefined;
    dirty.value = false;
    error.value = "";
    sequence += 1;
  }

  function update(update: Partial<CanvasSettings>) {
    if (!settings.value) return;
    settings.value = { ...settings.value, ...update };
    dirty.value = true;
    error.value = "";
    if (timer) clearTimeout(timer);
    const requested = ++sequence;
    busy.value = true;
    timer = setTimeout(() => void request(requested), 20);
  }

  async function request(requested = ++sequence) {
    const root = context.project.value?.project_root;
    const parent = context.view.value?.metadata.revision;
    if (!root || !parent || !settings.value) return;
    busy.value = true;
    try {
      const result = await api.previewComposition(
        root,
        context.assetId.value,
        parent,
        settings.value,
      );
      if (requested === sequence) {
        context.preview.value = result;
        error.value = "";
      }
    } catch (caught) {
      if (requested === sequence) {
        error.value = caught instanceof Error ? caught.message : String(caught);
      }
    } finally {
      if (requested === sequence) busy.value = false;
    }
  }

  async function commitIfDirty() {
    const root = context.project.value?.project_root;
    const parent = context.view.value?.metadata.revision;
    if (!dirty.value) return true;
    if (!root || !parent || !settings.value) return false;
    let committed = false;
    await context.run(async () => {
      const result = await api.commitComposition(
        root,
        context.assetId.value,
        parent,
        settings.value!,
        "user",
      );
      await context.refresh();
      context.view.value = await api.loadRevision(
        root,
        context.assetId.value,
        result.revision,
      );
      reset();
      committed = true;
    });
    return committed;
  }

  return { settings, dirty, busy, error, reset, update, commitIfDirty };
}
