# Milestone 0 — Research and Decision Record

Status: **approved 2026-08-23**
Date: 2026-08-23

## Scope and gate

M0 defines what PixelPipe is and fixes the boundaries needed to begin a narrow
foundation. It contains no production implementation. Work must stop after this
record and `ARCHITECTURE.md` until the owner approves or changes the decisions.

## Evidence reviewed

### Original PixelPipe proof of concept

The original Python PoC demonstrated the full product insight on Last Light
assets, not just image conversion:

```text
smooth generated reference
→ backdrop cleanup and crop/registration
→ hard majority/dominant cell reduction
→ exact family palette
→ structured pixel-poc/v1 JSON
→ indexed PNG + exact nearest preview
→ native/enlarged/family visual review
→ regenerate, tune a general parameter, or make one narrow edit
```

Observed strengths worth preserving:

- editable structured pixels as source of truth, with PNG as export;
- strict palette, transparency, pivot, bounds, component, and sheet metadata;
- deterministic no-filter/no-dither reduction and exact nearest previews;
- retained smooth references and conversion settings;
- textual inspection useful to both people and agents;
- visual review at native, enlarged, and asset-family scale.

The workflow produced useful pickups, enemies, bosses, effects, and UI assets.
Review caught technically valid but unusable results: a box-like barrel, flat
potion, muddy logs, non-overhead bosses, a squashed crawler, detached reduction
artifacts, false baked shadows, backdrop mistakes, and family inconsistency.
These are evidence that review is a workflow state, not a validation checkbox.

Observed failures to replace rather than reproduce:

- many overlapping conversion and `build_*` scripts with hard-coded paths and
  inconsistent metadata;
- immediate in-place JSON writes with no transaction, undo, history, branch, or
  durable run record;
- long coordinate commands and draw-order coupling;
- asset-specific cleanup hidden in wrappers;
- no semantic ability to rescue a bad camera angle, silhouette, or contact point.

The PoC had provenance fragments, not a provenance system. PixelPipe should use
its assets as golden behavior fixtures only where licensing and repository size
permit; it should not wholesale-copy scripts.

### Shipyard

Shipyard's public Tauri/Vue implementation demonstrates useful desktop craft:
typed Tauri services, composable feature state, semantic child events,
authoritative reconciliation of native events, scoped keyboard commands,
keyboard-resizable sidebars, focus restoration, destructive preflight,
container-aware dense panels, restrained tokens, subtle motion, and explicit
titlebar geometry.

PixelPipe should adopt those interaction principles, not Shipyard's identity or
Git workflow. In particular, it should not copy the dark-only palette,
macOS-private APIs, fixed titlebar insets, localStorage domain persistence,
module-global runner singleton, permissive CSP, or raw DOM/CSS integration
workarounds.

Primary repository reviewed: <https://github.com/AlexanderHeffernan/Shipyard>

### Amp and Jellyware

Amp's official manual supports durable repository guidance, explicit thread
context, isolated workspaces, bounded delegation, and reviewable thread history.
The useful methodology here is not “use more agents”; it is to put ground truth
and feedback loops where an agent can inspect them, keep tasks bounded, and make
verification artifacts cheap.

“Jellyware” is described by Amp as high-quality software designed to change
quickly: accept software's increased malleability and lower bug-fix cost without
lowering the quality bar. The same discussion stresses getting foundations and
invariants right, documenting them for agents, reviewing blast radius and wrong
abstractions, manually exercising end-to-end behavior, and using exhaustive
artifact-based verification.

PixelPipe's interpretation is intentionally conservative: make workflow policy
and frontends easy to change, while keeping schemas, deterministic operations,
provenance, and exports small and rigorous. “Easy to fix” is not permission to
make the pixel engine vague or to accept untraceable assets.

Primary sources:

- <https://ampcode.com/manual>
- <https://ampcode.com/podcast/season-02/episode-02>

## Decisions proposed for approval

1. **Nondeterministic boundary:** generated references become deterministic inputs
   only when selected, imported, and hashed.
2. **Architecture:** Rust workspace; pure deterministic core; project store;
   shared application use cases; thin CLI/Tauri adapters; AI subprocess adapters
   above the application boundary.
3. **Desktop:** Tauri 2 + Vue 3 + TypeScript, borrowing Shipyard interaction
   lessons without copying its product or macOS-only implementation.
4. **Project format:** project-root `.pixelpipe/`; TOML human manifests; stable
   JSON recipes/results; immutable revision directories; PNG review artifacts;
   ignored cache/temp; project-relative contained paths.
5. **History:** revision DAG through parent links and head pointers, with no merge
   operation. Approval is a pointer to an immutable revision.
6. **Parity:** all mutations are shared versioned application use cases. CLI is
   complete and scriptable; UI provides visual ergonomics; agents use CLI or the
   same use cases and receive no extra authority.
7. **AI integration:** a user-configured one-shot JSON stdin/stdout subprocess
   protocol, initially supporting reference generation, critique, and refinement
   proposals. No provider SDK or MCP.
8. **Visual quality:** machine validation and explicit human/AI visual review are
   separate. Approval is never inferred from AI critique.
9. **Canonical model:** indexed raster + palette + metadata, without layers in the
   first product arc. Layering is reconsidered only after focused editor evidence.
10. **Compatibility:** schemas and recipes are versioned from their first commit;
    deterministic outputs are guarded by approved PoC-derived golden fixtures.

## Tradeoffs and weak ideas rejected

| Choice | Benefit | Cost / reason rejected or constrained |
| --- | --- | --- |
| Keep Python PoC architecture | Fastest apparent start | Preserves script sprawl and inconsistent contracts; use fixtures, not topology |
| Put AI in the engine | Fewer visible layers | Destroys determinism and provider independence |
| Build every provider directly | Convenient demos | Auth/retry/API churn dominates the product; use user adapters first |
| Let `.pixelpipe` configure executable commands | Portable projects | Opening a repository could execute untrusted code |
| Browser-only UI | Easy distribution | Poor fit for project filesystem, subprocess, indexed encoding, and export access |
| Electron | Familiar web stack | Larger runtime and duplicate backend role without an initial requirement |
| Svelte solely because it is fashionable/small | Pleasant frontend | Existing maintainer evidence favors Vue; either works, so minimize novelty |
| Layers from day one | Familiar editor concept | Complicates canonical state, operations, compositing, and history before need is proven |
| Validators approve art | Automatable | PoC directly disproves this; validity did not imply readability |
| Content-address everything visibly | Deduplication | Obscures review and adds machinery; use hashes for integrity, readable revision paths for ownership |
| Generic node-graph pipeline | Maximum flexibility | Recreates orchestration software instead of delivering the proven workflow |

## Gate decision

M0 was approved with three clarifications:

1. **Frontend:** Tauri 2 + Vue 3/TypeScript is accepted. It remains behind the
   application-use-case boundary, and Shipyard's macOS-private APIs, persistence,
   CSP, titlebar geometry, and DOM workarounds are not inherited.
2. **PoC fixtures:** use a small representative corpus only. Copy smooth Last
   Light references only when redistribution rights and provenance are clear;
   otherwise use synthetic/new references and record the missing provenance.
3. **Git policy:** track manifests, recipes, canonical indexed JSON, native runtime
   PNGs, and small nearest previews by default. References require clear rights
   and project opt-in. Reproducible review/run bundles are ignored by default and
   may be explicitly recorded. Git LFS is opt-in, not a baseline dependency.

## Proposed next milestone

**M1 — Deterministic project foundation**

After explicit M0 approval, create only:

- the minimal Rust workspace (`core`, `project`, `app`, `cli`);
- versioned palette, indexed-raster, project, asset, recipe, revision, and
  provenance schemas;
- `.pixelpipe` discovery/init and atomic immutable revision writes;
- one tiny deterministic operation path from structured raster to indexed PNG
  and nearest preview;
- CLI JSON output and conformance/golden tests proving repeated byte identity;
- architecture guidance for future agent work.

M1 will not add reference generation, the full PoC converter, Tauri UI, editor,
or provider adapters. It ends at another review gate.
