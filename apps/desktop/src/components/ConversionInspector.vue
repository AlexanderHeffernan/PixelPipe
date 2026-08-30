<script setup lang="ts">
import { useWorkspace } from "../workspace/context";
import { useSidebarResize } from "../workspace/sidebar-resize";
import PixelizeInspector from "./PixelizeInspector.vue";
import RigInspector from "./RigInspector.vue";
import TouchUpInspector from "./TouchUpInspector.vue";

const workspace = useWorkspace();
const MIN_WIDTH = 286;
const MAX_WIDTH = 460;
const { width, isResizing, startResize, resizeWithKeyboard } = useSidebarResize(
  {
    edge: "right",
    initialWidth: 326,
    minWidth: MIN_WIDTH,
    maxWidth: MAX_WIDTH,
  },
);
</script>

<template>
  <aside
    class="conversion-inspector"
    :class="{ 'is-resizing': isResizing }"
    :style="{ '--sidebar-width': `${width}px` }"
  >
    <div class="conversion-inspector__body">
      <PixelizeInspector v-if="workspace.mode.value === 'convert'" />
      <RigInspector v-else-if="workspace.rig.rig.value" />
      <TouchUpInspector v-else />
    </div>
    <div
      class="sidebar-resize-handle sidebar-resize-handle--left"
      role="separator"
      aria-label="Resize inspector"
      aria-orientation="vertical"
      :aria-valuemin="MIN_WIDTH"
      :aria-valuemax="MAX_WIDTH"
      :aria-valuenow="width"
      tabindex="0"
      @pointerdown="startResize"
      @keydown="resizeWithKeyboard"
    />
  </aside>
</template>
