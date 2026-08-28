<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "quiet" | "danger";
    size?: "small" | "regular";
    type?: "button" | "submit";
    disabled?: boolean;
  }>(),
  { variant: "secondary", size: "regular", type: "button", disabled: false },
);
</script>

<template>
  <button
    :type="type"
    :disabled="disabled"
    class="app-button"
    :class="[`app-button--${variant}`, `app-button--${size}`]"
  >
    <slot />
  </button>
</template>
