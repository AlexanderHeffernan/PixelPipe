# Milestone 2 — Proven Deterministic Reference Conversion

Status: **ready for review**
Started: 2026-08-23

## Scope

- Deterministic RGBA PNG decoding.
- Explicit alpha or border-connected backdrop cleanup.
- Visible-bounds crop, target padding, center/bottom registration.
- Hard dominant-cell reduction with stable palette and count tie rules.
- Exact ordered-palette mapping and indexed output.
- Asset-aware connected-component expectations.
- Regular sheets with shared scale, baseline, palette, and frame metadata.
- Synthetic/new fixtures with explicit provenance and golden hashes only.

## Excluded

Last Light/Painter material, AI generation, Tauri/Vue, editing, approval,
provider adapters, semantic repair, filtered resizing, dithering, and layers.

## Delivered contract

- `pixelpipe-core` decodes PNG color forms to RGBA8, cleans alpha or an explicit
  border-connected color, crops to visible bounds, pre-maps to a fixed palette,
  and performs hard dominant-cell reduction using integer arithmetic only.
- Equal palette-distance and modal-count ties choose the lower palette index.
  Foreground coverage equal to the configured threshold is retained.
- Center and bottom registration produce stable pivots and bounds metadata.
- Connected-component ranges are asset settings and are validated after
  reduction. Conversion fails before revision creation when they do not match.
- Regular sheets are split into exact cells before cleanup. Frames use a shared
  rational scale, baseline/registration, palette, per-frame component checks,
  and stable sheet/frame metadata.
- `pixelpipe-app::convert_revision` is the sole application use case. Through
  `pixelpipe-project`, it imports the original PNG bytes into an immutable,
  content-addressed reference path, records conversion plus render operations,
  and uses M1's locked atomic revision transaction.
- `pixelpipe revision convert` is a thin JSON CLI adapter for the same use case;
  `--conversion sheet` selects the sheet settings contract.

The project store freezes selected reference bytes at
`assets/<asset>/references/selected/<sha256>.png`. References are ignored by Git by
default; projects may opt into tracking only references with clear rights.

## Deliberate decisions

- Palette mapping occurs before modal reduction, matching the successful PoC
  path and preventing averaged colors.
- Backdrop removal requires an explicit RGB target and tolerance. It does not
  guess whether black or white is background.
- Alpha above the threshold establishes foreground coverage; palette distance
  uses straight RGB and emits the palette's exact RGBA value.
- M2 validates components rather than implicitly retaining only the largest.
  Silent deletion is unsafe for effects and multipart sprites; a future explicit
  component-selection operation needs a demonstrated fixture and review.
- Source references retain their original bytes in the content-addressed
  reference store. Canonical pixels, native PNGs, recipes, validation, and small
  previews remain revision payloads.

## Synthetic fixture evidence

`fixtures/m2/` is a small CC0-1.0 geometric corpus with separate provenance. It
contains no Last Light, Painter, Shipyard, or third-party image material. Tests
encode fixture RGBA arrays to PNG in memory.

Golden reference hashes:

- canonical raster JSON: `19af2370aeee6415ceec8e2a16aad78d746672a7e202252b72c5aeb37b597b67`
- native indexed PNG: `9e1345c3b488327bb6839c177830c0f50f5121b21450de9eda42e1d923e4721e`
- 8× nearest preview: `22d22ac34a6764531e972636f4c0d10e17c01e9752ad01b3b2e9c4a44305f201`

Golden sheet hashes:

- native indexed PNG: `fce359e660986518efae60130e4227af5b4dc8c0d24070bc1ee91ec8455f1132`
- 8× nearest preview: `871d96fabd73d3c265f1c0ff05067c1a37d545434678fe0da7241c52d93cbe9a`

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

The process-level CLI test covers init → PNG conversion → atomic revision,
asserts both output hashes, and inspects the serialized recipe and validation.

## Proposed next milestone

**M3 — Inspect, refine, and review immutable revisions.** Add headless native/text
inspection, deterministic pixel-patch and palette-remap operations, explicit
parent selection for branching/undo, revision comparison, and durable visual
review status through the application/CLI contract. Do not add the desktop UI,
AI adapters, layers, brushes, or export yet. This directly addresses the PoC's
unsafe overwrite/edit loop while establishing the use cases the later UI must
call.

## Review gate

M2 stops here until the parent approves the conversion contract and M3 scope.
