use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::flasher::{BoardId, FirmwareProfile};
use crate::protocol::{BoardIdentity, FirmwareMismatch, ToolId};
use crate::transport::HotWireTransport;

/// Reusable `esptool-js` Web Flasher panel that supports multiple board profiles,
/// automatic board selection from `0x83` capabilities, firmware mismatch alerts,
/// full/app-only flashing, and custom `.bin` upload.
#[component]
pub fn FlasherPanel(
    profiles: Vec<FirmwareProfile>,
    transport: StoredValue<HotWireTransport, LocalStorage>,
    #[prop(optional)] expected_tool: Option<ToolId>,
    #[prop(optional)] identity: Option<RwSignal<Option<BoardIdentity>>>,
    #[prop(optional)] mismatch: Option<RwSignal<Option<FirmwareMismatch>>>,
) -> impl IntoView {
    let profiles_stored = StoredValue::new(profiles);
    let initial_board = profiles_stored
        .with_value(|p| p.first().map(|x| x.board_id.clone()))
        .unwrap_or_else(|| BoardId::Esp32S3.as_str().to_string());

    let selected_board_id = RwSignal::new(initial_board);
    let flash_logs = RwSignal::new(Vec::<String>::new());
    let flash_progress = RwSignal::new(0u32);
    let flashing = RwSignal::new(false);
    let flash_error = RwSignal::new(None::<String>);
    let show_custom_bin = RwSignal::new(false);
    let selected_file_name = RwSignal::new(None::<String>);
    let selected_file = StoredValue::new_local(None::<web_sys::File>);
    let flash_offset = RwSignal::new("full".to_string());
    let log_div_ref = NodeRef::<html::Div>::new();

    // Auto-select matching board profile when capabilities handshake or mismatch detects the chip
    if let Some(id_sig) = identity {
        Effect::new(move |_| {
            if let Some(id) = id_sig.get() {
                if let Some(board) = BoardId::from_chip(id.chip) {
                    let target = board.as_str();
                    let has_profile = profiles_stored
                        .with_value(|list| list.iter().any(|p| p.board_id == target));
                    if has_profile {
                        selected_board_id.set(target.to_string());
                    }
                }
            }
        });
    }
    if let Some(mm_sig) = mismatch {
        Effect::new(move |_| {
            if let Some(mm) = mm_sig.get() {
                if let Some(board) = BoardId::from_chip(mm.chip) {
                    let target = board.as_str();
                    let has_profile = profiles_stored
                        .with_value(|list| list.iter().any(|p| p.board_id == target));
                    if has_profile {
                        selected_board_id.set(target.to_string());
                    }
                }
            }
        });
    }

    let mismatch_message = move || {
        if let Some(mm) = mismatch.and_then(|s| s.get()) {
            return Some(mm.summary());
        }
        let expected = expected_tool?;
        let id = identity?.get()?;
        match id.validate(expected) {
            Ok(()) => None,
            Err(mm) => Some(mm.summary()),
        }
    };

    let current_profile = move || {
        let target = selected_board_id.get();
        profiles_stored.with_value(|list| {
            list.iter()
                .find(|p| p.board_id == target)
                .cloned()
                .or_else(|| list.first().cloned())
        })
    };

    let start_flash = move |force_other_port: bool| {
        if flashing.get_untracked() {
            return;
        }
        let Some(profile) = current_profile() else {
            flash_error.set(Some("No firmware profile configured.".to_string()));
            return;
        };

        flashing.set(true);
        flash_error.set(None);
        if let Some(mm_sig) = mismatch {
            mm_sig.set(None);
        }
        flash_progress.set(0);
        flash_logs.set(Vec::new());

        let offset = flash_offset.get_untracked();
        let custom = selected_file.get_value();
        let tr = transport.get_value();

        spawn_local(async move {
            let res = tr
                .flash_firmware(
                    &profile,
                    force_other_port,
                    &offset,
                    custom.as_ref(),
                    move |line| {
                        flash_logs.update(|logs| {
                            logs.push(line);
                            if logs.len() > 140 {
                                let drain = logs.len() - 140;
                                logs.drain(0..drain);
                            }
                        });
                        if let Some(el) = log_div_ref.get_untracked() {
                            el.set_scroll_top(el.scroll_height());
                        }
                    },
                    move |pct| flash_progress.set(pct),
                )
                .await;

            if let Err(err) = res {
                flash_error.set(Some(format!(
                    "{err} (If USB re-enumerated, hold BOOT + tap RESET and click \"Select Other Port\")"
                )));
                flash_logs.update(|logs| logs.push(format!("ERROR: {err}")));
            }
            flashing.set(false);
        });
    };

    view! {
        <div class="hw-flash-col">
            {move || {
                mismatch_message().map(|msg| {
                    view! { <div class="hw-flash-warning">{msg}</div> }
                })
            }}

            <div class="hw-form-row">
                <label class="group-label">"Target Board"</label>
                <select
                    class="hw-select"
                    disabled=move || flashing.get()
                    prop:value=move || selected_board_id.get()
                    on:change=move |ev| {
                        selected_board_id.set(event_target_value(&ev));
                    }
                >
                    {move || {
                        let sel = selected_board_id.get();
                        profiles_stored.with_value(|list| {
                            list.iter()
                                .map(|p| {
                                    let val = p.board_id.clone();
                                    let label = p.label.clone();
                                    let is_sel = val == sel;
                                    view! {
                                        <option value=val selected=is_sel>{label}</option>
                                    }
                                })
                                .collect_view()
                        })
                    }}
                </select>
            </div>

            <p class="hw-flash-hint">
                {move || current_profile().map(|p| p.hint).unwrap_or_default()}
            </p>

            <div class="hw-form-row">
                <button
                    class="hw-btn hw-btn-primary"
                    disabled=move || flashing.get()
                    on:click=move |_| start_flash(false)
                >
                    {move || if flashing.get() { "Flashing..." } else { "⚡ Flash Firmware" }}
                </button>
                <button
                    class="hw-btn hw-btn-sm"
                    disabled=move || flashing.get()
                    on:click=move |_| start_flash(true)
                    title="Force browser USB/COM port picker (use after holding BOOT + tapping RESET)"
                >
                    "Select Other Port"
                </button>
                <button
                    class="hw-btn hw-btn-ghost hw-btn-sm"
                    disabled=move || flashing.get()
                    on:click=move |_| {
                        let next = !show_custom_bin.get_untracked();
                        show_custom_bin.set(next);
                        if !next {
                            selected_file.set_value(None);
                            selected_file_name.set(None);
                            flash_offset.set("full".to_string());
                        }
                    }
                >
                    {move || if show_custom_bin.get() { "Hide Custom .bin" } else { "Custom .bin..." }}
                </button>
            </div>

            {move || {
                if show_custom_bin.get() {
                    Some(view! {
                        <div class="hw-form-row">
                            <input
                                type="file"
                                accept=".bin"
                                disabled=move || flashing.get()
                                on:change=move |ev| {
                                    let file = ev
                                        .target()
                                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                        .and_then(|input| input.files())
                                        .and_then(|files| files.get(0));
                                    selected_file_name.set(file.as_ref().map(|f| f.name()));
                                    selected_file.set_value(file);
                                    if selected_file_name.get_untracked().is_some() {
                                        flash_offset.set("0x10000".to_string());
                                    }
                                }
                            />
                            <select
                                class="hw-select"
                                disabled=move || flashing.get()
                                prop:value=move || flash_offset.get()
                                on:change=move |ev| flash_offset.set(event_target_value(&ev))
                            >
                                <option value="full" selected=move || flash_offset.get() == "full">
                                    "Full Bundled Image"
                                </option>
                                <option value="0x10000" selected=move || flash_offset.get() == "0x10000">
                                    "App @ 0x10000"
                                </option>
                                <option value="0x0" selected=move || flash_offset.get() == "0x0">
                                    "Merged @ 0x0"
                                </option>
                            </select>
                        </div>
                    })
                } else {
                    None
                }
            }}

            {move || {
                selected_file_name.get().map(|name| {
                    view! {
                        <span class="hw-flash-hint">
                            {format!("Custom binary selected: {name}")}
                        </span>
                    }
                })
            }}

            <div class="hw-progress-bar">
                <div
                    class="hw-progress-fill"
                    style=move || format!("width: {}%;", flash_progress.get())
                />
            </div>

            {move || {
                flash_error.get().map(|err| {
                    view! { <div class="hw-flash-error">{err}</div> }
                })
            }}

            <div class="hw-flash-log" node_ref=log_div_ref>
                {move || {
                    let logs = flash_logs.get();
                    if logs.is_empty() {
                        view! {
                            <div class="hw-flash-hint">
                                "Plug in XIAO ESP32-S3 or ESP32-C6 and click \"Flash Firmware\" to program bundled firmware directly from Chrome/Edge."
                            </div>
                        }
                        .into_any()
                    } else {
                        logs.into_iter()
                            .map(|line| view! { <div>{line}</div> })
                            .collect_view()
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}
