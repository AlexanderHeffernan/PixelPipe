# Milestone 3 — Inspect, Refine, and Review Immutable Revisions

Status: **ready for review**
Started: 2026-08-23

## Approved scope

- Deterministic native/text revision inspection.
- Atomic pixel-patch and explicit palette-index remap operations.
- Explicit parent revision selection for branching and undo-by-new-revision.
- Machine-readable revision comparison and deterministic visual diffs.
- Durable human/agent review records that never auto-approve aesthetics.
- Application/CLI parity over the existing project and core boundaries.

## Guardrails

- Every successful mutation creates a new immutable parent-linked revision.
- Failed patch/remap validation publishes nothing and does not advance asset head.
- Existing revision payloads remain byte-stable.
- Transparent indices and palette mappings are explicit serialized inputs.
- Retain-largest-component remains deferred and is not an implicit heuristic.
- M3 remains headless: no UI, AI adapter, layers, brushes, or export expansion.

## Delivered contract

- `pixelpipe.patch/v1` stores a complete row/column/index edit list. The core
  rejects every edit atomically on duplicate coordinates, bounds, palette, or
  inherited structure failure.
- `pixelpipe.palette-remap/v1` stores the complete replacement palette and an
  old-index → new-index table. The old transparent index must explicitly map to
  the new transparent index.
- Structure rules are inherited and serialized. Single rasters use one
  four-connected component range; sheets preserve the M2 grid and validate the
  same range independently per frame.
- Patch/remap use cases require an explicit verified parent. They hash canonical
  parent pixels, preserve the brief unless replaced, and publish through the M1
  locked transaction. An empty patch supports undo-by-new-revision.
- Revision loading verifies the exact payload-file set, every manifest hash,
  schemas, asset/revision identity, provenance identity, and parent syntax.
- Inspection returns dimensions, pivot, visible bounds/count, complete palette
  counts, and stable two-digit hex index rows (`--` for the transparent index).
- Comparison returns row-major pixel/color differences, changed bounds, ordered
  palette differences, and deterministic indexed visual PNGs: red removed,
  green added, magenta changed.
- Review records are atomic event histories outside immutable revision payloads.
  Human and agent decisions are explicit; validation never writes review state,
  and review acceptance does not modify the separate approval pointer.
- CLI and application expose the same patch, remap, inspect, compare, and review
  use cases. Optional CLI visual-diff paths are frontend output destinations.

## Fixture and golden evidence

`fixtures/m3/` contains only synthetic CC0-1.0 patch/palette data with separate
provenance. The end-to-end fixture branches both patch and remap revisions from
`r000001`, proving explicit-parent history while the original payloads remain
byte-identical. It also proves failed edits publish no revision or head change.

- visual diff native PNG: `3de8b85fe254add0ac7525bc72732d5692dfaba83f81be3a94b1284ece90bb13`
- 8× visual diff preview: `f77d9c7a01bd0f341fd56913a598412915cf1022e33aa0613e8f72f13a106302`

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

The workspace has 24 passing tests. The process-level M3 test exercises create →
patch branch → palette-remap branch → inspect → compare/visual outputs → agent
review → failed atomic patch, checking parent links, hashes, head, review state,
and byte-stable original payloads.

## Proposed next milestone

**M4 — Desktop visual-review workstation.** Add the approved Tauri 2 + Vue 3/
TypeScript shell behind `pixelpipe-app`: project open/discovery, asset/revision
navigation, native and nearest views, palette/bounds inspection, revision compare
with visual diff, explicit review actions, and structured patch/remap submission.
Keep the Rust application use cases authoritative. Do not add AI generation,
layers/brush painting, semantic editing, or export expansion in M4.

## Review gate

M3 stops here until the parent approves the contracts and M4 scope.
