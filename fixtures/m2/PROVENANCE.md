# M2 fixture provenance

All data in this directory consists of small geometric RGBA arrays, palettes,
and settings created specifically for PixelPipe's deterministic conformance
tests. They are synthetic, contain no external artwork, and are made available
under `CC0-1.0` independently of PixelPipe's source-code license.

No Last Light, Painter, Shipyard, or other third-party image material is present.
Tests encode the RGBA arrays to PNG in memory so PNG import is exercised without
checking an ambiguously licensed binary reference into the repository.

Coverage:

- `reference.rgba.json`: connected near-white backdrop, visible-bounds crop,
  palette mapping, dominant reduction, bottom registration, and transparency.
- `sheet.rgba.json`: two transparent frames of different widths for shared scale
  and baseline conformance.
- `palette.json`: fixed ordered palette with an explicit transparent index.
- `*.settings.json`: complete conversion recipes with component expectations.
