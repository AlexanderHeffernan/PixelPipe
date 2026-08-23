<script setup lang="ts">
import { computed, ref } from "vue";
import { useWorkspace, type WorkspaceStage } from "../workspace/context";

const workspace = useWorkspace();
const name = ref("");
const brief = ref("");
const asset = computed(() => workspace.selectedAsset.value?.asset);

const stages: Array<{ id: WorkspaceStage; label: string }> = [
  { id: "brief", label: "Brief" },
  { id: "reference", label: "Reference" },
  { id: "pixelize", label: "Pixelize" },
  { id: "review", label: "Review" },
  { id: "export", label: "Export" },
];

function available(stage: WorkspaceStage) {
  if (!asset.value) return false;
  if (stage === "brief") return true;
  if (stage === "reference") return Boolean(asset.value.brief.text.trim());
  if (stage === "pixelize") return Boolean(asset.value.selected_reference);
  return Boolean(asset.value.head);
}

function complete(stage: WorkspaceStage) {
  if (!asset.value) return false;
  if (stage === "brief") return Boolean(asset.value.brief.text.trim());
  if (stage === "reference") return Boolean(asset.value.selected_reference);
  if (stage === "pixelize") return Boolean(asset.value.head);
  return false;
}

async function create() {
  await workspace.createAsset(name.value, brief.value);
  if (!workspace.error.value) {
    name.value = "";
    brief.value = "";
    workspace.creatingAsset.value = false;
  }
}
</script>

<template>
  <aside class="project-sidebar">
    <div class="project-identity">
      <span class="project-avatar">{{
        workspace.project.value!.project.name.slice(0, 1).toUpperCase()
      }}</span>
      <div>
        <strong>{{ workspace.project.value!.project.name }}</strong
        ><small>PixelPipe project</small>
      </div>
    </div>

    <section class="sidebar-section">
      <div class="section-heading">
        <span>Assets</span
        ><button
          class="add-button"
          aria-label="Create sprite"
          @click="
            workspace.creatingAsset.value = !workspace.creatingAsset.value
          "
        >
          +
        </button>
      </div>
      <form
        v-if="workspace.creatingAsset.value"
        class="new-asset-form"
        @submit.prevent="create"
      >
        <input v-model="name" required autofocus placeholder="Sprite name" />
        <textarea
          v-model="brief"
          rows="3"
          placeholder="What should it look like?"
        ></textarea>
        <div>
          <button
            type="button"
            class="quiet"
            @click="workspace.creatingAsset.value = false"
          >
            Cancel</button
          ><button class="primary" :disabled="!name.trim()">Create</button>
        </div>
      </form>
      <nav class="asset-list" aria-label="Project assets">
        <button
          v-for="entry in workspace.project.value!.assets"
          :key="entry.asset.id"
          :aria-current="
            workspace.assetId.value === entry.asset.id ? 'page' : undefined
          "
          @click="workspace.selectAsset(entry.asset.id)"
        >
          <span class="asset-thumb">◆</span
          ><span
            ><strong>{{ entry.asset.id }}</strong
            ><small>{{ entry.asset.state.replaceAll("_", " ") }}</small></span
          >
        </button>
        <p v-if="!workspace.project.value!.assets.length">
          Create your first sprite.
        </p>
      </nav>
    </section>

    <section v-if="asset" class="sidebar-section workflow-nav">
      <div class="section-heading"><span>Workflow</span></div>
      <button
        v-for="(item, index) in stages"
        :key="item.id"
        :disabled="!available(item.id)"
        :aria-current="workspace.stage.value === item.id ? 'step' : undefined"
        @click="workspace.stage.value = item.id"
      >
        <span class="stage-status" :class="{ complete: complete(item.id) }">{{
          complete(item.id) ? "✓" : index + 1
        }}</span>
        <span>{{ item.label }}</span>
      </button>
    </section>

    <footer><span>Deterministic by default</span><code>v0.1</code></footer>
  </aside>
</template>
