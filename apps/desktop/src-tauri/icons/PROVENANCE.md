# Desktop icon provenance

The complete Pixelate macOS icon was created and provided by Alexander
Heffernan on 2026-08-27 as `icon-source.png`. The source already contains its
background, shape, spacing, and dark appearance.

The desktop PNG, ICNS, and ICO variants are generated directly from that source
with the Tauri icon tool. Do not add transparent padding: macOS applies its own
container to padded icons, making the complete Pixelate icon appear inset twice.

`Pixelate.icon` packages the same artwork in Apple's native Icon Composer format
for macOS 26 and later. Keep `icon.icns` as the fallback for earlier versions.
