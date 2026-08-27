# Pixelate architecture

Pixelate is a project-aware pixel-art workstation. This document records the
current boundaries that keep the code deterministic, small, and easy to change.
Historical milestone and migration documents are intentionally not retained.

## Product contract

Pixelate owns one visible workflow:

```text
brief → reference → pixelize → review → refine → export
```

- A selected reference freezes the nondeterministic input boundary.
- Indexed pixels and their ordered palette are the editable source of truth.
- Every pixel mutation creates an immutable parent-linked revision.
- Machine validation and visual judgement are separate. Nothing auto-approves.
- Desktop and CLI call the same typed application use cases.
- Pixelate never launches or manages agents. Agents use the CLI from the
  embedded terminal and receive no capability unavailable to a human.

## Dependency direction

```text
Vue desktop ──▶ Tauri adapter ──┐
                                ├──▶ pixelate-app ──┬──▶ pixelate-project
coding agent ──▶ pixelate CLI ──┘                   └──▶ pixelate-core
                                                              ▲
pixelate-project ──────────────────────────────────────────────┘
```

Dependencies point inward:

- `pixelate-core` owns versioned pixel types and pure deterministic operations:
  decode, backdrop removal, crop, reduction, palette mapping, composition,
  editing, validation, inspection, and exact PNG rendering. It knows nothing
  about filesystems, Tauri, agents, providers, or UI state.
- `pixelate-project` owns `.pixelate` schemas and storage: project discovery,
  atomic writes, resources, selected references, immutable revisions, reviews,
  locking, identity, hashes, and contained paths.
- `pixelate-app` owns workflow use cases: onboarding, import, preview,
  pixelization, revision creation, editing, inspection, review, and export. It is
  the only domain surface frontends invoke.
- `pixelate-cli` is the complete non-interactive JSON adapter. It also owns
  `pixelate guide`, the concise machine-readable workflow for coding agents.
- `apps/desktop/src-tauri` is a thin native adapter. `apps/desktop/src` owns only
  transient Vue interaction state and presentation.

All four crates and the desktop package are active. `apps/desktop` exists because
an application contains both a TypeScript frontend and a Rust Tauri shell; it is
not a reusable library and does not belong under `crates`.

## Project format

Pixelate discovers a game root by walking upward for `.pixelate/project.toml`.
Initialization occurs only at an explicitly selected root.

```text
game-root/
  .pixelate/
    project.toml
    palettes/<id>.json
    recipes/<id>.json
    assets/<asset-id>/
      asset.toml
      references/selected/<sha256>.<format>
      revisions/r000001/
        brief.md
        native.png
        pixels.json
        preview.png
        provenance.json
        recipe.json
        validation.json
      reviews/<revision>.json
```

Schemas use the `pixelate.<document>/vN` namespace. This unreleased codebase uses
only the current Pixelate schemas and `.pixelate` path; there is no compatibility
layer for the former project name.

Selected references are content-addressed and verified. Conversion snapshots the
brief, palette, settings, reference hash, and resource hashes into a revision so
later resource edits cannot change historical bytes. Paths in project documents
are project-relative and may not escape the game root.

## Determinism and revisions

- Conversion uses explicit integer rules and stable tie-breaking.
- Indexed output never introduces interpolation, antialiasing, or implicit
  dithering.
- Native and nearest-neighbour preview PNG hashes are golden-tested.
- Patch, fill, palette, composition, and conversion mutations publish complete
  revisions atomically; failed operations publish nothing.
- Review events do not alter revision bytes or imply approval.
- Preview is read-only and uses integer nearest-neighbour scaling. Enlarged
  previews are the default agent visual-review artifact because vision tools can
  assess them more reliably without changing pixel structure.

Synthetic test data lives beside the crate that owns its behavior under
`tests/fixtures`. There is no repository-wide fixture or milestone-document
hierarchy.

## CLI, UI, and agent parity

Every user-visible capability must have equivalent CLI semantics over the same
application use case. Equivalent does not mean identical presentation: the UI
may show pixels while the CLI returns paths, dimensions, and hashes.

| Capability | CLI family | Desktop |
| --- | --- | --- |
| Project and resources | `init`, `project` | folder onboarding and settings |
| Asset brief/lifecycle | `asset` | asset browser and forms |
| Reference import | `reference` | native file picker |
| Pixelization/composition | `revision pixelize`, `compose` | conversion workspace |
| Inspect/preview/compare | `revision inspect`, `preview`, `compare` | canvas and inspector |
| Pixel and palette edits | `revision draw`, `fill`, `recolor`, `remap` | editor tools |
| Review and history | `revision review`, `set-head` | review/history controls |
| Export | `asset export` | explicit export action |

When adding or changing a capability:

1. Put behavior in `pixelate-app` (or the core/store boundary it coordinates).
2. Expose it through both relevant adapters without duplicating domain logic.
3. Update `pixelate guide` when an agent should discover or sequence it.
4. Verify deterministic hashes and immutable revision guarantees where affected.

## Non-goals

- Launching agents, provider SDKs, MCP servers, or project-selected commands.
- A generic node graph, plugin marketplace, or configurable image pipeline.
- Automatic reference selection, mutation, review acceptance, or approval.
- Photoshop-class layers, vectors, blend modes, or semantic image repair.
- Cloud accounts, collaboration services, or runtime game-engine integration.
