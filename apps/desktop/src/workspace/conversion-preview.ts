import { onScopeDispose, ref, toRaw, type ComputedRef, type Ref } from "vue";
import * as api from "../api";
import type {
  AssetStyle,
  ConversionPreviewResponse,
  ConversionRecipeDocument,
  ConversionSettings,
  PaletteColorOverride,
  ProjectBrowser,
} from "../types";

interface PreviewContext {
  project: Ref<ProjectBrowser | undefined>;
  assetId: Ref<string>;
  preview: Ref<ConversionPreviewResponse | undefined>;
  thumbnails: Ref<Record<string, string>>;
  recipes: ComputedRef<ConversionRecipeDocument[]>;
}

export function createConversionPreview(context: PreviewContext) {
  const recipeId = ref("");
  const paletteId = ref("");
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

  function chooseDefaultRecipe() {
    const recipe =
      context.recipes.value.find(({ id }) => id === "sprite-32") ??
      context.recipes.value[0];
    recipeId.value = recipe?.id ?? "";
    paletteId.value = recipe?.palette ?? "";
    colorCount.value = 16;
    paletteOverrides.value = [];
    backgroundAutomatic.value = true;
    settings.value =
      recipe?.mode.type === "reference"
        ? pixelizeSettings(recipe.mode.settings)
        : undefined;
    error.value = "";
  }

  function chooseRecipe(id: string) {
    const recipe = context.recipes.value.find(
      (candidate) => candidate.id === id,
    );
    if (!recipe || recipe.mode.type !== "reference") return;
    recipeId.value = recipe.id;
    paletteId.value = recipe.palette;
    paletteOverrides.value = [];
    backgroundAutomatic.value = true;
    settings.value = pixelizeSettings(recipe.mode.settings);
    error.value = "";
    void request();
  }

  function chooseAssetStyle(style: AssetStyle) {
    if (!context.recipes.value.some(({ id }) => id === style.recipe)) {
      chooseDefaultRecipe();
      return;
    }
    recipeId.value = style.recipe;
    paletteId.value = style.palette ?? "";
    colorCount.value = style.color_count ?? 16;
    paletteOverrides.value = [];
    backgroundAutomatic.value = true;
    settings.value = pixelizeSettings(style.settings);
    error.value = "";
  }

  function choosePalette(id: string) {
    paletteId.value = id;
    error.value = "";
    void request();
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
    if (!project || !recipeId.value || !settings.value) return;
    busy.value = true;
    try {
      const result = await api.previewSelectedReference(
        project.project_root,
        context.assetId.value,
        recipeId.value,
        undefined,
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
    recipeId,
    paletteId,
    colorCount,
    paletteOverrides,
    backgroundAutomatic,
    settings,
    busy,
    error,
    chooseDefaultRecipe,
    chooseRecipe,
    chooseAssetStyle,
    choosePalette,
    updateSettings,
    setColorCount,
    setBackgroundAutomatic,
    recolor,
    request,
  };
}
