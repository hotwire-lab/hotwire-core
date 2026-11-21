use crate::protocol::framing::compute_xor_checksum;
use crate::protocol::{PROTOCOL_SYNC_0, PROTOCOL_SYNC_1, TYPE_BOARD_CAPABILITIES};

/// Length validation rule for a binary frame type accepted by [`FrameDemuxer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameRule {
    pub type_id: u8,
    pub min_len: usize,
    pub max_len: usize,
}

impl FrameRule {
    pub const fn new(type_id: u8, min_len: usize, max_len: usize) -> Self {
        Self {
            type_id,
            min_len,
            max_len,
        }
    }

    /// Standard rule for `0x83` JSON board capabilities frames.
    pub const fn board_capabilities() -> Self {
        Self::new(TYPE_BOARD_CAPABILITIES, 2, 4096)
    }
}

/// Borrowed view of a verified binary frame emitted by [`FrameDemuxer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawFrame<'a> {
    pub type_id: u8,
    pub payload: &'a [u8],
}

/// Streaming demultiplexer for HotWire `[0xAA, 0x55, type, len_lo, len_hi, payload..., xor]`
/// frames multiplexed over a single USB CDC stream alongside optional UTF-8 text logs.
pub struct FrameDemuxer {
    rules: Vec<FrameRule>,
    max_payload_len: usize,
    buffer: Vec<u8>,
}

impl FrameDemuxer {
    pub fn new(rules: Vec<FrameRule>) -> Self {
        let max_payload_len = rules.iter().map(|r| r.max_len).max().unwrap_or(4096);
        Self {
            rules,
            max_payload_len,
            buffer: Vec::with_capacity(4096),
        }
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
    }

    /// Feeds incoming bytes, invoking `on_frame` for each valid binary frame and
    /// discarding non-frame bytes.
    pub fn feed<F>(&mut self, chunk: &[u8], on_frame: F)
    where
        F: FnMut(RawFrame<'_>),
    {
        self.feed_with_text(chunk, on_frame, |_| {});
    }

    /// Feeds incoming bytes, invoking `on_frame` for each valid binary frame and
    /// `on_text` for interleaved non-frame bytes (e.g. serial console logs).
    ///
    /// Uses a zero-copy fast path when no partial frame is buffered from the previous
    /// chunk, and reuses the internal buffer in-place otherwise to avoid per-chunk
    /// heap allocations on high-rate USB streams.
    pub fn feed_with_text<F, T>(&mut self, chunk: &[u8], mut on_frame: F, mut on_text: T)
    where
        F: FnMut(RawFrame<'_>),
        T: FnMut(&[u8]),
    {
        if chunk.is_empty() {
            return;
        }

        let max_tail = self.max_payload_len + 16;

        if self.buffer.is_empty() {
            let cursor = Self::scan_slice(
                &self.rules,
                self.max_payload_len,
                chunk,
                &mut on_frame,
                &mut on_text,
            );
            if cursor < chunk.len() {
                let remaining = &chunk[cursor..];
                if remaining.len() <= max_tail {
                    self.buffer.extend_from_slice(remaining);
                } else {
                    let drop_len = remaining.len() - max_tail;
                    on_text(&remaining[..drop_len]);
                    self.buffer.extend_from_slice(&remaining[drop_len..]);
                }
            }
            return;
        }

        self.buffer.extend_from_slice(chunk);
        let cursor = Self::scan_slice(
            &self.rules,
            self.max_payload_len,
            &self.buffer,
            &mut on_frame,
            &mut on_text,
        );

        let total_len = self.buffer.len();
        if cursor >= total_len {
            self.buffer.clear();
        } else {
            let remaining_len = total_len - cursor;
            let keep_start = if remaining_len <= max_tail {
                cursor
            } else {
                let drop_len = remaining_len - max_tail;
                on_text(&self.buffer[cursor..cursor + drop_len]);
                cursor + drop_len
            };
            let keep_len = total_len - keep_start;
            self.buffer.copy_within(keep_start..total_len, 0);
            self.buffer.truncate(keep_len);
        }
    }

    fn scan_slice<F, T>(
        rules: &[FrameRule],
        max_payload_len: usize,
        data: &[u8],
        on_frame: &mut F,
        on_text: &mut T,
    ) -> usize
    where
        F: FnMut(RawFrame<'_>),
        T: FnMut(&[u8]),
    {
        let mut cursor = 0usize;
        let mut text_start = 0usize;
        let len = data.len();

        while cursor < len {
            if data[cursor] != PROTOCOL_SYNC_0 {
                cursor += 1;
                continue;
            }
            if cursor + 1 >= len {
                break;
            }
            if data[cursor + 1] != PROTOCOL_SYNC_1 {
                cursor += 1;
                continue;
            }
            if cursor + 5 > len {
                break;
            }

            let type_id = data[cursor + 2];
            let payload_len = (data[cursor + 3] as usize) | ((data[cursor + 4] as usize) << 8);

            let valid_rule = payload_len <= max_payload_len
                && rules.iter().any(|r| {
                    r.type_id == type_id && payload_len >= r.min_len && payload_len <= r.max_len
                });
            if !valid_rule {
                cursor += 1;
                continue;
            }

            let total_frame_len = 6 + payload_len;
            if cursor + total_frame_len > len {
                break;
            }

            let frame_slice = &data[cursor..cursor + total_frame_len];
            let expected_xor = compute_xor_checksum(&frame_slice[..total_frame_len - 1]);
            let actual_xor = frame_slice[total_frame_len - 1];

            if expected_xor != actual_xor {
                cursor += 1;
                continue;
            }

            if cursor > text_start {
                on_text(&data[text_start..cursor]);
            }

            on_frame(RawFrame {
                type_id,
                payload: &frame_slice[5..5 + payload_len],
            });
            cursor += total_frame_len;
            text_start = cursor;
        }

        if cursor > text_start {
            on_text(&data[text_start..cursor]);
        }

        cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::framing::encode_frame;

    #[test]
    fn demuxes_split_frames_and_interleaved_text() {
        let mut demux = FrameDemuxer::new(vec![
            FrameRule::board_capabilities(),
            FrameRule::new(0x86, 4, 1024),
        ]);

        let caps_payload = br#"{"board":"XIAO-ESP32-S3","scope":true}"#;
        let frame1 = encode_frame(TYPE_BOARD_CAPABILITIES, caps_payload);
        let frame2 = encode_frame(0x86, &[1, 2, 3, 4, 5, 6]);

        let mut stream = Vec::new();
        stream.extend_from_slice(b"BOOT OK\r\n");
        stream.extend_from_slice(&frame1);
        stream.extend_from_slice(b"LOG: ready\n");
        stream.extend_from_slice(&frame2);

        let mut frames: Vec<(u8, Vec<u8>)> = Vec::new();
        let mut text: Vec<u8> = Vec::new();

        // Feed in awkward 3-byte chunks to verify streaming reassembly
        for chunk in stream.chunks(3) {
            demux.feed_with_text(
                chunk,
                |f| frames.push((f.type_id, f.payload.to_vec())),
                |t| text.extend_from_slice(t),
            );
        }

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].0, TYPE_BOARD_CAPABILITIES);
        assert_eq!(frames[0].1, caps_payload);
        assert_eq!(frames[1].0, 0x86);
        assert_eq!(frames[1].1, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(String::from_utf8_lossy(&text), "BOOT OK\r\nLOG: ready\n");
    }

    #[test]
    fn recovers_from_corrupt_checksum_and_false_sync() {
        let mut demux = FrameDemuxer::new(vec![FrameRule::new(0x81, 4, 4)]);

        let mut bad = encode_frame(0x81, &[1, 2, 3, 4]);
        *bad.last_mut().unwrap() ^= 0xff; // corrupt checksum
        let good = encode_frame(0x81, &[9, 8, 7, 6]);

        let mut stream = vec![0xaa, 0x55, 0xff]; // false sync + unknown type
        stream.extend_from_slice(&bad);
        stream.extend_from_slice(&good);

        let mut frames = Vec::new();
        demux.feed(&stream, |f| frames.push((f.type_id, f.payload.to_vec())));

        assert_eq!(frames, vec![(0x81, vec![9, 8, 7, 6])]);
    }

    #[test]
    fn fast_path_leaves_buffer_empty_on_aligned_frames() {
        let mut demux = FrameDemuxer::new(vec![FrameRule::new(0x86, 4, 1024)]);
        let frame = encode_frame(0x86, &[10, 20, 30, 40]);

        let mut received = Vec::new();
        demux.feed(&frame, |f| received.push((f.type_id, f.payload.to_vec())));
        assert_eq!(received, vec![(0x86, vec![10, 20, 30, 40])]);
        assert!(demux.buffer.is_empty());

        // Split frame across two chunks, then aligned frame again
        let (left, right) = frame.split_at(4);
        demux.feed(left, |f| received.push((f.type_id, f.payload.to_vec())));
        assert_eq!(received.len(), 1);
        assert_eq!(demux.buffer.len(), 4);

        demux.feed(right, |f| received.push((f.type_id, f.payload.to_vec())));
        assert_eq!(received.len(), 2);
        assert!(demux.buffer.is_empty());
    }
}
