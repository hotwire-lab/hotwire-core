use serde::{Deserialize, Serialize};

use crate::protocol::BoardChip;

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

impl FirmwareProfile {
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
