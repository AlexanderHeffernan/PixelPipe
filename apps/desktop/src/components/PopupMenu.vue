<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";

const emit = defineEmits<{ close: [] }>();
const root = ref<HTMLElement>();
const outside = (event: PointerEvent) => {
  if (!root.value?.contains(event.target as Node)) emit("close");
};
const escape = (event: KeyboardEvent) => {
  if (event.key === "Escape") emit("close");
};
onMounted(() => {
  document.addEventListener("pointerdown", outside);
  document.addEventListener("keydown", escape);
});
onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", outside);
  document.removeEventListener("keydown", escape);
});
</script>

<template>
  <div ref="root" role="menu"><slot /></div>
</template>
