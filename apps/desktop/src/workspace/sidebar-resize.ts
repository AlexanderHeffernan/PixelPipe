import { onBeforeUnmount, ref } from "vue";

type SidebarEdge = "left" | "right";

type SidebarResizeOptions = {
  edge: SidebarEdge;
  initialWidth: number;
  minWidth: number;
  maxWidth: number;
};

export function useSidebarResize(options: SidebarResizeOptions) {
  const width = ref(options.initialWidth);
  const isResizing = ref(false);

  function clamp(value: number) {
    return Math.min(options.maxWidth, Math.max(options.minWidth, value));
  }

  function resizeTo(clientX: number) {
    const value =
      options.edge === "left" ? clientX : window.innerWidth - clientX;
    width.value = clamp(value);
  }

  function stopResize() {
    isResizing.value = false;
    document.body.classList.remove("is-resizing-sidebar");
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", stopResize);
    window.removeEventListener("pointercancel", stopResize);
  }

  function onPointerMove(event: PointerEvent) {
    resizeTo(event.clientX);
  }

  function startResize(event: PointerEvent) {
    if (event.button !== 0) return;
    event.preventDefault();
    isResizing.value = true;
    document.body.classList.add("is-resizing-sidebar");
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", stopResize);
    window.addEventListener("pointercancel", stopResize);
  }

  function resizeWithKeyboard(event: KeyboardEvent) {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const direction = event.key === "ArrowLeft" ? -1 : 1;
    const edgeDirection = options.edge === "left" ? direction : -direction;
    width.value = clamp(width.value + edgeDirection * 10);
  }

  onBeforeUnmount(stopResize);

  return { width, isResizing, startResize, resizeWithKeyboard };
}
