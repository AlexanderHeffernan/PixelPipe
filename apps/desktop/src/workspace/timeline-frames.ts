import { nextTick, onBeforeUnmount, onMounted, ref, type Ref } from "vue";
import type { createAnimation } from "./animation";

export function useTimelineFrames(
  animation: ReturnType<typeof createAnimation>,
  strip: Ref<HTMLElement | undefined>,
) {
  const contextFrameId = ref("");
  const editingFrameId = ref("");
  const editName = ref("");
  const draggedFrameId = ref("");
  const dropPosition = ref<number>();
  let pendingFrameId = "";
  let pointerStartX = 0;
  let pointerStartY = 0;

  function setDuration(event: Event) {
    const value = Math.max(
      1,
      Number((event.target as HTMLInputElement).value) || 1,
    );
    void animation.mutate({
      type: "set_all_durations",
      duration_ms: value,
    });
  }

  function reorder(frameId: string, offset: number) {
    const index = animation.frames.value.findIndex(
      (frame) => frame.id === frameId,
    );
    const position = Math.max(
      0,
      Math.min(animation.frames.value.length - 1, index + offset),
    );
    if (position !== index)
      void animation.mutate(
        { type: "reorder", frame_id: frameId, position },
        frameId,
      );
  }

  function openFrameMenu(event: MouseEvent, frameId: string) {
    event.preventDefault();
    contextFrameId.value = frameId;
  }

  function closeFrameMenu(event: PointerEvent) {
    if (
      contextFrameId.value &&
      !(event.target as HTMLElement | null)?.closest(".frame-context")
    )
      contextFrameId.value = "";
  }

  async function beginRename(frameId: string) {
    const index = animation.frames.value.findIndex(
      (frame) => frame.id === frameId,
    );
    const frame = animation.frames.value[index];
    if (!frame) return;
    contextFrameId.value = "";
    editingFrameId.value = frameId;
    editName.value = frame.name ?? `Frame ${index + 1}`;
    await nextTick();
    strip.value
      ?.querySelector<HTMLInputElement>(`[data-frame-name="${frameId}"]`)
      ?.select();
  }

  async function finishRename(frameId: string) {
    if (editingFrameId.value !== frameId) return;
    const name = editName.value.trim();
    editingFrameId.value = "";
    if (name)
      await animation.mutate(
        { type: "rename", frame_id: frameId, name },
        frameId,
      );
  }

  function deleteFrame(frameId: string) {
    contextFrameId.value = "";
    void animation.mutate({ type: "delete", frame_id: frameId });
  }

  function pointerDown(event: PointerEvent, frameId: string) {
    if (event.button !== 0 || (event.target as HTMLElement).closest("input"))
      return;
    pendingFrameId = frameId;
    pointerStartX = event.clientX;
    pointerStartY = event.clientY;
  }

  function pointerMove(event: PointerEvent) {
    if (!pendingFrameId) return;
    if (
      !draggedFrameId.value &&
      Math.hypot(event.clientX - pointerStartX, event.clientY - pointerStartY) <
        5
    )
      return;
    event.preventDefault();
    draggedFrameId.value = pendingFrameId;
    const cards = Array.from(
      strip.value?.querySelectorAll<HTMLElement>("[data-frame-index]") ?? [],
    );
    dropPosition.value = cards.findIndex((card) => {
      const bounds = card.getBoundingClientRect();
      return event.clientX < bounds.left + bounds.width / 2;
    });
    if (dropPosition.value < 0) dropPosition.value = cards.length;
  }

  function pointerUp() {
    const frameId = draggedFrameId.value;
    const slot = dropPosition.value;
    pendingFrameId = "";
    draggedFrameId.value = "";
    dropPosition.value = undefined;
    if (!frameId || slot === undefined) return;
    const source = animation.frames.value.findIndex(
      (frame) => frame.id === frameId,
    );
    const position = slot > source ? slot - 1 : slot;
    if (source !== position)
      void animation.mutate(
        { type: "reorder", frame_id: frameId, position },
        frameId,
      );
  }

  onMounted(() => {
    document.addEventListener("pointerdown", closeFrameMenu);
    window.addEventListener("pointermove", pointerMove);
    window.addEventListener("pointerup", pointerUp);
    window.addEventListener("pointercancel", pointerUp);
  });
  onBeforeUnmount(() => {
    document.removeEventListener("pointerdown", closeFrameMenu);
    window.removeEventListener("pointermove", pointerMove);
    window.removeEventListener("pointerup", pointerUp);
    window.removeEventListener("pointercancel", pointerUp);
  });

  return {
    contextFrameId,
    editingFrameId,
    editName,
    draggedFrameId,
    dropPosition,
    setDuration,
    reorder,
    openFrameMenu,
    beginRename,
    finishRename,
    deleteFrame,
    pointerDown,
  };
}
