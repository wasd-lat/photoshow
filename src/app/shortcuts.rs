//! Atalhos globais de teclado.
//!
//! Mesmos atalhos do egui: `←/→` navegam, `+ - 0` zoomam, `F2` renomeia,
//! `Ctrl+Z / Ctrl+Y` desfaz/refaz, `Enter` aplica crop, `F9` maximiza o
//! visualizador, `F11` fullscreen e `Esc` sai de fullscreen/crop.
//!
//! O `keyboard-types` separa o que é *tecla nomeada* (`NamedKey`) do que é
//! *posição física* (`Code`): `Z`, `+` e `0` chegam como `Code`, e é isso que
//! usamos para não confundir `+` do teclado numérico com `=`.
//!
//! Este arquivo roda dentro de um event handler, então usa os acessos sem
//! hook ([`state::station`], [`services`]) em vez de `use_radio`/`use_consume`.

use crate::prelude::*;

use super::services::{CropCommand, Services};
use super::state::{self, AppChannel};
use super::window;

/// Acesso aos serviços sem hook (dentro de handler).
#[must_use]
pub fn services() -> Services {
    consume_context::<Services>()
}

/// Handler global (montado no root para valer em qualquer painel).
pub fn global_shortcuts(e: Event<KeyboardEventData>) {
    if apply(&e) {
        e.stop_propagation();
    }
}

/// Aplica um atalho; `true` quando consumiu o evento.
pub fn apply(e: &Event<KeyboardEventData>) -> bool {
    let ctrl = e.modifiers.contains(Modifiers::CONTROL) || e.modifiers.contains(Modifiers::META);
    let shift = e.modifiers.contains(Modifiers::SHIFT);
    let services = services();
    let code = e.code;

    // Teclas nomeadas: função, setas, Escape, Enter.
    if let Key::Named(named) = &e.key {
        return named_shortcut(*named, services);
    }

    // Teclas de posição: ctrl+Z / ctrl+Y e os de zoom.
    if ctrl && code == Code::KeyZ {
        if shift {
            edit(|st| {
                state::redo(st, &services);
            });
        } else {
            edit(|st| {
                state::undo(st, &services);
            });
        }
        return true;
    }
    if ctrl && code == Code::KeyY {
        edit(|st| {
            state::redo(st, &services);
        });
        return true;
    }
    match code {
        Code::Minus | Code::NumpadSubtract => {
            zoom_by(1.0 / 1.2);
            true
        }
        Code::Equal | Code::NumpadAdd => {
            zoom_by(1.2);
            true
        }
        Code::Digit0 | Code::Numpad0 => {
            viewer(|st| {
                st.zoom = 1.0;
                st.offset = Vector2D::new(0.0, 0.0);
            });
            true
        }
        _ => false,
    }
}

/// Atalhos baseados em `NamedKey`.
fn named_shortcut(key: NamedKey, services: Services) -> bool {
    if key == NamedKey::Escape {
        return escape();
    }
    if key == NamedKey::F11 {
        return toggle_fullscreen();
    }
    if key == NamedKey::F9 {
        viewer(state::toggle_maximize);
        return true;
    }
    if key == NamedKey::F2 {
        state::update(AppChannel::Dialogs, state::open_rename);
        return true;
    }
    if key == NamedKey::ArrowRight {
        photos(|st| state::step(st, &services, 1));
        return true;
    }
    if key == NamedKey::ArrowLeft {
        photos(|st| state::step(st, &services, -1));
        return true;
    }
    if key == NamedKey::Enter {
        // O rect é estado local do viewer: só ele sabe convertê-lo em px.
        if state::snapshot().crop_mode {
            services.request_crop(CropCommand::Apply);
            return true;
        }
        return false;
    }
    false
}

/// `Esc`: fecha modal, sai de fullscreen ou cancela o crop (nesta ordem).
fn escape() -> bool {
    if state::snapshot().rename_open || state::snapshot().settings_open {
        state::update(AppChannel::Dialogs, |st| {
            st.rename_open = false;
            st.settings_open = false;
        });
        return true;
    }
    let fullscreen = state::snapshot().fullscreen;
    let crop_mode = state::snapshot().crop_mode;
    if fullscreen {
        viewer(|st| st.fullscreen = false);
        window::set_fullscreen(false);
        return true;
    }
    if crop_mode {
        viewer(state::exit_crop_mode);
        return true;
    }
    false
}

/// Multiplica o zoom, respeitando os limites.
fn zoom_by(factor: f32) {
    viewer(|st| {
        st.zoom = (st.zoom * factor).clamp(state::ZOOM_MIN, state::ZOOM_MAX);
    });
}

/// Alterna fullscreen, sincronizando o estado com a janela real.
pub fn toggle_fullscreen() -> bool {
    let next = !state::snapshot().fullscreen;
    viewer(|st| st.fullscreen = next);
    window::set_fullscreen(next);
    true
}

// Atalhos de escrita por canal, sem hook (chamados de handlers).
fn viewer(f: impl FnOnce(&mut state::AppState)) {
    state::update(AppChannel::Viewer, f);
}

fn photos(f: impl FnOnce(&mut state::AppState)) {
    state::update(AppChannel::Photos, f);
}

fn edit(f: impl FnOnce(&mut state::AppState)) {
    state::update(AppChannel::Edit, f);
}
