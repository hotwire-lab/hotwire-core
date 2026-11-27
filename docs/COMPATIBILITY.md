# HotWire Protocol & Compatibility

## Wire framing

All HotWire firmware uses the same framing:

```
[0xAA, 0x55, type_or_cmd, len_lo, len_hi, payload..., xor]
```

- Fixed host commands are 8 bytes: `[0xAA, 0x55, cmd, p1, p2, p3, p4, xor]`.
- `xor` is the XOR of every preceding byte.
- Shared helpers live in `hotwire-core/src/protocol/framing.rs` and `firmware/*/hotwire_protocol.h`.

## Shared commands and types

| ID | Name | Direction | Notes |
|---|---|---|---|
| `0x01` | `CMD_PING_STATUS` | Host → device | Returns status/capabilities |
| `0x02` | `CMD_GET_CAPS` | Host → device | Returns `0x83` JSON |
| `0xF0` | `CMD_BOOTLOADER` | Host → device | Requires magic `B0 07 10 AD` |
| `0x83` | `TYPE_BOARD_CAPABILITIES` | Device → host | JSON capabilities |
| `0x87` | `TYPE_UART_RX` | Device → host | Serial Terminal bridge RX |

Tool-specific commands (scope, logic, SDR, SigGen, UART config/TX) are documented in each app's `protocol` module.

## Capabilities schema v2

Firmware emits a JSON object in `0x83`:

```json
{
  "schema": 2,
  "tool": "scope",
  "board": "XIAO-ESP32-S3",
  "firmware": "1.1.1-scope",
  "max_channels": 4,
  "max_rate_hz": 1000000,
  "adc_bits": 12
}
```

Rules:

- `schema` is `2` for current firmware; missing/`0` means legacy.
- `tool` is one of `logic`, `scope`, `sdr`, `serial`, `siggen`.
- Common keys use `snake_case`. Parsers also accept legacy camelCase (`maxRateHz`, `maxDepth`).
- Unknown keys are preserved in `BoardIdentity::raw` and must be ignored by older apps.
- Apps must clamp UI controls to the values reported by the connected board.

`hotwire-core::protocol::BoardIdentity` exposes:

- `schema`, `declared_tool`, `detected_tool`
- `is_current_schema()`
- `capabilities() -> CapabilitySet`
- `get_u64_any(&["max_rate_hz", "maxRateHz"])`

## Tool / firmware matrix

| Tool | Slug | Firmware min | Bundled sketches |
|---|---|---|---|
| Logic Analyzer | `logic` | `1.2.1` | `HotWire_XIAO_ESP32S3.ino`, `...C6.ino` |
| Oscilloscope | `scope` | `1.1.1` | `HotWire_Scope_XIAO_ESP32S3.ino`, `...C6.ino` |
| SDR | `sdr` | `2.1.1` | `HotWire_SDR_XIAO_ESP32S3.ino`, `...C6.ino` |
| Serial Terminal | `serial` | `1.1.1` | `HotWire_UART_XIAO_ESP32S3.ino`, `...C6.ino` |
| Signal Generator | `siggen` | `1.1.1` | `HotWire_SigGen_XIAO_ESP32S3.ino`, `...C6.ino` |

Minimum versions are defined once in `ToolId::min_firmware_version()`.

## Bootloader entry

`0xF0` is only honoured when `p1..p4 == B0 07 10 AD`. This prevents a stray
command from rebooting an active capture/sweep. The shared helper is
`hotwire_is_bootloader_magic()` in `hotwire_protocol.h`.

## Connection handshake

The shared `use_hotwire_session()` hook:

1. Arms a 5 s timeout after a physical USB connection.
2. Verifies `tool` and `firmware >= ToolId::min_firmware_version()`.
3. Disconnects and raises `FirmwareMismatch` on mismatch.
4. Clears identity and calls the app reset callback on disconnect.
5. Accepts Demo-mode capabilities without version enforcement.

Serial Terminal direct-USB mode disables both demuxing and the handshake so raw
terminal bytes are never interpreted as HotWire frames.

## Backward compatibility

- Apps must keep accepting v0/v1 capabilities payloads.
- Firmware should keep emitting legacy keys for one release after a schema change.
- Removing or renaming a command/type requires a firmware major/minor bump and a
  corresponding `ToolId::min_firmware_version()` update.

## Known limitations (2026-11)

- **PL2303:** WebUSB bulk transfer and default line coding work, but
  vendor-specific PL2303 control requests are not implemented. Prefer
  CDC-ACM/FTDI/CP210x/CH34x adapters when possible.
- **ANSI CSI sequences:** sequences split exactly across USB chunk boundaries
  may render a raw fragment until the next chunk arrives.
- **Serial viewport:** the DOM renders the newest 256 KB of formatted output;
  the full ring buffer is retained for export.
- **Settings persistence:** Serial Terminal and Logic channel labels persist
  across reloads; Scope, SDR and SigGen currently keep their instrument
  settings for the session only.
- **WASM optimisation:** Trunk `wasm-opt` is disabled (`data-wasm-opt="0"`)
  because binaryen 131 rejects Rust 1.100 bulk-memory output. Bundles are
  slightly larger but builds are reproducible across local and CI toolchains.
- **Firmware long stalls:** a host stall longer than the firmware TX buffer
  window may truncate a streaming frame; reconnect to resynchronise.