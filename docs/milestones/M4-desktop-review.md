# Milestone 4 — Desktop Visual-Review Workstation

Status: **ready for review**
Started: 2026-08-23

## Approved scope

- Tauri 2 + Vue 3/TypeScript desktop shell.
- Project discovery/open and asset/revision navigation.
- Native and nearest-neighbour revision views.
- Palette, visible-bounds, pivot, and validation inspection.
- Machine and visual revision comparison.
- Explicit human/agent review actions.
- Structured pixel-patch and palette-remap submission.

## Guardrails

- Vue/Tauri invoke typed `pixelpipe-app` use cases; they own no domain logic.
- No localStorage or frontend state is authoritative.
- Browsing and review never move asset head.
- Pixel mutations are explicit and always revision-producing.
- Keyboard, focus, accessible naming, loading, empty, and failure states are part
  of the central review surface.
- AI, layers, brushes, semantic editing, retain-largest, and export remain out.

## Delivered contract

- `pixelpipe-project` lists asset and revision manifests in stable ID order and
  loads native/preview bytes only through the existing full revision verifier.
- `pixelpipe-app` owns typed browse and visual-revision use cases. File-based CLI
  patch/remap adapters delegate to the same typed document functions used by
  desktop, including parent-brief inheritance and structural validation.
- The fifth, outer `pixelpipe-desktop` crate is a thin Tauri adapter over
  `pixelpipe-app`. Six typed commands cover browse, revision load, compare,
  review, patch, and remap. PNGs cross IPC as base64 verified bytes, not paths.
- Vue state is transient. Opening refreshes authoritative manifests; selection
  and comparison do not write project state; review only appends its durable
  record; patch/remap visibly name their parent and create a child revision.
- The workstation exposes native and nearest views, palette usage and explicit
  transparency, bounds/pivot/validation, visual and numeric differences, review
  history/forms, coordinate patches, and complete RGBA/index palette remaps.
- Loading, empty, success, and focused failure states are explicit. Asset lists
  support Arrow Up/Down, Home, and End; controls are labelled, focus rings are
  visible, and reduced-motion preferences are respected.

## Deliberate decisions

- M4 uses request/response commands only. No approved operation is long-running,
  so adding an event stream now would create duplicate synchronization state.
  Typed events should arrive with asynchronous generation in a later milestone.
- The webview does not receive canonical-raster file paths or duplicate raster
  validation. It builds serialized operation documents; Rust remains responsible
  for schema, palette, transparent-index, component, atomicity, and history rules.
- A manual project-path field keeps project discovery explicit and cross-platform
  without adding a native dialog permission surface in this slice.
- Shipyard influenced compact navigation, strong hierarchy, keyboard handling,
  and desktop polish only. PixelPipe uses standard Tauri windows, a restrictive
  CSP, system light/dark themes, and no private macOS APIs, titlebar assumptions,
  or DOM workarounds.
- The desktop icon is a newly generated geometric project mark.
  No Last Light, Painter, Shipyard, or uncleared reference art is included.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/desktop && npm run build && npm test
git diff --check
```

All 24 Rust tests pass and verify stable browsing, verified image loading, and
that review does not move head in addition to all M1–M3 conformance checks. Two
Vue tests exercise
open/load rendering, data-URL transport, keyboard list navigation, non-mutating
review, and focused command-failure feedback. The production frontend build is
type-checked and bundled under the restrictive Tauri CSP. A browser-driven pass
at 1440×900 exercised project open, revision load, compare, independently
scrolling work areas, and review recording against real synthetic M1/M3 outputs;
the final dark- and light-theme WCAG 2 A/AA axe scans reported zero violations.

## Subsequent direction

The configured-agent milestone proposed here was implemented and later removed.
Pixilate now leaves agent execution to the user's terminal and exposes the same
application capabilities through its CLI.

## Review gate

This historical milestone stopped here for review before subsequent work.
