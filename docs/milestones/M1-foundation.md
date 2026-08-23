# Milestone 1 — Deterministic Project Foundation

Status: **approved 2026-08-23**
Started: 2026-08-23
Completed: 2026-08-23

## Scope

- Minimal Rust workspace with core, project, application, and CLI boundaries.
- Versioned palette, indexed-raster, recipe, project, asset, revision,
  provenance, validation, and export-mapping schemas.
- Project-root discovery and initialization.
- Atomic writes and complete-before-visible immutable revision directories.
- Structured indexed raster to indexed native PNG and exact nearest preview.
- Stable JSON CLI responses and golden/conformance tests.

## Deliberately excluded

Reference generation, smooth-reference conversion, PoC script ports, AI/model
adapters, Tauri/Vue code, manual editing, approval/export workflows, and layers.

## Review gate

M1 stops after the headless foundation and verification evidence are reviewed.

## Delivered contract

- `pixelpipe-core` validates versioned indexed rasters and ordered RGBA palettes,
  uses stable collection ordering, and encodes 8-bit indexed PNGs with explicit
  palette, transparency, compression, and no-filter settings.
- Exact integer nearest-neighbour previews reuse the canonical palette and are
  bounded to safe dimensions.
- `pixelpipe-project` initializes/discovers `.pixelpipe`, ignores references and
  reproducible run/review artifacts by default, serializes concurrent writers
  with an OS file lock, stages complete revision directories under ignored temp,
  then atomically renames them without overwriting history.
- Revisions contain a brief snapshot, canonical raster, render recipe,
  validation, provenance, payload hashes, native PNG, and preview PNG. Asset head
  advances through an atomic manifest replacement; repeated source content still
  creates a new revision with a parent link.
- `pixelpipe-app` owns the only create-revision use case. `pixelpipe-cli` is a
  thin JSON adapter for init, project show, revision create, and asset inspect.
- The synthetic M1 fixture copies no Last Light asset. Its checked golden SHA-256
  values lock native and preview encoder output without creating redistribution
  uncertainty.
- `AGENTS.md` records the architecture and verification invariants for future
  work; `.agents/setup` installs the pinned Rust toolchain components in orbs.

## Verification evidence

```text
cargo fmt --all -- --check
  passed

cargo clippy --workspace --all-targets -- -D warnings
  passed

cargo test --workspace
  8 passed; 0 failed
```

The process-level CLI test exercises init → revision create → inspect and checks
the same native golden hash. Manual end-to-end inspection identified the outputs
as 4×4 and 16×16, 8-bit colormap PNGs. Cross-platform byte identity is encoded as
a locked test expectation but has only run in the Linux orb during M1; a platform
matrix belongs to later hardening.

## Gate decision

M1 was approved without architecture changes. Source code and schemas use the
standard Rust dual license, `MIT OR Apache-2.0`; this does not grant rights to
generated/reference art. All Last Light material is treated as not cleared for
public redistribution. M2 uses only synthetic/new fixtures or material with
explicit redistributable provenance.

## Proposed next milestone

**M2 — Proven deterministic reference conversion**

Port behavior, not scripts, from the PoC into `pixelpipe-core`: RGBA PNG import,
explicit alpha/border-connected backdrop cleanup, visible-bounds crop/pad,
center/bottom registration, hard dominant cell reduction with stable tie rules,
exact palette mapping, and asset-aware structural validation. Add only the narrow
approved fixture families whose redistribution provenance is clear, using
synthetic/new substitutes otherwise, and lock canonical JSON/native output hashes.

M2 remains headless and does not add AI generation, Tauri/Vue, editing, approval,
or provider adapters. It ends at another review gate.
