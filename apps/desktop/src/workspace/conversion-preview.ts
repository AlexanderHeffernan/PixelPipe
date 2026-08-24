import { ref, toRaw, type ComputedRef, type Ref } from "vue";
import * as api from "../api";
import type {
  ConversionPreviewResponse,
  ConversionRecipeDocument,
  ConversionSettings,
  ProjectBrowser,
} from "../types";

interface PreviewContext {
  project: Ref<ProjectBrowser | undefined>;
  assetId: Ref<string>;
  preview: Ref<ConversionPreviewResponse | undefined>;
  thumbnails: Ref<Record<string, string>>;
  recipes: ComputedRef<ConversionRecipeDocument[]>;
  error: Ref<string>;
}

export function createConversionPreview(context: PreviewContext) {
  const recipeId = ref("");
  const settings = ref<ConversionSettings>();
  const busy = ref(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let sequence = 0;

  function chooseDefaultRecipe() {
    const recipe =
      context.recipes.value.find(({ id }) => id === "sprite-32") ??
      context.recipes.value[0];
    recipeId.value = recipe?.id ?? "";
    settings.value =
      recipe?.mode.type === "reference"
        ? structuredClone(toRaw(recipe.mode.settings))
        : undefined;
  }

  function updateSettings(update: Partial<ConversionSettings>) {
    if (!settings.value) return;
    settings.value = { ...settings.value, ...update };
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => void request(), 60);
  }

  async function request() {
    const project = context.project.value;
    if (!project || !recipeId.value || !settings.value) return;
    const requested = ++sequence;
    busy.value = true;
    try {
      const result = await api.previewSelectedReference(
        project.project_root,
        context.assetId.value,
        recipeId.value,
        settings.value,
      );
      if (requested === sequence) {
        context.preview.value = result;
        context.thumbnails.value[context.assetId.value] = api.pngDataUrl(
          result.native_png_base64,
        );
      }
    } catch (caught) {
      if (requested === sequence) {
        context.error.value =
          caught instanceof Error ? caught.message : String(caught);
      }
    } finally {
      if (requested === sequence) busy.value = false;
    }
  }

  return {
    recipeId,
    settings,
    busy,
    chooseDefaultRecipe,
    updateSettings,
    request,
  };
}
