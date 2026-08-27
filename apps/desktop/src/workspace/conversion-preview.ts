import { onScopeDispose, ref, toRaw, type Ref } from "vue";
import * as api from "../api";
import type {
  AssetStyle,
  ConversionPreviewResponse,
  ConversionSettings,
  PaletteColorOverride,
  ProjectBrowser,
} from "../types";

interface PreviewContext {
  project: Ref<ProjectBrowser | undefined>;
  assetId: Ref<string>;
  preview: Ref<ConversionPreviewResponse | undefined>;
  thumbnails: Ref<Record<string, string>>;
}

export function createConversionPreview(context: PreviewContext) {
  const colorCount = ref(16);
  const paletteOverrides = ref<PaletteColorOverride[]>([]);
  const backgroundAutomatic = ref(true);
  const settings = ref<ConversionSettings>();
  const busy = ref(false);
  const error = ref("");
  let timer: ReturnType<typeof setTimeout> | undefined;
  let sequence = 0;

  onScopeDispose(() => {
    if (timer) clearTimeout(timer);
    sequence += 1;
  });

  function pixelizeSettings(source: ConversionSettings) {
    return {
      ...structuredClone(toRaw(source)),
      margin: 0,
      subject_scale_percent: 100,
      offset_x: 0,
      offset_y: 0,
      registration: "center" as const,
    };
  }

  function chooseDefaults() {
    const defaults = context.project.value?.pixelization;
    colorCount.value = defaults?.color_count ?? 16;
    paletteOverrides.value = [];
    backgroundAutomatic.value = true;
    settings.value = defaults ? pixelizeSettings(defaults.settings) : undefined;
    error.value = "";
  }

  function chooseAssetStyle(style: AssetStyle) {
    colorCount.value = style.color_count;
    paletteOverrides.value = [];
    backgroundAutomatic.value = true;
    settings.value = pixelizeSettings(style.settings);
    error.value = "";
  }

  function updateSettings(update: Partial<ConversionSettings>) {
    if (!settings.value) return;
    if ("backdrop" in update) paletteOverrides.value = [];
    settings.value = { ...settings.value, ...update };
    error.value = "";
    if (timer) clearTimeout(timer);
    const requested = ++sequence;
    busy.value = true;
    timer = setTimeout(() => void request(requested), 100);
  }

  function setColorCount(count: number) {
    colorCount.value = count;
    paletteOverrides.value = [];
    updateSettings({});
  }

  function setBackgroundAutomatic(automatic: boolean) {
    backgroundAutomatic.value = automatic;
    updateSettings({});
  }

  function recolor(index: number, hex: string) {
    const rgba: [number, number, number, number] = [
      Number.parseInt(hex.slice(1, 3), 16),
      Number.parseInt(hex.slice(3, 5), 16),
      Number.parseInt(hex.slice(5, 7), 16),
      255,
    ];
    paletteOverrides.value = [
      ...paletteOverrides.value.filter((entry) => entry.index !== index),
      { index, rgba },
    ].sort((left, right) => left.index - right.index);
    updateSettings({});
  }

  async function request(requested = ++sequence) {
    const project = context.project.value;
    if (!project || !settings.value) return;
    busy.value = true;
    try {
      const result = await api.previewSelectedReference(
        project.project_root,
        context.assetId.value,
        colorCount.value,
        paletteOverrides.value,
        settings.value,
        backgroundAutomatic.value,
      );
      if (requested === sequence) {
        context.preview.value = result;
        error.value = "";
        context.thumbnails.value[context.assetId.value] = api.pngDataUrl(
          result.native_png_base64,
        );
      }
    } catch (caught) {
      if (requested === sequence) {
        error.value = caught instanceof Error ? caught.message : String(caught);
      }
    } finally {
      if (requested === sequence) busy.value = false;
    }
  }

  return {
    colorCount,
    paletteOverrides,
    backgroundAutomatic,
    settings,
    busy,
    error,
    chooseDefaults,
    chooseAssetStyle,
    updateSettings,
    setColorCount,
    setBackgroundAutomatic,
    recolor,
    request,
  };
}
