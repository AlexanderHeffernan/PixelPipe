<script setup lang="ts">
import { useWorkspace } from "../workspace/context";
const workspace = useWorkspace();
</script>

<template>
  <section class="welcome-view">
    <div class="welcome-icon" aria-hidden="true">
      <span></span><span></span><span></span><span></span>
    </div>
    <p class="eyebrow">AI-first pixel art</p>
    <h1>Make game-ready sprites<br />inside your project.</h1>
    <p class="welcome-copy">
      Choose your game folder. PixelPipe adds its project metadata, sensible
      starter recipes, and keeps every revision reproducible.
    </p>
    <button
      class="primary large"
      :disabled="workspace.busy.value"
      @click="workspace.chooseProject"
    >
      {{ workspace.busy.value ? "Opening…" : "Open Project Folder…" }}
    </button>
    <small
      >Your existing files stay where they are. PixelPipe works from a hidden
      <code>.pixelpipe</code> folder.</small
    >
  </section>
</template>
