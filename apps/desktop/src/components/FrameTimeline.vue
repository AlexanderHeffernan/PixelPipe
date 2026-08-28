<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import {
  PhArrowsOutSimple,
  PhArrowsInSimple,
  PhCaretLeft,
  PhCaretRight,
  PhCopy,
  PhPause,
  PhPlay,
  PhPlus,
  PhTrash,
} from "@phosphor-icons/vue";
import { useWorkspace } from "../workspace/context";

const workspace = useWorkspace();
const strip = ref<HTMLElement>();
const overflowOpen = ref(false);
const animation = workspace.animation;

watch(animation.selectedFrameId, () => void keepSelectedVisible());

async function keepSelectedVisible() {
  await nextTick();
  strip.value
    ?.querySelector<HTMLElement>("[aria-current='true']")
    ?.scrollIntoView?.({ block: "nearest", inline: "nearest" });
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
  const position = animation.selectedIndex.value + 1;
  void animation.mutate({
    type: "duplicate",
    frame_id: animation.selectedFrameId.value,
    position,
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
    v-if="workspace.mode.value === 'edit' && workspace.view.value"
    class="frame-timeline"
    :class="`is-${animation.density.value}`"
    aria-label="Frame timeline"
  >
    <div v-if="animation.frames.value.length === 1" class="single-frame-bar">
      <span>1 frame</span><i aria-hidden="true">—</i>
      <button @click="addFrame"><PhPlus /> Add Frame</button>
    </div>

    <template v-else>
      <header class="timeline-controls">
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
          <button
            :aria-label="
              animation.density.value === 'compact'
                ? 'Expand timeline'
                : 'Collapse timeline'
            "
            @click="
              animation.density.value =
                animation.density.value === 'compact' ? 'expanded' : 'compact'
            "
          >
            <PhArrowsOutSimple v-if="animation.density.value === 'compact'" />
            <PhArrowsInSimple v-else />
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
      </header>

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
