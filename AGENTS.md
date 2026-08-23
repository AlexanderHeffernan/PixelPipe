# PixelPipe engineering guidance

Read `ARCHITECTURE.md` and the active record under `docs/milestones/` before
changing product code.

- Keep model/provider/process code outside `pixelpipe-core`.
- Route every frontend through `pixelpipe-app`; the CLI is an adapter, not a
  second implementation.
- Version persisted schemas and reject unknown schema identifiers.
- Deterministic code must use stable ordering and explicit encoder settings.
- A selected reference is the start of reproducibility; do not describe reference
  generation itself as deterministic.
- Do not overwrite revision directories.
- Add dependencies or crates only for a current milestone requirement.

Before reporting a code milestone, run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
