pub mod gestures;

#[cfg(feature = "ui")]
pub mod bottom_sheet;
#[cfg(feature = "ui")]
pub mod mismatch_modal;
#[cfg(feature = "ui")]
pub mod toolbar;

pub use gestures::{apply_anchored_zoom, apply_horizontal_pan, GestureDelta, PinchPanTracker, TouchPoint};

#[cfg(feature = "ui")]
pub use bottom_sheet::BottomSheet;
#[cfg(feature = "ui")]
pub use mismatch_modal::FirmwareMismatchModal;
#[cfg(feature = "ui")]
pub use toolbar::HotWireToolbar;

/// Raw shared CSS palette and base component styles for HotWire web apps.
pub const CORE_CSS: &str = include_str!("../../styles/core.css");

/// Injects the shared HotWire dark theme and component stylesheet (`styles/core.css`)
/// into the document. Place once at the root of your Leptos `<App />`.
#[cfg(feature = "ui")]
#[leptos::prelude::component]
pub fn HotWireTheme() -> impl leptos::prelude::IntoView {
    use leptos::prelude::*;
    view! {
        <style id="hotwire-core-theme">{CORE_CSS}</style>
    }
}
