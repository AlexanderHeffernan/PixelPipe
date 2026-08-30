<script setup lang="ts">
import { ref, watch } from "vue";
import {
  PhArrowsOutLineHorizontal,
  PhCheck,
  PhHammer,
} from "@phosphor-icons/vue";
import { useWorkspace } from "../workspace/context";

const workspace = useWorkspace();
const rig = workspace.rig;
const swapTarget = ref("");

watch(rig.selectedNodeId, () => {
  swapTarget.value = "";
});

function number(event: Event) {
  return Number((event.target as HTMLInputElement).value);
}

function setRotation(event: Event) {
  void rig.updateSelected({
    rotation_millidegrees: Math.round(number(event) * 1000),
  });
}

function setScale(axis: "x" | "y", event: Event) {
  const value = Math.round(number(event) * 10);
  void rig.updateSelected(
    axis === "x" ? { scale_x_millis: value } : { scale_y_millis: value },
  );
}

function setDepth(event: Event) {
  void rig.updateSelected({ depth: Math.round(number(event)) });
}

function setPart(event: Event) {
  void rig.updateSelected({
    part_id: (event.target as HTMLSelectElement).value,
  });
}

function setVisible(event: Event) {
  void rig.updateSelected({
    visible: (event.target as HTMLInputElement).checked,
  });
}

function setInbetweens(event: Event) {
  const value = Math.max(0, Math.min(120, Math.round(number(event))));
  void rig.mutate({
    type: "set_interpolation",
    inbetweens: value,
    looped: rig.rig.value?.interpolation.looped ?? false,
  });
}

function setLoop(event: Event) {
  void rig.mutate({
    type: "set_interpolation",
    inbetweens: rig.rig.value?.interpolation.inbetweens ?? 0,
    looped: (event.target as HTMLInputElement).checked,
  });
}

function setDuration(event: Event) {
  void rig.mutate({
    type: "set_duration",
    duration_ms: Math.max(1, Math.round(number(event))),
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
  <div class="rig-toolbar" aria-label="Rig controls">
    <label>
      <span>Node</span>
      <select v-model="rig.selectedNodeId.value" aria-label="Selected rig node">
        <option
          v-for="node in rig.rig.value?.nodes"
          :key="node.id"
          :value="node.id"
        >
          {{ node.id }}
        </option>
      </select>
    </label>
    <label>
      <span>Part</span>
      <select
        aria-label="Assigned pixel part"
        :value="rig.selectedNode.value?.part_id"
        @change="setPart"
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
    <label>
      <span>Rotate</span>
      <input
        aria-label="Node rotation in degrees"
        type="number"
        step="1"
        :value="(rig.selectedNode.value?.rotation_millidegrees ?? 0) / 1000"
        @change="setRotation"
      />
      <small>°</small>
    </label>
    <label>
      <span>Scale</span>
      <input
        aria-label="Node horizontal scale percent"
        type="number"
        :value="(rig.selectedNode.value?.scale_x_millis ?? 1000) / 10"
        @change="setScale('x', $event)"
      />
      <span>×</span>
      <input
        aria-label="Node vertical scale percent"
        type="number"
        :value="(rig.selectedNode.value?.scale_y_millis ?? 1000) / 10"
        @change="setScale('y', $event)"
      />
    </label>
    <label>
      <span>Depth</span>
      <input
        aria-label="Node depth"
        type="number"
        :value="rig.selectedNode.value?.depth ?? 0"
        @change="setDepth"
      />
    </label>
    <label class="rig-check">
      <input
        aria-label="Show selected node"
        type="checkbox"
        :checked="rig.selectedNode.value?.visible"
        @change="setVisible"
      />
      <PhCheck />
      <span>Visible</span>
    </label>
    <span class="toolbar-divider"></span>
    <label>
      <span>Between</span>
      <input
        aria-label="Automatic frames between manual poses"
        type="number"
        min="0"
        max="120"
        :value="rig.rig.value?.interpolation.inbetweens ?? 0"
        @change="setInbetweens"
      />
    </label>
    <label class="rig-check">
      <input
        aria-label="Interpolate final pose to first pose"
        type="checkbox"
        :checked="rig.rig.value?.interpolation.looped"
        @change="setLoop"
      />
      <PhCheck />
      <span>Close loop</span>
    </label>
    <label>
      <span>Timing</span>
      <input
        aria-label="Rig frame duration in milliseconds"
        type="number"
        min="1"
        :value="rig.rig.value?.frame_duration_ms"
        @change="setDuration"
      />
      <small>ms</small>
    </label>
    <span class="toolbar-divider"></span>
    <label>
      <span>Swap with</span>
      <select v-model="swapTarget" aria-label="Node to swap parts with">
        <option value="">Choose…</option>
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
      aria-label="Swap assigned parts"
      :disabled="!swapTarget"
      @click="swapParts"
    >
      <PhArrowsOutLineHorizontal />
    </button>
    <button
      aria-label="Bake rig for pixel editing"
      title="Bake rig"
      @click="rig.bake"
    >
      <PhHammer />
    </button>
  </div>
</template>
