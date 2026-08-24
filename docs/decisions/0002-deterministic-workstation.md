# ADR 0002: Deterministic workstation as the primary product surface

Status: Accepted, 2026-08-24

## Context

The staged brief → embedded agent → reference → conversion wizard proved the
application boundaries, but it hid PixelPipe's strongest capability and made a
simple image conversion feel like setup work. The useful daily task is tuning a
high-resolution image into readable game art, then correcting specific indexed
pixels. Coding agents can already create and import references through the same
application and CLI boundary without occupying the desktop's primary UI.

## Decision

- Open a game folder directly; initialize `.pixelpipe` through the application
  boundary when needed.
- Use a native-style shell with the project name in the titlebar, independently
  collapsible translucent sidebars, restrained cool accents, and an opaque
  neutral canvas for reliable colour judgement.
- Show asset names and pixelated thumbnails in the left sidebar.
- Show one large fitted nearest-neighbour pixel canvas. Native dimensions remain
  metadata, not a permanent duplicate view.
- In Convert mode, update deterministic previews after a short debounce. Control
  changes do not create revisions. Entering Edit freezes the current recipe as
  the immutable editing base when no head exists.
- Keep Create Asset narrow: a user can choose an image or create an awaiting-
  reference asset for a coding agent. A missing optional brief defaults to the
  asset name so setup cannot dead-end conversion.
- Keep coding-agent interaction CLI-first. Embedded adapters remain supported
  infrastructure, not the workstation's main navigation.
- Preserve UI/CLI parity: references can be imported and selected-reference
  conversions can be previewed or committed with explicit settings through the
  same application use cases.

## Consequences

This removes the staged wizard and dual native/enlarged review layout. The first
slice provides the native shell, asset browser, live conversion controls, and
revision checkpoint. Direct import of existing indexed pixel art, project-
default/asset-override persistence, pencil/fill tools, and export controls remain
separate follow-up slices. The UI must state those limits rather than presenting
unfinished controls as working features.
