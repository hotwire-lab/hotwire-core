use serde::{Deserialize, Serialize};

use crate::protocol::{BoardChip, ToolId};

#[cfg(feature = "ui")]
pub mod modal;

#[cfg(feature = "ui")]
pub use modal::FlasherPanel;

/// Target microcontroller board supported by the HotWire Web Flasher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BoardId {
    Esp32S3,
    Esp32C6,
}

impl BoardId {
    pub const fn as_str(self) -> &'static str {
        match self {
            BoardId::Esp32S3 => "esp32s3",
            BoardId::Esp32C6 => "esp32c6",
        }
    }

    pub fn parse(s: &str) -> Self {
        if s.eq_ignore_ascii_case("esp32c6") || s.to_uppercase().contains("C6") {
            BoardId::Esp32C6
        } else {
            BoardId::Esp32S3
        }
    }

    pub fn from_chip(chip: BoardChip) -> Option<Self> {
        match chip {
            BoardChip::Esp32S3 => Some(BoardId::Esp32S3),
            BoardChip::Esp32C6 => Some(BoardId::Esp32C6),
            BoardChip::Unknown => None,
        }
    }

    pub const fn default_label(self) -> &'static str {
        match self {
            BoardId::Esp32S3 => "Seeed Studio XIAO ESP32-S3",
            BoardId::Esp32C6 => "Seeed Studio XIAO ESP32-C6",
        }
    }
}

/// Describes a bundled firmware image set for a specific board, passed to `esptool-js`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareProfile {
    pub board_id: String,
    pub label: String,
    pub hint: String,
    pub prefix: String,
    pub bootloader_offset: u32,
    pub flash_size: String,
    pub base_url: String,
}

/// Canonical bundled-firmware filenames and short descriptions per tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolFirmwareInfo {
    pub s3_prefix: &'static str,
    pub c6_prefix: &'static str,
    pub s3_hint: &'static str,
    pub c6_hint: &'static str,
}

impl ToolId {
    /// Canonical firmware metadata used by the shared Web Flasher.
    pub const fn firmware_info(self) -> ToolFirmwareInfo {
        match self {
            ToolId::LogicAnalyzer => ToolFirmwareInfo {
                s3_prefix: "HotWire_XIAO_ESP32S3.ino",
                c6_prefix: "HotWire_XIAO_ESP32C6.ino",
                s3_hint: "8-Ch Logic Analyzer (D0-D7 / GPIO1-6, 43-44), 20 MHz / 32k samples",
                c6_hint:
                    "8-Ch Logic Analyzer (D0-D7), Dedicated GPIO capture + Tri-State Force I/O",
            },
            ToolId::Oscilloscope => ToolFirmwareInfo {
                s3_prefix: "HotWire_Scope_XIAO_ESP32S3.ino",
                c6_prefix: "HotWire_Scope_XIAO_ESP32C6.ino",
                s3_hint:
                    "4-Ch ADC1 (D0-D3), Dual-Core GDMA, 83.3 kSPS spec + up to 1 MSPS Turbo OC",
                c6_hint: "3-Ch ADC1 (D0-D2), GDMA Continuous up to 83.3 kSPS, eFuse calibration",
            },
            ToolId::Bench => ToolFirmwareInfo {
                s3_prefix: "HotWire_Bench_XIAO_ESP32S3.ino",
                c6_prefix: "HotWire_Bench_XIAO_ESP32C6.ino",
                s3_hint: "Mixed-signal bench: 2 analog (D0-D1) + 8 digital, 1 MSPS + LEDC generator",
                c6_hint: "Mixed-signal bench (S3 firmware is the V0.1 reference)",
            },
            ToolId::Sdr => ToolFirmwareInfo {
                s3_prefix: "HotWire_SDR_XIAO_ESP32S3.ino",
                c6_prefix: "HotWire_SDR_XIAO_ESP32C6.ino",
                s3_hint: "1.84-2.79 GHz RF Spectrum Analyzer (External U.FL, 8 MB Flash)",
                c6_hint: "1.84-2.79 GHz RF Spectrum Analyzer (External U.FL default)",
            },
            ToolId::SerialTerminal => ToolFirmwareInfo {
                s3_prefix: "HotWire_UART_XIAO_ESP32S3.ino",
                c6_prefix: "HotWire_UART_XIAO_ESP32C6.ino",
                s3_hint: "3.3 V TTL USB-to-UART Bridge: D6 (GPIO43) TX, D7 (GPIO44) RX",
                c6_hint: "3.3 V TTL USB-to-UART Bridge: D6 (GPIO16) TX, D7 (GPIO17) RX",
            },
            ToolId::SignalGenerator => ToolFirmwareInfo {
                s3_prefix: "HotWire_SigGen_XIAO_ESP32S3.ino",
                c6_prefix: "HotWire_SigGen_XIAO_ESP32C6.ino",
                s3_hint: "Dual-Ch DDS + HW LEDC (CH1=D0, CH2=D1), 4-Ch Pattern on D2-D5",
                c6_hint: "Dual-Ch DDS + HW LEDC (CH1=D0, CH2=D1), 4-Ch Pattern on D2-D5",
            },
        }
    }
}

impl FirmwareProfile {
    /// Builds the canonical profile for a tool and board.
    pub fn for_tool(tool: ToolId, board: BoardId) -> Self {
        let info = tool.firmware_info();
        match board {
            BoardId::Esp32S3 => Self::xiao_esp32s3(info.s3_prefix, info.s3_hint),
            BoardId::Esp32C6 => Self::xiao_esp32c6(info.c6_prefix, info.c6_hint),
        }
    }

    /// Builds the canonical profile for a tool and detected chip.
    pub fn for_tool_chip(tool: ToolId, chip: BoardChip) -> Option<Self> {
        BoardId::from_chip(chip).map(|board| Self::for_tool(tool, board))
    }

    /// Returns the S3 and C6 profiles in canonical order.
    pub fn standard_profiles(tool: ToolId) -> Vec<Self> {
        vec![
            Self::for_tool(tool, BoardId::Esp32S3),
            Self::for_tool(tool, BoardId::Esp32C6),
        ]
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }

    /// Creates a standard Seeed Studio XIAO ESP32-S3 profile (8 MB flash, bootloader @ 0x0).
    pub fn xiao_esp32s3(prefix: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            board_id: BoardId::Esp32S3.as_str().to_string(),
            label: BoardId::Esp32S3.default_label().to_string(),
            hint: hint.into(),
            prefix: prefix.into(),
            bootloader_offset: 0x0,
            flash_size: "8MB".to_string(),
            base_url: "./firmware".to_string(),
        }
    }

    /// Creates a standard Seeed Studio XIAO ESP32-C6 profile (4 MB flash, bootloader @ 0x0).
    pub fn xiao_esp32c6(prefix: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            board_id: BoardId::Esp32C6.as_str().to_string(),
            label: BoardId::Esp32C6.default_label().to_string(),
            hint: hint.into(),
            prefix: prefix.into(),
            bootloader_offset: 0x0,
            flash_size: "4MB".to_string(),
            base_url: "./firmware".to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_bootloader_offset(mut self, offset: u32) -> Self {
        self.bootloader_offset = offset;
        self
    }

    pub fn board(&self) -> BoardId {
        BoardId::parse(&self.board_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_firmware_profile_for_js_bridge() {
        let p = FirmwareProfile::xiao_esp32s3("HotWire_Scope_XIAO_ESP32S3.ino", "4-Ch ADC1");
        let json = serde_json::to_string(&p).expect("serialize");
        assert!(json.contains("\"board_id\":\"esp32s3\""));
        assert!(json.contains("\"flash_size\":\"8MB\""));
        assert!(json.contains("\"base_url\":\"./firmware\""));
    }
}
