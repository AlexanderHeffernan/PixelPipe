import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

const MIN_HEIGHT = 124;
const MAX_HEIGHT = 300;
const DEFAULT_HEIGHT = 150;
const HEIGHT_KEY = "pixelate.timeline-height";

function savedHeight() {
  try {
    const stored = Number(window.localStorage?.getItem(HEIGHT_KEY));
    return Number.isFinite(stored) && stored >= MIN_HEIGHT
      ? Math.min(MAX_HEIGHT, stored)
      : DEFAULT_HEIGHT;
  } catch {
    return DEFAULT_HEIGHT;
  }
}

function saveHeight(value: number) {
  try {
    window.localStorage?.setItem(HEIGHT_KEY, String(value));
  } catch {
    // Preference storage can be unavailable in hardened webviews.
  }
}

export function useTimelineDrawer() {
  const resizing = ref(false);
  const height = ref(savedHeight());
  const windowHeight = ref(window.innerHeight);
  const resizeStart = { y: 0, height: 0 };
  const maximumHeight = computed(() =>
    Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, windowHeight.value - 300)),
  );
  const minimal = computed(() => height.value <= 145);
  const expanded = computed(() => height.value >= 235);

  function clampHeight(value: number) {
    return Math.round(
      Math.min(maximumHeight.value, Math.max(MIN_HEIGHT, value)),
    );
  }

  function startResize(event: PointerEvent) {
    if (event.button !== 0) return;
    event.preventDefault();
    resizing.value = true;
    resizeStart.y = event.clientY;
    resizeStart.height = height.value;
    window.addEventListener("pointermove", resize);
    window.addEventListener("pointerup", stopResize);
    window.addEventListener("pointercancel", stopResize);
  }

  function resize(event: PointerEvent) {
    if (!resizing.value) return;
    height.value = clampHeight(
      resizeStart.height + resizeStart.y - event.clientY,
    );
  }

  function stopResize() {
    resizing.value = false;
    window.removeEventListener("pointermove", resize);
    window.removeEventListener("pointerup", stopResize);
    window.removeEventListener("pointercancel", stopResize);
  }

  function resizeWithKeyboard(event: KeyboardEvent) {
    if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
    event.preventDefault();
    height.value = clampHeight(
      height.value + (event.key === "ArrowUp" ? 12 : -12),
    );
  }

  function windowResized() {
    windowHeight.value = window.innerHeight;
    height.value = clampHeight(height.value);
  }

  watch(height, saveHeight);
  onMounted(() => window.addEventListener("resize", windowResized));
  onBeforeUnmount(() => {
    window.removeEventListener("resize", windowResized);
    stopResize();
  });

  return {
    resizing,
    height,
    maximumHeight,
    minimal,
    expanded,
    minimumHeight: MIN_HEIGHT,
    startResize,
    resizeWithKeyboard,
  };
}
