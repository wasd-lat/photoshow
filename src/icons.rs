//! Ícones: SVGs Lucide embutidos pelo crate `freya` (feature `icons-lucide`).
//!
//! Substituem o `egui-phosphor`: o Freya renderiza SVG nativo (Skia), então
//! os mesmos casos de uso são cobertos com `SvgViewer` + `theme_color()`.

use crate::prelude::*;
use freya::components::SvgViewer;
use freya::icons::lucide;

/// Tamanho padrão dos ícones da barra de ferramentas.
pub const ICON: f32 = 16.0;

/// Nomes de ícone reconhecidos (o fallback é `square`).
///
/// Fonte única de verdade: `source()` cobre todos, e o teste garante isso.
#[allow(dead_code)]
pub const NAMES: &[&str] = &[
    "check",
    "chevron-down",
    "chevron-right",
    "chevron-up",
    "clipboard",
    "columns-2",
    "copy",
    "crop",
    "external-link",
    "file-image",
    "folder",
    "folder-open",
    "fullscreen",
    "git-compare-arrows",
    "grid-2x2",
    "help-circle",
    "image-off",
    "info",
    "layout-dashboard",
    "maximize",
    "minimize",
    "panel-bottom",
    "panel-left",
    "pause",
    "pencil",
    "play",
    "plus",
    "redo",
    "refresh-cw",
    "rotate-ccw",
    "rotate-cw",
    "save",
    "scan",
    "search",
    "settings",
    "sliders-horizontal",
    "square",
    "star",
    "trash",
    "undo",
    "x",
];

/// Bytes do SVG para o nome; desconhecidos caem em `square`.
fn source(name: &str) -> (&'static str, Bytes) {
    match name {
        "folder" => ("folder", lucide::folder()),
        "folder-open" => ("folder-open", lucide::folder_open()),
        "file-image" => ("file-image", lucide::file_image()),
        "save" => ("save", lucide::save()),
        "pencil" => ("pencil", lucide::pencil()),
        "settings" => ("settings", lucide::settings()),
        "fullscreen" => ("fullscreen", lucide::fullscreen()),
        "minimize" => ("minimize", lucide::minimize()),
        "maximize" => ("maximize", lucide::maximize()),
        "star" => ("star", lucide::star()),
        "crop" => ("crop", lucide::crop()),
        "rotate-cw" => ("rotate-cw", lucide::rotate_cw()),
        "rotate-ccw" => ("rotate-ccw", lucide::rotate_ccw()),
        "undo" => ("undo", lucide::undo()),
        "redo" => ("redo", lucide::redo()),
        "check" => ("check", lucide::check()),
        "x" => ("x", lucide::x()),
        "plus" => ("plus", lucide::plus()),
        "chevron-right" => ("chevron-right", lucide::chevron_right()),
        "chevron-down" => ("chevron-down", lucide::chevron_down()),
        "chevron-up" => ("chevron-up", lucide::chevron_up()),
        "play" => ("play", lucide::play()),
        "pause" => ("pause", lucide::pause()),
        "info" => ("info", lucide::info()),
        "help-circle" => ("help-circle", lucide::circle_question_mark()),
        "search" => ("search", lucide::search()),
        "clipboard" => ("clipboard", lucide::clipboard()),
        "copy" => ("copy", lucide::copy()),
        "external-link" => ("external-link", lucide::external_link()),
        "image-off" => ("image-off", lucide::image_off()),
        "grid-2x2" => ("grid-2x2", lucide::grid_2x2()),
        "layout-dashboard" => ("layout-dashboard", lucide::layout_dashboard()),
        "panel-left" => ("panel-left", lucide::panel_left()),
        "panel-bottom" => ("panel-bottom", lucide::panel_bottom()),
        "sliders-horizontal" => ("sliders-horizontal", lucide::sliders_horizontal()),
        "trash" => ("trash", lucide::trash_2()),
        "columns-2" => ("columns-2", lucide::columns_2()),
        "git-compare-arrows" => ("git-compare-arrows", lucide::git_compare_arrows()),
        "scan" => ("scan", lucide::scan()),
        "refresh-cw" => ("refresh-cw", lucide::refresh_cw()),
        _ => ("square", lucide::square()),
    }
}

/// Monta um `SvgViewer` de ícone, herdando a cor do tema.
#[must_use]
pub fn icon(name: &str) -> SvgViewer {
    SvgViewer::new(source(name))
        .theme_color()
        .width(Size::px(ICON))
        .height(Size::px(ICON))
}

/// `SvgViewer` colorido fixamente (usado no placeholder de erro e no star).
#[must_use]
pub fn icon_tinted(name: &str, size: f32, color: Color) -> SvgViewer {
    SvgViewer::new(source(name))
        .color(color)
        .width(Size::px(size))
        .height(Size::px(size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_declared_name_resolves_to_its_own_svg() {
        let fallback = lucide::square();
        for name in NAMES.iter().filter(|n| **n != "square") {
            assert_ne!(source(name).1, fallback, "ícone sem SVG dedicado: {name}");
        }
    }

    #[test]
    fn unknown_name_falls_back_to_square() {
        assert_eq!(source("nao-existe"), ("square", lucide::square()));
    }

    #[test]
    fn declared_names_are_unique() {
        let mut sorted = NAMES.to_vec();
        sorted.sort_unstable();
        let count = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), count, "NAMES tem entrada duplicada");
    }
}
