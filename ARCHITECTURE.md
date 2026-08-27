# Pixelate architecture

Pixelate owns one opinionated workflow:

```text
brief → reference → pixelize → inspect → refine → export
```

Indexed pixels and their ordered palette are the editable source of truth. Every
mutation creates an immutable parent-linked revision. Visual judgement remains
an explicit human or agent action; Pixelate has no durable review or approval
state and never launches an agent.

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

Assets currently have one universal form. Project palettes, configurable
conversion recipes, asset kinds, sheets, durable reviews, approvals, and stored
preview images are intentionally absent. A revision's `recipe.json` is retained
only as immutable provenance describing the operations that produced it.

Selected references and revision payloads are content-verified. Enlarged preview
PNGs are rendered on demand with exact nearest-neighbour scaling, never persisted
inside revisions.

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
| History navigation | `revision set-head` |
| Bundle or named image export | `asset export`, `export-file` |

`pixelate guide --root .` is the machine-readable source of truth for agents.
Update it whenever an agent-facing command or workflow changes.

## Non-goals

- Agent launchers, provider SDKs, or repository-selected commands.
- A generic image pipeline, resource/profile system, or plugin marketplace.
- Asset-kind-specific sheet, tile, or UI behavior before those products exist.
- Automatic source selection, conversion commit, visual acceptance, or export.
