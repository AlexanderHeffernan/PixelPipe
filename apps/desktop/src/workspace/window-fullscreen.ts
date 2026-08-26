import { getCurrentWindow } from "@tauri-apps/api/window";
import { onBeforeUnmount, onMounted, ref } from "vue";

export function useWindowFullscreen() {
  const isFullscreen = ref(false);
  let disposed = false;
  let unlisten: (() => void) | undefined;

  async function sync() {
    try {
      const fullscreen = await getCurrentWindow().isFullscreen();
      if (!disposed) isFullscreen.value = fullscreen;
    } catch {
      if (!disposed) isFullscreen.value = false;
    }
  }

  onMounted(() => {
    void sync();
    try {
      void getCurrentWindow()
        .onResized(() => void sync())
        .then((next) => {
          if (disposed) next();
          else unlisten = next;
        });
    } catch {
      // Browser previews do not have a native window bridge.
    }
  });
  onBeforeUnmount(() => {
    disposed = true;
    unlisten?.();
  });
  return isFullscreen;
}
