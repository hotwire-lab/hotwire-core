use leptos::prelude::*;

use crate::protocol::{BoardIdentity, ToolId};
use crate::transport::{ConnectionStatus, TransportKind, TransportSnapshot};

/// Shared HotWire top toolbar with tool switcher (`Logic / Scope / SDR / Serial`),
/// live connection & firmware-mismatch pill, app-specific action slot, and
/// Connect / Disconnect / Demo / Flasher controls.
#[component]
pub fn HotWireToolbar(
    active_tool: ToolId,
    #[prop(into)] subtitle: Signal<String>,
    transport: RwSignal<TransportSnapshot>,
    #[prop(optional)] identity: Option<RwSignal<Option<BoardIdentity>>>,
    on_connect: Callback<()>,
    on_disconnect: Callback<()>,
    on_start_demo: Callback<()>,
    flasher_open: RwSignal<bool>,
    on_toggle_flasher: Callback<()>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let is_active = move || transport.get().status.is_active();
    let is_connecting = move || transport.get().status == ConnectionStatus::Connecting;

    let is_mismatch = move || {
        let Some(id_sig) = identity else {
            return false;
        };
        let Some(id) = id_sig.get() else {
            return false;
        };
        transport.get().status.is_usb_connected() && id.validate(active_tool).is_err()
    };

    let status_class = move || {
        let base = format!("hw-status-pill hw-status-{}", transport.get().status.as_class());
        if is_mismatch() {
            format!("{base} hw-status-mismatch")
        } else {
            base
        }
    };

    let status_title = move || {
        let t = transport.get();
        if is_mismatch() {
            if let Some(id) = identity.and_then(|s| s.get()) {
                if let Err(mm) = id.validate(active_tool) {
                    return mm.summary();
                }
            }
        }
        t.error_message.unwrap_or(t.device_name)
    };

    let status_label = move || {
        let t = transport.get();
        if t.status.is_active() {
            if let Some(id) = identity.and_then(|s| s.get()) {
                if id.validate(active_tool).is_err() && t.status.is_usb_connected() {
                    if !id.matches_tool(active_tool) {
                        let other = id
                            .detected_tool
                            .map(|tool| tool.short_label())
                            .unwrap_or("Other");
                        format!("{} ({} FW!)", id.board, other)
                    } else {
                        format!("{} (Old FW!)", id.board)
                    }
                } else if t.transport == TransportKind::Demo {
                    format!("{} (Demo)", id.board)
                } else {
                    id.board
                }
            } else {
                t.device_name
            }
        } else {
            t.status.label().to_string()
        }
    };

    let accent = active_tool.accent_color();

    view! {
        <header class="hw-toolbar">
            <div class="hw-toolbar-row">
                <div class="hw-brand-cluster">
                    <div class="hw-brand-logo" aria-hidden="true">
                        <svg viewBox="0 0 24 24" width="20" height="20">
                            <path
                                d="M2 12 C5 4, 8 4, 11 12 C14 20, 17 20, 20 12 L22 12"
                                fill="none"
                                stroke=accent
                                stroke-width="2.2"
                                stroke-linecap="round"
                            />
                        </svg>
                    </div>
                    <div class="hw-brand-text">
                        <span class="hw-brand-title">{active_tool.label()}</span>
                        <span class="hw-brand-badge">{move || subtitle.get()}</span>
                    </div>
                </div>

                <nav class="hw-tool-switcher" aria-label="HotWire Tools">
                    {ToolId::ALL
                        .into_iter()
                        .map(|tool| {
                            let is_cur = tool == active_tool;
                            let cls = if is_cur {
                                "hw-tool-tab active"
                            } else {
                                "hw-tool-tab"
                            };
                            let dot_style = format!("background: {};", tool.accent_color());
                            let href = if is_cur { "#" } else { tool.live_url() };
                            view! {
                                <a
                                    class=cls
                                    href=href
                                    title=tool.label()
                                    aria-current=if is_cur { Some("page") } else { None }
                                    on:click=move |ev| {
                                        if is_cur {
                                            ev.prevent_default();
                                        }
                                    }
                                >
                                    <span class="hw-tool-dot" style=dot_style />
                                    <span>{tool.short_label()}</span>
                                </a>
                            }
                        })
                        .collect_view()}
                </nav>

                <div class=status_class title=status_title>
                    <span class="hw-status-dot" />
                    <span class="hw-status-label">{status_label}</span>
                    {move || {
                        let t = transport.get();
                        if t.status.is_active() && !t.vid_pid.is_empty() {
                            Some(view! { <span class="hw-status-vidpid">{t.vid_pid}</span> })
                        } else {
                            None
                        }
                    }}
                </div>

                <div class="hw-spacer" />

                <div class="hw-toolbar-actions">
                    {children.map(|c| c())}

                    {move || {
                        if is_active() {
                            view! {
                                <button
                                    class="hw-btn hw-btn-ghost"
                                    on:click=move |_| on_disconnect.run(())
                                >
                                    "Disconnect"
                                </button>
                            }
                            .into_any()
                        } else {
                            view! {
                                <button
                                    class="hw-btn hw-btn-primary"
                                    disabled=is_connecting
                                    on:click=move |_| on_connect.run(())
                                >
                                    {move || if is_connecting() { "Connecting..." } else { "Connect USB" }}
                                </button>
                                <button
                                    class="hw-btn hw-btn-accent"
                                    on:click=move |_| on_start_demo.run(())
                                    title="Run offline hardware simulator (no USB required)"
                                >
                                    "Demo"
                                </button>
                            }
                            .into_any()
                        }
                    }}

                    <button
                        class=move || {
                            if is_mismatch() {
                                "hw-btn hw-btn-warn"
                            } else if flasher_open.get() {
                                "hw-btn hw-btn-accent active"
                            } else {
                                "hw-btn"
                            }
                        }
                        on:click=move |_| on_toggle_flasher.run(())
                        title="Web Flasher & Pinout"
                    >
                        {move || if is_mismatch() { "⚡ Switch FW" } else { "⚡ Flasher" }}
                    </button>
                </div>
            </div>
        </header>
    }
}
