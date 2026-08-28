# Pixelate

Pixelate is an opinionated workstation for turning visual references into
deterministic, game-ready pixel art. It keeps briefs, references, conversion
settings, indexed pixels, immutable revisions, and exports together in a
game repository.

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

The Assets browser discovers supported PNG, JPEG, and WebP artwork in the game
project's real folder structure while honoring ignore rules and excluding
internal, dependency, and build folders. Existing images are preview-only until
you explicitly choose **Edit in Pixelate**, which imports a verified internal
source copy without rewriting the project file. Managed assets keep stable IDs
and immutable history when images or containing folders move. Assets without a
linked project path appear under Drafts.

Pixelate can explicitly create, rename/move, and delete empty real folders, and
move linked images without overwriting collisions. It never recursively deletes
project files or creates `.gitkeep`; empty folders are not retained by Git.
Removing a linked asset from Pixelate leaves its project image intact.

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

Use `pixelate project catalog --root .` to inspect discovered and managed project
images, including missing or externally modified links. Folder operations and
adoption are available through `project create-folder|move-folder|delete-folder`
and `asset adopt|relink|move|update-linked-source`; all refuse unsafe paths and
silent overwrites.

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
