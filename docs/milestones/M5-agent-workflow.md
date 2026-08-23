# Milestone 5 — Configured Agent Workflow

Status: **ready for review**
Started: 2026-08-23

## Approved scope

- One-shot, provider-neutral subprocess adapter configured only in user-local
  settings with explicit executable approval and capability allowlisting.
- Asynchronous started/progress/log/candidate-ready/completed/failed/cancelled
  lifecycle for desktop; JSON lifecycle lines plus final JSON for CLI.
- Generation run capture, candidate validation/content addressing, explicit
  candidate selection, revision critique, and validated patch/remap proposals.
- Fake subprocess and synthetic protocol/candidate fixtures; no live provider.

## Delivered trust and protocol contract

PixelPipe reads profiles from the current user's configuration directory, never
from a game repository:

- Linux: `$XDG_CONFIG_HOME/pixelpipe/agents` or
  `$HOME/.config/pixelpipe/agents`
- macOS: `$HOME/Library/Application Support/pixelpipe/agents`
- Windows: `%APPDATA%/pixelpipe/agents`

`<profile-id>.json` uses `pixelpipe.agent-profile/v1`. It must set
`approved: true`, use an absolute executable, list exact arguments and
capabilities, and name (not contain) environment variables to inherit. Secret
environment values are replaced with `[REDACTED]` in prompt/run/log captures.
Executable paths, arguments, and credentials do not enter `.pixelpipe` project
configuration. The stored run contains only the profile ID and a SHA-256 of the
approved executable/argument tuple.

User-local profile shape (the placeholder must be replaced by the user and this
file must not be committed to a game repository):

```json
{
  "schema": "pixelpipe.agent-profile/v1",
  "id": "my-agent",
  "approved": true,
  "executable": "<absolute executable path chosen by the user>",
  "args": ["<adapter arguments>"],
  "capabilities": [
    "generate_references",
    "critique_asset",
    "propose_refinement"
  ],
  "environment": [],
  "secret_environment": ["PROVIDER_API_KEY"],
  "timeout_seconds": 300
}
```

The adapter receives one canonical `pixelpipe.agent-request/v1` JSON document on
stdin in a fresh OS-temporary task directory. The environment is cleared before
only profile-named variables are restored. Revision operations receive copied,
hashed native PNG, preview PNG, and canonical raster inputs. The writable output
directory is explicit. Stdout must contain one strict
`pixelpipe.agent-response/v1` response; stderr is a diagnostic/progress stream.

This is a **trusted executable boundary, not an OS sandbox**. A user-approved
process has that user's operating-system permissions. PixelPipe limits what it
reveals, uses an isolated working directory, and refuses to import absolute,
missing, symlink-escaped, out-of-root, invalid-PNG, or hash-mismatched candidates.
Cross-platform process sandboxing is deliberately not implied.

## Workflow state contract

1. Starting generation/critique/proposal allocates a task and immediately
   returns control to the desktop.
2. Typed events report lifecycle and redacted diagnostics. Cancellation kills
   the child and records a cancelled run.
3. A finished run is atomically stored under ignored `.pixelpipe/runs/<task>/`
   with status, duration, exit code, prompt, redacted stdout/stderr, approved
   profile command hash, reported adapter/provider/model/capabilities, error,
   critique/proposal, and candidate hashes.
4. Candidate files are validated RGBA PNGs stored by SHA-256. Their arrival is
   candidate import, not selection.
5. Selection is a separate explicit application use case. It copies verified
   bytes into the content-addressed selected-reference store and atomically writes
   the selection record into the asset manifest; it does not move head, create a
   revision, review, or approve.
6. Critique remains prose attached to its run. A proposal is validated read-only
   against its explicit revision and inherited structure rule. Desktop can load
   it into an editable form, but only the existing patch/remap use case can create
   an immutable child revision. No proposal is auto-applied.

## CLI and desktop parity

```sh
pixelpipe agent run --root /game --asset signal-flare \
  --profile my-agent --operation generate --prompt "..."
pixelpipe agent run --root /game --asset signal-flare --revision r000003 \
  --profile my-agent --operation critique --prompt "Review native readability"
pixelpipe agent runs --root /game --asset signal-flare
pixelpipe agent candidate --root /game --run <task> \
  --candidate candidate-one --output candidate.png
pixelpipe reference select --root /game --asset signal-flare \
  --run <task> --candidate candidate-one
```

The desktop uses the same application records/use cases. Only process execution
is asynchronous; deterministic operations remain short request/response
commands. The UI exposes Generate, Critique, Propose, Cancel, candidate review
and explicit Select, plus an explicit “Load into editable form” step before the
existing revision-producing action.

## Deliberate non-goals

- Provider SDKs, MCP, repository-selected commands, credential storage, retries,
  or a universal streaming provider protocol.
- Automatic candidate selection, conversion, proposal application, review,
  approval, or export.
- Layers, brushes, semantic editing, retain-largest, or export expansion.
- Claiming that arbitrary user-approved executables are sandboxed.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/desktop && npm run build && npm test && npm audit --audit-level=high
git diff --check
```

The synthetic CC0 fake covers success, fixed candidate bytes/hash, candidate
selection without head movement, critique, proposal validation without mutation,
explicit approval refusal, non-zero exit, malformed JSON, hash mismatch, escaped
path, redaction, and cancellation. Frontend tests cover nonblocking launch and
that loading a proposal does not call the revision mutation command. All 29 Rust
tests and all three Vue tests pass; the production frontend builds, npm reports
zero known vulnerabilities, a real CLI fake-agent run emitted the typed lifecycle
and preserved `r000001` head through explicit selection, and browser-driven dark
and light WCAG 2 A/AA axe scans reported zero violations.

## Proposed next milestone

**M6 — Project recipe and selected-reference conversion handoff.** Add concise
project-owned briefs, palettes, and complete conversion recipes, then expose one
CLI/desktop use case that converts the explicitly selected reference into an
immutable revision through the existing M2 engine. Keep approval/export, richer
editing, provider SDKs, and autonomous behavior deferred.

## Review gate

M5 stops here until the parent approves the trust/protocol/state contract and M6
scope.
