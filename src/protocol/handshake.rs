use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parsed semantic version (`major.minor.patch`) extracted from a HotWire firmware string
/// such as `"1.2.0"`, `"1.1.1-scope-c6"`, or `"2.1.1-sdr"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FirmwareVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl FirmwareVersion {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parses leading numeric `major.minor[.patch]` components, ignoring an optional
    /// leading `'v'`/`'V'` and any pre-release/target suffix after `'-'` or `'+'`.
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim().trim_start_matches(['v', 'V']);
        let core = trimmed
            .split(['-', '+', ' '])
            .next()
            .filter(|s| !s.is_empty())?;
        let mut parts = core.split('.');
        let major = parts.next()?.parse::<u16>().ok()?;
        let minor = parts.next()?.parse::<u16>().ok()?;
        let patch = match parts.next() {
            Some(p) => p.parse::<u16>().ok()?,
            None => 0,
        };
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for FirmwareVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Canonical landing page URL for the HotWire Lab web suite.
pub const HOTWIRE_HOME_URL: &str = "https://hotwire-lab.github.io/";

/// Current capabilities JSON schema version emitted by HotWire firmware.
///
/// Schema v2 adds explicit `"schema": 2` and `"tool": "<slug>"` fields plus
/// snake_case capability keys. Parsers must keep accepting v0/v1 payloads that
/// omit these fields and use camelCase keys (Logic Analyzer historically used
/// `maxRateHz` / `maxDepth`).
pub const CAPS_SCHEMA_VERSION: u64 = 2;

/// Identifies one of the six HotWire browser tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolId {
    LogicAnalyzer,
    Oscilloscope,
    Bench,
    Sdr,
    SerialTerminal,
    SignalGenerator,
}

impl ToolId {
    pub const ALL: [ToolId; 6] = [
        ToolId::LogicAnalyzer,
        ToolId::Oscilloscope,
        ToolId::Bench,
        ToolId::Sdr,
        ToolId::SerialTerminal,
        ToolId::SignalGenerator,
    ];

    pub const fn slug(self) -> &'static str {
        match self {
            ToolId::LogicAnalyzer => "logic",
            ToolId::Oscilloscope => "scope",
            ToolId::Bench => "bench",
            ToolId::Sdr => "sdr",
            ToolId::SerialTerminal => "serial",
            ToolId::SignalGenerator => "siggen",
        }
    }

    /// Parses a tool slug from a capabilities payload (`"tool": "scope"`).
    /// Accepts repo names and common aliases for backwards compatibility.
    pub fn from_slug(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "logic" | "logic-analyzer" | "logic_analyzer" | "la" => Some(ToolId::LogicAnalyzer),
            "scope" | "oscilloscope" => Some(ToolId::Oscilloscope),
            "bench" | "mixed-signal" | "mixed_signal" | "mixed-signal-bench" => {
                Some(ToolId::Bench)
            }
            "sdr" | "software-defined-radio" | "software_defined_radio" => Some(ToolId::Sdr),
            "serial" | "serial-terminal" | "serial_terminal" | "uart" | "uart-bridge" => {
                Some(ToolId::SerialTerminal)
            }
            "siggen" | "signal-generator" | "signal_generator" | "gen" => {
                Some(ToolId::SignalGenerator)
            }
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            ToolId::LogicAnalyzer => "HotWire Logic Analyzer",
            ToolId::Oscilloscope => "HotWire Oscilloscope",
            ToolId::Bench => "HotWire Bench",
            ToolId::Sdr => "HotWire SDR",
            ToolId::SerialTerminal => "HotWire Serial Terminal",
            ToolId::SignalGenerator => "HotWire Signal Generator",
        }
    }

    pub const fn short_label(self) -> &'static str {
        match self {
            ToolId::LogicAnalyzer => "Logic",
            ToolId::Oscilloscope => "Scope",
            ToolId::Bench => "Bench",
            ToolId::Sdr => "SDR",
            ToolId::SerialTerminal => "Serial",
            ToolId::SignalGenerator => "Gen",
        }
    }

    pub const fn live_url(self) -> &'static str {
        match self {
            ToolId::LogicAnalyzer => "https://hotwire-lab.github.io/logic/",
            ToolId::Oscilloscope => "https://hotwire-lab.github.io/scope/",
            ToolId::Bench => "https://hotwire-lab.github.io/bench/",
            ToolId::Sdr => "https://hotwire-lab.github.io/sdr/",
            ToolId::SerialTerminal => "https://hotwire-lab.github.io/serial/",
            ToolId::SignalGenerator => "https://hotwire-lab.github.io/siggen/",
        }
    }

    pub const fn repo_url(self) -> &'static str {
        match self {
            ToolId::LogicAnalyzer => "https://github.com/hotwire-lab/logic",
            ToolId::Oscilloscope => "https://github.com/hotwire-lab/scope",
            ToolId::Bench => "https://github.com/hotwire-lab/bench",
            ToolId::Sdr => "https://github.com/hotwire-lab/sdr",
            ToolId::SerialTerminal => "https://github.com/hotwire-lab/serial",
            ToolId::SignalGenerator => "https://github.com/hotwire-lab/siggen",
        }
    }

    /// Exact dropdown label matching `.github/ISSUE_TEMPLATE/bug_report.yml`.
    pub const fn issue_tool_option(self) -> &'static str {
        match self {
            ToolId::LogicAnalyzer => "Logic Analyzer",
            ToolId::Oscilloscope => "Oscilloscope",
            ToolId::Bench => "Bench",
            ToolId::Sdr => "SDR",
            ToolId::SerialTerminal => "Serial Terminal",
            ToolId::SignalGenerator => "Signal Generator",
        }
    }

    pub const fn accent_color(self) -> &'static str {
        match self {
            ToolId::LogicAnalyzer => "#58a6ff",
            ToolId::Oscilloscope => "#facc15",
            ToolId::Bench => "#2dd4bf",
            ToolId::Sdr => "#a855f7",
            ToolId::SerialTerminal => "#4ade80",
            ToolId::SignalGenerator => "#fb7185",
        }
    }

    /// Minimum compatible ESP32 firmware version required by each HotWire web app.
    pub const fn min_firmware_version(self) -> FirmwareVersion {
        match self {
            ToolId::LogicAnalyzer => FirmwareVersion::new(1, 2, 1),
            ToolId::Oscilloscope => FirmwareVersion::new(1, 1, 1),
            ToolId::Bench => FirmwareVersion::new(0, 1, 0),
            ToolId::Sdr => FirmwareVersion::new(2, 1, 1),
            ToolId::SerialTerminal => FirmwareVersion::new(1, 1, 1),
            ToolId::SignalGenerator => FirmwareVersion::new(1, 1, 1),
        }
    }
}

/// Detected microcontroller family from the `0x83` capabilities handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BoardChip {
    Esp32S3,
    Esp32C6,
    Unknown,
}

impl BoardChip {
    pub fn from_board_name(board: &str) -> Self {
        let up = board.to_uppercase();
        if up.contains("C6") {
            BoardChip::Esp32C6
        } else if up.contains("S3") {
            BoardChip::Esp32S3
        } else {
            BoardChip::Unknown
        }
    }

    pub const fn id(self) -> &'static str {
        match self {
            BoardChip::Esp32S3 => "esp32s3",
            BoardChip::Esp32C6 => "esp32c6",
            BoardChip::Unknown => "unknown",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            BoardChip::Esp32S3 => "XIAO ESP32-S3",
            BoardChip::Esp32C6 => "XIAO ESP32-C6",
            BoardChip::Unknown => "Unknown ESP32",
        }
    }

    /// Exact dropdown label matching `.github/ISSUE_TEMPLATE/bug_report.yml`.
    pub const fn issue_board_option(self) -> &'static str {
        match self {
            BoardChip::Esp32S3 => "Seeed Studio XIAO ESP32-S3",
            BoardChip::Esp32C6 => "Seeed Studio XIAO ESP32-C6",
            BoardChip::Unknown => "Other ESP32 / USB-UART adapter",
        }
    }
}

/// Reason why a connected USB board failed HotWire firmware compatibility verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirmwareMismatchKind {
    /// Device responded with `0x83` capabilities for a different HotWire tool or unknown SW.
    WrongTargetSw { detected_tool: Option<ToolId> },
    /// Target SW matched, but the reported firmware version is missing or below `required`.
    VersionTooOld {
        found: Option<FirmwareVersion>,
        required: FirmwareVersion,
    },
    /// Device did not reply to `CMD_GET_CAPS` (`0x02`) within the handshake timeout.
    NoHandshakeResponse,
}

/// Structured mismatch report used to auto-disconnect and display the Flash prompt pop-up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareMismatch {
    pub expected_tool: ToolId,
    pub required_version: FirmwareVersion,
    pub board: String,
    pub chip: BoardChip,
    pub firmware_raw: String,
    pub detected_version: Option<FirmwareVersion>,
    pub kind: FirmwareMismatchKind,
}

impl FirmwareMismatch {
    pub fn no_handshake(
        expected_tool: ToolId,
        required_version: FirmwareVersion,
        device_name: impl Into<String>,
    ) -> Self {
        let board = device_name.into();
        let chip = BoardChip::from_board_name(&board);
        Self {
            expected_tool,
            required_version,
            board,
            chip,
            firmware_raw: "No response".to_string(),
            detected_version: None,
            kind: FirmwareMismatchKind::NoHandshakeResponse,
        }
    }

    pub fn detected_tool(&self) -> Option<ToolId> {
        match self.kind {
            FirmwareMismatchKind::WrongTargetSw { detected_tool } => detected_tool,
            FirmwareMismatchKind::VersionTooOld { .. } => Some(self.expected_tool),
            FirmwareMismatchKind::NoHandshakeResponse => None,
        }
    }

    pub fn title(&self) -> &'static str {
        match self.kind {
            FirmwareMismatchKind::WrongTargetSw { .. } => "Wrong Firmware Detected",
            FirmwareMismatchKind::VersionTooOld { .. } => "Firmware Update Required",
            FirmwareMismatchKind::NoHandshakeResponse => "Compatible Firmware Not Found",
        }
    }

    pub fn summary(&self) -> String {
        match self.kind {
            FirmwareMismatchKind::WrongTargetSw {
                detected_tool: Some(other),
            } => format!(
                "{} is running {} ({}), but this app requires {} (v{}+).",
                self.board,
                other.label(),
                self.firmware_raw,
                self.expected_tool.label(),
                self.required_version,
            ),
            FirmwareMismatchKind::WrongTargetSw {
                detected_tool: None,
            } => format!(
                "{} reported unrecognized firmware ({}). Flash {} (v{}+) to use this tool.",
                self.board,
                self.firmware_raw,
                self.expected_tool.label(),
                self.required_version,
            ),
            FirmwareMismatchKind::VersionTooOld {
                found: Some(v),
                required,
            } => format!(
                "{} is running {} v{} ({}), which is older than the required v{}.",
                self.board,
                self.expected_tool.label(),
                v,
                self.firmware_raw,
                required,
            ),
            FirmwareMismatchKind::VersionTooOld {
                found: None,
                required,
            } => format!(
                "{} reported an unknown firmware version ({}). Please flash {} v{}+.",
                self.board,
                self.firmware_raw,
                self.expected_tool.label(),
                required,
            ),
            FirmwareMismatchKind::NoHandshakeResponse => format!(
                "The connected USB device ({}) did not respond to the HotWire capabilities handshake. Flash {} (v{}+) onto your XIAO ESP32-S3 or ESP32-C6.",
                self.board,
                self.expected_tool.label(),
                self.required_version,
            ),
        }
    }
}

/// Unified device identity parsed from a `0x83` (`TYPE_BOARD_CAPABILITIES`) JSON payload.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardIdentity {
    /// Capabilities schema version (`0` for legacy payloads without `"schema"`).
    pub schema: u64,
    pub board: String,
    pub chip: BoardChip,
    pub firmware: String,
    pub version: Option<FirmwareVersion>,
    /// Tool declared explicitly by `"tool"` in schema v2, if present.
    pub declared_tool: Option<ToolId>,
    /// Resolved tool: `declared_tool` when present, otherwise inferred from flags/keys.
    pub detected_tool: Option<ToolId>,
    pub raw: Value,
}

/// Typed view over the common capability keys shared by all HotWire firmware.
///
/// Tool-specific keys remain available through [`BoardIdentity::get_u64`] and
/// [`BoardIdentity::get_str`]; this struct centralises the fields that are
/// commonly needed for capability gating in the web apps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CapabilitySet {
    pub schema: u64,
    pub declared_tool: Option<ToolId>,
    pub channels: Option<u64>,
    pub max_channels: Option<u64>,
    pub max_rate_hz: Option<u64>,
    pub max_depth: Option<u64>,
    pub max_iq_kb: Option<u64>,
    pub max_baud: Option<u64>,
    pub adc_bits: Option<u64>,
    pub pattern_channels: Option<u64>,
    pub max_pwm_hz: Option<u64>,
    pub dds_rate_hz: Option<u64>,
}

impl BoardIdentity {
    /// Decodes a `0x83` JSON capabilities payload into a unified [`BoardIdentity`].
    pub fn decode_capabilities(payload: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(payload).ok()?;
        let raw: Value = serde_json::from_str(text).ok()?;
        if !raw.is_object() {
            return None;
        }

        let board = raw
            .get("board")
            .and_then(Value::as_str)
            .unwrap_or("XIAO-ESP32-S3")
            .to_string();
        let chip = BoardChip::from_board_name(&board);
        let firmware = raw
            .get("firmware")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let version = FirmwareVersion::parse(&firmware);
        let schema = raw.get("schema").and_then(Value::as_u64).unwrap_or(0);
        let declared_tool = raw
            .get("tool")
            .and_then(Value::as_str)
            .and_then(ToolId::from_slug);
        let detected_tool = declared_tool.or_else(|| infer_tool(&raw, &firmware));

        Some(Self {
            schema,
            board,
            chip,
            firmware,
            version,
            declared_tool,
            detected_tool,
            raw,
        })
    }

    /// Returns `true` if the connected firmware matches `expected`, or if the
    /// firmware did not report enough metadata to determine a mismatch.
    pub fn matches_tool(&self, expected: ToolId) -> bool {
        match self.detected_tool {
            Some(found) => found == expected,
            None => true,
        }
    }

    /// Strictly verifies BOTH target software (`expected_tool`) and minimum firmware version.
    /// Returns `Err(FirmwareMismatch)` if the target SW does not match or the version is too old / missing.
    pub fn validate_for_tool(
        &self,
        expected_tool: ToolId,
        min_version: FirmwareVersion,
    ) -> Result<(), FirmwareMismatch> {
        if self.detected_tool != Some(expected_tool) {
            return Err(FirmwareMismatch {
                expected_tool,
                required_version: min_version,
                board: self.board.clone(),
                chip: self.chip,
                firmware_raw: self.firmware.clone(),
                detected_version: self.version,
                kind: FirmwareMismatchKind::WrongTargetSw {
                    detected_tool: self.detected_tool,
                },
            });
        }

        match self.version {
            Some(v) if v >= min_version => Ok(()),
            found => Err(FirmwareMismatch {
                expected_tool,
                required_version: min_version,
                board: self.board.clone(),
                chip: self.chip,
                firmware_raw: self.firmware.clone(),
                detected_version: found,
                kind: FirmwareMismatchKind::VersionTooOld {
                    found,
                    required: min_version,
                },
            }),
        }
    }

    /// Validates against `expected_tool` and its default [`ToolId::min_firmware_version`].
    pub fn validate(&self, expected_tool: ToolId) -> Result<(), FirmwareMismatch> {
        self.validate_for_tool(expected_tool, expected_tool.min_firmware_version())
    }

    /// Convenience accessor for reading a boolean field from the raw JSON payload.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.raw.get(key).and_then(Value::as_bool)
    }

    /// Convenience accessor for reading a `u64` field from the raw JSON payload.
    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.raw.get(key).and_then(Value::as_u64)
    }

    /// Convenience accessor for reading a string field from the raw JSON payload.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.raw.get(key).and_then(Value::as_str)
    }

    /// Reads the first present unsigned integer from a list of key aliases.
    pub fn get_u64_any(&self, keys: &[&str]) -> Option<u64> {
        keys.iter().find_map(|key| self.get_u64(key))
    }

    /// Returns `true` when the firmware emits the current [`CAPS_SCHEMA_VERSION`].
    pub fn is_current_schema(&self) -> bool {
        self.schema >= CAPS_SCHEMA_VERSION
    }

    /// Typed view over the common capability keys, with camelCase aliases for
    /// legacy payloads. Tool-specific fields stay in [`Self::raw`].
    pub fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            schema: self.schema,
            declared_tool: self.declared_tool,
            channels: self.get_u64_any(&["channels"]),
            max_channels: self.get_u64_any(&["max_channels", "maxChannels"]),
            max_rate_hz: self.get_u64_any(&["max_rate_hz", "maxRateHz"]),
            max_depth: self.get_u64_any(&["max_depth", "maxDepth"]),
            max_iq_kb: self.get_u64_any(&["max_iq_kb", "maxIqKb"]),
            max_baud: self.get_u64_any(&["max_baud", "maxBaud"]),
            adc_bits: self.get_u64_any(&["adc_bits", "adcBits"]),
            pattern_channels: self.get_u64_any(&["pattern_channels", "patternChannels"]),
            max_pwm_hz: self.get_u64_any(&["max_pwm_hz", "maxPwmHz"]),
            dds_rate_hz: self.get_u64_any(&["dds_rate_hz", "ddsRateHz"]),
        }
    }
}

fn infer_tool(raw: &Value, firmware: &str) -> Option<ToolId> {
    if let Some(tool) = raw
        .get("tool")
        .and_then(Value::as_str)
        .and_then(ToolId::from_slug)
    {
        return Some(tool);
    }
    if raw.get("bench").and_then(Value::as_bool) == Some(true) {
        return Some(ToolId::Bench);
    }
    if raw.get("siggen").and_then(Value::as_bool) == Some(true) {
        return Some(ToolId::SignalGenerator);
    }
    if raw.get("scope").and_then(Value::as_bool) == Some(true) {
        return Some(ToolId::Oscilloscope);
    }
    if raw.get("uart_bridge").and_then(Value::as_bool) == Some(true) {
        return Some(ToolId::SerialTerminal);
    }
    if raw.get("sdr").and_then(Value::as_bool) == Some(true)
        || raw.get("min_freq_mhz").is_some()
        || raw.get("minFreqMhz").is_some()
    {
        return Some(ToolId::Sdr);
    }
    if raw.get("channels").and_then(Value::as_u64) == Some(8) {
        return Some(ToolId::LogicAnalyzer);
    }

    let fw_lower = firmware.to_lowercase();
    if fw_lower.contains("siggen") {
        Some(ToolId::SignalGenerator)
    } else if fw_lower.contains("bench") {
        Some(ToolId::Bench)
    } else if fw_lower.contains("scope") {
        Some(ToolId::Oscilloscope)
    } else if fw_lower.contains("sdr") {
        Some(ToolId::Sdr)
    } else if fw_lower.contains("uart") || fw_lower.contains("serial") {
        Some(ToolId::SerialTerminal)
    } else if fw_lower.contains("logic") || fw_lower.contains("la-") {
        Some(ToolId::LogicAnalyzer)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_firmware_versions_with_suffixes() {
        assert_eq!(
            FirmwareVersion::parse("1.2.0"),
            Some(FirmwareVersion::new(1, 2, 0))
        );
        assert_eq!(
            FirmwareVersion::parse("v1.0.0-scope-c6"),
            Some(FirmwareVersion::new(1, 0, 0))
        );
        assert_eq!(
            FirmwareVersion::parse("2.0.0-sdr"),
            Some(FirmwareVersion::new(2, 0, 0))
        );
        assert_eq!(
            FirmwareVersion::parse("1.0.0-uart"),
            Some(FirmwareVersion::new(1, 0, 0))
        );
        assert_eq!(FirmwareVersion::parse("unknown"), None);
    }

    #[test]
    fn validates_real_firmware_payloads_for_all_four_tools() {
        let scope = BoardIdentity::decode_capabilities(
            br#"{"board":"XIAO-ESP32-C6","firmware":"1.1.1-scope-c6","scope":true,"max_channels":3}"#,
        )
        .unwrap();
        assert_eq!(scope.chip, BoardChip::Esp32C6);
        assert!(scope.validate(ToolId::Oscilloscope).is_ok());

        let sdr = BoardIdentity::decode_capabilities(
            br#"{"board":"XIAO-ESP32-S3","firmware":"2.1.1-sdr","sdr":true}"#,
        )
        .unwrap();
        assert!(sdr.validate(ToolId::Sdr).is_ok());

        let la = BoardIdentity::decode_capabilities(
            br#"{"board":"XIAO-ESP32-S3","firmware":"1.2.1","channels":8,"maxRateHz":20000000}"#,
        )
        .unwrap();
        assert!(la.validate(ToolId::LogicAnalyzer).is_ok());

        let uart = BoardIdentity::decode_capabilities(
            br#"{"board":"XIAO-ESP32-S3","firmware":"1.1.1-uart","uart_bridge":true,"tx_pin":"D6","rx_pin":"D7"}"#,
        )
        .unwrap();
        assert!(uart.validate(ToolId::SerialTerminal).is_ok());

        let siggen = BoardIdentity::decode_capabilities(
            br#"{"board":"XIAO-ESP32-S3","firmware":"1.1.1-siggen","siggen":true,"max_channels":2}"#,
        )
        .unwrap();
        assert!(siggen.validate(ToolId::SignalGenerator).is_ok());

        let bench = BoardIdentity::decode_capabilities(
            br#"{"schema":2,"tool":"bench","bench":true,"board":"XIAO-ESP32-S3","firmware":"0.1.0-bench","channels":8,"analogChannels":2,"max_rate_hz":1000000}"#,
        )
        .unwrap();
        assert_eq!(bench.chip, BoardChip::Esp32S3);
        assert_eq!(bench.declared_tool, Some(ToolId::Bench));
        assert!(bench.validate(ToolId::Bench).is_ok());
        assert_eq!(ToolId::from_slug("mixed-signal"), Some(ToolId::Bench));
    }

    #[test]
    fn parses_v2_schema_and_explicit_tool() {
        let id = BoardIdentity::decode_capabilities(
            br#"{"schema":2,"tool":"scope","board":"XIAO-ESP32-C6","firmware":"1.1.1-scope-c6","scope":true,"max_channels":3,"max_rate_hz":83333,"adc_bits":12}"#,
        )
        .unwrap();
        assert_eq!(id.schema, CAPS_SCHEMA_VERSION);
        assert!(id.is_current_schema());
        assert_eq!(id.declared_tool, Some(ToolId::Oscilloscope));
        assert_eq!(id.detected_tool, Some(ToolId::Oscilloscope));
        let caps = id.capabilities();
        assert_eq!(caps.max_channels, Some(3));
        assert_eq!(caps.max_rate_hz, Some(83_333));
        assert_eq!(caps.adc_bits, Some(12));
        assert_eq!(caps.max_depth, None);
    }

    #[test]
    fn accepts_legacy_camel_case_capability_aliases() {
        let id = BoardIdentity::decode_capabilities(
            br#"{"board":"XIAO-ESP32-S3","firmware":"1.2.1","channels":8,"maxRateHz":20000000,"maxDepth":32768}"#,
        )
        .unwrap();
        assert_eq!(id.schema, 0);
        assert!(!id.is_current_schema());
        assert_eq!(id.detected_tool, Some(ToolId::LogicAnalyzer));
        let caps = id.capabilities();
        assert_eq!(caps.channels, Some(8));
        assert_eq!(caps.max_rate_hz, Some(20_000_000));
        assert_eq!(caps.max_depth, Some(32_768));
    }

    #[test]
    fn parses_tool_slugs_and_aliases() {
        assert_eq!(ToolId::from_slug("logic"), Some(ToolId::LogicAnalyzer));
        assert_eq!(ToolId::from_slug("SCOPE"), Some(ToolId::Oscilloscope));
        assert_eq!(
            ToolId::from_slug("serial-terminal"),
            Some(ToolId::SerialTerminal)
        );
        assert_eq!(
            ToolId::from_slug("signal_generator"),
            Some(ToolId::SignalGenerator)
        );
        assert_eq!(ToolId::from_slug("unknown"), None);
    }

    #[test]
    fn rejects_wrong_target_sw_and_outdated_versions() {
        let sdr = BoardIdentity::decode_capabilities(
            br#"{"board":"XIAO-ESP32-S3","firmware":"2.1.1-sdr","sdr":true}"#,
        )
        .unwrap();
        let err = sdr.validate(ToolId::Oscilloscope).unwrap_err();
        assert_eq!(
            err.kind,
            FirmwareMismatchKind::WrongTargetSw {
                detected_tool: Some(ToolId::Sdr)
            }
        );
        assert!(err.summary().contains("HotWire SDR"));

        let old_la = BoardIdentity::decode_capabilities(
            br#"{"board":"XIAO-ESP32-S3","firmware":"1.0.0","channels":8}"#,
        )
        .unwrap();
        let err = old_la.validate(ToolId::LogicAnalyzer).unwrap_err();
        assert_eq!(
            err.kind,
            FirmwareMismatchKind::VersionTooOld {
                found: Some(FirmwareVersion::new(1, 0, 0)),
                required: FirmwareVersion::new(1, 2, 1),
            }
        );
        assert!(err.summary().contains("older than the required"));
    }
}
