import { nextTick, ref, type ComputedRef } from "vue";

export function createCanvasLoading(image: ComputedRef<string>) {
  const active = ref(false);
  const artwork = ref("");
  const message = ref("");
  let depth = 0;

  async function run<T>(label: string, action: () => Promise<T>) {
    if (depth === 0) artwork.value = image.value;
    depth += 1;
    active.value = true;
    message.value = label;
    await nextTick();
    try {
      return await action();
    } finally {
      depth -= 1;
      if (depth === 0) {
        active.value = false;
        artwork.value = "";
        message.value = "";
      }
    }
  }

  return { active, artwork, message, run };
}
