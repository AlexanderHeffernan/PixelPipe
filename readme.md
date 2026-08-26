# PixelPipe

PixelPipe is an AI-first, project-aware workstation for producing deterministic,
game-ready pixel-art assets from selected visual references.

The desktop app now presents one opinionated path: choose a game folder, create
a sprite brief, create or import a smooth reference, pixelize it with starter
defaults, inspect the native result, and export PNG/JSON runtime assets.

- [Architecture contract](ARCHITECTURE.md)
- [Milestone 0 research and decision record](docs/milestones/M0-charter.md)
- [Milestone 1 foundation record](docs/milestones/M1-foundation.md)
- [Milestone 2 conversion record](docs/milestones/M2-conversion.md)
- [Milestone 3 refinement/review record](docs/milestones/M3-review-and-refinement.md)
- [Milestone 4 desktop review record](docs/milestones/M4-desktop-review.md)
- [Milestone 6 pre-revision asset record](docs/milestones/M6-pre-revision-assets.md)

## Run the desktop app

Prerequisites: Rust 1.98+, Node.js 22+, and the platform dependencies required
by [Tauri 2](https://v2.tauri.app/start/prerequisites/). Then:

```sh
cd apps/desktop
npm install
npm run app
```

Choose your game folder in the native dialog. PixelPipe creates `.pixelpipe`
project metadata and starter 16×16, 32×32, and 64×64 sprite recipes without
changing other files. The embedded terminal supports the user's coding agent of
choice; PixelPipe itself does not launch or manage agents.

The Tauri CLI starts both Vite and the native Rust process. Running
`cargo run -p pixelpipe-desktop` alone is not supported: the debug window
expects Vite at `http://localhost:1420` and will otherwise be blank.

## CLI foundation

```sh
cargo run -p pixelpipe-cli -- init --root /path/to/game --name "My Game"
cargo run -p pixelpipe-cli -- revision create \
  --root /path/to/game \
  --asset signal-flare \
  --pixels fixtures/m1/tiny-raster.json \
  --preview-scale 4
```

Commands emit JSON.

## Try the M2 path

`revision convert` accepts an RGBA PNG, a versioned palette JSON file, and a
complete conversion-settings JSON file. The synthetic M2 fixtures are stored as
RGBA arrays rather than a binary reference; the conformance tests encode those
arrays to PNG in memory and exercise this command end to end.

```sh
cargo run -p pixelpipe-cli -- revision convert \
  --root /path/to/game \
  --asset signal-flare \
  --source /path/to/selected-reference.png \
  --palette /path/to/palette.json \
  --settings /path/to/conversion.settings.json \
  --preview-scale 8
```

Use `--conversion sheet --kind sheet` with a `SheetSettings` JSON document for
a regular frame grid.

## M3 refinement and review

```sh
pixelpipe revision inspect --root /path/to/game --asset signal-flare
pixelpipe revision patch --root /path/to/game --asset signal-flare \
  --parent r000001 --patch patch.json
pixelpipe revision remap --root /path/to/game --asset signal-flare \
  --parent r000001 --remap palette-remap.json
pixelpipe revision compare --root /path/to/game --asset signal-flare \
  --left r000001 --right r000002 --visual-preview diff.png
pixelpipe revision review --root /path/to/game --asset signal-flare \
  --revision r000002 --actor-kind human --actor alexander \
  --decision accepted --note "Reads clearly at native size"
```

Patch/remap commands always require an explicit immutable parent. An empty patch
from an older revision creates a new identical child and acts as undo without
moving or rewriting history. Review events never change revision bytes or the
separate approval pointer.

## M6 pre-revision assets

```sh
pixelpipe asset init --root /path/to/game --asset signal-flare \
  --kind sprite --brief "Strict overhead signal flare"
pixelpipe project set-palette --root /path/to/game \
  --id synthetic-flare --file fixtures/m6/palette.json
pixelpipe project set-recipe --root /path/to/game \
  --file fixtures/m6/recipe.json
# Create a source with any external coding agent, then import it.
pixelpipe reference import --root /path/to/game \
  --asset signal-flare --file /path/to/reference.png
pixelpipe revision convert-selected --root /path/to/game \
  --asset signal-flare --recipe synthetic-flare
```

Assets serialize `draft`, `awaiting_reference`, `selected_reference`, and
`revisioned` lifecycle states. Patch, remap, compare, and review remain
revision-only. First conversion resolves project resources and snapshots their
content and hashes into `r000001`; editing those resources later never rewrites
the revision.

## License

PixelPipe source code and schemas are licensed under either the MIT License or
Apache License 2.0, at your option. Generated/reference art is not covered by
that blanket grant; fixture and art provenance is recorded separately.
