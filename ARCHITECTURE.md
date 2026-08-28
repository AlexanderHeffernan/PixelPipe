# Pixelate architecture

Pixelate owns one opinionated workflow:

```text
brief → reference → pixelize → inspect → refine → export
```

Every asset is one ordered sequence with at least one indexed frame. The shared
canvas, ordered palette, transparent index, and pivot plus stable frame IDs,
durations, and frame pixels are the editable source of truth. A static sprite is
simply a one-frame sequence; there are no separate sprite and animation kinds.
Every mutation creates an immutable parent-linked revision of the entire
sequence. Visual judgement remains an explicit human or agent action; Pixelate
has no durable review or approval state and never launches an agent.

## Boundaries

```text
Vue desktop ──▶ Tauri adapter ──┐
                                ├──▶ pixelate-app ──┬──▶ pixelate-project
coding agent ──▶ pixelate CLI ──┘                   └──▶ pixelate-core
                                                              ▲
pixelate-project ──────────────────────────────────────────────┘
```

- `pixelate-core` owns deterministic image conversion, indexed editing,
  validation, inspection, and rendering. It has no filesystem or UI knowledge.
- `pixelate-project` owns discovery, atomic persistence, selected references,
  immutable revisions, hashes, and contained paths.
- `pixelate-app` owns the typed use cases shared by both adapters.
- `pixelate-cli` is the non-interactive JSON adapter and owns `pixelate guide`.
- `apps/desktop` groups the Vue interface with its thin Tauri adapter. It is an
  application rather than a reusable crate, so this folder is intentional.

All four crates and the desktop application are active.

## Project format

```text
game-root/
  .pixelate/
    project.toml
    assets/<asset-id>/
      asset.toml
      references/selected/<sha256>.png
      revisions/r000001/
        brief.md
        native.png
        pixels.json
        provenance.json
        recipe.json
        validation.json
```

Assets have one universal one-or-more-frame form and represent one animation
clip each. Named clips, state machines, direction models, project palettes,
configurable conversion recipes, asset kinds, durable reviews, approvals, and
stored preview images are intentionally absent. A revision's `recipe.json` is retained
only as immutable provenance describing the operations that produced it.

Selected references and revision payloads are content-verified. Enlarged preview
PNGs are rendered on demand with exact nearest-neighbour scaling, never persisted
inside revisions. Existing `pixelate.raster/v1` revision payloads are hash-verified
unchanged and normalized in memory as one `pixelate.sequence/v1` frame, preserving
their pixels, palette order, native PNG, hashes, and ancestry. New revisions store
the complete sequence payload; unchanged frame content is not speculatively split.
Optional human-readable frame names travel with stable IDs and are stored by the
same whole-sequence immutable revision operations.

Multi-reference conversion derives one shared palette across the explicitly
ordered batch before converting any frame. Multi-frame canonical export is an
indexed horizontal PNG sheet (fixed shared cells in frame order) plus
`pixelate.spritesheet/v1` JSON containing source identity, rectangles, timing,
canvas, and pivot. One-frame PNG, lossless WebP, and indexed JSON export remain
compatible with the original contracts.
Deterministic motion inspection counts exact adjacent-frame and loop-closing
changes, distinguishing silhouette movement from opaque palette-index churn.
It diagnoses inconsistent inputs without mutating or temporally smoothing the
indexed source of truth. Review warnings use fixed ratio thresholds and identify
the destination frame for an immutable `frame replace` correction.

## CLI parity

Every desktop capability must use the same application use case as a CLI route.

| Capability | CLI |
| --- | --- |
| Project discovery | `init`, `project show` |
| Asset lifecycle and brief | `asset list`, `init`, `inspect`, `set-brief`, `rename`, `delete` |
| Source import/replacement | `reference import`, `asset update-source` |
| Pixelization and canvas placement | `revision pixelize`, `compose` |
| Inspection and vision-friendly preview | `revision inspect`, `preview` |
| Pixel and palette editing | `revision draw`, `fill`, `recolor`, `remap` |
| Frame editing and ordered import | `frame add`, `duplicate`, `import`, `replace`, `import-sequence`, `import-sheet`, `delete`, `reorder`, `duration`, `rename` |
| History navigation | `revision set-head` |
| Bundle or named image export | `asset export`, `export-file` |
| Installed version | `version` |
| Explicit software update | `update` |

`pixelate guide --root .` is the machine-readable source of truth for agents.
Update it whenever an agent-facing command or workflow changes.

Software distribution remains adapter-specific: the desktop uses Tauri's signed
bundle updater, while a standalone CLI verifies and replaces a signed binary.
The CLI never checks for updates during ordinary commands. The CLI bundled in
the desktop app is replaced as part of the app bundle rather than modifying a
signed `.app` in place.

## Non-goals

- Agent launchers, provider SDKs, or repository-selected commands.
- A generic image pipeline, resource/profile system, or plugin marketplace.
- Named clips, animation graphs, directions, skeletal animation, tweening, or video import.
- Automatic source selection, conversion commit, visual acceptance, or export.
