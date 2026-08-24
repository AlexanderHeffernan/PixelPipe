<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { useWorkspace, type AssetSource } from "../workspace/context";

const workspace = useWorkspace();
const name = ref("");
const brief = ref("");
const source = ref<AssetSource>("reference");
const nameInput = ref<HTMLInputElement>();
const previousFocus = document.activeElement as HTMLElement | null;

const create = () =>
  workspace.createAsset(name.value, brief.value, source.value);
onMounted(() => nameInput.value?.focus());
onUnmounted(() => previousFocus?.focus());
</script>

<template>
  <div
    class="dialog-backdrop"
    @click.self="workspace.createAssetOpen.value = false"
  >
    <form
      class="create-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="create-asset-title"
      @submit.prevent="create"
      @keydown.esc="workspace.createAssetOpen.value = false"
    >
      <header>
        <div>
          <h1 id="create-asset-title">Create Asset</h1>
          <p>Start with an image, or prepare a place for your coding agent.</p>
        </div>
        <button
          type="button"
          class="icon-button"
          aria-label="Close"
          @click="workspace.createAssetOpen.value = false"
        >
          ×
        </button>
      </header>

      <label class="dialog-field">
        <span>Name</span>
        <input
          ref="nameInput"
          v-model="name"
          required
          placeholder="Health Potion"
        />
      </label>
      <label class="dialog-field">
        <span>Brief <small>Optional</small></span>
        <textarea
          v-model="brief"
          rows="3"
          placeholder="A small round red health potion with a cork…"
        ></textarea>
      </label>

      <fieldset class="source-options">
        <legend>Starting point</legend>
        <label :class="{ selected: source === 'reference' }">
          <input v-model="source" type="radio" value="reference" />
          <span class="source-icon">↥</span>
          <span
            ><strong>Choose an image</strong
            ><small>Import a smooth PNG reference</small></span
          >
        </label>
        <label :class="{ selected: source === 'agent' }">
          <input v-model="source" type="radio" value="agent" />
          <span class="source-icon">⌁</span>
          <span
            ><strong>Use my coding agent</strong
            ><small
              >Create the asset now; references can arrive through the
              CLI</small
            ></span
          >
        </label>
      </fieldset>

      <footer>
        <button
          type="button"
          class="quiet"
          @click="workspace.createAssetOpen.value = false"
        >
          Cancel
        </button>
        <button
          class="primary"
          :disabled="!name.trim() || workspace.busy.value"
        >
          {{ workspace.busy.value ? "Creating…" : "Create Asset" }}
        </button>
      </footer>
    </form>
  </div>
</template>
