use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Identifies one of the four HotWire browser tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolId {
    LogicAnalyzer,
    Oscilloscope,
    Sdr,
    SerialTerminal,
}

impl ToolId {
    pub const ALL: [ToolId; 4] = [
        ToolId::LogicAnalyzer,
        ToolId::Oscilloscope,
        ToolId::Sdr,
        ToolId::SerialTerminal,
    ];

    pub const fn slug(self) -> &'static str {
        match self {
            ToolId::LogicAnalyzer => "logic",
            ToolId::Oscilloscope => "scope",
            ToolId::Sdr => "sdr",
            ToolId::SerialTerminal => "serial",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            ToolId::LogicAnalyzer => "HotWire Logic Analyzer",
            ToolId::Oscilloscope => "HotWire Oscilloscope",
            ToolId::Sdr => "HotWire SDR",
            ToolId::SerialTerminal => "HotWire Serial Terminal",
        }
    }

    pub const fn short_label(self) -> &'static str {
        match self {
            ToolId::LogicAnalyzer => "Logic",
            ToolId::Oscilloscope => "Scope",
            ToolId::Sdr => "SDR",
            ToolId::SerialTerminal => "Serial",
        }
    }

    pub const fn live_url(self) -> &'static str {
        match self {
            ToolId::LogicAnalyzer => "https://kavierim.github.io/HotWire-Logic-Analyzer/",
            ToolId::Oscilloscope => "https://kavierim.github.io/HotWire-Oscilloscope/",
            ToolId::Sdr => "https://kavierim.github.io/HotWire-SDR/",
            ToolId::SerialTerminal => "https://kavierim.github.io/HotWire-Serial-Terminal/",
        }
    }

    pub const fn accent_color(self) -> &'static str {
        match self {
            ToolId::LogicAnalyzer => "#58a6ff",
            ToolId::Oscilloscope => "#facc15",
            ToolId::Sdr => "#a855f7",
            ToolId::SerialTerminal => "#4ade80",
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
}

/// Unified device identity parsed from a `0x83` (`TYPE_BOARD_CAPABILITIES`) JSON payload.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardIdentity {
    pub board: String,
    pub chip: BoardChip,
    pub firmware: String,
    pub detected_tool: Option<ToolId>,
    pub raw: Value,
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
        let detected_tool = infer_tool(&raw, &firmware);

        Some(Self {
            board,
            chip,
            firmware,
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
}

fn infer_tool(raw: &Value, firmware: &str) -> Option<ToolId> {
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
    if fw_lower.contains("scope") {
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
    fn parses_scope_caps_and_detects_chip_and_tool() {
        let json = br#"{"board":"XIAO-ESP32-C6","firmware":"hotwire-scope-1.0","scope":true,"max_channels":3,"max_rate_hz":83333}"#;
        let id = BoardIdentity::decode_capabilities(json).expect("valid json");
        assert_eq!(id.chip, BoardChip::Esp32C6);
        assert_eq!(id.detected_tool, Some(ToolId::Oscilloscope));
        assert!(id.matches_tool(ToolId::Oscilloscope));
        assert!(!id.matches_tool(ToolId::Sdr));
        assert_eq!(id.get_u64("max_channels"), Some(3));
    }

    #[test]
    fn detects_logic_analyzer_and_uart_bridge() {
        let la = br#"{"board":"XIAO-ESP32-S3","firmware":"hotwire-la-2.0","channels":8,"maxRateHz":20000000}"#;
        let la_id = BoardIdentity::decode_capabilities(la).unwrap();
        assert_eq!(la_id.chip, BoardChip::Esp32S3);
        assert_eq!(la_id.detected_tool, Some(ToolId::LogicAnalyzer));

        let uart = br#"{"board":"XIAO-ESP32-S3","firmware":"hotwire-uart-1.0","uart_bridge":true,"tx_pin":"D6","rx_pin":"D7"}"#;
        let uart_id = BoardIdentity::decode_capabilities(uart).unwrap();
        assert_eq!(uart_id.detected_tool, Some(ToolId::SerialTerminal));
        assert_eq!(uart_id.get_str("tx_pin"), Some("D6"));
    }
}
