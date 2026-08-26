<script setup lang="ts">
import { PhQuestion } from "@phosphor-icons/vue";
import { onBeforeUnmount, onMounted, ref } from "vue";

defineProps<{ text: string; label?: string; embedded?: boolean }>();

const anchor = ref<HTMLElement>();
const visible = ref(false);
const position = ref<Record<string, string>>({});

function placeTooltip() {
  if (!anchor.value || !visible.value) return;
  const rect = anchor.value.getBoundingClientRect();
  const width = Math.min(230, window.innerWidth - 16);
  const below = rect.top < 90;
  position.value = {
    left: `${Math.max(8, Math.min(window.innerWidth - width - 8, rect.right - width))}px`,
    top: `${below ? rect.bottom + 7 : rect.top - 7}px`,
    width: `${width}px`,
    transform: below ? "none" : "translateY(-100%)",
  };
}

function showTooltip() {
  visible.value = true;
  placeTooltip();
}

function hideTooltip() {
  visible.value = false;
}

onMounted(() => {
  window.addEventListener("resize", placeTooltip);
  document.addEventListener("scroll", placeTooltip, true);
});
onBeforeUnmount(() => {
  window.removeEventListener("resize", placeTooltip);
  document.removeEventListener("scroll", placeTooltip, true);
});
</script>

<template>
  <span
    ref="anchor"
    class="hint-tip"
    :tabindex="embedded ? -1 : 0"
    :aria-label="label || text"
    @click.stop
    @mouseenter="showTooltip"
    @mouseleave="hideTooltip"
    @focus="showTooltip"
    @blur="hideTooltip"
  >
    <PhQuestion weight="bold" aria-hidden="true" />
  </span>
  <Teleport to="body">
    <Transition name="hint-popover">
      <span
        v-if="visible"
        class="hint-popover"
        :style="position"
        role="tooltip"
      >
        {{ text }}
      </span>
    </Transition>
  </Teleport>
</template>
