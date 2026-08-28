<script setup lang="ts">
import logoUrl from "../../src-tauri/icons/icon-source.png";
import { useWorkspace } from "../workspace/context";

const workspace = useWorkspace();
</script>

<template>
  <section class="welcome-view">
    <div class="welcome-content">
      <img class="welcome-logo" :src="logoUrl" alt="" />
      <h1>Pixelate</h1>
      <p>From reference to game-ready pixel art.</p>
      <button
        class="primary welcome-open-button"
        :disabled="workspace.busy.value"
        @click="workspace.chooseProject"
      >
        {{ workspace.busy.value ? "Opening…" : "Open Project…" }}
      </button>
      <small>Choose a new or existing game folder to begin.</small>
    </div>
  </section>
</template>
