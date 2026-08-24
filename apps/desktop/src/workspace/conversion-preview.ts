import { onScopeDispose, ref, toRaw, type ComputedRef, type Ref } from "vue";
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
}

export function createConversionPreview(context: PreviewContext) {
  const recipeId = ref("");
  const settings = ref<ConversionSettings>();
  const busy = ref(false);
  const error = ref("");
  let timer: ReturnType<typeof setTimeout> | undefined;
  let sequence = 0;

  onScopeDispose(() => {
    if (timer) clearTimeout(timer);
    sequence += 1;
  });

  function chooseDefaultRecipe() {
    const recipe =
      context.recipes.value.find(({ id }) => id === "sprite-32") ??
      context.recipes.value[0];
    recipeId.value = recipe?.id ?? "";
    settings.value =
      recipe?.mode.type === "reference"
        ? structuredClone(toRaw(recipe.mode.settings))
        : undefined;
    error.value = "";
  }

  function chooseRecipe(id: string) {
    const recipe = context.recipes.value.find(
      (candidate) => candidate.id === id,
    );
    if (!recipe || recipe.mode.type !== "reference") return;
    recipeId.value = recipe.id;
    settings.value = structuredClone(toRaw(recipe.mode.settings));
    error.value = "";
    void request();
  }

  function updateSettings(update: Partial<ConversionSettings>) {
    if (!settings.value) return;
    settings.value = { ...settings.value, ...update };
    error.value = "";
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
    settings,
    busy,
    error,
    chooseDefaultRecipe,
    chooseRecipe,
    updateSettings,
    request,
  };
}
