<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  PhArrowLeft,
  PhArrowRight,
  PhArrowsOutLineHorizontal,
  PhCaretRight,
  PhEye,
  PhEyeSlash,
} from "@phosphor-icons/vue";
import { useWorkspace } from "../workspace/context";
import HintTip from "./HintTip.vue";

const workspace = useWorkspace();
const rig = workspace.rig;
const swapTarget = ref("");
const interpolationEnabled = computed(
  () => (rig.rig.value?.interpolation.inbetweens ?? 0) > 0,
);

watch(rig.selectedNodeId, () => {
  swapTarget.value = "";
});

function number(event: Event) {
  return Number((event.target as HTMLInputElement).value);
}

function updateNode(values: Parameters<typeof rig.updateSelected>[0]) {
  void rig.updateSelected(values);
}

function toggleInterpolation(event: Event) {
  void rig.mutate({
    type: "set_interpolation",
    inbetweens: (event.target as HTMLInputElement).checked ? 1 : 0,
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

function setLoop(event: Event) {
  void rig.mutate({
    type: "set_interpolation",
    inbetweens: rig.rig.value?.interpolation.inbetweens ?? 1,
    looped: (event.target as HTMLInputElement).checked,
  });
}

function swapParts() {
  if (!swapTarget.value || !rig.selectedNodeId.value) return;
  void rig.mutate({
    type: "swap_parts",
    first_node_id: rig.selectedNodeId.value,
    second_node_id: swapTarget.value,
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
      <p class="selected-rig-node">
        <span>Joint</span><strong>{{ rig.selectedNode.value.node_id }}</strong>
      </p>
      <label class="rig-field">
        <span>Sprite</span>
        <select
          aria-label="Sprite assigned to selected joint"
          :value="rig.selectedNode.value.part_id"
          @change="
            updateNode({
              part_id: ($event.target as HTMLSelectElement).value,
            })
          "
        >
          <option
            v-for="part in rig.rig.value?.parts"
            :key="part.id"
            :value="part.id"
          >
            {{ part.id }}
          </option>
        </select>
      </label>
      <div class="rig-swap">
        <label class="rig-field">
          <span>Swap sprites with</span>
          <select v-model="swapTarget" aria-label="Joint to swap sprites with">
            <option value="">Choose another joint…</option>
            <option
              v-for="node in rig.rig.value?.nodes.filter(
                (node) => node.id !== rig.selectedNodeId.value,
              )"
              :key="node.id"
              :value="node.id"
            >
              {{ node.id }}
            </option>
          </select>
        </label>
        <button
          aria-label="Swap joint sprites"
          :disabled="!swapTarget"
          @click="swapParts"
        >
          <PhArrowsOutLineHorizontal />
        </button>
      </div>
      <div class="rig-layer-actions" role="group" aria-label="Sprite layer">
        <button
          @click="updateNode({ depth: rig.selectedNode.value.depth - 1 })"
        >
          Send backward
        </button>
        <button
          @click="updateNode({ depth: rig.selectedNode.value.depth + 1 })"
        >
          Bring forward
        </button>
      </div>
      <button
        class="rig-visibility"
        :aria-label="
          rig.selectedNode.value.visible
            ? 'Hide selected sprite'
            : 'Show selected sprite'
        "
        :aria-pressed="!rig.selectedNode.value.visible"
        @click="updateNode({ visible: !rig.selectedNode.value.visible })"
      >
        <PhEye v-if="rig.selectedNode.value.visible" />
        <PhEyeSlash v-else />
        {{ rig.selectedNode.value.visible ? "Visible" : "Hidden" }}
      </button>
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
      <label class="rig-toggle">
        <input
          type="checkbox"
          :checked="interpolationEnabled"
          @change="toggleInterpolation"
        />
        <span>Enable interpolation</span>
      </label>
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
      <label class="rig-toggle" :class="{ disabled: !interpolationEnabled }">
        <input
          type="checkbox"
          :disabled="!interpolationEnabled"
          :checked="rig.rig.value?.interpolation.looped"
          @change="setLoop"
        />
        <span>Blend the last pose back to the first</span>
      </label>
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
