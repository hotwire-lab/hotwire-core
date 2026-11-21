pub mod demuxer;
pub mod framing;
pub mod handshake;

pub use demuxer::{FrameDemuxer, FrameRule, RawFrame};
pub use framing::{
    build_bootloader_cmd, build_bootloader_cmd_zero, build_bootloader_frame, build_get_caps_cmd,
    build_get_caps_frame, build_ping_status_cmd, compute_crc16_ccitt, compute_crc8,
    compute_xor_checksum, encode_command_8, encode_frame,
};
pub use handshake::{BoardChip, BoardIdentity, ToolId};

pub const PROTOCOL_SYNC_0: u8 = 0xaa;
pub const PROTOCOL_SYNC_1: u8 = 0x55;

pub const CMD_PING_STATUS: u8 = 0x01;
pub const CMD_GET_CAPS: u8 = 0x02;
pub const CMD_BOOTLOADER: u8 = 0xf0;

pub const TYPE_BOARD_CAPABILITIES: u8 = 0x83;

pub const BOOTLOADER_MAGIC: [u8; 4] = [0xb0, 0x07, 0x10, 0xad];
