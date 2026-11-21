use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
mod bridge;

/// Underlying physical or simulated transport backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportKind {
    None,
    WebSerial,
    WebUsb,
    Demo,
}

/// Unified HotWire connection state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Demo,
}

impl ConnectionStatus {
    /// Returns `true` when either a physical USB device or the Demo simulator is active.
    pub const fn is_active(self) -> bool {
        matches!(self, ConnectionStatus::Connected | ConnectionStatus::Demo)
    }

    /// Returns `true` only when a physical USB transport is connected.
    pub const fn is_usb_connected(self) -> bool {
        matches!(self, ConnectionStatus::Connected)
    }

    pub const fn as_class(self) -> &'static str {
        match self {
            ConnectionStatus::Disconnected => "waiting",
            ConnectionStatus::Connecting => "connecting",
            ConnectionStatus::Connected => "connected",
            ConnectionStatus::Demo => "connected",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            ConnectionStatus::Disconnected => "Disconnected",
            ConnectionStatus::Connecting => "Connecting...",
            ConnectionStatus::Connected => "Connected",
            ConnectionStatus::Demo => "Demo",
        }
    }
}

/// Serial port and auto-reconnect configuration shared across Desktop Web Serial and Android WebUSB.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerialConfig {
    pub baud_rate: u32,
    pub data_bits: u8,
    pub stop_bits: u8,
    /// `0` = None, `1` = Odd, `2` = Even
    pub parity: u8,
    /// `0` = None, `1` = Hardware RTS/CTS
    pub flow_control: u8,
    pub buffer_size: u32,
    /// Explicit DTR override (`None` uses per-VID smart defaults).
    pub force_dtr: Option<bool>,
    /// Explicit RTS override (`None` uses per-VID smart defaults).
    pub force_rts: Option<bool>,
    pub auto_connect: bool,
    pub storage_key: String,
}

impl SerialConfig {
    /// Default high-throughput CDC-ACM config for ESP32-S3/C6 HotWire hardware tools
    /// (Logic Analyzer, Oscilloscope, SDR).
    ///
    /// Uses `DTR=true, RTS=false` (`SET_CONTROL_LINE_STATE = 0x01`), which is the required
    /// normal run-mode state on ESP32-S3/C6 native USB-Serial-JTAG (`VID 0x303A / PID 0x1001`).
    /// Asserting `RTS=true` (`0x03`) interferes with the silicon's reset/download-boot circuit.
    pub fn hotwire_cdc(storage_key: impl Into<String>) -> Self {
        Self {
            baud_rate: 921_600,
            data_bits: 8,
            stop_bits: 1,
            parity: 0,
            flow_control: 0,
            buffer_size: 131_072,
            force_dtr: Some(true),
            force_rts: Some(false),
            auto_connect: true,
            storage_key: storage_key.into(),
        }
    }

    /// Config tailored for interactive serial terminal usage with per-VID DTR/RTS defaults.
    pub fn serial_terminal(baud_rate: u32) -> Self {
        Self {
            baud_rate,
            data_bits: 8,
            stop_bits: 1,
            parity: 0,
            flow_control: 0,
            buffer_size: 16_384,
            force_dtr: None,
            force_rts: None,
            auto_connect: true,
            storage_key: "hotwire_serial_last_device".to_string(),
        }
    }
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self::hotwire_cdc("hotwire_last_device")
    }
}

/// Immutable snapshot of the current transport state for reactive UI rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportSnapshot {
    pub status: ConnectionStatus,
    pub transport: TransportKind,
    pub device_name: String,
    pub vid_pid: String,
    pub port_index: u32,
    pub port_count: u32,
    pub dtr: bool,
    pub rts: bool,
    pub error_message: Option<String>,
}

impl Default for TransportSnapshot {
    fn default() -> Self {
        Self {
            status: ConnectionStatus::Disconnected,
            transport: TransportKind::None,
            device_name: "Waiting for USB...".to_string(),
            vid_pid: String::new(),
            port_index: 0,
            port_count: 0,
            dtr: false,
            rts: false,
            error_message: None,
        }
    }
}

/// Tool-specific offline simulator injected into [`HotWireTransport`] for Demo mode.
pub trait DemoBackend {
    fn device_name(&self) -> &str {
        "XIAO-ESP32-S3 (Demo)"
    }

    fn vid_pid(&self) -> &str {
        "DEMO:0001"
    }

    /// Optional periodic timer interval in milliseconds (`None` disables periodic ticks).
    fn tick_interval_ms(&self) -> Option<i32> {
        Some(50)
    }

    /// Optional bytes emitted immediately when entering Demo mode.
    fn on_start(&mut self) -> Vec<u8> {
        Vec::new()
    }

    /// Processes outgoing bytes sent by the app and returns immediate simulated reply bytes.
    fn handle_command(&mut self, bytes: &[u8]) -> Vec<u8>;

    /// Periodic streaming tick (e.g. oscilloscope frames, telemetry, SDR sweeps).
    fn tick(&mut self) -> Option<Vec<u8>> {
        None
    }
}

/// No-op demo backend for apps or tests that do not attach a simulator.
#[derive(Default)]
pub struct NoopDemoBackend;

impl DemoBackend for NoopDemoBackend {
    fn tick_interval_ms(&self) -> Option<i32> {
        None
    }

    fn handle_command(&mut self, _bytes: &[u8]) -> Vec<u8> {
        Vec::new()
    }
}

#[cfg(feature = "wasm")]
mod wasm_transport {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use serde::Deserialize;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::spawn_local;

    use crate::flasher::FirmwareProfile;
    use crate::protocol::{
        build_bootloader_cmd, build_get_caps_cmd, BoardIdentity, FirmwareMismatch, FirmwareVersion,
        ToolId,
    };
    use crate::transport::bridge::*;
    use crate::transport::{
        ConnectionStatus, DemoBackend, NoopDemoBackend, SerialConfig, TransportKind,
        TransportSnapshot,
    };

    #[derive(Deserialize)]
    struct JsStatePayload {
        status: String,
        transport: String,
        device_name: String,
        vid_pid: String,
        port_index: u32,
        port_count: u32,
        dtr: bool,
        rts: bool,
        error_message: String,
    }

    type StateCallback = Rc<RefCell<Option<Box<dyn FnMut(TransportSnapshot)>>>>;
    type BytesCallback = Rc<RefCell<Option<Box<dyn FnMut(&[u8])>>>>;
    type DemoClosureSlot = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;
    type SharedDemo = Rc<RefCell<Box<dyn DemoBackend>>>;

    /// Unified browser USB transport (Web Serial + WebUSB + Demo mode + Web Flasher).
    #[derive(Clone)]
    pub struct HotWireTransport {
        config: Rc<RefCell<SerialConfig>>,
        init_cmds: Rc<RefCell<Vec<u8>>>,
        bootloader_cmd: Rc<RefCell<Vec<u8>>>,
        snapshot: Rc<RefCell<TransportSnapshot>>,
        demo: SharedDemo,
        demo_timer_id: Rc<RefCell<Option<i32>>>,
        demo_closure: DemoClosureSlot,
        on_state: StateCallback,
        on_bytes: BytesCallback,
    }

    impl Default for HotWireTransport {
        fn default() -> Self {
            Self::new(SerialConfig::default())
        }
    }

    impl HotWireTransport {
        pub fn new(config: SerialConfig) -> Self {
            Self::with_demo(config, NoopDemoBackend)
        }

        pub fn with_demo<D: DemoBackend + 'static>(config: SerialConfig, demo: D) -> Self {
            Self {
                config: Rc::new(RefCell::new(config)),
                init_cmds: Rc::new(RefCell::new(build_get_caps_cmd().to_vec())),
                bootloader_cmd: Rc::new(RefCell::new(build_bootloader_cmd().to_vec())),
                snapshot: Rc::new(RefCell::new(TransportSnapshot::default())),
                demo: Rc::new(RefCell::new(Box::new(demo))),
                demo_timer_id: Rc::new(RefCell::new(None)),
                demo_closure: Rc::new(RefCell::new(None)),
                on_state: Rc::new(RefCell::new(None)),
                on_bytes: Rc::new(RefCell::new(None)),
            }
        }

        /// Overrides the initial handshake bytes sent right after opening USB (e.g. empty for raw serial terminal).
        pub fn set_init_commands(&self, bytes: Vec<u8>) {
            *self.init_cmds.borrow_mut() = bytes;
        }

        /// Overrides the `0xF0` bootloader entry bytes sent before invoking `esptool-js`.
        pub fn set_bootloader_command(&self, bytes: Vec<u8>) {
            *self.bootloader_cmd.borrow_mut() = bytes;
        }

        pub fn snapshot(&self) -> TransportSnapshot {
            self.snapshot.borrow().clone()
        }

        pub fn init<FS, FB>(&self, on_state: FS, on_bytes: FB)
        where
            FS: FnMut(TransportSnapshot) + 'static,
            FB: FnMut(&[u8]) + 'static,
        {
            *self.on_state.borrow_mut() = Some(Box::new(on_state));
            *self.on_bytes.borrow_mut() = Some(Box::new(on_bytes));

            let snap_ref = self.snapshot.clone();
            let on_state_ref = self.on_state.clone();
            let state_cb = Closure::<dyn FnMut(String)>::new(move |json_str: String| {
                let Ok(parsed) = serde_json::from_str::<JsStatePayload>(&json_str) else {
                    return;
                };
                let error_message = if parsed.error_message.is_empty() {
                    None
                } else {
                    Some(parsed.error_message)
                };
                let next = TransportSnapshot {
                    status: match parsed.status.as_str() {
                        "connecting" => ConnectionStatus::Connecting,
                        "connected" => ConnectionStatus::Connected,
                        _ => ConnectionStatus::Disconnected,
                    },
                    transport: match parsed.transport.as_str() {
                        "webserial" => TransportKind::WebSerial,
                        "webusb" => TransportKind::WebUsb,
                        _ => TransportKind::None,
                    },
                    device_name: if parsed.device_name.is_empty() {
                        "Waiting for USB...".to_string()
                    } else {
                        parsed.device_name
                    },
                    vid_pid: parsed.vid_pid,
                    port_index: parsed.port_index,
                    port_count: parsed.port_count,
                    dtr: parsed.dtr,
                    rts: parsed.rts,
                    error_message,
                };

                // Do not clobber active Demo mode unless a real USB connection succeeds
                if snap_ref.borrow().status == ConnectionStatus::Demo
                    && next.status != ConnectionStatus::Connected
                {
                    return;
                }

                *snap_ref.borrow_mut() = next.clone();
                if let Some(cb) = on_state_ref.borrow_mut().as_mut() {
                    cb(next);
                }
            });

            let on_bytes_ref = self.on_bytes.clone();
            let bytes_cb =
                Closure::<dyn FnMut(js_sys::Uint8Array)>::new(move |arr: js_sys::Uint8Array| {
                    let data = arr.to_vec();
                    if let Some(cb) = on_bytes_ref.borrow_mut().as_mut() {
                        cb(&data);
                    }
                });

            let cfg_json = serde_json::to_string(&*self.config.borrow()).unwrap_or_default();
            let init_cmds = self.init_cmds.borrow().clone();
            js_init_transport(
                state_cb.as_ref().unchecked_ref(),
                bytes_cb.as_ref().unchecked_ref(),
                &cfg_json,
                &init_cmds,
            );
            state_cb.forget();
            bytes_cb.forget();
        }

        pub fn connect_usb(&self) {
            let cfg = self.config.borrow().clone();
            self.connect_usb_with_config(cfg);
        }

        pub fn connect_usb_with_config(&self, cfg: SerialConfig) {
            self.stop_demo_timer();
            *self.config.borrow_mut() = cfg.clone();
            let cfg_json = serde_json::to_string(&cfg).unwrap_or_default();
            let init_cmds = self.init_cmds.borrow().clone();
            let snap_ref = self.snapshot.clone();
            let on_state_ref = self.on_state.clone();
            spawn_local(async move {
                let res =
                    wasm_bindgen_futures::JsFuture::from(js_connect_usb(&cfg_json, &init_cmds))
                        .await;
                if let Err(err) = res {
                    let msg = extract_js_error(err, "USB connection failed");
                    let next = TransportSnapshot {
                        status: ConnectionStatus::Disconnected,
                        transport: TransportKind::None,
                        device_name: "Disconnected".to_string(),
                        error_message: Some(msg),
                        ..Default::default()
                    };
                    *snap_ref.borrow_mut() = next.clone();
                    if let Some(cb) = on_state_ref.borrow_mut().as_mut() {
                        cb(next);
                    }
                }
            });
        }

        pub fn start_demo(&self) {
            self.stop_demo_timer();
            let (dev_name, vid_pid, interval_ms, start_bytes) = {
                let mut demo = self.demo.borrow_mut();
                let name = demo.device_name().to_string();
                let vp = demo.vid_pid().to_string();
                let interval = demo.tick_interval_ms();
                let mut bytes = demo.on_start();
                let init_cmds = self.init_cmds.borrow().clone();
                if !init_cmds.is_empty() {
                    bytes.extend_from_slice(&demo.handle_command(&init_cmds));
                }
                (name, vp, interval, bytes)
            };

            let snap = TransportSnapshot {
                status: ConnectionStatus::Demo,
                transport: TransportKind::Demo,
                device_name: dev_name,
                vid_pid,
                port_index: 0,
                port_count: 1,
                dtr: true,
                rts: false,
                error_message: None,
            };
            *self.snapshot.borrow_mut() = snap.clone();
            if let Some(cb) = self.on_state.borrow_mut().as_mut() {
                cb(snap);
            }

            if !start_bytes.is_empty() {
                if let Some(cb) = self.on_bytes.borrow_mut().as_mut() {
                    cb(&start_bytes);
                }
            }

            if let Some(ms) = interval_ms {
                let demo_ref = self.demo.clone();
                let on_bytes_ref = self.on_bytes.clone();
                let closure = Closure::<dyn FnMut()>::new(move || {
                    let bytes_opt = demo_ref.borrow_mut().tick();
                    if let Some(bytes) = bytes_opt {
                        if !bytes.is_empty() {
                            if let Some(cb) = on_bytes_ref.borrow_mut().as_mut() {
                                cb(&bytes);
                            }
                        }
                    }
                });
                if let Some(win) = web_sys::window() {
                    if let Ok(id) = win.set_interval_with_callback_and_timeout_and_arguments_0(
                        closure.as_ref().unchecked_ref(),
                        ms,
                    ) {
                        *self.demo_timer_id.borrow_mut() = Some(id);
                        *self.demo_closure.borrow_mut() = Some(closure);
                    }
                }
            }
        }

        pub fn disconnect(&self) {
            self.stop_demo_timer();
            if self.snapshot.borrow().status == ConnectionStatus::Demo {
                let next = TransportSnapshot::default();
                *self.snapshot.borrow_mut() = next.clone();
                if let Some(cb) = self.on_state.borrow_mut().as_mut() {
                    cb(next);
                }
                return;
            }
            spawn_local(async move {
                let _ = wasm_bindgen_futures::JsFuture::from(js_disconnect_usb()).await;
            });
        }

        /// Signals the JS transport that a valid `0x83` capabilities handshake was received,
        /// cancelling any remaining initial probe retries so firmware setup is never re-triggered.
        pub fn notify_handshake_ok(&self) {
            if self.snapshot.borrow().status.is_usb_connected() {
                js_notify_handshake_ok();
            }
        }

        pub fn send_bytes(&self, bytes: &[u8]) {
            if self.snapshot.borrow().status == ConnectionStatus::Demo {
                let reply = self.demo.borrow_mut().handle_command(bytes);
                if !reply.is_empty() {
                    if let Some(cb) = self.on_bytes.borrow_mut().as_mut() {
                        cb(&reply);
                    }
                }
                return;
            }
            let owned = bytes.to_vec();
            spawn_local(async move {
                let _ = wasm_bindgen_futures::JsFuture::from(js_write_bytes(&owned)).await;
            });
        }

        pub fn set_signals(&self, dtr: bool, rts: bool) {
            if self.snapshot.borrow().status == ConnectionStatus::Demo {
                let mut snap = self.snapshot.borrow_mut();
                snap.dtr = dtr;
                snap.rts = rts;
                let cloned = snap.clone();
                drop(snap);
                if let Some(cb) = self.on_state.borrow_mut().as_mut() {
                    cb(cloned);
                }
                return;
            }
            spawn_local(async move {
                let _ = wasm_bindgen_futures::JsFuture::from(js_set_signals(dtr, rts)).await;
            });
        }

        pub fn pulse_reset(&self) {
            if self.snapshot.borrow().status == ConnectionStatus::Demo {
                return;
            }
            spawn_local(async move {
                let _ = wasm_bindgen_futures::JsFuture::from(js_pulse_reset()).await;
            });
        }

        pub fn reconfigure(&self, cfg: SerialConfig) {
            *self.config.borrow_mut() = cfg.clone();
            if self.snapshot.borrow().status == ConnectionStatus::Demo {
                return;
            }
            let cfg_json = serde_json::to_string(&cfg).unwrap_or_default();
            spawn_local(async move {
                let _ = wasm_bindgen_futures::JsFuture::from(js_reconfigure_serial(&cfg_json)).await;
            });
        }

        pub fn switch_usb_port(&self, port_idx: u32) {
            if self.snapshot.borrow().status == ConnectionStatus::Demo {
                return;
            }
            spawn_local(async move {
                let _ = wasm_bindgen_futures::JsFuture::from(js_switch_usb_port(port_idx)).await;
            });
        }

        pub fn set_auto_connect(&self, enabled: bool) {
            self.config.borrow_mut().auto_connect = enabled;
            let cfg_json = serde_json::to_string(&*self.config.borrow()).unwrap_or_default();
            js_set_auto_connect(enabled, &cfg_json);
        }

        #[allow(clippy::too_many_arguments)]
        pub async fn flash_firmware<FL, FP>(
            &self,
            profile: &FirmwareProfile,
            force_other_port: bool,
            flash_offset: &str,
            custom_file: Option<&web_sys::File>,
            mut on_log: FL,
            mut on_progress: FP,
        ) -> Result<(), String>
        where
            FL: FnMut(String) + 'static,
            FP: FnMut(u32) + 'static,
        {
            self.stop_demo_timer();
            let profile_json =
                serde_json::to_string(profile).map_err(|e| format!("Invalid profile: {e}"))?;
            let log_cb = Closure::<dyn FnMut(String)>::new(move |msg: String| on_log(msg));
            let prog_cb = Closure::<dyn FnMut(u32)>::new(move |pct: u32| on_progress(pct));
            let boot_cmd = self.bootloader_cmd.borrow().clone();
            let init_cmds = self.init_cmds.borrow().clone();

            let promise = js_flash_firmware(
                &profile_json,
                force_other_port,
                flash_offset,
                custom_file,
                &boot_cmd,
                &init_cmds,
                log_cb.as_ref().unchecked_ref(),
                prog_cb.as_ref().unchecked_ref(),
            );
            let res = wasm_bindgen_futures::JsFuture::from(promise).await;
            drop(log_cb);
            drop(prog_cb);
            res.map(|_| ())
                .map_err(|err| extract_js_error(err, "Firmware flashing failed"))
        }

        fn stop_demo_timer(&self) {
            if let Some(id) = self.demo_timer_id.borrow_mut().take() {
                if let Some(win) = web_sys::window() {
                    win.clear_interval_with_handle(id);
                }
            }
            *self.demo_closure.borrow_mut() = None;
        }
    }

    fn extract_js_error(err: JsValue, fallback: &str) -> String {
        err.as_string()
            .or_else(|| {
                js_sys::Reflect::get(&err, &JsValue::from_str("message"))
                    .ok()
                    .and_then(|v| v.as_string())
            })
            .unwrap_or_else(|| fallback.to_string())
    }

    /// Guards USB connections by enforcing target SW + minimum firmware version checks
    /// on incoming `0x83` capabilities frames and firing a mismatch callback if no
    /// handshake arrives within `timeout_ms`.
    pub struct HandshakeGuard {
        expected_tool: ToolId,
        min_version: FirmwareVersion,
        armed: Rc<Cell<bool>>,
        timer_id: Option<i32>,
        timer_cb: Option<Closure<dyn FnMut()>>,
    }

    impl HandshakeGuard {
        pub fn new(expected_tool: ToolId) -> Self {
            Self {
                expected_tool,
                min_version: expected_tool.min_firmware_version(),
                armed: Rc::new(Cell::new(false)),
                timer_id: None,
                timer_cb: None,
            }
        }

        pub fn with_min_version(expected_tool: ToolId, min_version: FirmwareVersion) -> Self {
            Self {
                expected_tool,
                min_version,
                armed: Rc::new(Cell::new(false)),
                timer_id: None,
                timer_cb: None,
            }
        }

        pub fn cancel(&mut self) {
            self.armed.set(false);
            if let Some(id) = self.timer_id.take() {
                if let Some(win) = web_sys::window() {
                    win.clear_timeout_with_handle(id);
                }
            }
            self.timer_cb = None;
        }

        /// Starts (or restarts) the handshake timeout for a freshly connected physical USB port.
        pub fn arm_timeout<F>(&mut self, device_name: String, timeout_ms: i32, on_timeout: F)
        where
            F: FnOnce(FirmwareMismatch) + 'static,
        {
            self.cancel();
            let token = Rc::new(Cell::new(true));
            self.armed = token.clone();
            let expected = self.expected_tool;
            let min_ver = self.min_version;
            let mut cb_opt = Some(on_timeout);
            let closure = Closure::<dyn FnMut()>::new(move || {
                if !token.get() {
                    return;
                }
                token.set(false);
                if let Some(cb) = cb_opt.take() {
                    let mismatch =
                        FirmwareMismatch::no_handshake(expected, min_ver, device_name.clone());
                    cb(mismatch);
                }
            });
            if let Some(win) = web_sys::window() {
                if let Ok(id) = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    timeout_ms,
                ) {
                    self.timer_id = Some(id);
                    self.timer_cb = Some(closure);
                }
            }
        }

        /// Cancels any pending handshake timeout and validates target SW + minimum firmware version.
        pub fn verify(&mut self, identity: &BoardIdentity) -> Result<(), FirmwareMismatch> {
            self.cancel();
            identity.validate_for_tool(self.expected_tool, self.min_version)
        }
    }

    impl Drop for HandshakeGuard {
        fn drop(&mut self) {
            self.cancel();
        }
    }
}

#[cfg(feature = "wasm")]
pub use wasm_transport::{HandshakeGuard, HotWireTransport};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_status_helpers_match_state_machine() {
        assert!(!ConnectionStatus::Disconnected.is_active());
        assert!(!ConnectionStatus::Connecting.is_active());
        assert!(ConnectionStatus::Connected.is_active());
        assert!(ConnectionStatus::Demo.is_active());
        assert!(ConnectionStatus::Connected.is_usb_connected());
        assert!(!ConnectionStatus::Demo.is_usb_connected());
    }

    #[test]
    fn serial_config_presets_serialize() {
        let cdc = SerialConfig::hotwire_cdc("hotwire_scope_last");
        assert_eq!(cdc.baud_rate, 921_600);
        assert_eq!(cdc.force_dtr, Some(true));
        assert_eq!(cdc.force_rts, Some(false));
        let term = SerialConfig::serial_terminal(115_200);
        assert_eq!(term.force_dtr, None);
    }
}
