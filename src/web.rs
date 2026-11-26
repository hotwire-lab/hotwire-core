//! Small browser helpers shared by all HotWire PWAs.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Wall-clock milliseconds since the Unix epoch (for buffer timestamps).
pub fn now_ms() -> u64 {
    js_sys::Date::now() as u64
}

/// High-resolution monotonic timestamp from `performance.now()`.
pub fn perf_now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or_else(js_sys::Date::now)
}

/// Caps the device pixel ratio used for instrument canvases.
///
/// Rendering at 3× on high-DPI phones triples canvas memory and fill cost with
/// little visible benefit for 1–2 px traces. All HotWire canvases use the same
/// cap so rendering cost is predictable across instruments.
pub fn capped_dpr(dpr: f64) -> f64 {
    if dpr.is_finite() {
        dpr.clamp(1.0, 2.5)
    } else {
        1.0
    }
}

/// Reads a value from `localStorage` (returns `None` in private mode / no storage).
pub fn local_storage_get(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(key).ok().flatten())
}

/// Writes a value to `localStorage`, ignoring quota/privacy errors.
pub fn local_storage_set(key: &str, value: &str) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item(key, value);
    }
}

/// Reads the current window's device pixel ratio with the shared cap applied.
pub fn capped_device_pixel_ratio() -> f64 {
    web_sys::window()
        .map(|w| capped_dpr(w.device_pixel_ratio()))
        .unwrap_or(1.0)
}

/// Triggers a browser download for UTF-8 text (CSV/VCD/log export).
pub fn download_text_file(filename: &str, content: &str, mime_type: &str) {
    let Some(win) = web_sys::window() else {
        return;
    };
    let Some(doc) = win.document() else {
        return;
    };
    let arr = js_sys::Array::new();
    arr.push(&content.into());
    let bag = web_sys::BlobPropertyBag::new();
    bag.set_type(mime_type);
    let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&arr, &bag) else {
        return;
    };
    download_blob(&win, &doc, &blob, filename);
}

/// Triggers a browser download for raw bytes (e.g. binary captures).
pub fn download_bytes(filename: &str, bytes: &[u8], mime_type: &str) {
    let Some(win) = web_sys::window() else {
        return;
    };
    let Some(doc) = win.document() else {
        return;
    };
    let arr = js_sys::Array::new();
    arr.push(&js_sys::Uint8Array::from(bytes).into());
    let bag = web_sys::BlobPropertyBag::new();
    bag.set_type(mime_type);
    let Ok(blob) = web_sys::Blob::new_with_buffer_source_sequence_and_options(&arr, &bag) else {
        return;
    };
    download_blob(&win, &doc, &blob, filename);
}

fn download_blob(
    win: &web_sys::Window,
    doc: &web_sys::Document,
    blob: &web_sys::Blob,
    filename: &str,
) {
    let Ok(url) = web_sys::Url::create_object_url_with_blob(blob) else {
        return;
    };
    if let Ok(el) = doc.create_element("a") {
        if let Ok(link) = el.dyn_into::<web_sys::HtmlAnchorElement>() {
            link.set_href(&url);
            link.set_download(filename);
            if let Some(body) = doc.body() {
                let _ = body.append_child(&link);
                link.click();
                link.remove();
            }
        }
    }
    let url_clone = url.clone();
    let cb = Closure::once_into_js(move || {
        let _ = web_sys::Url::revoke_object_url(&url_clone);
    });
    let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 1000);
}
