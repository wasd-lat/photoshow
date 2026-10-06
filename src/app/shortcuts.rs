//! Atalhos globais de teclado.
//!
//! Mesmos atalhos do egui: `←/→` navegam, `+ - 0` zoomam, `F2` renomeia,
//! `Ctrl+Z / Ctrl+Y` desfaz/refaz, `Enter` aplica crop, `F9` maximiza o
//! visualizador, `F11` fullscreen e `Esc` sai de fullscreen/crop.
//! `Ctrl+O` abre arquivos e `Ctrl+Shift+O` abre pasta — o mesmo par de
//! comandos que qualquer visualizador de fotos usa.
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
use super::{toolbar, window};

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

    // Abrir: Ctrl+O (arquivos) e Ctrl+Shift+O (pasta).
    if ctrl && code == Code::KeyO {
        if shift {
            toolbar::open_folder_dialog(services.clone());
        } else {
            toolbar::open_files_dialog(services.clone());
        }
        return true;
    }

    // Espaço: alternar slideshow
    if code == Code::Space && !ctrl && !shift {
        state::update(AppChannel::Photos, |st| {
            state::toggle_slideshow(st);
        });
        return true;
    }

    if ctrl && code == Code::KeyI {
        state::update(AppChannel::Dialogs, |st| st.exif_open = true);
        return true;
    }

    // Painéis: Ctrl+1 / Ctrl+2 / Ctrl+3 recolhem navegador, galeria e ajustes.
    // Ctrl+Shift+1 / Ctrl+Shift+2 recolhem árvore e lista de fotos.
    if ctrl && code == Code::Digit1 {
        if shift {
            toggle_panel(Panel::Tree);
        } else {
            toggle_panel(Panel::Browser);
        }
        return true;
    }
    if ctrl && code == Code::Digit2 {
        if shift {
            toggle_panel(Panel::Photos);
        } else {
            toggle_panel(Panel::Gallery);
        }
        return true;
    }
    if ctrl && code == Code::Digit3 {
        toggle_panel(Panel::Adjust);
        return true;
    }
    // Comparador: Ctrl+B alterna original × editado.
    if ctrl && code == Code::KeyB && !shift {
        toggle_compare(&services);
        return true;
    }
    if ctrl && code == Code::Digit0 {
        restore_panels();
        return true;
    }
    // Escala tipográfica: Ctrl+= / Ctrl+- (com shift, menor).
    if ctrl && matches!(code, Code::Equal | Code::NumpadAdd) {
        if shift {
            scale_ui(-crate::config::UI_SCALE_STEP);
        } else {
            scale_ui(crate::config::UI_SCALE_STEP);
        }
        return true;
    }
    if ctrl && matches!(code, Code::Minus | Code::NumpadSubtract) {
        scale_ui(-crate::config::UI_SCALE_STEP);
        return true;
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
    if !ctrl && !shift && !state::snapshot().crop_mode {
        match code {
            Code::Digit1 | Code::Numpad1 => {
                state::update(AppChannel::Photos, |st| state::rate_current_photo(st, 1));
                return true;
            }
            Code::Digit2 | Code::Numpad2 => {
                state::update(AppChannel::Photos, |st| state::rate_current_photo(st, 2));
                return true;
            }
            Code::Digit3 | Code::Numpad3 => {
                state::update(AppChannel::Photos, |st| state::rate_current_photo(st, 3));
                return true;
            }
            Code::Digit4 | Code::Numpad4 => {
                state::update(AppChannel::Photos, |st| state::rate_current_photo(st, 4));
                return true;
            }
            Code::Digit5 | Code::Numpad5 => {
                state::update(AppChannel::Photos, |st| state::rate_current_photo(st, 5));
                return true;
            }
            _ => {}
        }
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
            if !ctrl && !shift && !state::snapshot().crop_mode {
                state::update(AppChannel::Photos, |st| state::rate_current_photo(st, 0));
            }
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
    if key == NamedKey::F1 {
        state::update(AppChannel::Dialogs, |st| st.help_open = true);
        return true;
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
    if key == NamedKey::Delete {
        photos(|st| state::delete_current_photo(st, &services));
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

/// Painel recolhível da janela principal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    /// Navegador lateral (inteiro).
    Browser,
    /// Galeria inferior.
    Gallery,
    /// Árvore de pastas/favoritas (topo do navegador).
    Tree,
    /// Lista de fotos (baixo do navegador).
    Photos,
    /// Painel de ajustes (histograma + sliders).
    Adjust,
}

/// Recolhe/expande um painel e persiste (função pura de decisão).
///
/// Separado da escrita para ser testável sem janela: ver os testes no fim.
#[must_use]
pub fn next_visibility(current: bool) -> bool {
    !current
}

/// Recolhe/expande um painel.
pub fn toggle_panel(panel: Panel) {
    state::update(AppChannel::Config, |st| match panel {
        Panel::Browser => st.config.hide_browser = next_visibility(st.config.hide_browser),
        Panel::Gallery => st.config.hide_gallery = next_visibility(st.config.hide_gallery),
        Panel::Tree => st.config.hide_tree = next_visibility(st.config.hide_tree),
        Panel::Photos => st.config.hide_photos = next_visibility(st.config.hide_photos),
        Panel::Adjust => st.config.hide_adjust = next_visibility(st.config.hide_adjust),
    });
    state::update(AppChannel::Config, |st| {
        st.config.save().ok();
    });
}

/// Mostra os dois painéis de volta (Ctrl+0 e "Restaurar layout").
pub fn restore_panels() {
    state::update(AppChannel::Config, |st| {
        st.config.hide_browser = false;
        st.config.hide_gallery = false;
        st.config.hide_tree = false;
        st.config.hide_photos = false;
        st.config.hide_adjust = false;
        st.config.save().ok();
    });
}

/// Ajusta a escala tipográfica e persiste.
pub fn scale_ui(delta: f32) {
    state::update(AppChannel::Config, |st| {
        st.config.ui_scale = crate::config::sanitize_ui_scale(st.config.ui_scale + delta);
        st.config.save().ok();
    });
}

/// `Esc`: fecha modal, sai de apresentação, fullscreen ou cancela o crop.
fn escape() -> bool {
    let mut dialog_closed = false;
    state::update(AppChannel::Dialogs, |st| {
        if st.rename_open || st.settings_open || st.help_open || st.exif_open {
            st.rename_open = false;
            st.settings_open = false;
            st.help_open = false;
            st.exif_open = false;
            dialog_closed = true;
        }
    });
    if dialog_closed {
        return true;
    }
    let presentation = state::snapshot().presentation_mode;
    if presentation {
        state::update(AppChannel::Photos, |st| st.presentation_mode = false);
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

/// Alterna o comparador original × editado (botão e atalho `Ctrl+B`).
///
/// Mantida aqui — e não em dois handlers — porque o toggle mexe em dois
/// lugares de uma vez (o flag do `Viewer` e a textura no `ImageStore`), e
/// duas cópias da mesma sequência divergem na primeira correção.
pub fn toggle_compare(services: &Services) {
    let mut radio = state::station().write_channel(AppChannel::Viewer);
    let on = state::toggle_compare(&mut radio);
    drop(radio);
    services.images.set_compare(on);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_toggles_both_ways() {
        assert!(next_visibility(false));
        assert!(!next_visibility(true));
    }

    #[test]
    fn scale_delta_clamps_at_both_ends() {
        // Comportamento equivalente ao que o handler faz, sem janela.
        let mut scale = 1.0_f32;
        for _ in 0..40 {
            scale = crate::config::sanitize_ui_scale(scale + crate::config::UI_SCALE_STEP);
        }
        assert_eq!(scale, crate::config::UI_SCALE_MAX);
        for _ in 0..40 {
            scale = crate::config::sanitize_ui_scale(scale - crate::config::UI_SCALE_STEP);
        }
        assert_eq!(scale, crate::config::UI_SCALE_MIN);
    }

    #[test]
    fn panel_enum_is_distinct() {
        assert_ne!(Panel::Browser, Panel::Gallery);
    }
}
