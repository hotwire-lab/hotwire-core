# `hotwire-core`

[![CI](https://github.com/hotwire-lab/hotwire-core/actions/workflows/ci.yml/badge.svg)](https://github.com/hotwire-lab/hotwire-core/actions/workflows/ci.yml)
[![HotWire Lab](https://img.shields.io/badge/Website-hotwire--lab.github.io-0ea5e9)](https://hotwire-lab.github.io/)
[![Rust + WASM](https://img.shields.io/badge/Stack-Rust_%2B_Leptos_0.7_WASM-f97316?logo=rust&logoColor=white)](https://leptos.dev/)
[![License](https://img.shields.io/badge/License-Apache_2.0-38bdf8)](LICENSE)

Shared Rust/WebAssembly foundation (`v0.7.1`) for the six **[HotWire Lab](https://hotwire-lab.github.io/)** browser hardware tools:

* **[HotWire Bench](https://github.com/hotwire-lab/bench)** ([Live](https://hotwire-lab.github.io/bench/)) - `v0.2.0+`
* **[HotWire Logic Analyzer](https://github.com/hotwire-lab/logic)** ([Live](https://hotwire-lab.github.io/logic/)) - `v1.2.1+`
* **[HotWire Oscilloscope](https://github.com/hotwire-lab/scope)** ([Live](https://hotwire-lab.github.io/scope/)) — `v1.1.1+`
* **[HotWire SDR](https://github.com/hotwire-lab/sdr)** ([Live](https://hotwire-lab.github.io/sdr/)) — `v2.1.1+`
* **[HotWire Serial Terminal](https://github.com/hotwire-lab/serial)** ([Live](https://hotwire-lab.github.io/serial/)) — `v1.1.1+` (XIAO UART Bridge mode)
* **[HotWire Signal Generator](https://github.com/hotwire-lab/siggen)** ([Live](https://hotwire-lab.github.io/siggen/)) — `v1.1.1+`

`hotwire-core` encapsulates hardware connectivity, the browser JS bridge, binary framing & firmware verification, `esptool-js` Web Flashing, and the common Leptos UI shell so each tool repository focuses purely on signal processing and visualization.

## Modules

1. **`protocol`** ([`src/protocol/`](src/protocol/))
   * Shared `0xAA 0x55` binary framing (`encode_command_8`, `encode_frame`) and XOR / CRC-8 / CRC-16 checksum helpers.
   * Zero-copy fast-path streaming [`FrameDemuxer`](src/protocol/demuxer.rs) with in-place tail compaction and optional interleaved UTF-8 text output.
   * `0x83` JSON capabilities handshake ([`BoardIdentity`](src/protocol/handshake.rs), [`FirmwareVersion`](src/protocol/handshake.rs), [`FirmwareMismatch`](src/protocol/handshake.rs)) identifying the chip (`ESP32-S3` vs `ESP32-C6`), target software, and minimum compatible firmware version (`ToolId::min_firmware_version()`).
   * Capabilities schema v2 ([`CapabilitySet`](src/protocol/handshake.rs)) with explicit `"schema": 2`, `"tool"`, snake_case common keys and legacy camelCase aliases.
2. **`transport`** ([`src/transport/`](src/transport/) + [`js/web_serial_bridge.js`](js/web_serial_bridge.js))
   * Unified browser USB transport ([`HotWireTransport`](src/transport/mod.rs)) bundling **Web Serial API** (desktop), **WebUSB API** user-space drivers (`CDC-ACM`, `FTDI`, `CP210x`, `CH34x`, `PL2303` on Android & desktop fallback), FIFO write queue serialization, Screen Wake Lock, hot-plug auto-reconnect, and a `Disconnected / Connecting / Connected / Demo` state machine with an injectable `DemoBackend` trait.
   * [`HandshakeGuard`](src/transport/mod.rs) with race-free cancellation token and a configurable timeout (HotWire apps use `5 s`) that validates incoming `0x83` frames and triggers disconnect + flash prompt on incompatible or missing firmware.
3. **`session`** ([`src/session.rs`](src/session.rs))
   * [`use_hotwire_session()`](src/session.rs) bundles transport callbacks, `0x83` handshake arming/verification, mismatch signals, demo transitions, disconnect reset and frame demultiplexing. All five apps use the same session; Serial Terminal can toggle handshake/demux per mode.
4. **`flasher`** ([`src/flasher/`](src/flasher/))
   * `esptool-js` browser flasher [`FirmwareProfile`](src/flasher/mod.rs) presets and [`ToolId::firmware_info()`](src/flasher/mod.rs) canonical filenames/hints for `XIAO ESP32-S3` and `XIAO ESP32-C6`.
   * Reusable Leptos [`FlasherPanel`](src/flasher/modal.rs) with automatic board selection and custom `.bin` flashing; apps ship a vendored `esptool-js` bundle under `public/vendor/`.
5. **`ui`** ([`src/ui/`](src/ui/) + [`styles/core.css`](styles/core.css))
   * Shared dark theme CSS ([`HotWireTheme`](src/ui/mod.rs)), `.hw-app` shell, self-hosted IBM Plex fonts and reduced-motion/focus-visible defaults.
   * Slottable top toolbar with home link (`hotwire-lab.github.io`), `Logic / Scope / Bench / SDR / Serial / Gen` tool switcher, About/Help diagnostics modal (app + core + minimum firmware versions), and firmware status pill ([`HotWireToolbar`](src/ui/toolbar.rs)).
   * Automatic [`FirmwareMismatchModal`](src/ui/mismatch_modal.rs) pop-up offering one-click jump to the Web Flasher or the matching HotWire tool.
   * Swipe-to-dismiss mobile [`BottomSheet`](src/ui/bottom_sheet.rs) and pure-Rust [`PinchPanTracker`](src/ui/gestures.rs) + cursor-anchored zoom/pan math helpers.
6. **`format` / `web`** ([`src/format.rs`](src/format.rs), [`src/web.rs`](src/web.rs))
   * Shared SI/time/frequency/voltage formatting, `localStorage` helpers, file downloads, `performance.now()` and capped high-DPI canvas helpers.

## Usage in a HotWire App

```toml
[dependencies]
# Pinned GitHub release tag (used in CI / GitHub Pages builds):
hotwire-core = { git = "https://github.com/hotwire-lab/hotwire-core", tag = "v0.7.1" }
```

For local multi-repo development, temporarily add a Cargo patch in `.cargo/config.toml`:

```toml
[patch."https://github.com/hotwire-lab/hotwire-core"]
hotwire-core = { path = "../hotwire-core" }
```

## Verification

```bash
rustup target add wasm32-unknown-unknown
cargo fmt --all -- --check
cargo test
cargo test --no-default-features
cargo clippy --all-targets -- -D warnings
cargo clippy --target wasm32-unknown-unknown -- -D warnings
node --check js/web_serial_bridge.js
```

## License

Copyright © 2026 Kari Vierimaa, Kempele — Licensed under the [Apache License 2.0](LICENSE).




