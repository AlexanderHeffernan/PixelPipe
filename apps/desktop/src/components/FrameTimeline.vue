<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import {
  PhCaretDown,
  PhCaretLeft,
  PhCaretRight,
  PhCopy,
  PhFilmStrip,
  PhPause,
  PhPlay,
  PhPlus,
  PhTrash,
} from "@phosphor-icons/vue";
import { useWorkspace } from "../workspace/context";
import { useTimelineDrawer } from "../workspace/timeline-drawer";

const workspace = useWorkspace();
const animation = workspace.animation;
const strip = ref<HTMLElement>();
const overflowOpen = ref(false);
const {
  open,
  resizing,
  height,
  maximumHeight,
  expanded,
  minimumHeight,
  startResize,
  resizeWithKeyboard,
} = useTimelineDrawer();

watch(animation.selectedFrameId, () => void keepSelectedVisible());
watch(open, (isOpen) => {
  if (isOpen) void keepSelectedVisible();
});
watch(
  () => animation.frames.value.length,
  (count) => {
    if (count <= 1) closeTimeline();
  },
);

async function keepSelectedVisible() {
  await nextTick();
  strip.value
    ?.querySelector<HTMLElement>("[aria-current='true']")
    ?.scrollIntoView?.({ block: "nearest", inline: "nearest" });
}

function closeTimeline() {
  open.value = false;
  overflowOpen.value = false;
  animation.pause();
}

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

function frameKeydown(event: KeyboardEvent, frameId: string) {
  if (!event.shiftKey || !["ArrowLeft", "ArrowRight"].includes(event.key))
    return;
  event.preventDefault();
  reorder(frameId, event.key === "ArrowLeft" ? -1 : 1);
}

function addFrame() {
  void animation.mutate({ type: "add_blank" });
}

function duplicate() {
  void animation.mutate({
    type: "duplicate",
    frame_id: animation.selectedFrameId.value,
    position: animation.selectedIndex.value + 1,
  });
}

function remove() {
  void animation.mutate({
    type: "delete",
    frame_id: animation.selectedFrameId.value,
  });
}
</script>

<template>
  <section
    v-if="
      workspace.mode.value === 'edit' &&
      workspace.view.value &&
      animation.frames.value.length > 1
    "
    class="frame-timeline"
    :class="{
      open,
      resizing,
      'is-compact': !expanded,
      'is-expanded': expanded,
    }"
    :style="open ? { height: `${height}px` } : undefined"
    aria-label="Frame timeline"
  >
    <div v-if="!open" class="timeline-closed">
      <button aria-label="Open frame timeline" @click="open = true">
        <PhFilmStrip />
        <span>Animation</span>
        <small>{{ animation.frames.value.length }} frames</small>
      </button>
    </div>

    <template v-else>
      <div
        class="timeline-resize-handle"
        role="separator"
        aria-label="Resize frame timeline"
        aria-orientation="horizontal"
        :aria-valuemin="minimumHeight"
        :aria-valuemax="maximumHeight"
        :aria-valuenow="height"
        tabindex="0"
        @pointerdown="startResize"
        @keydown="resizeWithKeyboard"
      ></div>

      <header class="timeline-heading">
        <span>
          <PhFilmStrip /> Animation
          <small>{{ animation.frames.value.length }} frames</small>
        </span>
        <button aria-label="Close frame timeline" @click="closeTimeline">
          <PhCaretDown />
        </button>
      </header>

      <div class="timeline-controls">
        <div class="playback-controls" aria-label="Playback controls">
          <button aria-label="Previous frame" @click="animation.previous">
            <PhCaretLeft />
          </button>
          <button
            :aria-label="
              animation.playing.value ? 'Pause animation' : 'Play animation'
            "
            :aria-pressed="animation.playing.value"
            @click="
              animation.playing.value ? animation.pause() : animation.play()
            "
          >
            <PhPause v-if="animation.playing.value" /><PhPlay v-else />
          </button>
          <button aria-label="Next frame" @click="animation.next">
            <PhCaretRight />
          </button>
          <button
            class="loop-toggle"
            :aria-pressed="animation.loop.value"
            @click="animation.loop.value = !animation.loop.value"
          >
            Loop
          </button>
        </div>

        <label class="frame-duration">
          <span>Duration</span>
          <input
            aria-label="Selected frame duration in milliseconds"
            type="number"
            min="1"
            :value="
              animation.frames.value[animation.selectedIndex.value]?.duration_ms
            "
            @change="setDuration"
          />
          <span>ms</span>
        </label>

        <div class="timeline-actions">
          <button aria-label="Add blank frame" @click="addFrame">
            <PhPlus />
          </button>
          <button aria-label="Duplicate selected frame" @click="duplicate">
            <PhCopy />
          </button>
          <button aria-label="Delete selected frame" @click="remove">
            <PhTrash />
          </button>
          <div class="timeline-overflow">
            <button
              aria-label="More frame actions"
              :aria-expanded="overflowOpen"
              @click="overflowOpen = !overflowOpen"
            >
              •••
            </button>
            <div v-if="overflowOpen" role="menu">
              <button role="menuitem" @click="duplicate">
                <PhCopy /> Duplicate
              </button>
              <button role="menuitem" @click="remove">
                <PhTrash /> Delete
              </button>
            </div>
          </div>
        </div>
      </div>

      <ol ref="strip" class="frame-strip" aria-label="Ordered frames">
        <li v-for="(frame, index) in animation.frames.value" :key="frame.id">
          <button
            class="frame-card"
            :class="{
              'is-playhead':
                animation.playing.value &&
                frame.id === animation.selectedFrameId.value,
            }"
            :aria-label="`Frame ${index + 1}, ${frame.duration_ms} milliseconds`"
            :aria-current="
              frame.id === animation.selectedFrameId.value ? 'true' : undefined
            "
            @click="animation.select(frame.id)"
            @keydown="frameKeydown($event, frame.id)"
          >
            <img :src="animation.thumbnails.value[frame.id]" alt="" />
            <span>Frame {{ index + 1 }}</span>
            <small>{{ frame.duration_ms }} ms</small>
          </button>
        </li>
        <li>
          <button class="add-frame-card" @click="addFrame">
            <PhPlus /><span>Add Frame</span>
          </button>
        </li>
      </ol>
      <p class="timeline-keyboard-hint">
        Shift + ←/→ reorders the focused frame
      </p>
    </template>
  </section>
</template>
