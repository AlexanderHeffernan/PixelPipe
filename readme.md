# PixelPipe

PixelPipe is an AI-first, project-aware workstation for producing deterministic,
game-ready pixel-art assets from selected visual references.

**Milestone 1 is approved.** The headless Rust foundation can initialize a
project and turn a structured indexed raster into an immutable revision
containing an indexed PNG and exact nearest-neighbour preview. Milestone 2 will
add the proven deterministic reference-conversion behavior.

- [Architecture contract](ARCHITECTURE.md)
- [Milestone 0 research and decision record](docs/milestones/M0-charter.md)
- [Milestone 1 foundation record](docs/milestones/M1-foundation.md)

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

## License

PixelPipe source code and schemas are licensed under either the MIT License or
Apache License 2.0, at your option. Generated/reference art is not covered by
that blanket grant; fixture and art provenance is recorded separately.
