# `hotwire-core`

Shared Rust/WebAssembly foundation for the four HotWire browser hardware tools:
* **HotWire Logic Analyzer**
* **HotWire Oscilloscope**
* **HotWire SDR**
* **HotWire Serial Terminal**

## Modules

1. **`protocol`** — Shared `0xAA 0x55` binary framing (`encode_command_8`, `encode_frame`), XOR / CRC-8 / CRC-16 helpers, streaming [`FrameDemuxer`](src/protocol/demuxer.rs) with optional interleaved UTF-8 text output, and `0x83` capabilities handshake ([`BoardIdentity`](src/protocol/handshake.rs)) with ESP32-S3 / ESP32-C6 chip identification and firmware mismatch detection.
2. **`transport`** — Unified browser USB transport ([`HotWireTransport`](src/transport/mod.rs)) bundling [`js/web_serial_bridge.js`](js/web_serial_bridge.js) with **Web Serial API** (desktop), **WebUSB API** user-space drivers (`CDC-ACM`, `FTDI`, `CP210x`, `CH34x`, `PL2303` on Android & desktop fallback), Wake Lock, auto-reconnect, and a `Disconnected / Connecting / Connected / Demo` state machine with an injectable `DemoBackend` trait.
3. **`flasher`** — `esptool-js` browser flasher [`FirmwareProfile`](src/flasher/mod.rs) definitions and the reusable Leptos [`FlasherPanel`](src/flasher/modal.rs) component.
4. **`ui`** — Shared dark theme CSS ([`HotWireTheme`](src/ui/mod.rs)), slottable top toolbar with `Logic / Scope / SDR / Serial` tool switcher ([`HotWireToolbar`](src/ui/toolbar.rs)), swipe-to-dismiss mobile [`BottomSheet`](src/ui/bottom_sheet.rs), and pure-Rust touch [`PinchPanTracker`](src/ui/gestures.rs) + anchored zoom/pan math helpers.

## Usage in a HotWire App

```toml
[dependencies]
# During local workspace development:
hotwire-core = { path = "../hotwire-core" }

# After publishing hotwire-core to GitHub (recommended for GitHub Actions deploys):
# hotwire-core = { git = "https://github.com/kavierim/hotwire-core", tag = "v0.1.0" }
```

## Verification

```bash
cargo test
cargo test --no-default-features
cargo clippy --all-targets -- -D warnings
cargo check --target wasm32-unknown-unknown
```

## License

Copyright © 2026 Kari Vierimaa, Kempele — Licensed under the [Apache License 2.0](LICENSE).
