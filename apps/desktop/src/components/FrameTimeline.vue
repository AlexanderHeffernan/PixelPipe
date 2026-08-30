<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
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
import { useTimelineFrames } from "../workspace/timeline-frames";

const workspace = useWorkspace();
const animation = workspace.animation;
const open = animation.timelineOpen;
const strip = ref<HTMLElement>();
const overflowOpen = ref(false);
const timelineFrames = computed(() =>
  workspace.rig.rig.value
    ? workspace.rig.manualFrames.value
    : animation.frames.value,
);
const timelineActions = {
  frames: timelineFrames,
  async mutate(
    action: import("../api").FrameMutationAction,
    preferredId?: string,
  ) {
    if (!workspace.rig.rig.value) return animation.mutate(action, preferredId);
    if (action.type === "set_all_durations")
      return workspace.rig.mutate({
        type: "set_duration",
        duration_ms: action.duration_ms,
      });
    if (action.type === "reorder")
      return workspace.rig.mutate(
        {
          type: "reorder_pose",
          pose_id: action.frame_id,
          position: action.position,
        },
        preferredId,
      );
    if (action.type === "rename")
      return workspace.rig.mutate(
        { type: "rename_pose", pose_id: action.frame_id, name: action.name },
        preferredId,
      );
    if (action.type === "delete")
      return workspace.rig.mutate({
        type: "delete_pose",
        pose_id: action.frame_id,
      });
  },
};
const {
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
} = useTimelineFrames(timelineActions, strip);
const {
  resizing,
  height,
  maximumHeight,
  minimal,
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
  const selected = strip.value?.querySelector<HTMLElement>(
    "[aria-current='true']",
  );
  if (!selected || !strip.value) return;
  const stripBounds = strip.value.getBoundingClientRect();
  const selectedBounds = selected.getBoundingClientRect();
  if (
    selectedBounds.left < stripBounds.left ||
    selectedBounds.right > stripBounds.right
  )
    selected.scrollIntoView?.({
      block: "nearest",
      inline: "nearest",
      behavior: "smooth",
    });
}

function closeTimeline() {
  open.value = false;
  overflowOpen.value = false;
  animation.pause();
}

function addFrame() {
  if (workspace.rig.rig.value)
    void workspace.rig.duplicatePose(animation.selectedFrameId.value);
  else void animation.addFrameFromImage();
}

function duplicate() {
  if (workspace.rig.rig.value) {
    void workspace.rig.duplicatePose(animation.selectedFrameId.value);
    return;
  }
  void animation.mutate({
    type: "duplicate",
    frame_id: animation.selectedFrameId.value,
    position: animation.selectedIndex.value + 1,
  });
}

function remove() {
  if (workspace.rig.rig.value) {
    void workspace.rig.mutate({
      type: "delete_pose",
      pose_id: animation.selectedFrameId.value,
    });
    return;
  }
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
      'is-frame-dragging': draggedFrameId,
      'is-minimal': minimal,
      'is-compact': !expanded,
      'is-expanded': expanded,
    }"
    :style="
      open
        ? { height: `${height}px`, '--timeline-height': `${height}px` }
        : undefined
    "
    aria-label="Frame timeline"
  >
    <div v-if="!open" class="timeline-closed">
      <button aria-label="Open frame timeline" @click="open = true">
        <PhFilmStrip />
        <span>Animation</span>
        <small>
          {{ timelineFrames.length }}
          {{ workspace.rig.rig.value ? "poses" : "frames" }}
        </small>
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
          <small>
            {{ timelineFrames.length }}
            {{ workspace.rig.rig.value ? "poses" : "frames" }}
            <template v-if="workspace.rig.rig.value?.interpolation.inbetweens">
              ·
              {{ animation.frames.value.length - timelineFrames.length }}
              automatic
            </template>
          </small>
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
          <span>Frame duration</span>
          <input
            aria-label="Animation frame duration in milliseconds"
            type="number"
            min="1"
            :value="animation.frames.value[0]?.duration_ms"
            @change="setDuration"
          />
          <span>ms</span>
        </label>

        <div class="timeline-actions">
          <button
            :aria-label="
              workspace.rig.rig.value
                ? 'Duplicate pose'
                : 'Add frame from image'
            "
            @click="addFrame"
          >
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
              <button
                role="menuitem"
                @click="reorder(animation.selectedFrameId.value, -1)"
              >
                <PhCaretLeft /> Move earlier
              </button>
              <button
                role="menuitem"
                @click="reorder(animation.selectedFrameId.value, 1)"
              >
                <PhCaretRight /> Move later
              </button>
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
        <li
          v-for="(frame, index) in timelineFrames"
          :key="frame.id"
          :data-frame-index="index"
          :class="{
            'is-dragging': draggedFrameId === frame.id,
            'drop-before': draggedFrameId && dropPosition === index,
            'drop-after':
              draggedFrameId &&
              dropPosition === timelineFrames.length &&
              index === timelineFrames.length - 1,
          }"
          @pointerdown="pointerDown($event, frame.id)"
          @contextmenu="openFrameMenu($event, frame.id)"
        >
          <div class="frame-card">
            <button
              class="frame-select"
              :class="{
                'is-playhead':
                  animation.playing.value &&
                  frame.id === animation.selectedFrameId.value,
              }"
              :aria-label="`${frame.name ?? `Frame ${index + 1}`}, ${frame.duration_ms} milliseconds`"
              :aria-current="
                frame.id === animation.selectedFrameId.value
                  ? 'true'
                  : undefined
              "
              @click="animation.select(frame.id)"
            >
              <img
                :src="animation.thumbnails.value[frame.id]"
                alt=""
                draggable="false"
              />
            </button>
            <input
              v-if="editingFrameId === frame.id"
              v-model="editName"
              :data-frame-name="frame.id"
              aria-label="Frame name"
              @click.stop
              @keydown.enter.prevent="finishRename(frame.id)"
              @keydown.esc.prevent="editingFrameId = ''"
              @blur="finishRename(frame.id)"
            />
            <span v-else>{{ frame.name ?? `Frame ${index + 1}` }}</span>
          </div>
          <div
            v-if="contextFrameId === frame.id"
            class="frame-context"
            role="menu"
          >
            <button role="menuitem" @click="beginRename(frame.id)">
              Rename
            </button>
            <button role="menuitem" @click="deleteFrame(frame.id)">
              Delete
            </button>
          </div>
        </li>
      </ol>
    </template>
  </section>
</template>
