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
  art/…                    # real game-project raster folders and files
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

An asset manifest may link its stable Pixelate identity to one project-relative
raster path and records the last verified project-file hash. Existing manifests
without a path remain valid and are presented at the project root as unexported
assets. New assets receive an explicit path or default to `<asset-id>.png`. The real game-project folder hierarchy is
the only user-facing hierarchy; `.pixelate/assets/<asset-id>` remains internal
immutable storage and never moves when its linked image moves. Discovered PNG,
JPEG, and WebP files have no Pixelate identity until explicit adoption imports a
verified internal source copy. Reference sources can be hidden through a reversible
project-level discovery preference; this is not a virtual asset hierarchy. Exact
pixel-art adoption commits a lossless indexed revision and begins at editing.
Discovery honors project ignore rules, excludes
internal/dependency/output folders, and does not follow symlink escapes.

Real folder creation, rename/move, linked-image moves, and empty-folder deletion
are explicit contained operations. Collisions are refused. A folder move updates
every affected asset manifest with rollback on failure. Removing a linked asset
from Pixelate retains the project image. A separate confirmed operation may
delete one supported project image without deleting its Pixelate identity or
history; recursive deletion remains absent. Empty folders receive no `.gitkeep`
and therefore are not retained by Git.

## CLI parity

Every desktop capability must use the same application use case as a CLI route.

| Capability | CLI |
| --- | --- |
| Project discovery | `init`, `project show` |
| Project image catalog, visibility, deletion, and real folders | `project catalog`, `hide-image`, `show-image`, `delete-image`, `create-folder`, `move-folder`, `delete-folder` |
| Asset lifecycle and brief | `asset list`, `init`, `inspect`, `set-brief`, `rename`, `delete` |
| Reference/exact-pixel adoption, relink, move, external update | `asset adopt`, `adopt-pixel-art`, `relink`, `move`, `update-linked-source` |
| Source import/replacement | `reference import`, `asset update-source` |
| Pixelization and canvas placement | `revision pixelize`, `compose` |
| Inspection and vision-friendly preview | `revision inspect`, `preview` |
| Pixel and palette editing | `revision draw`, `fill`, `recolor`, `remap` |
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
- Asset-kind-specific sheet, tile, or UI behavior before those products exist.
- Automatic source selection, conversion commit, visual acceptance, or export.
