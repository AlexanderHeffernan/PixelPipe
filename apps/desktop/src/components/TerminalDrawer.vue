<script setup lang="ts">
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { FitAddon as XtermFitAddon } from "@xterm/addon-fit";
import type { Terminal as XtermTerminal } from "@xterm/xterm";
import { PhCaretDown, PhTerminalWindow } from "@phosphor-icons/vue";
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from "vue";
import * as api from "../api";
import { useWorkspace } from "../workspace/context";
import HintTip from "./HintTip.vue";

const workspace = useWorkspace();
const open = ref(false);
const host = ref<HTMLElement>();
const height = ref(280);
const resizing = ref(false);
const resizeStart = { y: 0, height: 0 };
const windowHeight = ref(window.innerHeight);
const maximumHeight = computed(() =>
  Math.max(120, Math.min(windowHeight.value - 400, windowHeight.value * 0.55)),
);
const SESSION = "project";
const terminalHelp =
  "Launch your preferred coding-agent CLI here, such as amp, codex, or claude. Tell it to run `pixelate guide --root .` before making anything. Pixelate is available on this terminal's PATH; no MCP server or special skill is required.";
let terminal: XtermTerminal | undefined;
let fit: XtermFitAddon | undefined;
let observer: ResizeObserver | undefined;
let unlisten: UnlistenFn | undefined;
let syncTimer: ReturnType<typeof setInterval> | undefined;

async function openTerminal() {
  open.value = true;
  await nextTick();
  startProjectSync();
  if (terminal) {
    terminal.focus();
    return;
  }
  if (!host.value || !workspace.project.value) return;
  const [{ Terminal }, { FitAddon }] = await Promise.all([
    import("@xterm/xterm"),
    import("@xterm/addon-fit"),
  ]);
  terminal = new Terminal({
    cursorBlink: true,
    fontFamily: "SFMono-Regular, Menlo, monospace",
    fontSize: 12,
    lineHeight: 1.2,
    scrollback: 5000,
    theme: {
      background: "#0c0910",
      foreground: "#ede4e3",
      cursor: "#fb771f",
      selectionBackground: "#5a382c",
    },
  });
  fit = new FitAddon();
  terminal.loadAddon(fit);
  terminal.open(host.value);
  fit.fit();
  unlisten = await listen<{ session: string; data: string }>(
    "terminal-output",
    ({ payload }) => {
      if (payload.session !== SESSION) return;
      const bytes = Uint8Array.from(atob(payload.data), (character) =>
        character.charCodeAt(0),
      );
      terminal?.write(bytes);
    },
  );
  terminal.onData((data) => void api.writeTerminal(SESSION, data));
  await api.startTerminal(
    SESSION,
    workspace.project.value.project_root,
    terminal.cols,
    terminal.rows,
  );
  observer = new ResizeObserver(() => {
    fit?.fit();
    if (terminal)
      void api.resizeTerminal(SESSION, terminal.cols, terminal.rows);
  });
  observer.observe(host.value);
  terminal.focus();
}

function startProjectSync() {
  if (syncTimer) return;
  syncTimer = setInterval(
    () => void workspace.syncExternalChanges().catch(() => undefined),
    650,
  );
}

function disposeTerminal() {
  const hadTerminal = Boolean(terminal);
  observer?.disconnect();
  observer = undefined;
  unlisten?.();
  unlisten = undefined;
  terminal?.dispose();
  terminal = undefined;
  fit = undefined;
  if (syncTimer) clearInterval(syncTimer);
  syncTimer = undefined;
  if (hadTerminal) void api.closeTerminal(SESSION);
}

function hideTerminal() {
  open.value = false;
  if (syncTimer) clearInterval(syncTimer);
  syncTimer = undefined;
  void workspace.syncExternalChanges().catch(() => undefined);
}

function clampHeight(value: number) {
  return Math.round(Math.min(maximumHeight.value, Math.max(120, value)));
}

function windowResized() {
  windowHeight.value = window.innerHeight;
  height.value = clampHeight(height.value);
}

function startResize(event: PointerEvent) {
  resizing.value = true;
  resizeStart.y = event.clientY;
  resizeStart.height = height.value;
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}

function resize(event: PointerEvent) {
  if (!resizing.value) return;
  height.value = clampHeight(
    resizeStart.height + resizeStart.y - event.clientY,
  );
}

function stopResize() {
  resizing.value = false;
}

function resizeWithKeyboard(event: KeyboardEvent) {
  if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
  event.preventDefault();
  height.value = clampHeight(
    height.value + (event.key === "ArrowUp" ? 12 : -12),
  );
}

watch(
  () => workspace.project.value?.project_root,
  async (root, previous) => {
    if (!previous || root === previous || !terminal) return;
    disposeTerminal();
    if (open.value && root) {
      await nextTick();
      await openTerminal();
    }
  },
);

onMounted(() => window.addEventListener("resize", windowResized));
onBeforeUnmount(() => {
  window.removeEventListener("resize", windowResized);
  disposeTerminal();
});
</script>

<template>
  <section
    class="terminal-drawer"
    :class="{ open, resizing }"
    :style="open ? { height: `${height}px` } : undefined"
  >
    <div v-if="!open" class="terminal-closed">
      <button
        class="terminal-open"
        aria-label="Open project terminal"
        @click="openTerminal"
      >
        <PhTerminalWindow weight="regular" />
        Terminal
      </button>
      <HintTip :text="terminalHelp" label="How to use the project terminal" />
    </div>
    <template v-if="terminal || open">
      <div
        v-show="open"
        class="terminal-resize-handle"
        role="separator"
        aria-label="Resize project terminal"
        aria-orientation="horizontal"
        :aria-valuemin="120"
        :aria-valuemax="maximumHeight"
        :aria-valuenow="height"
        tabindex="0"
        @pointerdown="startResize"
        @pointermove="resize"
        @pointerup="stopResize"
        @pointercancel="stopResize"
        @keydown="resizeWithKeyboard"
      ></div>
      <header v-show="open">
        <span>
          <PhTerminalWindow /> Project Terminal
          <HintTip
            :text="terminalHelp"
            label="How to use the project terminal"
          />
        </span>
        <button aria-label="Close terminal" @click="hideTerminal">
          <PhCaretDown />
        </button>
      </header>
      <div v-show="open" ref="host" class="terminal-host"></div>
    </template>
  </section>
</template>
