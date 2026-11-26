# HotWire Design System

## Layout

- Every app root uses `class="... hw-app"` (in addition to its instrument class).
- `.hw-app` provides `100dvh`, flex column layout, overflow handling, safe-area
  padding, font inheritance and focus-visible styling.
- Shared theme variables are defined in `hotwire-core/styles/core.css`:
  `--bg`, `--surface`, `--surface-2`, `--border`, `--text`, `--muted`,
  `--accent`, `--ok`, `--warn`, `--danger`, `--font-sans`, `--font-mono`.

## Typography

IBM Plex Sans (400/500/600/700) and IBM Plex Mono (400/500/600) are self-hosted
as `woff2` under each app's `public/fonts/`. Do not add Google Fonts links.

## Shared components

- `HotWireToolbar`: brand, tool switcher, status pill, connection actions,
  Flasher button, About/Help modal.
- `BottomSheet`: mobile flasher/settings drawer.
- `FirmwareMismatchModal`: mismatch + one-click flash prompt.
- `.hw-btn` is the canonical button class; `.btn` remains as a compatibility
  alias for older app code.
- `.hw-btn-primary`, `.hw-btn-accent`, `.hw-btn-danger`, `.hw-btn-warn`,
  `.hw-btn-ghost`.

## Breakpoints

Use the shared breakpoints:

- `720px` — desktop/mobile split (toolbar, drawers, control bars)
- `480px` — compact phones
- `max-height: 500px` + `orientation: landscape` — bench/laptop landscape

## Canvas rendering

- Use `hotwire_core::web::capped_device_pixel_ratio()`; do not read
  `devicePixelRatio` directly. The cap is 2.5× to bound memory/fill cost.
- Coalesce draws with `requestAnimationFrame`.
- Reuse buffers in hot paths; avoid per-frame `String`/`Vec` clones.

## PWA assets

Each app ships:

- `manifest.webmanifest` with a unique `short_name`:
  `HotWire Logic`, `HotWire Scope`, `HotWire SDR`, `HotWire Serial`,
  `HotWire Gen`.
- `icon.svg`, `icon-180.png` (Apple touch), `icon-192.png`, `icon-512.png`.
- Canonical URL, Open Graph, Twitter card and JSON-LD SoftwareApplication.
- Service worker with a versioned cache name and offline app shell.
- Self-hosted fonts and a vendored `esptool-js` bundle under `public/vendor/`.

## Accessibility

- Minimum 44 px touch targets on mobile.
- Do not rely on colour alone for state; pair with text/icon.
- `:focus-visible` is provided by `.hw-app`.
- Honor `prefers-reduced-motion` for non-essential animation.
