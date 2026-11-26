//! Shared HotWire connection session: transport state, `0x83` handshake,
//! firmware-mismatch signals, demo transition and frame demultiplexing.
//!
//! Apps provide their tool id, frame rules, and three callbacks:
//! - `on_connected`: a valid `0x83` identity arrived (typed caps can be read
//!   from `identity.raw` / [`BoardIdentity::capabilities`]); the second argument
//!   is `true` for physical USB and `false` for Demo mode.
//! - `on_disconnected`: the transport dropped and app state should reset.
//! - `on_frame`: a non-capabilities frame arrived (copy payloads if retained).
//!
//! The returned [`HotWireSession`] exposes the reactive transport snapshot,
//! identity and mismatch signals used by the shared toolbar/modals.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::protocol::{
    BoardIdentity, FirmwareMismatch, FrameDemuxer, FrameRule, ToolId, TYPE_BOARD_CAPABILITIES,
};
use crate::transport::{HandshakeGuard, HotWireTransport, TransportSnapshot};

/// Default handshake timeout used by all HotWire apps.
pub const DEFAULT_HANDSHAKE_TIMEOUT_MS: i32 = 5_000;

/// Configuration for [`use_hotwire_session`].
pub struct SessionConfig {
    pub tool: ToolId,
    /// Binary frame rules accepted by the tool (capabilities is added automatically).
    pub frame_rules: Vec<FrameRule>,
    pub handshake_timeout_ms: i32,
    /// Serial Terminal direct-USB mode disables the HotWire handshake.
    pub handshake_enabled: bool,
    /// Serial Terminal direct-USB mode bypasses binary demuxing entirely.
    pub demux_enabled: bool,
}

impl SessionConfig {
    pub fn new(tool: ToolId, mut frame_rules: Vec<FrameRule>) -> Self {
        frame_rules.push(FrameRule::board_capabilities());
        Self {
            tool,
            frame_rules,
            handshake_timeout_ms: DEFAULT_HANDSHAKE_TIMEOUT_MS,
            handshake_enabled: true,
            demux_enabled: true,
        }
    }

    pub fn without_handshake(tool: ToolId, frame_rules: Vec<FrameRule>) -> Self {
        let mut config = Self::new(tool, frame_rules);
        config.handshake_enabled = false;
        config
    }

    pub fn with_handshake_timeout(mut self, timeout_ms: i32) -> Self {
        self.handshake_timeout_ms = timeout_ms;
        self
    }

    /// Disables binary demuxing so all received bytes are delivered to `on_text`
    /// (Serial Terminal direct-USB mode).
    pub fn with_demux_enabled(mut self, enabled: bool) -> Self {
        self.demux_enabled = enabled;
        self
    }
}

/// Reactive handles and internals of a HotWire connection session.
#[derive(Clone)]
pub struct HotWireSession {
    pub transport: RwSignal<TransportSnapshot>,
    pub identity: RwSignal<Option<BoardIdentity>>,
    pub mismatch: RwSignal<Option<FirmwareMismatch>>,
    pub flasher_mismatch: RwSignal<Option<FirmwareMismatch>>,
    handshake_enabled: RwSignal<bool>,
    demux_enabled: RwSignal<bool>,
    guard: Rc<RefCell<HandshakeGuard>>,
    transport_handle: HotWireTransport,
    handshake_timeout_ms: i32,
}

impl HotWireSession {
    /// Enables or disables the `0x83` handshake (Serial Terminal bridge mode).
    pub fn set_handshake_enabled(&self, enabled: bool) {
        self.handshake_enabled.set(enabled);
        if !enabled {
            self.guard.borrow_mut().cancel();
        }
    }

    pub fn handshake_enabled(&self) -> bool {
        self.handshake_enabled.get_untracked()
    }

    /// Enables or disables binary frame demuxing. When disabled, all bytes are
    /// delivered to the session's `on_text` callback.
    pub fn set_demux_enabled(&self, enabled: bool) {
        self.demux_enabled.set(enabled);
    }

    /// Arms a handshake timeout immediately (used when switching an already
    /// connected Serial Terminal into XIAO UART Bridge mode).
    pub fn arm_handshake(&self, device_name: String) {
        if !self.handshake_enabled.get_untracked() {
            return;
        }
        let mismatch = self.mismatch;
        let flasher_mismatch = self.flasher_mismatch;
        let transport = self.transport_handle.clone();
        self.guard
            .borrow_mut()
            .arm_timeout(device_name, self.handshake_timeout_ms, move |mm| {
                mismatch.set(Some(mm.clone()));
                flasher_mismatch.set(Some(mm));
                transport.disconnect();
            });
    }

    /// Cancels a pending handshake timeout (e.g. when leaving bridge mode).
    pub fn cancel_handshake(&self) {
        self.guard.borrow_mut().cancel();
    }

    /// Clears identity and mismatch state without touching the transport.
    pub fn clear_identity(&self) {
        self.identity.set(None);
        self.mismatch.set(None);
        self.flasher_mismatch.set(None);
    }
}

/// Installs shared transport callbacks and returns the reactive session state.
///
/// Must be called once during component setup (it uses Leptos signals and takes
/// ownership of the transport's callbacks).
pub fn use_hotwire_session<C, D, F, T>(
    transport: HotWireTransport,
    config: SessionConfig,
    on_connected: C,
    on_disconnected: D,
    on_frame: F,
    on_text: T,
) -> HotWireSession
where
    C: FnMut(&BoardIdentity, bool) + 'static,
    D: FnMut() + 'static,
    F: FnMut(u8, &[u8]) + 'static,
    T: FnMut(&[u8]) + 'static,
{
    let snapshot = RwSignal::new(transport.snapshot());
    let identity = RwSignal::new(None::<BoardIdentity>);
    let mismatch = RwSignal::new(None::<FirmwareMismatch>);
    let flasher_mismatch = RwSignal::new(None::<FirmwareMismatch>);
    let handshake_enabled = RwSignal::new(config.handshake_enabled);
    let demux_enabled = RwSignal::new(config.demux_enabled);
    let guard = Rc::new(RefCell::new(HandshakeGuard::new(config.tool)));
    let demuxer = Rc::new(RefCell::new(FrameDemuxer::new(config.frame_rules)));
    let on_connected = Rc::new(RefCell::new(on_connected));
    let on_disconnected = Rc::new(RefCell::new(on_disconnected));
    let on_frame = Rc::new(RefCell::new(on_frame));
    let on_text = Rc::new(RefCell::new(on_text));

    let state_transport = transport.clone();
    let state_guard = guard.clone();
    let state_identity = identity;
    let state_mismatch = mismatch;
    let state_flasher_mismatch = flasher_mismatch;
    let state_enabled = handshake_enabled;
    let state_demuxer = demuxer.clone();
    let state_on_disconnected = on_disconnected.clone();
    let timeout_ms = config.handshake_timeout_ms;

    let bytes_transport = transport.clone();
    let bytes_guard = guard.clone();
    let bytes_demuxer = demuxer.clone();
    let bytes_identity = identity;
    let bytes_mismatch = mismatch;
    let bytes_flasher_mismatch = flasher_mismatch;
    let bytes_enabled = handshake_enabled;
    let bytes_demux_enabled = demux_enabled;
    let bytes_on_connected = on_connected;
    let bytes_on_frame = on_frame;
    let bytes_on_text = on_text;

    transport.init(
        move |snap: TransportSnapshot| {
            let connected = snap.status.is_active();
            let is_usb = snap.status.is_usb_connected();
            let dev_name = snap.device_name.clone();
            snapshot.set(snap);

            if !connected {
                state_guard.borrow_mut().cancel();
                state_identity.set(None);
                state_mismatch.set(None);
                state_flasher_mismatch.set(None);
                state_demuxer.borrow_mut().reset();
                state_on_disconnected.borrow_mut()();
            } else if is_usb && state_enabled.get_untracked() {
                state_mismatch.set(None);
                state_flasher_mismatch.set(None);
                let mismatch_sig = state_mismatch;
                let flasher_sig = state_flasher_mismatch;
                let transport_for_timeout = state_transport.clone();
                state_guard
                    .borrow_mut()
                    .arm_timeout(dev_name, timeout_ms, move |mm| {
                        mismatch_sig.set(Some(mm.clone()));
                        flasher_sig.set(Some(mm));
                        transport_for_timeout.disconnect();
                    });
            } else {
                state_guard.borrow_mut().cancel();
                if !is_usb {
                    state_mismatch.set(None);
                    state_flasher_mismatch.set(None);
                }
            }
        },
        move |bytes: &[u8]| {
            if !bytes_demux_enabled.get_untracked() {
                bytes_on_text.borrow_mut()(bytes);
                return;
            }
            bytes_demuxer.borrow_mut().feed_with_text(
                bytes,
                |raw| {
                    if raw.type_id == TYPE_BOARD_CAPABILITIES {
                        let Some(id) = BoardIdentity::decode_capabilities(raw.payload) else {
                            return;
                        };
                        bytes_transport.notify_handshake_ok();
                        let is_usb = snapshot.get_untracked().status.is_usb_connected();
                        if is_usb && bytes_enabled.get_untracked() {
                            let verdict = bytes_guard.borrow_mut().verify(&id);
                            if let Err(mm) = verdict {
                                bytes_mismatch.set(Some(mm.clone()));
                                bytes_flasher_mismatch.set(Some(mm));
                                bytes_transport.disconnect();
                                return;
                            }
                        } else {
                            bytes_guard.borrow_mut().cancel();
                        }
                        bytes_mismatch.set(None);
                        bytes_flasher_mismatch.set(None);
                        bytes_identity.set(Some(id.clone()));
                        bytes_on_connected.borrow_mut()(&id, is_usb);
                    } else {
                        bytes_on_frame.borrow_mut()(raw.type_id, raw.payload);
                    }
                },
                |text| bytes_on_text.borrow_mut()(text),
            );
        },
    );

    HotWireSession {
        transport: snapshot,
        identity,
        mismatch,
        flasher_mismatch,
        handshake_enabled,
        demux_enabled,
        guard,
        transport_handle: transport,
        handshake_timeout_ms: config.handshake_timeout_ms,
    }
}
