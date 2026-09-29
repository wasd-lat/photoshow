//! Ponte com a janela do SO (winit): fullscreen.
//!
//! O egui usava `ViewportCommand::Fullscreen`; no Freya o equivalente é
//! pedir ao `winit` via `WinitPlatformExt::with_window`.

use crate::prelude::*;
use freya::winit::window::{Fullscreen, Window};

/// Entra/sai de fullscreen (borderless, o que maximiza o aproveitamento).
pub fn set_fullscreen(enabled: bool) {
    let window_id = Platform::window_id();
    Platform::get().with_window(window_id, move |window| {
        apply_fullscreen(window, enabled);
    });
}

/// Aplica o modo direto numa `Window` (usado nos testes e no botão).
pub fn apply_fullscreen(window: &mut Window, enabled: bool) {
    let mode = if enabled {
        Some(Fullscreen::Borderless(None))
    } else {
        None
    };
    window.set_fullscreen(mode);
}
