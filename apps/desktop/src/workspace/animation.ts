import { computed, onScopeDispose, ref, watch, type Ref } from "vue";
import * as api from "../api";
import { chooseFrameImage } from "../services/dialogs";
import type { ProjectBrowser, RevisionViewResponse } from "../types";

interface AnimationContext {
  project: Ref<ProjectBrowser | undefined>;
  assetId: Ref<string>;
  view: Ref<RevisionViewResponse | undefined>;
  refresh: () => Promise<void>;
  run: (action: () => Promise<void>) => Promise<void>;
  notice: (message: string) => void;
  onMutation: () => void;
}

export function createAnimation(context: AnimationContext) {
  const playing = ref(false);
  const loop = ref(true);
  const timelineOpen = ref(false);
  const thumbnails = ref<Record<string, string>>({});
  let timer: ReturnType<typeof setTimeout> | undefined;
  let playbackGeneration = 0;
  let thumbnailGeneration = 0;

  const frames = computed(() =>
    context.view.value?.metadata.asset === context.assetId.value
      ? context.view.value.metadata.frames
      : [],
  );
  const selectedFrameId = computed(() =>
    context.view.value?.metadata.asset === context.assetId.value
      ? context.view.value.metadata.selected_frame_id
      : "",
  );
  const selectedIndex = computed(() =>
    frames.value.findIndex((frame) => frame.id === selectedFrameId.value),
  );

  watch(
    () =>
      `${context.assetId.value}:${context.view.value?.metadata.asset}:${context.view.value?.metadata.revision}:${frames.value.map((frame) => frame.id).join(",")}`,
    () => void loadThumbnails().catch(() => undefined),
  );
  onScopeDispose(pause);

  async function select(frameId: string) {
    pause();
    await loadFrame(frameId);
  }

  async function loadFrame(frameId: string, generation?: number) {
    const root = context.project.value?.project_root;
    const revision = context.view.value?.metadata.revision;
    if (!root || !revision || frameId === selectedFrameId.value) return;
    const loaded = await api.loadRevision(
      root,
      context.assetId.value,
      revision,
      frameId,
    );
    if (generation === undefined || generation === playbackGeneration)
      context.view.value = loaded;
  }

  function pause() {
    playing.value = false;
    playbackGeneration += 1;
    if (timer) clearTimeout(timer);
    timer = undefined;
  }

  function play() {
    if (frames.value.length < 2 || playing.value) return;
    playing.value = true;
    schedule(playbackGeneration);
  }

  function schedule(generation: number) {
    if (!playing.value || generation !== playbackGeneration) return;
    const current = frames.value[selectedIndex.value] ?? frames.value[0];
    timer = setTimeout(
      () => void advancePlayback(generation),
      current.duration_ms,
    );
  }

  async function advancePlayback(generation: number) {
    if (!playing.value || generation !== playbackGeneration) return;
    const next = selectedIndex.value + 1;
    if (next >= frames.value.length && !loop.value) {
      pause();
      return;
    }
    await loadFrame(frames.value[next % frames.value.length].id, generation);
    schedule(generation);
  }

  async function previous() {
    pause();
    if (!frames.value.length) return;
    const index =
      (selectedIndex.value - 1 + frames.value.length) % frames.value.length;
    await loadFrame(frames.value[index].id);
  }

  async function next() {
    pause();
    if (!frames.value.length) return;
    await loadFrame(
      frames.value[(selectedIndex.value + 1) % frames.value.length].id,
    );
  }

  async function mutate(
    action: api.FrameMutationAction,
    preferredId = selectedFrameId.value,
  ) {
    pause();
    const root = context.project.value?.project_root;
    const parent = context.view.value?.metadata.revision;
    if (!root || !parent) return;
    const oldIds = new Set(frames.value.map((frame) => frame.id));
    await context.run(async () => {
      const result = await api.mutateFrames(
        root,
        context.assetId.value,
        parent,
        action,
        "user",
      );
      context.onMutation();
      await context.refresh();
      let loaded = await api.loadRevision(
        root,
        context.assetId.value,
        result.revision,
      );
      const added = loaded.metadata.frames.find(
        (frame) => !oldIds.has(frame.id),
      )?.id;
      const target =
        added ??
        (loaded.metadata.frames.some((frame) => frame.id === preferredId)
          ? preferredId
          : loaded.metadata.frames[0].id);
      if (target !== loaded.metadata.selected_frame_id)
        loaded = await api.loadRevision(
          root,
          context.assetId.value,
          result.revision,
          target,
        );
      context.view.value = loaded;
      context.notice("Frame change saved as a new revision");
    });
  }

  async function addFrameFromImage() {
    const file = await chooseFrameImage();
    if (!file) return;
    timelineOpen.value = true;
    await mutate({
      type: "import_frame",
      file,
      position: selectedIndex.value + 1,
    });
  }

  async function loadThumbnails() {
    const root = context.project.value?.project_root;
    const revision = context.view.value?.metadata.revision;
    const asset = context.assetId.value;
    const generation = ++thumbnailGeneration;
    if (!root || !revision || context.view.value?.metadata.asset !== asset) {
      thumbnails.value = {};
      return;
    }
    const loaded = await Promise.all(
      frames.value.map(
        async (frame) =>
          [
            frame.id,
            api.pngDataUrl(
              (await api.loadRevision(root, asset, revision, frame.id))
                .native_png_base64,
            ),
          ] as const,
      ),
    );
    if (
      generation === thumbnailGeneration &&
      context.assetId.value === asset &&
      context.view.value?.metadata.revision === revision
    )
      thumbnails.value = Object.fromEntries(loaded);
  }

  return {
    playing,
    loop,
    timelineOpen,
    frames,
    selectedFrameId,
    selectedIndex,
    thumbnails,
    select,
    pause,
    play,
    previous,
    next,
    mutate,
    addFrameFromImage,
  };
}
