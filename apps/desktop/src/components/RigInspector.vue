<script setup lang="ts">
import { computed } from "vue";
import { PhArrowLeft, PhArrowRight, PhCaretRight } from "@phosphor-icons/vue";
import { useWorkspace } from "../workspace/context";
import HintTip from "./HintTip.vue";

const workspace = useWorkspace();
const rig = workspace.rig;
const interpolationEnabled = computed(
  () => (rig.rig.value?.interpolation.inbetweens ?? 0) > 0,
);

function number(event: Event) {
  return Number((event.target as HTMLInputElement).value);
}

function updateNode(values: Parameters<typeof rig.updateSelected>[0]) {
  void rig.updateSelected(values);
}

function toggleInterpolation() {
  void rig.mutate({
    type: "set_interpolation",
    inbetweens: interpolationEnabled.value ? 0 : 1,
    looped: rig.rig.value?.interpolation.looped ?? false,
  });
}

function setInbetweens(event: Event) {
  void rig.mutate({
    type: "set_interpolation",
    inbetweens: Math.max(1, Math.min(120, Math.round(number(event)))),
    looped: rig.rig.value?.interpolation.looped ?? false,
  });
}

function setLoop() {
  void rig.mutate({
    type: "set_interpolation",
    inbetweens: rig.rig.value?.interpolation.inbetweens ?? 1,
    looped: !(rig.rig.value?.interpolation.looped ?? false),
  });
}
</script>

<template>
  <header class="inspector-heading">
    <div><span>Step 2</span><strong>Rig Motion</strong></div>
  </header>

  <details class="inspector-panel" open>
    <summary>
      <strong>Selected Sprite</strong>
      <HintTip
        text="Select a joint on the canvas, then correct its sprite or layer."
      />
      <PhCaretRight class="section-chevron" aria-hidden="true" />
    </summary>
    <div v-if="rig.selectedNode.value" class="panel-content rig-inspector">
      <div class="selected-rig-node">
        <span>Selected joint</span>
        <strong>{{ rig.selectedNode.value.node_id }}</strong>
      </div>
      <p class="rig-part-instruction">
        Choose its sprite. Hover or focus to preview before saving.
      </p>
      <div class="rig-part-grid" aria-label="Available sprites">
        <button
          v-for="part in rig.partChoices.value"
          :key="part.id"
          :aria-label="`Use sprite ${part.id}`"
          :aria-pressed="rig.selectedNode.value.part_id === part.id"
          @mouseenter="rig.previewSelectedPart(part.id)"
          @mouseleave="rig.clearPartPreview"
          @focus="rig.previewSelectedPart(part.id)"
          @blur="rig.clearPartPreview"
          @click="rig.assignSelectedPart(part.id)"
        >
          <span class="rig-part-thumbnail checker">
            <img :src="part.href" alt="" />
          </span>
          <span>{{ part.id }}</span>
        </button>
      </div>
    </div>
    <div v-else class="panel-content rig-empty-selection">
      Select a joint on the canvas to edit its sprite.
    </div>
  </details>

  <details v-if="rig.selectedNode.value" class="inspector-panel" open>
    <summary>
      <strong>Transform</strong>
      <HintTip
        text="Fine-tune the selected sprite after positioning its joint visually."
      />
      <PhCaretRight class="section-chevron" aria-hidden="true" />
    </summary>
    <div class="panel-content rig-transform-grid">
      <label
        ><span>Rotation</span
        ><input
          aria-label="Selected sprite rotation in degrees"
          type="number"
          step="1"
          :value="rig.selectedNode.value.rotation_millidegrees / 1000"
          @change="
            updateNode({
              rotation_millidegrees: Math.round(number($event) * 1000),
            })
          "
        /><small>°</small></label
      >
      <label
        ><span>Layer</span
        ><input
          aria-label="Selected sprite layer"
          type="number"
          step="1"
          :value="rig.selectedNode.value.depth"
          @change="updateNode({ depth: Math.round(number($event)) })"
        /><small>z</small></label
      >
      <label
        ><span>Width</span
        ><input
          aria-label="Selected sprite width percent"
          type="number"
          :value="rig.selectedNode.value.scale_x_millis / 10"
          @change="
            updateNode({ scale_x_millis: Math.round(number($event) * 10) })
          "
        /><small>%</small></label
      >
      <label
        ><span>Height</span
        ><input
          aria-label="Selected sprite height percent"
          type="number"
          :value="rig.selectedNode.value.scale_y_millis / 10"
          @change="
            updateNode({ scale_y_millis: Math.round(number($event) * 10) })
          "
        /><small>%</small></label
      >
    </div>
  </details>

  <details class="inspector-panel" open>
    <summary>
      <strong>Automatic In-betweens</strong>
      <HintTip
        text="Pixelate can add hidden frames between each manual pose."
      />
      <PhCaretRight class="section-chevron" aria-hidden="true" />
    </summary>
    <div class="panel-content rig-motion-settings">
      <button
        class="rig-setting-toggle"
        role="switch"
        :aria-checked="interpolationEnabled"
        @click="toggleInterpolation"
      >
        <span>Enable interpolation</span><i aria-hidden="true"></i>
      </button>
      <label class="rig-field" :class="{ disabled: !interpolationEnabled }">
        <span>Frames between each pose</span>
        <input
          aria-label="Automatic frames between manual poses"
          type="number"
          min="1"
          max="120"
          :disabled="!interpolationEnabled"
          :value="rig.rig.value?.interpolation.inbetweens || 1"
          @change="setInbetweens"
        />
      </label>
      <button
        class="rig-setting-toggle"
        role="switch"
        :disabled="!interpolationEnabled"
        :aria-checked="rig.rig.value?.interpolation.looped ?? false"
        @click="setLoop"
      >
        <span>Blend last pose back to first</span><i aria-hidden="true"></i>
      </button>
    </div>
  </details>

  <div class="inspector-spacer"></div>
  <footer class="phase-action canvas-actions">
    <button
      v-if="workspace.canConvert.value"
      class="back-button continue-button"
      :disabled="workspace.busy.value"
      @click="workspace.reconvert"
    >
      <PhArrowLeft aria-hidden="true" /><span>Back to Pixelize</span>
    </button>
    <button
      class="primary continue-button"
      :disabled="workspace.busy.value"
      @click="rig.bake"
    >
      Proceed to Touch Ups <PhArrowRight aria-hidden="true" />
    </button>
  </footer>
</template>
