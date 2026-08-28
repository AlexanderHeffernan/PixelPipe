<script setup lang="ts">
import { PhX } from "@phosphor-icons/vue";

defineProps<{ message: string; error?: boolean }>();
defineEmits<{ close: [] }>();
</script>

<template>
  <div class="toast" :class="{ error }" :role="error ? 'alert' : 'status'">
    <span>{{ message }}</span>
    <button aria-label="Dismiss message" @click="$emit('close')">
      <PhX aria-hidden="true" />
    </button>
  </div>
</template>
