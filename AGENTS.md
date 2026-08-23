# PixelPipe engineering contract

PixelPipe is opinionated software. Build the brief → reference → pixelize →
review → export workflow we believe is best; do not turn it into a generic image
pipeline or a collection of provider settings.

## Structure

- Target roughly 200 lines per production source file. Five hundred lines is an
  exceptional ceiling and requires a cohesive responsibility that becomes less
  clear when split.
- `main.rs`, `lib.rs`, `mod.rs`, and `App.vue` are composition surfaces. Keep
  domain behavior in named feature modules and components.
- Organize Rust by domain capability with thin module facades and selective
  exports. Organize Vue by feature with typed services and focused composables.
- Name files for the responsibility they own. Avoid generic `helpers`, `utils`,
  or `manager` modules when a domain name exists.
- Prefer changing the owning source of truth over adding wrappers or parallel
  state. Project files and Rust application use cases remain authoritative.
- Keep tests outside production implementation files where practical: Rust
  sibling `tests/` modules or integration tests; colocated `*.test.ts` files for
  Vue and TypeScript. Split large test files by behavior.

## Product boundaries

- The deterministic pixel engine does not know about Tauri, agents, providers,
  filesystems, or UI state.
- Amp, Codex, and future agent connectors sit above the application boundary.
  They receive no privileged operation unavailable to CLI/UI users.
- Repository-controlled files may not silently select or execute a command.
  Connecting a detected local agent requires explicit user consent.
- AI generation, candidate selection, conversion, review, approval, and export
  are separate visible states. Never auto-select, auto-apply, or auto-approve.
- Prefer a few excellent built-in defaults over requiring users to create
  palettes, recipes, profiles, or JSON before their first sprite.

## Verification

- Run the cheapest focused check while iterating, then one coherent cross-layer
  check for a completed vertical slice.
- Verify UI behavior through DOM/accessibility facts and exercised interactions;
  use screenshots for visual judgment, not functional claims.
- Preserve deterministic golden hashes and immutable revision guarantees.
