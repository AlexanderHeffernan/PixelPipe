<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import {
  browseProject,
  compareRevisions,
  loadRevision,
  patchRevision,
  pngDataUrl,
  recordReview,
  remapRevision,
} from "./api";
import type {
  PaletteDraft,
  PixelEdit,
  ProjectBrowser,
  RevisionComparisonResponse,
  RevisionViewResponse,
  ReviewActorKind,
  ReviewDecision,
  Rgba,
} from "./types";

const projectPath = ref("");
const project = ref<ProjectBrowser>();
const assetId = ref("");
const revisionId = ref("");
const compareId = ref("");
const view = ref<RevisionViewResponse>();
const comparison = ref<RevisionComparisonResponse>();
const busy = ref(false);
const error = ref("");
const notice = ref("Enter a game project path to begin.");
const statusElement = ref<HTMLElement>();

const actor = ref("user");
const actorKind = ref<ReviewActorKind>("human");
const reviewDecision = ref<ReviewDecision>("reviewed");
const reviewNote = ref("");
const patchEdits = ref<PixelEdit[]>([{ x: 0, y: 0, index: 0 }]);
const paletteDraft = ref<PaletteDraft>();

const selectedAsset = computed(() =>
  project.value?.assets.find(({ asset }) => asset.id === assetId.value),
);
const revisions = computed(() => selectedAsset.value?.revisions ?? []);
const nativeUrl = computed(() =>
  view.value ? pngDataUrl(view.value.native_png_base64) : "",
);
const previewUrl = computed(() =>
  view.value ? pngDataUrl(view.value.preview_png_base64) : "",
);
const diffUrl = computed(() =>
  comparison.value ? pngDataUrl(comparison.value.visual_preview_png_base64) : "",
);

watch(
  view,
  (loaded) => {
    if (!loaded) return;
    const metadata = loaded.metadata;
    patchEdits.value = [{ x: 0, y: 0, index: metadata.transparent_index }];
    paletteDraft.value = {
      name: metadata.palette_name,
      transparentIndex: metadata.transparent_index,
      colors: metadata.inspection.palette.map(({ rgba }) => [...rgba] as Rgba),
      indexMap: metadata.inspection.palette.map(({ index }) => index),
    };
    const candidates = revisions.value.filter(({ id }) => id !== metadata.revision);
    compareId.value = metadata.parent ?? candidates.at(-1)?.id ?? "";
    comparison.value = undefined;
  },
  { flush: "post" },
);

async function run(action: () => Promise<void>) {
  busy.value = true;
  error.value = "";
  try {
    await action();
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : String(caught);
    notice.value = "The operation failed. Project files were not changed.";
    await nextTick();
    statusElement.value?.focus();
  } finally {
    busy.value = false;
  }
}

async function openProject() {
  await run(async () => {
    const loaded = await browseProject(projectPath.value.trim());
    project.value = loaded;
    projectPath.value = loaded.project_root;
    notice.value = `Opened ${loaded.project.name}.`;
    if (loaded.assets.length === 0) {
      assetId.value = "";
      revisionId.value = "";
      view.value = undefined;
      return;
    }
    await selectAsset(loaded.assets[0].asset.id);
  });
}

async function selectAsset(id: string) {
  assetId.value = id;
  const asset = project.value?.assets.find((entry) => entry.asset.id === id);
  const target = asset?.asset.head ?? asset?.revisions.at(-1)?.id;
  if (!target) {
    revisionId.value = "";
    view.value = undefined;
    notice.value = `${id} has no revisions yet.`;
    return;
  }
  await selectRevision(target);
}

async function selectRevision(id: string) {
  revisionId.value = id;
  comparison.value = undefined;
  if (!project.value || !assetId.value) return;
  view.value = await loadRevision(project.value.project_root, assetId.value, id);
  notice.value = `Loaded ${assetId.value} ${id}. Browsing did not change head.`;
}

async function loadSelectedRevision(id: string) {
  await run(() => selectRevision(id));
}

async function refreshAndSelect(id: string) {
  if (!project.value) return;
  project.value = await browseProject(project.value.project_root);
  await selectRevision(id);
}

async function submitComparison() {
  if (!project.value || !assetId.value || !revisionId.value || !compareId.value) return;
  await run(async () => {
    comparison.value = await compareRevisions(
      project.value!.project_root,
      assetId.value,
      compareId.value,
      revisionId.value,
    );
    notice.value = `Compared ${compareId.value} → ${revisionId.value}.`;
  });
}

async function submitReview() {
  if (!project.value || !view.value || !actor.value.trim()) return;
  await run(async () => {
    view.value!.metadata.review = await recordReview(
      project.value!.project_root,
      assetId.value,
      revisionId.value,
      actor.value.trim(),
      actorKind.value,
      reviewDecision.value,
      reviewNote.value.trim(),
    );
    reviewNote.value = "";
    notice.value = `Recorded ${reviewDecision.value.replace("_", " ")} review. Head was unchanged.`;
  });
}

async function submitPatch() {
  if (!project.value || !view.value || !actor.value.trim() || patchEdits.value.length === 0) return;
  await run(async () => {
    const result = await patchRevision(
      project.value!.project_root,
      assetId.value,
      revisionId.value,
      patchEdits.value,
      actor.value.trim(),
    );
    await refreshAndSelect(result.revision);
    notice.value = `Created ${result.revision} from explicit parent ${result.parent}.`;
  });
}

async function submitRemap() {
  if (!project.value || !view.value || !paletteDraft.value || !actor.value.trim()) return;
  await run(async () => {
    const result = await remapRevision(
      project.value!.project_root,
      assetId.value,
      revisionId.value,
      paletteDraft.value!,
      actor.value.trim(),
    );
    await refreshAndSelect(result.revision);
    notice.value = `Created ${result.revision} with an explicit palette remap.`;
  });
}

function addPatch() {
  patchEdits.value.push({ x: 0, y: 0, index: view.value?.metadata.transparent_index ?? 0 });
}

function removePatch(index: number) {
  patchEdits.value.splice(index, 1);
}

function addPaletteColor() {
  paletteDraft.value?.colors.push([0, 0, 0, 255]);
}

function removePaletteColor(index: number) {
  if (!paletteDraft.value || paletteDraft.value.colors.length === 1) return;
  paletteDraft.value.colors.splice(index, 1);
  paletteDraft.value.transparentIndex = Math.min(
    paletteDraft.value.transparentIndex,
    paletteDraft.value.colors.length - 1,
  );
}

function moveListFocus(event: KeyboardEvent) {
  if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
  const list = event.currentTarget as HTMLElement;
  const buttons = [...list.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
  if (buttons.length === 0) return;
  const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
  const target = event.key === "Home"
    ? 0
    : event.key === "End"
      ? buttons.length - 1
      : (current + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
  event.preventDefault();
  buttons[target]?.focus();
}

function colorCss(rgba: Rgba) {
  return `rgba(${rgba.join(",")})`;
}

function formatBounds(bounds?: { x: number; y: number; width: number; height: number }) {
  return bounds ? `${bounds.x},${bounds.y} · ${bounds.width}×${bounds.height}` : "empty";
}
</script>

<template>
  <div class="app-shell">
    <header class="topbar">
      <div class="brand">
        <span class="brand-mark" aria-hidden="true">P</span>
        <div><strong>PixelPipe</strong><small>Visual review workstation</small></div>
      </div>
      <form class="open-project" aria-label="Open project" @submit.prevent="openProject">
        <label for="project-path">Project path</label>
        <input id="project-path" v-model="projectPath" required placeholder="/path/to/game" />
        <button class="primary" :disabled="busy">Open</button>
      </form>
    </header>

    <div ref="statusElement" class="status" :class="{ failure: error }" tabindex="-1" role="status" aria-live="polite">
      <span v-if="busy" class="spinner" aria-hidden="true"></span>
      <strong v-if="error">{{ error }}</strong>
      <span v-else>{{ notice }}</span>
    </div>

    <main v-if="project" class="workspace" :aria-busy="busy">
      <aside class="navigator" aria-label="Project navigator">
        <div class="project-title">
          <small>Project</small>
          <h1>{{ project.project.name }}</h1>
          <code>{{ project.project_root }}</code>
        </div>
        <section>
          <h2>Assets <span>{{ project.assets.length }}</span></h2>
          <div v-if="project.assets.length" class="nav-list" @keydown="moveListFocus">
            <button
              v-for="entry in project.assets"
              :key="entry.asset.id"
              :class="{ selected: entry.asset.id === assetId }"
              :aria-label="`${entry.asset.id}, ${entry.asset.kind}`"
              :aria-current="entry.asset.id === assetId ? 'true' : undefined"
              :disabled="busy"
              @click="run(() => selectAsset(entry.asset.id))"
            >
              <span>{{ entry.asset.id }}</span><small>{{ entry.asset.kind }}</small>
            </button>
          </div>
          <p v-else class="empty">No assets. Create one with the CLI, then reopen this project.</p>
        </section>
        <section v-if="selectedAsset">
          <h2>Revisions <span>{{ revisions.length }}</span></h2>
          <div class="nav-list revisions" @keydown="moveListFocus">
            <button
              v-for="revision in revisions"
              :key="revision.id"
              :class="{ selected: revision.id === revisionId }"
              :aria-current="revision.id === revisionId ? 'true' : undefined"
              :disabled="busy"
              @click="loadSelectedRevision(revision.id)"
            >
              <span>{{ revision.id }}</span>
              <small v-if="selectedAsset.asset.head === revision.id" class="badge">head</small>
              <small v-else-if="selectedAsset.asset.approved === revision.id" class="badge approved">approved</small>
              <small v-else>{{ revision.parent ? `from ${revision.parent}` : "root" }}</small>
            </button>
          </div>
        </section>
      </aside>

      <section v-if="view" class="canvas-area" aria-label="Revision review">
        <div class="revision-heading">
          <div>
            <small>{{ selectedAsset?.asset.kind }} · immutable revision</small>
            <h2>{{ assetId }} <span>/ {{ revisionId }}</span></h2>
          </div>
          <div class="revision-links">
            <span>parent <strong>{{ view.metadata.parent ?? "none" }}</strong></span>
            <span v-if="selectedAsset?.asset.head === revisionId" class="head-label">current head</span>
            <span v-else>historical branch point</span>
          </div>
        </div>

        <div class="image-grid">
          <figure class="image-card native-card">
            <figcaption><strong>Native</strong><span>{{ view.metadata.inspection.width }}×{{ view.metadata.inspection.height }}</span></figcaption>
            <div class="checker native-stage">
              <img :src="nativeUrl" :alt="`${assetId} ${revisionId} at native size`" />
            </div>
          </figure>
          <figure class="image-card">
            <figcaption><strong>Nearest preview</strong><span>no interpolation</span></figcaption>
            <div class="checker preview-stage">
              <img :src="previewUrl" :alt="`${assetId} ${revisionId} enlarged nearest-neighbour preview`" />
            </div>
          </figure>
        </div>

        <section class="comparison panel">
          <div class="panel-title">
            <div><small>Revision comparison</small><h3>Visual and machine diff</h3></div>
            <form @submit.prevent="submitComparison">
              <label for="compare-revision">From</label>
              <select id="compare-revision" v-model="compareId" :disabled="busy || revisions.length < 2">
                <option value="">Choose revision</option>
                <option v-for="revision in revisions.filter(({ id }) => id !== revisionId)" :key="revision.id" :value="revision.id">{{ revision.id }}</option>
              </select>
              <button :disabled="busy || !compareId">Compare</button>
            </form>
          </div>
          <div v-if="comparison" class="diff-result">
            <div class="checker diff-stage"><img :src="diffUrl" alt="Visual pixel difference: red removed, green added, magenta changed" /></div>
            <dl>
              <div><dt>Changed pixels</dt><dd>{{ comparison.metadata.diff.changed_pixels.length }}</dd></div>
              <div><dt>Palette changes</dt><dd>{{ comparison.metadata.diff.palette_differences.length }}</dd></div>
              <div><dt>Changed bounds</dt><dd>{{ formatBounds(comparison.metadata.diff.changed_bounds) }}</dd></div>
            </dl>
            <p class="legend"><span class="removed"></span>removed <span class="added"></span>added <span class="changed"></span>changed</p>
          </div>
          <p v-else class="empty inline">Choose another revision to produce a deterministic comparison.</p>
        </section>

        <div class="action-grid">
          <section class="panel">
            <div class="panel-title"><div><small>Review state</small><h3>Record judgement</h3></div></div>
            <form class="stack-form" @submit.prevent="submitReview">
              <div class="form-row">
                <label>Actor<input v-model="actor" required /></label>
                <label>Kind<select v-model="actorKind"><option value="human">Human</option><option value="agent">Agent</option></select></label>
                <label>Decision<select v-model="reviewDecision"><option value="reviewed">Reviewed</option><option value="changes_requested">Changes requested</option><option value="accepted">Accepted</option></select></label>
              </div>
              <label>Note<textarea v-model="reviewNote" rows="2" placeholder="What reads well or needs revision?"></textarea></label>
              <button class="primary" :disabled="busy || !actor.trim()">Record review</button>
              <p class="form-note">Review appends an event. It never moves head or aesthetically auto-approves.</p>
            </form>
            <ol v-if="view.metadata.review?.events.length" class="review-history">
              <li v-for="event in [...view.metadata.review.events].reverse()" :key="event.sequence">
                <span :class="`decision ${event.decision}`">{{ event.decision.replace("_", " ") }}</span>
                <strong>{{ event.actor }}</strong><small>{{ event.actor_kind }}</small><p>{{ event.note || "No note" }}</p>
              </li>
            </ol>
          </section>

          <section class="panel">
            <div class="panel-title"><div><small>Structured operation</small><h3>Pixel patch</h3></div><button type="button" @click="addPatch">Add coordinate</button></div>
            <form class="stack-form" @submit.prevent="submitPatch">
              <div v-for="(edit, index) in patchEdits" :key="index" class="patch-row">
                <label>X<input v-model.number="edit.x" type="number" min="0" :max="view.metadata.inspection.width - 1" required /></label>
                <label>Y<input v-model.number="edit.y" type="number" min="0" :max="view.metadata.inspection.height - 1" required /></label>
                <label>Index<select v-model.number="edit.index"><option v-for="color in view.metadata.inspection.palette" :key="color.index" :value="color.index">{{ color.index }} · {{ color.count }} px</option></select></label>
                <button type="button" class="icon-button" aria-label="Remove coordinate" @click="removePatch(index)">×</button>
              </div>
              <button class="primary" :disabled="busy || !patchEdits.length || !actor.trim()">Create child revision</button>
              <p class="form-note">All coordinates validate together against explicit parent {{ revisionId }}.</p>
            </form>
          </section>
        </div>

        <section v-if="paletteDraft" class="panel remap-panel">
          <div class="panel-title"><div><small>Structured operation</small><h3>Palette remap</h3></div><button type="button" @click="addPaletteColor">Add color</button></div>
          <form class="stack-form" @submit.prevent="submitRemap">
            <div class="form-row compact">
              <label>Palette name<input v-model="paletteDraft.name" required /></label>
              <label>Transparent index<input v-model.number="paletteDraft.transparentIndex" type="number" min="0" :max="paletteDraft.colors.length - 1" required /></label>
            </div>
            <h4>Replacement colors <span>{{ paletteDraft.colors.length }}</span></h4>
            <div class="palette-editor" role="group" aria-label="Replacement palette colors">
              <div v-for="(rgba, index) in paletteDraft.colors" :key="index" class="palette-edit-row">
                <span class="swatch" :style="{ background: colorCss(rgba) }" aria-hidden="true"></span>
                <strong>{{ index }}</strong>
                <label v-for="(_, channel) in rgba" :key="channel">
                  <span class="sr-only">{{ ["red", "green", "blue", "alpha"][channel] }} for index {{ index }}</span>
                  <input v-model.number="rgba[channel]" type="number" min="0" max="255" required />
                </label>
                <button type="button" class="icon-button" :aria-label="`Remove palette color ${index}`" :disabled="paletteDraft.colors.length === 1" @click="removePaletteColor(index)">×</button>
              </div>
            </div>
            <h4>Old → new index map</h4>
            <div class="index-map" role="group" aria-label="Old-to-new palette index map">
              <label v-for="color in view.metadata.inspection.palette" :key="color.index">
                <span class="swatch" :style="{ background: colorCss(color.rgba) }" aria-hidden="true"></span>
                <span>Old {{ color.index }} →</span>
                <input v-model.number="paletteDraft.indexMap[color.index]" type="number" min="0" :max="paletteDraft.colors.length - 1" required :aria-label="`Map old index ${color.index} to new index`" />
              </label>
            </div>
            <button class="primary" :disabled="busy || !actor.trim()">Create remapped child revision</button>
            <p class="form-note">Transparent mapping is explicit and validated by the Rust engine.</p>
          </form>
        </section>
      </section>

      <section v-else class="empty-workspace" aria-label="Empty project">
        <div><small>No revision selected</small><h2>{{ project.assets.length ? "Choose an asset revision" : "This project has no assets" }}</h2><p>{{ project.assets.length ? "Select an immutable revision from the navigator." : "Create an asset with the CLI, then open the project again." }}</p></div>
      </section>

      <aside v-if="view" class="inspector" aria-label="Revision inspector">
        <section>
          <h2>Inspection</h2>
          <dl class="facts">
            <div><dt>Dimensions</dt><dd>{{ view.metadata.inspection.width }}×{{ view.metadata.inspection.height }}</dd></div>
            <div><dt>Visible</dt><dd>{{ view.metadata.inspection.visible_pixels }} px</dd></div>
            <div><dt>Bounds</dt><dd>{{ formatBounds(view.metadata.inspection.visible_bounds) }}</dd></div>
            <div><dt>Pivot</dt><dd>{{ view.metadata.inspection.pivot?.join(", ") ?? "not set" }}</dd></div>
          </dl>
        </section>
        <section>
          <h2>Validation <span :class="view.metadata.validation.valid ? 'valid' : 'invalid'">{{ view.metadata.validation.valid ? "valid" : "invalid" }}</span></h2>
          <ul class="checks">
            <li v-for="check in view.metadata.validation.checks" :key="check.name"><span :class="check.passed ? 'pass' : 'fail'">{{ check.passed ? "✓" : "!" }}</span><div><strong>{{ check.name.replaceAll("_", " ") }}</strong><small>{{ check.detail }}</small></div></li>
          </ul>
          <p class="review-required">Visual review: <strong>{{ view.metadata.validation.visual_review }}</strong></p>
        </section>
        <section>
          <h2>Palette <span>{{ view.metadata.palette_name }}</span></h2>
          <ul class="palette-list">
            <li v-for="color in view.metadata.inspection.palette" :key="color.index">
              <span class="swatch" :style="{ background: colorCss(color.rgba) }" aria-hidden="true"></span>
              <strong>{{ color.index }}</strong><code>{{ color.rgba.join(" ") }}</code><small>{{ color.count }} px</small>
              <span v-if="color.index === view.metadata.transparent_index" class="transparent">T</span>
            </li>
          </ul>
        </section>
        <details>
          <summary>Indexed text grid</summary>
          <pre>{{ view.metadata.inspection.text_rows.join("\n") }}</pre>
        </details>
      </aside>
    </main>

    <main v-else class="welcome">
      <div class="welcome-mark" aria-hidden="true"><span></span><span></span><span></span><span></span></div>
      <small>PROJECT-AWARE PIXEL ART</small>
      <h1>Inspect the pixels.<br />Preserve the decisions.</h1>
      <p>Open a PixelPipe game project to review native assets, compare immutable revisions, and submit deterministic refinements.</p>
    </main>
  </div>
</template>
