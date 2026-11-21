use crate::protocol::{
    BOOTLOADER_MAGIC, CMD_BOOTLOADER, CMD_GET_CAPS, CMD_PING_STATUS, PROTOCOL_SYNC_0,
    PROTOCOL_SYNC_1,
};

/// Computes the 8-bit XOR checksum used by HotWire v1 binary frames.
pub fn compute_xor_checksum(bytes: &[u8]) -> u8 {
    let mut xor = 0u8;
    for &b in bytes {
        xor ^= b;
    }
    xor
}

/// Computes CRC-8 (polynomial 0x07, init 0x00) for extended framing/validation.
pub fn compute_crc8(bytes: &[u8]) -> u8 {
    let mut crc = 0u8;
    for &b in bytes {
        crc ^= b;
        for _ in 0..8 {
            if (crc & 0x80) != 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// Computes CRC-16/CCITT-FALSE (polynomial 0x1021, init 0xFFFF).
pub fn compute_crc16_ccitt(bytes: &[u8]) -> u16 {
    let mut crc = 0xffffu16;
    for &b in bytes {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// Encodes a fixed 8-byte host-to-device HotWire command frame:
/// `[0xAA, 0x55, cmd_id, p1, p2, p3, p4, xor]`.
pub fn encode_command_8(cmd_id: u8, p1: u8, p2: u8, p3: u8, p4: u8) -> [u8; 8] {
    let mut frame = [
        PROTOCOL_SYNC_0,
        PROTOCOL_SYNC_1,
        cmd_id,
        p1,
        p2,
        p3,
        p4,
        0,
    ];
    frame[7] = compute_xor_checksum(&frame[..7]);
    frame
}

/// Encodes a variable-length HotWire frame:
/// `[0xAA, 0x55, type_or_cmd, len_lo, len_hi, payload..., xor]`.
pub fn encode_frame(type_or_cmd: u8, payload: &[u8]) -> Vec<u8> {
    let len = payload.len();
    debug_assert!(len <= u16::MAX as usize);
    let mut frame = Vec::with_capacity(6 + len);
    frame.push(PROTOCOL_SYNC_0);
    frame.push(PROTOCOL_SYNC_1);
    frame.push(type_or_cmd);
    frame.push((len & 0xff) as u8);
    frame.push(((len >> 8) & 0xff) as u8);
    frame.extend_from_slice(payload);
    let chk = compute_xor_checksum(&frame);
    frame.push(chk);
    frame
}

pub fn build_ping_status_cmd() -> [u8; 8] {
    encode_command_8(CMD_PING_STATUS, 0, 0, 0, 0)
}

pub fn build_get_caps_cmd() -> [u8; 8] {
    encode_command_8(CMD_GET_CAPS, 0, 0, 0, 0)
}

pub fn build_get_caps_frame() -> Vec<u8> {
    encode_frame(CMD_GET_CAPS, &[])
}

pub fn build_bootloader_cmd() -> [u8; 8] {
    encode_command_8(
        CMD_BOOTLOADER,
        BOOTLOADER_MAGIC[0],
        BOOTLOADER_MAGIC[1],
        BOOTLOADER_MAGIC[2],
        BOOTLOADER_MAGIC[3],
    )
}

pub fn build_bootloader_cmd_zero() -> [u8; 8] {
    encode_command_8(CMD_BOOTLOADER, 0, 0, 0, 0)
}

pub fn build_bootloader_frame() -> Vec<u8> {
    encode_frame(CMD_BOOTLOADER, &BOOTLOADER_MAGIC)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_8_checksum_is_valid() {
        let cmd = build_bootloader_cmd();
        assert_eq!(cmd[0], 0xaa);
        assert_eq!(cmd[1], 0x55);
        assert_eq!(cmd[2], 0xf0);
        assert_eq!(&cmd[3..7], &BOOTLOADER_MAGIC);
        assert_eq!(cmd[7], compute_xor_checksum(&cmd[..7]));
    }

    #[test]
    fn variable_frame_roundtrip_layout() {
        let payload = b"{\"board\":\"XIAO-ESP32-S3\"}";
        let frame = encode_frame(0x83, payload);
        assert_eq!(frame[0], 0xaa);
        assert_eq!(frame[1], 0x55);
        assert_eq!(frame[2], 0x83);
        let len = u16::from_le_bytes([frame[3], frame[4]]) as usize;
        assert_eq!(len, payload.len());
        assert_eq!(&frame[5..5 + len], payload);
        assert_eq!(frame[5 + len], compute_xor_checksum(&frame[..5 + len]));
    }

    #[test]
    fn crc_helpers_match_known_vectors() {
        assert_eq!(compute_crc8(b"123456789"), 0xf4);
        assert_eq!(compute_crc16_ccitt(b"123456789"), 0x29b1);
    }
}
