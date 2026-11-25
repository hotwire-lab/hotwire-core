use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Mobile-friendly draggable bottom sheet (drawer) with backdrop click-to-close
/// and swipe-down-to-dismiss handle.
#[component]
pub fn BottomSheet(
    open: RwSignal<bool>,
    on_close: Callback<()>,
    #[prop(into)] title: Signal<String>,
    children: Children,
) -> impl IntoView {
    let dragging = RwSignal::new(false);
    let drag_start_y = RwSignal::new(0.0f64);
    let drag_offset_y = RwSignal::new(0.0f64);

    let on_drag_start = move |ev: web_sys::PointerEvent| {
        if let Some(el) = ev
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = el.set_pointer_capture(ev.pointer_id());
        }
        dragging.set(true);
        drag_start_y.set(ev.client_y() as f64);
        drag_offset_y.set(0.0);
    };

    let on_drag_move = move |ev: web_sys::PointerEvent| {
        if !dragging.get_untracked() {
            return;
        }
        let dy = (ev.client_y() as f64 - drag_start_y.get_untracked()).max(0.0);
        drag_offset_y.set(dy);
    };

    let finish_drag = move || {
        if !dragging.get_untracked() {
            return;
        }
        dragging.set(false);
        let dy = drag_offset_y.get_untracked();
        drag_offset_y.set(0.0);
        if dy > 72.0 {
            on_close.run(());
        }
    };

    let sheet_style = move || {
        let dy = drag_offset_y.get();
        if dy > 0.5 {
            format!("transform: translateY({dy:.1}px); transition: none;")
        } else {
            String::new()
        }
    };

    view! {
        <div
            class=move || if open.get() { "hw-sheet-backdrop open" } else { "hw-sheet-backdrop" }
            on:click=move |_| on_close.run(())
        >
            <div
                class="hw-sheet"
                style=sheet_style
                on:click=move |ev| ev.stop_propagation()
                on:pointermove=on_drag_move
                on:pointerup=move |_| finish_drag()
                on:pointercancel=move |_| finish_drag()
            >
                <div
                    class="hw-sheet-drag-zone"
                    title="Drag down to close"
                    on:pointerdown=on_drag_start
                >
                    <div class="hw-sheet-handle" />
                </div>
                <div class="hw-sheet-header">
                    <span class="hw-sheet-title">{move || title.get()}</span>
                    <button
                        class="hw-btn hw-btn-ghost hw-btn-sm"
                        on:click=move |_| on_close.run(())
                    >
                        "✕ Close"
                    </button>
                </div>
                <div class="hw-sheet-body">{children()}</div>
            </div>
        </div>
    }
}
