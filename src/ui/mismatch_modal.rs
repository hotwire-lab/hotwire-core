use leptos::prelude::*;

use crate::protocol::{FirmwareMismatch, FirmwareMismatchKind};

/// Centered pop-up dialog shown when a connected USB device has the wrong target
/// HotWire software, an outdated firmware version, or fails the `0x83` handshake.
///
/// The calling app automatically disconnects the USB transport before showing this modal.
#[component]
pub fn FirmwareMismatchModal(
    mismatch: RwSignal<Option<FirmwareMismatch>>,
    on_open_flasher: Callback<()>,
) -> impl IntoView {
    view! {
        {move || {
            let m = mismatch.get()?;
            let title = m.title();
            let summary = m.summary();
            let board_label = m.board.clone();
            let detected_sw = match m.kind {
                FirmwareMismatchKind::WrongTargetSw {
                    detected_tool: Some(t),
                } => format!("{} ({})", t.label(), m.firmware_raw),
                FirmwareMismatchKind::WrongTargetSw {
                    detected_tool: None,
                } => format!("Unknown ({})", m.firmware_raw),
                FirmwareMismatchKind::VersionTooOld { .. } => {
                    format!("{} ({})", m.expected_tool.label(), m.firmware_raw)
                }
                FirmwareMismatchKind::NoHandshakeResponse => {
                    "No HotWire handshake (0x83)".to_string()
                }
            };
            let required_sw = format!(
                "{} (v{}+)",
                m.expected_tool.label(),
                m.required_version
            );
            let flash_cta = format!("⚡ Flash {} Firmware", m.expected_tool.short_label());
            let switch_tool = m
                .detected_tool()
                .filter(|&other| other != m.expected_tool);

            Some(view! {
                <div
                    class="hw-modal-backdrop"
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="hw-mismatch-title"
                    on:click=move |_| mismatch.set(None)
                >
                    <div
                        class="hw-modal"
                        on:click=move |ev| ev.stop_propagation()
                    >
                        <div class="hw-modal-header">
                            <span class="hw-modal-icon" aria-hidden="true">"⚠"</span>
                            <h3 id="hw-mismatch-title" class="hw-modal-title">{title}</h3>
                            <button
                                type="button"
                                class="hw-btn hw-btn-ghost hw-btn-sm"
                                on:click=move |_| mismatch.set(None)
                                aria-label="Close"
                            >
                                "✕"
                            </button>
                        </div>

                        <div class="hw-modal-body">
                            <p class="hw-modal-summary">{summary}</p>

                            <div class="hw-modal-meta">
                                <div class="hw-modal-row">
                                    <span class="hw-modal-key">"Device"</span>
                                    <span class="hw-modal-val">{board_label}</span>
                                </div>
                                <div class="hw-modal-row">
                                    <span class="hw-modal-key">"Detected"</span>
                                    <span class="hw-modal-val hw-modal-val-warn">{detected_sw}</span>
                                </div>
                                <div class="hw-modal-row">
                                    <span class="hw-modal-key">"Required"</span>
                                    <span class="hw-modal-val hw-modal-val-ok">{required_sw}</span>
                                </div>
                            </div>

                            <p class="hw-modal-note">
                                "USB connection was disconnected automatically. Click below to flash the matching firmware directly from your browser."
                            </p>
                        </div>

                        <div class="hw-modal-actions">
                            <button
                                type="button"
                                class="hw-btn hw-btn-primary"
                                on:click=move |_| {
                                    on_open_flasher.run(());
                                }
                            >
                                {flash_cta}
                            </button>

                            {switch_tool.map(|other| {
                                let label = format!("Open {} ↗", other.short_label());
                                view! {
                                    <a
                                        class="hw-btn hw-btn-accent"
                                        href=other.live_url()
                                        style="text-decoration: none;"
                                    >
                                        {label}
                                    </a>
                                }
                            })}

                            <button
                                type="button"
                                class="hw-btn hw-btn-ghost"
                                on:click=move |_| mismatch.set(None)
                            >
                                "Dismiss"
                            </button>
                        </div>
                    </div>
                </div>
            })
        }}
    }
}
