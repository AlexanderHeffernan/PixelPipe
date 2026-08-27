<script setup lang="ts">
import { useWorkspace } from "../workspace/context";
const workspace = useWorkspace();
</script>

<template>
  <section class="welcome-view">
    <div class="welcome-icon" aria-hidden="true">
      <span></span><span></span><span></span><span></span>
    </div>
    <p class="eyebrow">Deterministic pixel artwork</p>
    <h1>Convert, refine, and ship<br />game-ready sprites.</h1>
    <p class="welcome-copy">
      Open a new or existing game folder. Pixelate keeps assets and revisions
      together with your project.
    </p>
    <button
      class="primary large"
      :disabled="workspace.busy.value"
      @click="workspace.chooseProject"
    >
      {{ workspace.busy.value ? "Opening…" : "Open Project Folder…" }}
    </button>
    <small
      >Your existing files stay where they are. Pixelate works from a hidden
      <code>.pixelate</code> folder.</small
    >
  </section>
</template>
