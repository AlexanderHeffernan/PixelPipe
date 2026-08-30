<script setup lang="ts">
import { useWorkspace } from "../workspace/context";

const workspace = useWorkspace();

function matrix(transform: {
  a: number;
  b: number;
  c: number;
  d: number;
  tx: number;
  ty: number;
}) {
  return `matrix(${transform.a} ${transform.b} ${transform.c} ${transform.d} ${transform.tx} ${transform.ty})`;
}
</script>

<template>
  <svg
    class="rig-artwork"
    :viewBox="`0 0 ${workspace.inspection.value?.width ?? 1} ${workspace.inspection.value?.height ?? 1}`"
    aria-hidden="true"
  >
    <image
      v-for="item in workspace.rig.artwork.value"
      :key="item.nodeId"
      :href="item.href"
      :x="-item.part.pivot[0]"
      :y="-item.part.pivot[1]"
      :width="item.part.width"
      :height="item.part.height"
      :transform="matrix(item.matrix)"
    />
  </svg>
</template>
