use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/js/web_serial_bridge.js")]
extern "C" {
    pub fn js_init_transport(
        on_state: &js_sys::Function,
        on_bytes: &js_sys::Function,
        cfg_json: &str,
        init_cmds: &[u8],
    );
    pub fn js_connect_usb(cfg_json: &str, init_cmds: &[u8]) -> js_sys::Promise;
    pub fn js_disconnect_usb() -> js_sys::Promise;
    pub fn js_write_bytes(bytes: &[u8]) -> js_sys::Promise;
    pub fn js_set_signals(dtr: bool, rts: bool) -> js_sys::Promise;
    pub fn js_pulse_reset() -> js_sys::Promise;
    pub fn js_reconfigure_serial(cfg_json: &str) -> js_sys::Promise;
    pub fn js_switch_usb_port(port_idx: u32) -> js_sys::Promise;
    pub fn js_set_auto_connect(enabled: bool, cfg_json: &str);
    #[allow(clippy::too_many_arguments)]
    pub fn js_flash_firmware(
        profile_json: &str,
        force_other_port: bool,
        flash_offset: &str,
        custom_file: Option<&web_sys::File>,
        bootloader_cmd: &[u8],
        init_cmds: &[u8],
        on_log: &js_sys::Function,
        on_progress: &js_sys::Function,
    ) -> js_sys::Promise;
}
