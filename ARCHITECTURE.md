# PixelPipe Architecture Contract

Status: **Accepted in Milestone 0 on 2026-08-23**

## Product charter

PixelPipe is a project-aware pixel-art asset workstation for programmers and game
developers who can describe and judge what they want but do not have the time or
specialist skill to draw every asset. It runs from a game repository root and
keeps art direction, palettes, references, recipes, revisions, provenance,
validation, approval, and export mappings together.

The default workflow is deliberately opinionated:

1. Write a structured brief in the game project.
2. Ask a configured user agent for several smooth, high-resolution references.
3. Select one reference. This freezes the nondeterministic boundary.
4. Deterministically remove the backdrop, crop/register, hard-reduce, map to an
   indexed palette, validate, and render native and nearest-neighbour previews.
5. Inspect the native asset, enlarged preview, and relevant asset family.
6. Accept proposed refinements or make the same serialized edits through the UI
   or CLI, producing a new immutable revision.
7. Approve a revision and export indexed PNG plus machine-readable metadata to
   the paths expected by the game.

PixelPipe is not “an image generator with an export button.” Its product value is
the repeatable project workflow around generation, deterministic reduction,
visual judgement, revision, and delivery.

## Hard invariants

1. **AI invents; the engine transforms.** Model/provider adapters never enter the
   deterministic pixel engine.
2. **Reproducibility starts at selection.** Reference generation is
   nondeterministic. A selected reference is frozen and hashed; every subsequent
   transformation must reproduce the same canonical raster and export bytes for
   the same engine version and recipe.
3. **Structured pixels are the editable source.** PNGs and enlarged previews are
   renders, not the authoring format.
4. **Indexed means indexed.** Approved native PNGs use an explicit palette and
   transparent index. The pipeline does not introduce interpolation,
   antialiasing, or dithering unless a future schema explicitly names a new
   deterministic operation.
5. **Every canonical raster mutation creates a revision.** Approved and
   historical revisions are never overwritten. Undo, restore, and branching move
   an asset head or create another revision; they do not rewrite history.
6. **One application surface.** UI, CLI, and automation invoke the same typed use
   cases and receive the same results. An agent has no privileged engine calls.
7. **Validation is necessary, not sufficient.** Dimensions, palette, alpha,
   bounds, components, pivots, hashes, and preview scaling are machine-checkable.
   Readability, camera angle, contact, cluster quality, and family coherence
   remain explicit visual-review states.
8. **Project files cannot silently execute code.** Agent executable configuration
   is user-local or supplied explicitly, never trusted from `.pixelpipe/`.

## System shape

```text
 UI (Tauri webview)       CLI              configured user agent
         │                 │                         │
         └──────────┬──────┘                         │ JSON protocol
                    ▼                                ▼
            application/use cases ◀──────── AI workflow service
     briefs · references · revisions · approval · export
                    │
                    ▼
             deterministic engine
 crop/register · reduce · palette · edit · validate · encode
                    │
                    ▼
          .pixelpipe project repository
```

The dependency direction is inward. Frontends and agent adapters depend on the
application layer; the application layer depends on project storage and the
engine; the engine knows nothing about Tauri, filesystems, subprocesses, models,
or game engines.

## Proposed implementation architecture

Use a Rust workspace with Tauri 2 and a Vue 3 + TypeScript frontend.

- Rust gives the low-level engine deterministic integer operations, explicit
  error handling, cross-platform native binaries, and direct reuse by CLI and
  desktop application.
- Tauri keeps the desktop shell small and lets native filesystem/process policy
  stay in Rust.
- Vue is recommended over introducing Svelte because Shipyard demonstrates a
  maintainable Vue/Tauri desktop pattern in the maintainer's existing work. The
  architecture does not depend on Vue, but choosing one stack now is better than
  preserving a hypothetical frontend swap.
- A browser-only app is rejected: project-root filesystem access, indexed image
  encoding, agent subprocesses, and game export paths are core capabilities.
- Electron is rejected for the initial product: it adds a second backend runtime
  and larger distribution footprint without solving a PixelPipe requirement.
- Python remains useful as fixture-generation evidence, not as the production
  engine. Port behavior from the PoC; do not port its script topology.

Proposed repository layout (names are contracts; crates are added only when their
milestone begins):

```text
crates/
  pixelpipe-core/       deterministic data model and pixel operations
  pixelpipe-project/    .pixelpipe schemas, storage, revisions, locking
  pixelpipe-app/        shared use cases, AI workflow, export coordination
  pixelpipe-cli/        thin command-line adapter
apps/
  desktop/              Tauri shell and Vue frontend
fixtures/
  poc/                  licensed/approved PoC golden inputs and expectations
docs/
  decisions/            accepted architecture decision records
  milestones/           scope and review gates
```

Do not split `pixelpipe-core` into operation-specific crates. A small cohesive
engine is easier to audit than a framework of abstractions.

## Domain boundaries

### Deterministic core

Owns versioned types and pure operations for indexed rasters, palettes, crop and
registration, hard cell reduction, palette mapping, pixel edits, validation,
nearest previews, and deterministic PNG/JSON encoding. Algorithms use explicit
integer/fixed rules and stable tie-breaking; unordered iteration must never
affect output.

The core does not choose whether an image is good, invoke AI, read project
configuration, or know export destinations.

### Deterministic conversion contract

The M2 reference boundary is an RGBA PNG byte stream plus an ordered palette and
a complete serialized settings object. PixelPipe hashes the original PNG bytes
and canonical palette; the recipe records every conversion setting.

Conversion uses this fixed order:

1. Decode to RGBA8 without color invention.
2. Apply an alpha threshold, optionally removing only target-colored pixels
   connected to the source border.
3. Find the visible alpha bounds and fit them into the target margin using one
   integer rational scale.
4. Map every visible source pixel to the nearest non-transparent palette entry
   by squared RGB distance.
5. For each target cell, enforce the integer foreground-coverage threshold and
   choose the modal palette index.
6. Center or bottom-register the result, validate connected-component
   expectations, then encode the canonical raster and exports.

All equal-distance and equal-count ties choose the lower ordered palette index;
coverage equal to the threshold is foreground. There is no filtering,
antialiasing, dithering, floating-point math, or implicit semantic repair.
Regular sheets apply cleanup per source cell and share one scale, registration,
baseline, and palette across frames. PixelPipe validates every frame rather than
silently deleting disconnected components.

### Project store

Owns schema migration, atomic writes, asset/revision identity, parent links,
head/approved pointers, content hashes, project locking, and path containment.
It exposes repositories to the application layer rather than leaking file layout
through every use case.

### Application/workflow

Owns briefs, candidate-reference runs, selection, recipe execution, revision
creation, review state, approval, export mappings, and AI jobs. It is the only
layer used by CLI commands and Tauri commands.

### AI adapters

Own the configured subprocess protocol, capability checks, timeouts,
cancellation, and run capture. They return references, critiques, or proposed
operations. They cannot write `.pixelpipe/` directly; the application imports
and validates their results.

### Frontends

The CLI is the complete scriptable interface. The desktop app is the focused
visual interface for briefs, candidate selection, native/enlarged/family review,
revision comparison, and operation editing. Neither frontend owns domain state.

The M4 desktop adapter is a fifth outer workspace crate at
`apps/desktop/src-tauri`; the four M1 crates remain the engine, store,
application, and CLI boundaries. Tauri commands deserialize application request
documents and return typed results. Verified PNG bytes cross the IPC boundary as
base64 rather than frontend-selected filesystem paths. Vue owns only transient
path, selection, form, loading, and error state; reopening or refreshing always
reconciles from `pixelpipe-app`.

Deterministic desktop operations remain request/response commands. M5 adds one
typed event stream only for subprocess work: started, progress/log,
candidate-ready, completed, failed, and cancelled. Tauri owns the transient task
registry/cancellation flags; durable run state is reconciled from the application
and project layers rather than localStorage.

## Project-root model

PixelPipe discovers the game root by walking upward for `.pixelpipe/project.toml`.
`pixelpipe init` creates it only in the explicitly selected root.

```text
game-root/
  .pixelpipe/
    project.toml                 schema, art direction, defaults, export maps
    palettes/<palette>.toml      ordered RGBA colours and transparent index
    runs/<task-id>/              ignored local agent run/candidate capture
      run.json
      candidates/<sha256>.png
    assets/<asset-id>/
      asset.toml                 brief identity, kind, head, approved revision
      references/
        selected/<sha256>.png    immutable selected reference bytes
        selection.json           explicit run/candidate/hash selection
      revisions/r000001/
        brief.md                 brief snapshot used for this revision
        recipe.json              selected reference + deterministic operations
        pixels.json              canonical indexed raster
        provenance.json          inputs, versions, hashes, actor/tool records
        validation.json          deterministic checks and review requirements
        native.png               review render
        preview.png              nearest-neighbour review render
      reviews/r000001.json       append-only review event history
    cache/                       disposable and ignored
    tmp/                         disposable and ignored
```

Manifests and canonical rasters are text and stable-key serialized for useful
diffs. Native runtime PNGs and small nearest previews are tracked by default.
Selected references are tracked only when their rights are clear and the project
opts in; Git LFS is optional for large references. Reproducible review bundles,
cache, temporary files, and agent runs are ignored unless explicitly recorded.
Ignored artifacts may still be named and hashed in provenance. Exported game
files live at paths declared in `project.toml`, normally outside `.pixelpipe/`.

Paths in project files are project-relative, use `/` separators, and may not
escape the root. Asset IDs are stable lowercase slugs. Revision IDs are
monotonic per asset (`r000001`); each revision records zero or one parent in M1.
Creating from an older revision naturally branches without needing a separate
branch entity. A later need for merges must be demonstrated before adding them.

## Revision and provenance contract

A revision is an immutable snapshot, not merely a saved image. It records:

- brief snapshot and selected-reference SHA-256;
- canonical recipe and operation parameters;
- engine, schema, and encoder versions;
- parent revision and initiating actor (`human`, `cli`, `desktop`, or named
  agent run; actor is attribution, not authority);
- canonical raster, native PNG, and validation hashes;
- palette identity and content hash;
- pivot, registration, frame/sheet metadata, and asset-aware component rules;
- automated validation results and pending/passed visual-review state;
- AI adapter identifier, capability, model/provider when reported, prompt/request
  record, and returned critique/proposal, with secrets redacted.

Approval is a mutable pointer in `asset.toml` to an immutable revision. Export
records the approved revision and output hashes. A critique never changes pixels;
accepted proposed operations create a revision through the same application use
case as manual operations.

### Refinement, comparison, and review contract

Pixel patches are complete coordinate/index lists. Duplicate coordinates,
out-of-bounds coordinates, and out-of-palette indices reject the whole operation
before storage. Palette remaps provide the complete replacement palette and one
new index for every old index; the old transparent index must map to the new
transparent index. Both operations inherit the parent's component rule. Sheets
retain per-frame rules rather than incorrectly validating the combined sheet as
one sprite.

Every refinement names an explicit parent, hashes that parent's canonical
raster, and creates a new monotonic revision. Creating from an older parent is a
branch; an empty patch from an older parent is undo-by-new-revision. Asset head
advances to the new child, but no prior revision or review render changes.

Inspection reports bounds, pivots, complete palette usage, and a stable hex-index
text grid. Comparison uses rendered RGBA values for pixel equality, reports
row-major coordinate/color differences and ordered palette differences, and can
emit indexed red/green/magenta removed/added/changed visual diffs.

Review history is stored outside immutable revision payloads as atomic event
records. Each event names a human or agent actor and an explicit `reviewed`,
`changes_requested`, or `accepted` decision. Deterministic validation never
creates a review event or aesthetic acceptance. Review acceptance is not the
asset's separate approval/export pointer.

## CLI, UI, and agent parity

The application defines versioned use cases such as:

| Capability | CLI example | Desktop | Agent route |
| --- | --- | --- | --- |
| Create/update brief | `pixelpipe brief set` | brief editor | CLI or application request |
| Generate references | `pixelpipe reference generate` | Generate action | configured adapter |
| Import/select reference | `pixelpipe reference select` | candidate picker | same use case |
| Convert/rebuild | `pixelpipe revision create` | Convert/refine action | same use case |
| Inspect/validate | `pixelpipe asset inspect`, `validate` | review workspace | CLI structured output |
| Apply pixel operations | `pixelpipe edit apply --ops ...` | editor tools | proposed ops, then same use case |
| Critique/propose | `pixelpipe critique`, `refine propose` | review actions | configured adapter |
| Approve/export | `pixelpipe approve`, `export` | explicit actions | CLI or same use case |

All CLI read commands support stable JSON output. Mutating commands support a
non-interactive mode and return revision IDs. Interactive convenience must wrap,
not replace, explicit arguments. The agent can drive PixelPipe through the CLI;
the desktop app may invoke the configured adapter through the application layer.

“Parity” means equivalent capability and semantics, not identical interaction.
For example, the UI may compare images visually while the CLI emits their paths
and hashes. Approval is available to automation but must always be explicit; AI
generation or critique never implies approval.

## Configurable agent protocol

PixelPipe has no model-provider dependency in its core and no MCP requirement.
The user configures a local executable through user-local settings. Repositories
may name desired capabilities but may not name an executable that PixelPipe runs
automatically.

Protocol `pixelpipe.agent.v1` is a one-shot process contract:

- PixelPipe writes one JSON request to stdin and closes it.
- The adapter writes one JSON response to stdout; diagnostics go to stderr.
- Every request includes an operation, project-context snapshot, input paths and
  hashes, constraints, and a fresh writable output directory.
- Output paths must remain inside that directory and are imported only after
  validation and hashing.
- The approved profile declares allowed capabilities; every response reports
  adapter capabilities and metadata for verification/capture.
- Initial operations are `generate_references`, `critique_asset`, and
  `propose_refinement`.
- Responses may return images, structured critique, and serialized pixel
  operations. They may not claim approval or mutate the project.
- Cancellation terminates the child process; timeout and output limits are user
  policy. Prompts and responses are provenance, except configured secret fields.

M5 concretizes that contract as `pixelpipe.agent-profile/v1`,
`pixelpipe.agent-request/v1`, `pixelpipe.agent-response/v1`,
`pixelpipe.agent-run/v1`, and `pixelpipe.agent-task-event/v1`. Profiles are read
only from the operating system's user configuration directory, require
`approved: true`, an absolute executable, and allowlisted capabilities, and may
name environment variables whose values should be inherited/redacted. The child
starts with a cleared environment and an isolated temporary working directory.
Candidates must be relative, canonicalize inside the assigned output directory,
decode as RGBA PNG, and match their reported SHA-256 before import.

The adapter is trusted code, not an OS sandbox. PixelPipe prevents returned paths
from escaping the import boundary and never grants an alternate project write
API, but a user-approved executable retains that user's ambient filesystem
permissions. A cross-platform sandbox must not be promised without a separately
reviewed platform design.

An adapter can wrap Amp, another coding agent, a local model, or a hosted provider.
Provider-specific authentication, streaming, and retries stay inside the adapter.
Do not build a universal provider SDK into PixelPipe until real adapters show the
one-shot contract is insufficient.

## Desktop interaction contract

Extract principles from Shipyard rather than copying its appearance:

- typed native services and reconciled feature state;
- clear shell, feature, and ephemeral interaction-state boundaries;
- scoped shortcuts that ignore editable targets;
- keyboard-operable resizing, tabs, menus, and dialogs with focus restoration;
- compact toolbars, container-aware layouts, restrained accent colour, subtle
  fast motion, and reduced-motion support;
- broad inert drag regions with interactive titlebar controls explicitly layered;
- native and enlarged views always available, plus family/context comparison.

Do not copy Shipyard's Git-centric information architecture, dark-only styling,
private macOS APIs, fixed traffic-light measurements, raw DOM integration
workarounds, or permissive CSP. PixelPipe is cross-platform; platform materials
are optional progressive enhancement, never a layout dependency.

### M4 command surface

The Tauri shell exposes six thin commands: browse project, load verified
revision, compare revisions, record review, patch revision, and remap revision.
Browse/load/compare/review never move asset head. Patch/remap accept structured
documents and always name the selected revision as explicit parent; only the
application layer may create and advance to their immutable child.

Native, nearest, and visual-diff PNGs are loaded and hash-verified by the project
store before IPC. The webview cannot request arbitrary revision paths. Asset and
revision ordering comes from the project store, not filesystem enumeration or
frontend sorting. Error and empty states remain visible, command failures receive
focus, list navigation supports arrow/Home/End keys, and reduced-motion/focus
styles are part of the baseline rather than optional polish.

### M5 agent task surface

Generation, critique, and refinement proposal share one application task request
and lifecycle. Final run records capture exit status, duration, redacted
stdout/stderr, prompt, approved profile command hash, adapter identity and
reported provider/model/capabilities, errors, and artifact hashes. Candidate
import, explicit selected-reference mutation, critique, proposal, and revision
application remain separate states. Proposals validate read-only against their
named revision; accepting one still invokes the existing explicit-parent
patch/remap use case and creates a child revision.

## Explicit non-goals for the first product arc

- Training or hosting image models.
- A provider marketplace, workflow-node graph, plugin system, or MCP server.
- Photoshop-class painting, layers, blend modes, vector editing, or animation
  timeline. A focused indexed-raster editor comes only after the pipeline works.
- Automatic aesthetic approval or pretending validators can replace visual review.
- Semantic reconstruction of a bad reference during deterministic reduction.
- Runtime game-engine integration or hot reload beyond file export.
- Collaboration server, cloud asset database, accounts, or billing.
- Revision merging. History may branch; revisions do not merge in the initial model.
- Supporting arbitrary image formats internally. Canonical pixels are indices
  into an ordered RGBA8 palette; initial import/export is PNG plus JSON metadata.
- Pixel-perfect imitation of Shipyard or macOS at the expense of Windows/Linux.

## Architecture fitness checks

Milestones must preserve these executable checks once their layers exist:

1. A fixed selected reference + recipe produces byte-identical canonical JSON and
   native PNG across repeated runs and supported platforms.
2. CLI and Tauri adapters pass the same use-case conformance suite.
3. An artifact outside an AI task's assigned output directory cannot be imported,
   and an AI response cannot bypass explicit selection or revision creation.
4. Every approved/exported asset resolves to immutable inputs, recipe, versions,
   validation, and hashes.
5. Every nearest preview is an exact integer enlargement of the native raster.
6. Golden PoC fixtures retain palette, registration, transparency, and expected
   visual-review artifacts without retaining PoC script structure.
