use leptos::prelude::*;

use crate::protocol::{BoardIdentity, ToolId, HOTWIRE_HOME_URL};
use crate::transport::{ConnectionStatus, TransportKind, TransportSnapshot};

fn percent_encode_query(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                use std::fmt::Write;
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out
}

fn extract_ua_version<'a>(ua: &'a str, marker: &str) -> Option<&'a str> {
    let rest = ua.split(marker).nth(1)?;
    let ver = rest.split(|c: char| c.is_whitespace() || c == ';').next()?;
    ver.split('.').next().filter(|v| !v.is_empty())
}

/// Condenses a browser User-Agent string into a concise `Browser Version · OS` label
/// suitable for pre-filling the GitHub Bug Report `env` field.
pub fn summarize_user_agent(ua: &str) -> String {
    let browser = if let Some(v) =
        extract_ua_version(ua, "EdgA/").or_else(|| extract_ua_version(ua, "Edg/"))
    {
        format!("Edge {v}")
    } else if let Some(v) = extract_ua_version(ua, "OPR/") {
        format!("Opera {v}")
    } else if let Some(v) = extract_ua_version(ua, "Chrome/") {
        format!("Chrome {v}")
    } else if let Some(v) = extract_ua_version(ua, "Firefox/") {
        format!("Firefox {v}")
    } else if ua.contains("Safari/") && !ua.contains("Chrome/") {
        extract_ua_version(ua, "Version/")
            .map(|v| format!("Safari {v}"))
            .unwrap_or_else(|| "Safari".to_string())
    } else {
        String::new()
    };

    let os = if ua.contains("Android") {
        "Android"
    } else if ua.contains("iPhone") || ua.contains("iPad") {
        "iOS"
    } else if ua.contains("Windows") {
        "Windows"
    } else if ua.contains("Macintosh") || ua.contains("Mac OS X") {
        "macOS"
    } else if ua.contains("CrOS") {
        "ChromeOS"
    } else if ua.contains("Linux") {
        "Linux"
    } else {
        ""
    };

    match (browser.is_empty(), os.is_empty()) {
        (false, false) => format!("{browser} on {os}"),
        (false, true) => browser,
        (true, false) => os.to_string(),
        (true, true) => ua.chars().take(80).collect(),
    }
}

/// Builds a GitHub `bug_report.yml` URL pre-filled with the active tool, detected board,
/// firmware/transport metadata, and browser/OS summary.
pub fn build_bug_report_url(
    active_tool: ToolId,
    transport: &TransportSnapshot,
    identity: Option<&BoardIdentity>,
    user_agent: &str,
) -> String {
    let mut url = format!(
        "{}/issues/new?template=bug_report.yml&tool={}",
        active_tool.repo_url(),
        percent_encode_query(active_tool.issue_tool_option()),
    );

    if transport.transport == TransportKind::Demo {
        url.push_str("&board=");
        url.push_str(&percent_encode_query("Offline Demo mode (no hardware)"));
    } else if let Some(id) = identity {
        url.push_str("&board=");
        url.push_str(&percent_encode_query(id.chip.issue_board_option()));
    }

    let transport_label = match transport.transport {
        TransportKind::WebSerial => "Web Serial",
        TransportKind::WebUsb => "WebUSB",
        TransportKind::Demo => "Demo mode",
        TransportKind::None => "",
    };

    let fw_summary = match (identity, transport_label) {
        (Some(id), t) if !id.firmware.is_empty() && !t.is_empty() => {
            format!("{} ({t})", id.firmware)
        }
        (Some(id), _) if !id.firmware.is_empty() => id.firmware.clone(),
        (None, t) if !t.is_empty() && !transport.vid_pid.is_empty() => {
            format!("{t} ({})", transport.vid_pid)
        }
        (None, t) if !t.is_empty() => t.to_string(),
        _ => String::new(),
    };

    if !fw_summary.is_empty() {
        url.push_str("&firmware=");
        url.push_str(&percent_encode_query(&fw_summary));
    }

    let env = summarize_user_agent(user_agent);
    if !env.is_empty() {
        url.push_str("&env=");
        url.push_str(&percent_encode_query(&env));
    }

    url
}

/// Builds a GitHub `feature_request.yml` URL pre-filled with the active tool.
pub fn build_feature_request_url(active_tool: ToolId) -> String {
    format!(
        "{}/issues/new?template=feature_request.yml&tool={}",
        active_tool.repo_url(),
        percent_encode_query(active_tool.issue_tool_option()),
    )
}

fn current_user_agent() -> String {
    web_sys::window()
        .and_then(|w| w.navigator().user_agent().ok())
        .unwrap_or_default()
}

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
    #[prop(into)] flasher_open: Signal<bool>,
    on_toggle_flasher: Callback<()>,
    #[prop(optional)] app_version: Option<&'static str>,
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
        let base = format!(
            "hw-status-pill hw-status-{}",
            transport.get().status.as_class()
        );
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

    let mobile_menu_open = RwSignal::new(false);
    let about_open = RwSignal::new(false);
    let accent = active_tool.accent_color();
    let user_agent = StoredValue::new(current_user_agent());
    let browser_summary = user_agent.with_value(|ua| {
        let s = summarize_user_agent(ua);
        if s.is_empty() {
            "Unknown browser".to_string()
        } else {
            s
        }
    });
    let issue_url = move || {
        let t = transport.get();
        let id = identity.and_then(|s| s.get());
        user_agent.with_value(|ua| build_bug_report_url(active_tool, &t, id.as_ref(), ua))
    };
    let feature_url = build_feature_request_url(active_tool);
    let board_diag = move || {
        let t = transport.get();
        let transport_label = match t.transport {
            TransportKind::WebSerial => "Web Serial",
            TransportKind::WebUsb => "WebUSB",
            TransportKind::Demo => "Demo mode",
            TransportKind::None => "Disconnected",
        };
        if let Some(id) = identity.and_then(|s| s.get()) {
            if !id.firmware.is_empty() {
                format!("{} · {} ({transport_label})", id.board, id.firmware)
            } else {
                format!("{} ({transport_label})", id.board)
            }
        } else if !t.vid_pid.is_empty() {
            format!("{transport_label} ({})", t.vid_pid)
        } else {
            transport_label.to_string()
        }
    };

    view! {
        <header class=move || {
            if mobile_menu_open.get() {
                "hw-toolbar hw-menu-open"
            } else {
                "hw-toolbar"
            }
        }>
            {move || {
                if mobile_menu_open.get() {
                    Some(view! {
                        <div
                            class="hw-menu-backdrop"
                            on:click=move |_| mobile_menu_open.set(false)
                        />
                    })
                } else {
                    None
                }
            }}

            <div class="hw-toolbar-row">
                <a
                    class="hw-brand-cluster"
                    href=HOTWIRE_HOME_URL
                    title="HotWire Lab Home"
                >
                    <div class="hw-brand-logo" aria-hidden="true">
                        <svg viewBox="0 0 24 24" width="18" height="18">
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
                </a>

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

                <div class="hw-app-slot">
                    {children.map(|c| c())}
                </div>

                <div class="hw-mobile-drawer">
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
                                            mobile_menu_open.set(false);
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

                    <div class="hw-conn-actions">
                        {move || {
                            if is_active() {
                                view! {
                                    <button
                                        class="hw-btn hw-btn-ghost"
                                        on:click=move |_| {
                                            mobile_menu_open.set(false);
                                            on_disconnect.run(());
                                        }
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
                                        on:click=move |_| {
                                            mobile_menu_open.set(false);
                                            on_connect.run(());
                                        }
                                    >
                                        {move || if is_connecting() { "Connecting..." } else { "Connect USB" }}
                                    </button>
                                    <button
                                        class="hw-btn hw-btn-accent"
                                        disabled=is_connecting
                                        on:click=move |_| {
                                            mobile_menu_open.set(false);
                                            on_start_demo.run(());
                                        }
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
                            on:click=move |_| {
                                mobile_menu_open.set(false);
                                on_toggle_flasher.run(());
                            }
                            title="Web Flasher & Pinout"
                        >
                            {move || if is_mismatch() { "⚡ Switch FW" } else { "⚡ Flasher" }}
                        </button>

                        <button
                            type="button"
                            class="hw-btn hw-btn-ghost hw-btn-feedback"
                            title="About, diagnostics, bug reports & feature requests"
                            aria-label="About and help"
                            on:click=move |_| {
                                mobile_menu_open.set(false);
                                about_open.set(true);
                            }
                        >
                            <span aria-hidden="true">"?"</span>
                            <span class="hw-mobile-only">" About"</span>
                        </button>
                    </div>
                </div>

                <button
                    type="button"
                    class=move || {
                        if mobile_menu_open.get() {
                            "hw-btn hw-mobile-menu-btn active"
                        } else {
                            "hw-btn hw-mobile-menu-btn"
                        }
                    }
                    aria-label=move || if mobile_menu_open.get() { "Close menu" } else { "Open menu" }
                    aria-expanded=move || if mobile_menu_open.get() { "true" } else { "false" }
                    on:click=move |_| mobile_menu_open.update(|open| *open = !*open)
                >
                    <span class="hw-mobile-menu-icon">
                        {move || if mobile_menu_open.get() { "✕" } else { "☰" }}
                    </span>
                    {move || {
                        if !mobile_menu_open.get() && (is_mismatch() || !is_active()) {
                            Some(view! { <span class="hw-menu-dot" aria-hidden="true" /> })
                        } else {
                            None
                        }
                    }}
                </button>
            </div>

            {move || {
                transport
                    .get()
                    .error_message
                    .map(|err| view! { <div class="hw-toolbar-error" role="alert">{err}</div> })
            }}
        </header>

        <Show when=move || about_open.get()>
            <div
                class="hw-modal-backdrop"
                role="dialog"
                aria-modal="true"
                aria-labelledby="hw-about-title"
                on:click=move |_| about_open.set(false)
            >
                <div class="hw-modal hw-about-modal" on:click=move |ev| ev.stop_propagation()>
                    <div class="hw-about-header">
                        <div class="hw-brand-logo" aria-hidden="true">
                            <svg viewBox="0 0 24 24" width="18" height="18">
                                <path
                                    d="M2 12 C5 4, 8 4, 11 12 C14 20, 17 20, 20 12 L22 12"
                                    fill="none"
                                    stroke=accent
                                    stroke-width="2.2"
                                    stroke-linecap="round"
                                />
                            </svg>
                        </div>
                        <div class="hw-about-heading">
                            <h2 id="hw-about-title" class="hw-about-title">
                                {active_tool.label()}
                            </h2>
                            <span class="hw-about-version">
                                {format!("HotWire Lab · hotwire-core v{}", env!("CARGO_PKG_VERSION"))}
                            </span>
                        </div>
                        <button
                            type="button"
                            class="hw-btn hw-btn-ghost hw-btn-sm"
                            aria-label="Close about dialog"
                            on:click=move |_| about_open.set(false)
                        >
                            "✕"
                        </button>
                    </div>

                    <div class="hw-modal-body">
                        <p class="hw-modal-summary">
                            "Zero-install Rust/WASM USB bench instrument for Seeed Studio XIAO ESP32-S3 and ESP32-C6 via Web Serial & WebUSB."
                        </p>

                        <div class="hw-modal-meta">
                            <div class="hw-modal-row">
                                <span class="hw-modal-key">"Instrument"</span>
                                <span class="hw-modal-val">{active_tool.issue_tool_option()}</span>
                            </div>
                            <div class="hw-modal-row">
                                <span class="hw-modal-key">"Board / FW"</span>
                                <span class="hw-modal-val">{board_diag}</span>
                            </div>
                            <div class="hw-modal-row">
                                <span class="hw-modal-key">"Browser"</span>
                                <span class="hw-modal-val">{browser_summary.clone()}</span>
                            </div>
                            <div class="hw-modal-row">
                                <span class="hw-modal-key">"Core"</span>
                                <span class="hw-modal-val">{format!("v{}", env!("CARGO_PKG_VERSION"))}</span>
                            </div>
                            {app_version.map(|version| {
                                view! {
                                    <div class="hw-modal-row">
                                        <span class="hw-modal-key">"App"</span>
                                        <span class="hw-modal-val">{format!("v{version}")}</span>
                                    </div>
                                }
                            })}
                            <div class="hw-modal-row">
                                <span class="hw-modal-key">"Min FW"</span>
                                <span class="hw-modal-val">
                                    {format!("v{}+", active_tool.min_firmware_version())}
                                </span>
                            </div>
                        </div>

                        <div class="hw-about-links">
                            <a
                                class="hw-btn hw-about-link"
                                href=issue_url
                                target="_blank"
                                rel="noreferrer"
                            >
                                "Report Issue ↗"
                            </a>
                            <a
                                class="hw-btn hw-about-link"
                                href=feature_url.clone()
                                target="_blank"
                                rel="noreferrer"
                            >
                                "Request Feature ↗"
                            </a>
                            <a
                                class="hw-btn hw-about-link"
                                href=HOTWIRE_HOME_URL
                                target="_blank"
                                rel="noreferrer"
                            >
                                "HotWire Lab Web ↗"
                            </a>
                            <a
                                class="hw-btn hw-about-link"
                                href=active_tool.repo_url()
                                target="_blank"
                                rel="noreferrer"
                            >
                                "Source & FW ↗"
                            </a>
                        </div>

                        <p class="hw-about-credit">
                            "Created by Kari Vierimaa, Kempele · Open Source (Apache-2.0 / GPL-3.0 SDR FW). "
                            <a
                                class="hw-about-inline-link"
                                href="https://hotwire-lab.github.io/#get-hardware"
                                target="_blank"
                                rel="noreferrer"
                            >
                                "Get XIAO Hardware ↗"
                            </a>
                        </p>
                    </div>

                    <div class="hw-modal-actions hw-about-actions">
                        <span class="hw-about-hint">"Bug reports auto-fill board, FW & browser."</span>
                        <button
                            type="button"
                            class="hw-btn hw-btn-primary"
                            on:click=move |_| about_open.set(false)
                        >
                            "Close"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::BoardChip;

    #[test]
    fn summarizes_common_desktop_and_mobile_user_agents() {
        assert_eq!(
            summarize_user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0"
            ),
            "Edge 131 on Windows"
        );
        assert_eq!(
            summarize_user_agent(
                "Mozilla/5.0 (Linux; Android 15; Pixel 8) AppleWebKit/537.36 Chrome/132.0.0.0 Mobile Safari/537.36"
            ),
            "Chrome 132 on Android"
        );
    }

    #[test]
    fn builds_prefilled_bug_report_url_for_usb_and_demo() {
        let t = TransportSnapshot {
            transport: TransportKind::WebUsb,
            ..Default::default()
        };
        let id = BoardIdentity {
            schema: crate::protocol::CAPS_SCHEMA_VERSION,
            board: "XIAO ESP32-S3".to_string(),
            chip: BoardChip::Esp32S3,
            firmware: "v1.2.0".to_string(),
            version: None,
            declared_tool: Some(ToolId::LogicAnalyzer),
            detected_tool: Some(ToolId::LogicAnalyzer),
            raw: serde_json::Value::Null,
        };
        let url = build_bug_report_url(
            ToolId::LogicAnalyzer,
            &t,
            Some(&id),
            "Mozilla/5.0 (Linux; Android 15) Chrome/132.0.0.0",
        );
        assert!(url.starts_with(
            "https://github.com/hotwire-lab/logic/issues/new?template=bug_report.yml"
        ));
        assert!(url.contains("&tool=Logic+Analyzer"));
        assert!(url.contains("&board=Seeed+Studio+XIAO+ESP32-S3"));
        assert!(url.contains("&firmware=v1.2.0+%28WebUSB%29"));
        assert!(url.contains("&env=Chrome+132+on+Android"));

        let demo = TransportSnapshot {
            transport: TransportKind::Demo,
            ..Default::default()
        };
        let demo_url = build_bug_report_url(ToolId::Sdr, &demo, None, "");
        assert!(demo_url
            .starts_with("https://github.com/hotwire-lab/sdr/issues/new?template=bug_report.yml"));
        assert!(demo_url.contains("&board=Offline+Demo+mode+%28no+hardware%29"));

        let feat_url = build_feature_request_url(ToolId::Oscilloscope);
        assert_eq!(
            feat_url,
            "https://github.com/hotwire-lab/scope/issues/new?template=feature_request.yml&tool=Oscilloscope"
        );
    }
}
