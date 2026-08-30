# Pixelate

Pixelate is an opinionated workstation for turning visual references into
deterministic, game-ready pixel art. It keeps briefs, references, conversion
settings, indexed pixels, immutable revisions, and exports together in a
game repository.

Every asset is one ordered clip containing one or more frames. Frames share one
canvas, indexed palette, transparent index, and pivot; each has a stable ID and
duration. A static sprite is the same model with one frame.

The product has one workflow:

```text
brief → reference → pixelize → visually review → refine → export
```

Pixelate never launches or manages coding agents. Run any agent in the embedded
terminal; it receives the same capabilities as a human through the `pixelate`
CLI and no privileged internal interface.

## Desktop

Prerequisites are Rust 1.98+, Node.js 22+, and the platform dependencies required
by [Tauri 2](https://v2.tauri.app/start/prerequisites/).

```sh
cd apps/desktop
npm install
npm run app
```

Choose a game folder. Pixelate creates a `.pixelate` project and supplies focused
pixelization defaults directly. Other game files remain untouched until an
explicit export.

The macOS app and Linux AppImage check for signed updates when they open and
every six hours. An available update is always shown before anything is
downloaded or installed. Open Settings to see the installed version, check
manually, install and restart, or disable automatic checks.

Pixelate also offers to install its bundled `pixelate` command on first launch:
`/usr/local/bin/pixelate` on macOS and `~/.local/bin/pixelate` for a Linux
AppImage. The Linux `.deb` installs `/usr/bin/pixelate` directly. The Command
line section in Settings reports the command's status and can install, repair,
or remove a managed command without overwriting an unrelated file.

`apps/desktop` is intentional: it groups the Vue interface and its thin Tauri
adapter as one application, separate from the reusable Rust crates. Start it
through `npm run app`; running the Rust desktop package alone does not start the
Vite frontend.

## CLI and coding agents

Build the CLI:

```sh
cargo build -p pixelate-cli
```

All commands are non-interactive and return JSON. From an initialized game
folder, an agent should begin with:

```sh
pixelate guide --root .
```

`guide` is the machine-readable source of truth for the current workflow and
capabilities. It tells an agent how to create or update an asset, inspect state,
refine pixels or colours, export, and visually verify its work without reading
internal `.pixelate` files.

The CLI never prints unsolicited update notices. Use `pixelate version` for
machine-readable version information and explicitly run `pixelate update` to
install the latest signed standalone macOS CLI. The CLI bundled inside the app
updates with the app instead.

A typical direct flow is:

```sh
pixelate init --root /path/to/game --name "My Game"
pixelate asset init --root /path/to/game --asset signal-flare \
  --brief "Strict overhead signal flare"
pixelate reference import --root /path/to/game --asset signal-flare \
  --file /path/to/reference.png
pixelate revision pixelize --root /path/to/game --asset signal-flare \
  --resolution 32 --colors 16 --background auto --actor agent
pixelate asset inspect --root /path/to/game --asset signal-flare
pixelate revision preview --root /path/to/game --asset signal-flare \
  --output /tmp/signal-flare-preview.png
pixelate asset export --root /path/to/game --asset signal-flare \
  --destination /path/to/game/assets --overwrite
pixelate asset export-file --root /path/to/game --asset signal-flare \
  --destination /path/to/game/assets/signal-flare.webp --overwrite
```

Turn that sprite into an animation without changing asset identity:

```sh
pixelate frame duplicate --root /path/to/game --asset signal-flare \
  --frame frame-0001
pixelate frame duration --root /path/to/game --asset signal-flare \
  --duration 140
pixelate frame rename --root /path/to/game --asset signal-flare \
  --frame frame-0002 --name 'Passing pose'
pixelate revision draw --root /path/to/game --asset signal-flare \
  --frame frame-0002 --pixel '12,8=3' --actor agent
pixelate revision inspect --root /path/to/game --asset signal-flare
```

`frame import-sequence` consumes repeated `--file` arguments in their explicit
order and derives one palette across the complete batch. `frame import-sheet`
requires explicit frame dimensions and repeated zero-based `--cell` values in
the intended order; Pixelate never guesses grids or directory order. Blank and
duplicate frames require no reference. For generated animation, create and
review one still pose at a time, then use `frame import` so each accepted pose
keeps the established canvas and palette. Do not ask an image generator for a
spritesheet; sheet import is only for explicitly reviewed existing artwork.

For multiple frames, `asset export` writes a deterministic horizontal PNG
spritesheet and companion JSON with stable IDs, order, rectangles, durations,
shared canvas, and pivot. One-frame PNG, lossless WebP, and indexed JSON exports
retain their existing behavior. `revision preview` produces an enlarged contact
sheet when no `--frame` is supplied; use `revision inspect` alongside it for
timing, and pass `--frame <stable-id>` to inspect one pose.
Inspection also reports exact adjacent and loop-closing pixel transitions,
separating silhouette changes from palette-index changes over opaque overlap.
Large opaque-colour change with little silhouette motion is a prompt to fix the
source pose, not to blur or automatically smooth the indexed result. Structured
motion warnings flag ≥40% opaque-colour churn or ≥20% silhouette replacement;
replace the warning's destination frame with `frame replace`, inspect again, and
do not call the animation complete until warnings are cleared or reviewed by the
human as intentional broad motion.

For motion that can reuse separated pixel-art parts, Pixelate also supports a
generic rigging route. Ask the image model for one source sheet of separated
parts—not a temporal animation spritesheet—then pixelize it. An agent can crop
explicit rectangles into reusable parts and define arbitrary parent-linked nodes
and manual poses with `rig create`. Use `rig mutate` to position, rotate, scale,
reorder by depth, hide, or reassign parts; swap assignments; adjust manual poses;
and choose a shared duration plus optional deterministic in-between frames. The
model is deliberately generic: it has no humanoid anatomy, IK, weapons, or named
attachments.

The desktop overlays the rig handles directly on the indexed sprite. Select a
manual pose in the timeline, drag a node to reposition it, use the rig controls
for rotation, scale, depth, visibility, part assignment, interpolation, and
timing, then play the complete rendered sequence. Automatic frames are derived
from adjacent manual poses and do not appear as editable timeline cards. Use
`rig bake` only when the motion is ready for ordinary frame-by-frame pixel touch
up; baking preserves the rendered sequence exactly and ends rig editing for that
revision. Run `pixelate guide --root .` for the versioned definition and mutation
JSON examples intended for agents.

### Visual verification

Agents should run `revision preview` after every conversion or edit and inspect
the resulting PNG with their vision tool. By default Pixelate enlarges the image
with exact nearest-neighbour scaling toward a 512-pixel longest edge. This is
better for vision analysis than a tiny native sprite while preserving every hard
pixel edge and colour exactly; scaling is capped at 64×. Use `--scale 1` only
when native resolution is specifically required.

Preview is read-only: it never creates a revision or changes asset head. Its JSON
result includes the native and output dimensions, integer scale, revision, path,
and SHA-256.

## Codebase

- `crates/pixelate-core`: pure deterministic pixel model and operations.
- `crates/pixelate-project`: `.pixelate` persistence, schemas, and revisions.
- `crates/pixelate-app`: shared application use cases used by every frontend.
- `crates/pixelate-cli`: complete scriptable adapter and agent guidance.
- `apps/desktop`: Vue/Tauri human interface over the application use cases.

All five workspace packages are active. See [ARCHITECTURE.md](ARCHITECTURE.md)
for their dependency rules and product invariants.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cd apps/desktop && npm run format:check && npm test && npm run build
```

## Releases

Every push to `main` creates the next patch release for Apple silicon and Intel
macOS plus x86-64 and ARM64 Linux. The workflow builds signed macOS bundles and
Linux AppImages and `.deb` packages, bundles each matching CLI, publishes signed
standalone CLI binaries, validates the complete updater manifest, and only then
exposes the GitHub Release as latest. A rerun for an already tagged commit
reuses its version.

The repository must define these GitHub Actions secrets:

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

The committed public key belongs only to Pixelate. Keep its matching private key
and password backed up securely; never rotate or lose them after a release,
because installed clients trust that key for every future update. Releases
currently use ad-hoc macOS signing; Apple Developer ID signing and notarization
can be added before broader distribution.

## License

Pixelate source code and schemas are available under either the MIT License or
Apache License 2.0, at your option. Generated and reference artwork is not
covered by that blanket grant.
