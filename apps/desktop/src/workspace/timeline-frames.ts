import { nextTick, ref, type Ref } from "vue";
import type { createAnimation } from "./animation";

export function useTimelineFrames(
  animation: ReturnType<typeof createAnimation>,
  strip: Ref<HTMLElement | undefined>,
) {
  const contextFrameId = ref("");
  const editingFrameId = ref("");
  const editName = ref("");
  const draggedFrameId = ref("");

  function setDuration(event: Event) {
    const value = Math.max(
      1,
      Number((event.target as HTMLInputElement).value) || 1,
    );
    void animation.mutate({
      type: "set_duration",
      frame_id: animation.selectedFrameId.value,
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

  function dragStart(event: DragEvent, frameId: string) {
    draggedFrameId.value = frameId;
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", frameId);
    }
  }

  function dropFrame(event: DragEvent, targetIndex: number) {
    event.preventDefault();
    const frameId =
      draggedFrameId.value || event.dataTransfer?.getData("text/plain") || "";
    draggedFrameId.value = "";
    const current = animation.frames.value.findIndex(
      (frame) => frame.id === frameId,
    );
    if (frameId && current !== targetIndex)
      void animation.mutate(
        { type: "reorder", frame_id: frameId, position: targetIndex },
        frameId,
      );
  }

  return {
    contextFrameId,
    editingFrameId,
    editName,
    draggedFrameId,
    setDuration,
    reorder,
    openFrameMenu,
    beginRename,
    finishRename,
    deleteFrame,
    dragStart,
    dropFrame,
  };
}
