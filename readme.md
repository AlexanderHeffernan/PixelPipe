# PixelPipe

PixelPipe is an AI-first, project-aware workstation for producing deterministic,
game-ready pixel-art assets from selected visual references.

**Milestone 3 is ready for review.** The headless Rust pipeline can initialize a
project, deterministically convert references, branch immutable revisions with
atomic pixel/palette operations, inspect and compare them, and record explicit
human or agent visual-review events.

- [Architecture contract](ARCHITECTURE.md)
- [Milestone 0 research and decision record](docs/milestones/M0-charter.md)
- [Milestone 1 foundation record](docs/milestones/M1-foundation.md)
- [Milestone 2 conversion record](docs/milestones/M2-conversion.md)
- [Milestone 3 refinement/review record](docs/milestones/M3-review-and-refinement.md)

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

## License

PixelPipe source code and schemas are licensed under either the MIT License or
Apache License 2.0, at your option. Generated/reference art is not covered by
that blanket grant; fixture and art provenance is recorded separately.
