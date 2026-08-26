# M6 — Pre-revision assets and conversion handoff

Status: ready for milestone review.

## Outcome

M6 closes the product's first complete headless/UI-parity workflow:

`brief → import reference → deterministic conversion → r000001`

An asset is no longer synonymous with a revision head. It has a stable ID, kind,
project-owned brief, explicit lifecycle, and optional selected-reference record.
The desktop can create and browse these assets, edit their brief, import a
reference, choose a project recipe, and create the first immutable revision.
Both frontends expose the same typed asset, file-oriented palette/recipe import,
and conversion use cases.

## Lifecycle contract

| State | Invariant | Allowed next work |
|---|---|---|
| `draft` | no head, empty brief, no selection | edit brief |
| `awaiting_reference` | no head, non-empty brief, no selection | import reference |
| `selected_reference` | no head, non-empty brief, verified selection | convert |
| `revisioned` | head exists | existing inspect/review/refinement flow |

State is serialized and checked against brief/selection/head whenever the asset
is loaded. Asset creation stages a complete directory and renames it into place;
brief and selection transitions atomically replace the authoritative manifest.
Legacy v1 revision-backed manifests remain readable and upgrade on mutation.

Generation and selection reject a pre-revision asset until its project brief is
non-empty. Conversion rejects missing selection, missing/invalid resources, kind
mismatch, bad PNG/hash, and deterministic conversion failures before any revision
or head is published. Revision-only operations continue to require a real
revision and the desktop does not render their controls before one exists.
Legacy direct raster creation and arbitrary-reference conversion remain available
for implicit/revision-backed imports, but cannot bypass an explicit pre-revision
asset; that asset must use selected-reference conversion for its first head.

## Project resources and snapshots

- `.pixelpipe/palettes/<id>.json` is a validated `pixelpipe.palette/v1` resource.
- `.pixelpipe/recipes/<id>.json` is a complete
  `pixelpipe.conversion-recipe/v1` resource naming asset kind, palette ID,
  preview scale, and reference/sheet settings.
- The versioned brief and selected-reference hash are stored in
  `pixelpipe.asset/v2`.
- Conversion snapshots brief text, the palette embedded in canonical pixels,
  exact operation settings in `recipe.json`, and brief/palette/recipe/selection/
  reference hashes in provenance.

These project resources are intentionally mutable and Git-friendly. They are
resolved inputs, not live links from old revisions. A conformance test changes
all three after conversion and proves the verified revision snapshot is equal.

## Deliberate exclusions

- No approval/export expansion, provider SDK, MCP, autonomy, layers, brushes,
  retain-largest operation, semantic editing, or additional image formats.
- No automatic generation, selection, conversion, review, or approval.
- No project-controlled executable selection and no claim of process sandboxing.
- No Last Light, Painter, PoC, screenshot, or unclear-rights asset redistribution.

The M6 fixture documents are synthetic CC0 resources. Reference PNG bytes used by
the integration test are rendered in memory from the existing synthetic M1
raster; provenance is recorded in `fixtures/m6/PROVENANCE.md`.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/desktop && npm run build && npm test && npm audit --audit-level=high
git diff --check
```

Coverage includes lifecycle transitions, no-head rejection, failed conversion
atomicity, first revision creation, resource hash snapshots, revision byte
stability after resource edits, all-or-nothing candidate publication, partial
output cancellation, and keyboard-accessible pre-revision UI states.

All 31 Rust tests and four Vue tests pass; the production frontend builds and npm
reports zero known vulnerabilities. Browser verification exercised the selected-
reference state and confirmed its conversion action is enabled while critique,
proposal, review, patch, and remap remain unavailable without a revision. Axe
4.12.1 reported zero WCAG 2 A/AA violations in both light and dark modes, and the
scrollable agent panel was visually inspected without clipping or overlap.

## Proposed next milestone

**M7 — Explicit approval and deterministic export.** Add human/agent-attributed
approval as a separate durable action over a reviewed revision, project-owned
export mappings, atomic runtime PNG/JSON export, stale-approval detection, and
CLI/desktop parity. Keep generation/provider scope and editing scope unchanged.

## Review gate

M6 stops here. Do not begin M7 until the parent approves this state/resource
contract and the proposed approval/export scope.
