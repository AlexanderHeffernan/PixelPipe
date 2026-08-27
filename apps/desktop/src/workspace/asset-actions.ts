import type { ComputedRef, Ref } from "vue";
import * as api from "../api";
import { chooseExportFile, chooseReferenceImage } from "../services/dialogs";
import type {
  AssetBrowser,
  ConversionPreviewResponse,
  ProjectBrowser,
} from "../types";
import type { createCanvasLoading } from "./canvas-loading";
import type { createCompositionPreview } from "./composition-preview";
import type { createConversionPreview } from "./conversion-preview";
import type { createPixelEditor, PixelTool } from "./pixel-editor";

interface AssetActionContext {
  project: Ref<ProjectBrowser | undefined>;
  assetId: Ref<string>;
  selectedAsset: ComputedRef<AssetBrowser | undefined>;
  mode: Ref<"convert" | "edit">;
  assetModes: Map<string, "convert" | "edit">;
  preview: Ref<ConversionPreviewResponse | undefined>;
  canConvert: ComputedRef<boolean>;
  conversion: ReturnType<typeof createConversionPreview>;
  composition: ReturnType<typeof createCompositionPreview>;
  editor: ReturnType<typeof createPixelEditor>;
  canvasLoading: ReturnType<typeof createCanvasLoading>;
  run: (action: () => Promise<void>) => Promise<void>;
  refresh: () => Promise<void>;
  notice: (message: string) => void;
  commitConversion: () => Promise<void>;
}

export function createAssetActions(context: AssetActionContext) {
  async function setMode(next: "convert" | "edit") {
    if (next === "convert" && !context.canConvert.value) return;
    if (next === "convert") {
      if (!context.conversion.settings.value)
        context.conversion.chooseDefaults();
      context.mode.value = next;
      context.assetModes.set(context.assetId.value, next);
      await context.conversion.request();
      return;
    }
    if (context.mode.value === "convert") {
      await context.commitConversion();
      return;
    }
    context.preview.value = undefined;
    context.mode.value = next;
    context.assetModes.set(context.assetId.value, next);
  }

  async function beginTool(tool: PixelTool) {
    if (context.mode.value === "convert") await context.commitConversion();
    if (context.mode.value === "edit") {
      if (!(await context.composition.commitIfDirty())) return;
      context.editor.selectTool(tool);
    }
  }

  async function prepareEditing() {
    return context.composition.commitIfDirty();
  }

  async function setDrawingColor(hex: string) {
    if (!(await prepareEditing())) return;
    await context.editor.setDrawingColor(hex);
  }

  async function undo() {
    if (!(await prepareEditing())) return;
    await context.editor.undo();
  }

  async function redo() {
    if (!(await prepareEditing())) return;
    await context.editor.redo();
  }

  async function recolorCurrent(index: number, hex: string) {
    if (context.mode.value === "convert")
      context.conversion.recolor(index, hex);
    else {
      if (!(await context.composition.commitIfDirty())) return;
      await context.editor.recolor(index, hex);
    }
  }

  async function reconvert() {
    if (!context.canConvert.value) return;
    await context.canvasLoading.run("Pixelizing source…", async () => {
      context.editor.resetHistory();
      if (!context.conversion.settings.value)
        context.conversion.chooseDefaults();
      context.mode.value = "convert";
      context.assetModes.set(context.assetId.value, "convert");
      await context.conversion.request();
    });
  }

  async function exportCurrent() {
    if (!context.project.value || !context.selectedAsset.value?.asset.head)
      return;
    if (!(await context.composition.commitIfDirty())) return;
    const destination = await chooseExportFile(context.assetId.value);
    if (!destination) return;
    await context.run(async () => {
      const result = await api.exportAssetFile(
        context.project.value!.project_root,
        context.assetId.value,
        destination,
        true,
      );
      context.notice(
        `Exported ${result.width}×${result.height} ${result.format.toUpperCase()}`,
      );
    });
  }

  async function replaceSource() {
    if (!context.project.value || !context.assetId.value) return;
    const file = await chooseReferenceImage();
    if (!file) return;
    await context.canvasLoading.run("Updating source…", () =>
      context.run(async () => {
        await api.importReference(
          context.project.value!.project_root,
          context.assetId.value,
          file,
        );
        await context.refresh();
        const asset = context.selectedAsset.value?.asset;
        if (asset?.style) context.conversion.chooseAssetStyle(asset.style);
        else context.conversion.chooseDefaults();
        context.mode.value = "convert";
        context.assetModes.set(context.assetId.value, "convert");
        context.editor.resetHistory();
        await context.conversion.request();
        context.notice("Source image replaced");
      }),
    );
  }

  return {
    setMode,
    beginTool,
    prepareEditing,
    setDrawingColor,
    undo,
    redo,
    recolorCurrent,
    reconvert,
    exportCurrent,
    replaceSource,
  };
}
