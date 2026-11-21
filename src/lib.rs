//! `hotwire-core` — Shared Rust/WASM foundation for the HotWire browser hardware tool suite
//! (Logic Analyzer, Oscilloscope, SDR, Serial Terminal).
//!
//! Encapsulates:
//! - [`protocol`]: Shared binary framing (`0xAA 0x55`), XOR/CRC checksums, streaming demuxer,
//!   and `0x83` capabilities handshake with chip & firmware mismatch detection.
//! - [`transport`]: Unified Web Serial + WebUSB (CDC-ACM, FTDI, CP210x, CH34x, PL2303) transport
//!   and `Disconnected / Connecting / Connected / Demo` state machine.
//! - [`flasher`]: `esptool-js` Web Flasher profiles and ready-to-use Leptos flasher panel.
//! - [`ui`]: Shared dark theme CSS, slottable top toolbar with tool switcher, mobile draggable
//!   bottom sheet, and pinch-to-zoom / pan gesture math helpers.

pub mod flasher;
pub mod protocol;
pub mod transport;
pub mod ui;
