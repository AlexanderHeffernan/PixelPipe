# PixelPipe

PixelPipe is an AI-first, project-aware workstation for producing deterministic,
game-ready pixel-art assets from selected visual references.

**Milestone 5 is ready for review.** PixelPipe now has a provider-neutral,
user-approved one-shot agent workflow for asynchronous reference generation,
candidate selection, critique, and validated-but-unapplied refinements.

- [Architecture contract](ARCHITECTURE.md)
- [Milestone 0 research and decision record](docs/milestones/M0-charter.md)
- [Milestone 1 foundation record](docs/milestones/M1-foundation.md)
- [Milestone 2 conversion record](docs/milestones/M2-conversion.md)
- [Milestone 3 refinement/review record](docs/milestones/M3-review-and-refinement.md)
- [Milestone 4 desktop review record](docs/milestones/M4-desktop-review.md)
- [Milestone 5 configured agent record](docs/milestones/M5-agent-workflow.md)

## Try the M1 path

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

## M4 desktop workstation

Install the frontend dependencies, then run the Tauri app from the desktop
directory:

```sh
cd apps/desktop
npm install
npm run build
cargo run -p pixelpipe-desktop
```

The desktop uses the same `pixelpipe-app` use cases as the CLI. It receives only
verified revision PNG bytes, keeps selection transient, and requires an explicit
parent for every pixel patch or palette remap.

## M5 configured agent

Agent executables are configured and explicitly approved in user-local settings,
never `.pixelpipe/project.toml`. `pixelpipe agent run` emits typed JSON lifecycle
events to stderr and a final JSON result to stdout. Generated candidates are
validated and content-addressed, then require a separate
`pixelpipe reference select` action. Critiques never change project state;
proposals must pass through the existing explicit-parent patch/remap use cases.

See the [M5 record](docs/milestones/M5-agent-workflow.md) for profile locations,
the strict JSON protocol, safety limits, and CLI examples.

## License

PixelPipe source code and schemas are licensed under either the MIT License or
Apache License 2.0, at your option. Generated/reference art is not covered by
that blanket grant; fixture and art provenance is recorded separately.
