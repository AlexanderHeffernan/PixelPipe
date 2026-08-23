<script setup lang="ts">
import { computed, ref } from "vue";
import { useWorkspace } from "../../workspace/context";
const workspace = useWorkspace();
const chosenConnector = ref("amp");
const approved = computed(
  () =>
    workspace.connectors.value.find(({ id }) => id === chosenConnector.value)
      ?.approved,
);
</script>

<template>
  <div class="stage-view">
    <div class="stage-intro">
      <span class="step-number">2</span>
      <div>
        <p class="eyebrow">Find the visual direction</p>
        <h2>Generate smooth references</h2>
        <p>
          PixelPipe asks your agent for options. Nothing becomes project art
          until you choose it.
        </p>
      </div>
    </div>

    <section class="connector-bar">
      <div class="connector-choice">
        <button
          v-for="connector in workspace.connectors.value"
          :key="connector.id"
          :class="{ selected: chosenConnector === connector.id }"
          :disabled="!connector.installed"
          @click="chosenConnector = connector.id"
        >
          <span class="connector-logo">{{ connector.name.slice(0, 1) }}</span
          ><span
            ><strong>{{ connector.name }}</strong
            ><small>{{
              connector.installed
                ? connector.approved
                  ? "Connected"
                  : "Installed"
                : "Not installed"
            }}</small></span
          >
        </button>
      </div>
      <button
        v-if="!approved"
        class="primary"
        :disabled="
          !workspace.connectors.value.find(({ id }) => id === chosenConnector)
            ?.installed
        "
        @click="workspace.connect(chosenConnector)"
      >
        Connect {{ chosenConnector === "amp" ? "Amp" : "Codex" }}
      </button>
      <button
        v-else-if="!workspace.agentBusy.value"
        class="primary"
        @click="workspace.generate(chosenConnector)"
      >
        Generate 3 Options
      </button>
      <button v-else @click="workspace.cancelGeneration">
        Cancel Generation
      </button>
      <span class="or">or</span
      ><button @click="workspace.importReference">Import PNG…</button>
    </section>

    <div v-if="workspace.agentBusy.value" class="generation-progress">
      <span class="spinner"></span>
      <div>
        <strong>{{ workspace.agentStatus.value }}</strong>
        <p>
          This can take a few minutes. Only fully downloaded and validated PNGs
          will appear. You can cancel safely at any time.
        </p>
      </div>
    </div>
    <section
      v-if="workspace.activeRun.value?.candidates.length"
      class="candidate-grid"
    >
      <article
        v-for="candidate in workspace.activeRun.value.candidates"
        :key="candidate.id"
      >
        <div class="candidate-image checker">
          <img
            :src="workspace.candidateImages.value[candidate.id]"
            :alt="`Generated option ${candidate.id}`"
          />
        </div>
        <footer>
          <div>
            <strong>{{ candidate.id }}</strong
            ><small>{{ candidate.width }}×{{ candidate.height }}</small>
          </div>
          <button
            class="primary"
            @click="
              workspace.selectCandidate(
                workspace.activeRun.value!.id,
                candidate.id,
              )
            "
          >
            Use This
          </button>
        </footer>
      </article>
    </section>
    <div v-else-if="!workspace.agentBusy.value" class="empty-state">
      <span>✦</span>
      <h3>No references yet</h3>
      <p>
        Connect the agent you already use, or import an existing smooth PNG.
      </p>
    </div>
  </div>
</template>
