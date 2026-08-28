import { onScopeDispose, ref, watch, type Ref } from "vue";
import * as api from "../api";
import type { AssetBrowser, ProjectBrowser } from "../types";

interface PreviewContext {
  project: Ref<ProjectBrowser | undefined>;
  selectedAssetId: Ref<string>;
  selectedFrames: Ref<{ id: string; duration_ms: number }[]>;
  selectedThumbnails: Ref<Record<string, string>>;
}

export function useSidebarAnimationPreview(context: PreviewContext) {
  const images = ref<Record<string, string>>({});
  let timer: ReturnType<typeof setTimeout> | undefined;
  let generation = 0;

  function stop() {
    generation += 1;
    if (timer) clearTimeout(timer);
    timer = undefined;
  }

  async function start(entry: AssetBrowser) {
    stop();
    const currentGeneration = generation;
    const project = context.project.value;
    if (!project) return;
    let frames: { id: string; duration_ms: number }[];
    let thumbnails: Record<string, string>;
    if (entry.asset.id === context.selectedAssetId.value) {
      frames = context.selectedFrames.value;
      thumbnails = context.selectedThumbnails.value;
    } else {
      if (!entry.asset.head) return;
      const first = await api.loadRevision(
        project.project_root,
        entry.asset.id,
        entry.asset.head,
      );
      frames = first.metadata.frames;
      if (frames.length < 2 || currentGeneration !== generation) return;
      const loaded = await Promise.all(
        frames.map(
          async (frame) =>
            [
              frame.id,
              api.pngDataUrl(
                (
                  await api.loadRevision(
                    project.project_root,
                    entry.asset.id,
                    entry.asset.head,
                    frame.id,
                  )
                ).native_png_base64,
              ),
            ] as const,
        ),
      );
      thumbnails = Object.fromEntries(loaded);
    }
    if (frames.length < 2 || currentGeneration !== generation) return;
    let index = 0;
    const advance = () => {
      if (currentGeneration !== generation) return;
      const frame = frames[index];
      const image = thumbnails[frame.id];
      if (image) images.value[entry.asset.id] = image;
      index = (index + 1) % frames.length;
      timer = setTimeout(advance, frame.duration_ms);
    };
    advance();
  }

  function startSelected() {
    const entry = context.project.value?.assets.find(
      ({ asset }) => asset.id === context.selectedAssetId.value,
    );
    if (entry) void start(entry);
    else stop();
  }

  watch(
    () => [
      context.selectedAssetId.value,
      context.selectedFrames.value.map((frame) => frame.id).join(","),
      Object.keys(context.selectedThumbnails.value).length,
    ],
    startSelected,
    { immediate: true },
  );
  onScopeDispose(stop);

  return { images, start, startSelected };
}
