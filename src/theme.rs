//! Temas visuais (Freya) — substituem os temas egui-elegance.
//!
//! Os quatro nomes do egui (slate, charcoal, frost, paper) são preservados
//! para não quebrar configs já gravados. Cada um tem uma [`Palette`] com
//! dados puros (testável sem janela) e um mapeamento para a `Theme` do Freya.

use crate::prelude::*;
use freya::components::{ColorsSheet, DARK_COLORS, LIGHT_COLORS, Theme, dark_theme, light_theme};

/// Temas disponíveis (nomes persistidos no config).
pub const THEMES: &[&str] = &["slate", "charcoal", "frost", "paper"];

/// Cor em RGB puro, sem passar pelo `Theme` (para poder testar a palette).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl From<Rgb> for Color {
    fn from(c: Rgb) -> Self {
        Color::from_rgb(c.0, c.1, c.2)
    }
}

/// Paleta de um tema: so as cores que o photoshow sobrescreve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// Cor de destaque (botoes, selecao, foco).
    pub primary: Rgb,
    /// Cor de destaque secundária.
    pub secondary: Rgb,
    /// Cor de destaque terciária.
    pub tertiary: Rgb,
    /// Fundo da janela.
    pub background: Rgb,
    /// Superfície de widgets em repouso.
    pub surface_primary: Rgb,
    /// Superfície de widgets ao passar o mouse.
    pub surface_secondary: Rgb,
    /// Superfície de containers (browser, galeria).
    pub surface_tertiary: Rgb,
    /// Superfície invertida (sobre fundo escuro).
    pub surface_inverse: Rgb,
    /// Variante intermediária da superfície invertida.
    pub surface_inverse_secondary: Rgb,
    /// Variante terciária da superfície invertida.
    pub surface_inverse_tertiary: Rgb,
    /// Bordas.
    pub border: Rgb,
    /// Borda de foco.
    pub border_focus: Rgb,
    /// Texto principal.
    pub text_primary: Rgb,
    /// Texto secundário.
    pub text_secondary: Rgb,
    /// Texto de placeholder.
    pub text_placeholder: Rgb,
    /// Texto sobre superfície invertida.
    pub text_inverse: Rgb,
    /// Realce de texto (links, seleção).
    pub text_highlight: Rgb,
    /// Anel de foco.
    pub focus: Rgb,
    /// Estado pressionado.
    pub active: Rgb,
    /// Estado desabilitado.
    pub disabled: Rgb,
}

// --- Paletas ---
//
// Todos os quatro temas são neutros e de baixo contraste: a hierarquia vem de
// borda de 1px e de espaço em branco, não de sombra nem de cor saturada. O
// acento é quase preto no claro e quase branco no escuro, como nos apps
// atuais da OpenAI.
//
// Nomes preservados do egui-elegance para não invalidar configs já gravados.

/// Slate: padrão, neutro escuro levemente azulado.
pub const SLATE: Palette = Palette {
    primary: Rgb(247, 247, 248),
    secondary: Rgb(190, 192, 196),
    tertiary: Rgb(120, 122, 128),
    background: Rgb(20, 20, 21),
    surface_primary: Rgb(37, 37, 39),
    surface_secondary: Rgb(45, 45, 48),
    surface_tertiary: Rgb(28, 28, 30),
    surface_inverse: Rgb(13, 13, 14),
    surface_inverse_secondary: Rgb(40, 40, 42),
    surface_inverse_tertiary: Rgb(64, 64, 68),
    border: Rgb(54, 54, 58),
    border_focus: Rgb(247, 247, 248),
    text_primary: Rgb(242, 242, 243),
    text_secondary: Rgb(158, 158, 164),
    text_placeholder: Rgb(150, 150, 156),
    text_inverse: Rgb(20, 20, 21),
    text_highlight: Rgb(247, 247, 248),
    focus: Rgb(120, 120, 126),
    active: Rgb(58, 58, 62),
    disabled: Rgb(78, 78, 82),
};

/// Charcoal: neutro puro, sem viés de cor.
pub const CHARCOAL: Palette = Palette {
    primary: Rgb(250, 250, 250),
    secondary: Rgb(184, 184, 184),
    tertiary: Rgb(112, 112, 112),
    background: Rgb(18, 18, 18),
    surface_primary: Rgb(35, 35, 35),
    surface_secondary: Rgb(44, 44, 44),
    surface_tertiary: Rgb(26, 26, 26),
    surface_inverse: Rgb(12, 12, 12),
    surface_inverse_secondary: Rgb(38, 38, 38),
    surface_inverse_tertiary: Rgb(62, 62, 62),
    border: Rgb(52, 52, 52),
    border_focus: Rgb(250, 250, 250),
    text_primary: Rgb(245, 245, 245),
    text_secondary: Rgb(160, 160, 160),
    text_placeholder: Rgb(150, 150, 156),
    text_inverse: Rgb(18, 18, 18),
    text_highlight: Rgb(250, 250, 250),
    focus: Rgb(118, 118, 118),
    active: Rgb(56, 56, 56),
    disabled: Rgb(76, 76, 76),
};

/// Frost: claro, quase branco.
pub const FROST: Palette = Palette {
    primary: Rgb(13, 13, 13),
    secondary: Rgb(120, 120, 124),
    tertiary: Rgb(64, 64, 68),
    background: Rgb(252, 252, 253),
    surface_primary: Rgb(243, 243, 245),
    surface_secondary: Rgb(235, 235, 238),
    surface_tertiary: Rgb(250, 250, 251),
    surface_inverse: Rgb(23, 23, 24),
    surface_inverse_secondary: Rgb(56, 56, 58),
    surface_inverse_tertiary: Rgb(88, 88, 92),
    border: Rgb(228, 228, 231),
    border_focus: Rgb(13, 13, 13),
    text_primary: Rgb(16, 16, 18),
    text_secondary: Rgb(104, 104, 110),
    text_placeholder: Rgb(104, 104, 110),
    text_inverse: Rgb(252, 252, 253),
    text_highlight: Rgb(13, 13, 13),
    focus: Rgb(150, 150, 154),
    active: Rgb(226, 226, 230),
    disabled: Rgb(196, 196, 200),
};

/// Paper: claro, levemente quente.
pub const PAPER: Palette = Palette {
    primary: Rgb(26, 24, 22),
    secondary: Rgb(126, 122, 116),
    tertiary: Rgb(70, 66, 62),
    background: Rgb(251, 250, 248),
    surface_primary: Rgb(243, 241, 238),
    surface_secondary: Rgb(234, 231, 227),
    surface_tertiary: Rgb(249, 248, 245),
    surface_inverse: Rgb(26, 24, 21),
    surface_inverse_secondary: Rgb(58, 55, 50),
    surface_inverse_tertiary: Rgb(90, 86, 80),
    border: Rgb(229, 226, 221),
    border_focus: Rgb(26, 24, 22),
    text_primary: Rgb(24, 22, 20),
    text_secondary: Rgb(108, 104, 98),
    text_placeholder: Rgb(106, 102, 96),
    text_inverse: Rgb(251, 250, 248),
    text_highlight: Rgb(26, 24, 22),
    focus: Rgb(154, 150, 144),
    active: Rgb(228, 225, 220),
    disabled: Rgb(198, 195, 190),
};

/// Paleta pelo nome salvo no config; desconhecido cai em `slate`.
#[must_use]
pub fn palette(name: &str) -> Palette {
    match name {
        "charcoal" => CHARCOAL,
        "frost" => FROST,
        "paper" => PAPER,
        _ => SLATE,
    }
}

/// O tema é escuro?
#[must_use]
pub fn is_dark(name: &str) -> bool {
    matches!(name, "slate" | "charcoal" | "")
}

/// Constrói a `Theme` do Freya a partir da paleta.
///
/// Precisa de um contexto Freya ativo (o `Theme` lê a cor de destaque do SO).
#[must_use]
pub fn build(name: &str) -> Theme {
    let p = palette(name);
    let mut theme = if is_dark(name) {
        dark_theme()
    } else {
        light_theme()
    };
    let base = if is_dark(name) {
        &DARK_COLORS
    } else {
        &LIGHT_COLORS
    };
    theme.name = THEMES
        .iter()
        .copied()
        .find(|n| *n == name)
        .unwrap_or("slate");
    theme.colors = ColorsSheet {
        primary: p.primary.into(),
        secondary: p.secondary.into(),
        tertiary: p.tertiary.into(),
        background: p.background.into(),
        surface_primary: p.surface_primary.into(),
        surface_secondary: p.surface_secondary.into(),
        surface_tertiary: p.surface_tertiary.into(),
        surface_inverse: p.surface_inverse.into(),
        surface_inverse_secondary: p.surface_inverse_secondary.into(),
        surface_inverse_tertiary: p.surface_inverse_tertiary.into(),
        border: p.border.into(),
        border_focus: p.border_focus.into(),
        text_primary: p.text_primary.into(),
        text_secondary: p.text_secondary.into(),
        text_placeholder: p.text_placeholder.into(),
        text_inverse: p.text_inverse.into(),
        text_highlight: p.text_highlight.into(),
        focus: Color::from(p.focus).with_a(60),
        active: p.active.into(),
        disabled: p.disabled.into(),
        ..base.clone()
    };
    theme
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_configured_theme_resolves_to_a_named_palette() {
        let known: Vec<Palette> = THEMES.iter().map(|n| palette(n)).collect();
        assert_eq!(known.len(), 4);
        for (name, p) in THEMES.iter().zip(known) {
            // A paleta de um tema não pode ser a de outro.
            let others: Vec<Palette> = THEMES
                .iter()
                .filter(|n| *n != name)
                .map(|n| palette(n))
                .collect();
            assert!(
                others.iter().all(|o| o != &p),
                "{name} colide com outro tema"
            );
        }
    }

    #[test]
    fn unknown_theme_falls_back_to_slate() {
        assert_eq!(palette("inexistente"), SLATE);
        assert_eq!(palette(""), SLATE);
    }

    #[test]
    fn palettes_are_pairwise_distinct() {
        let all = [SLATE, CHARCOAL, FROST, PAPER];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b, "paletas repetidas");
            }
        }
    }

    #[test]
    fn light_and_dark_are_classified() {
        assert!(is_dark("slate"));
        assert!(is_dark("charcoal"));
        assert!(!is_dark("frost"));
        assert!(!is_dark("paper"));
        assert!(!is_dark("desconhecido"));
    }

    /// Contraste relativo WCAG entre duas cores opacas.
    fn relative_luminance(c: Rgb) -> f32 {
        let channel = |v: u8| {
            let v = v as f32 / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(c.0) + 0.7152 * channel(c.1) + 0.0722 * channel(c.2)
    }

    /// Razão de contraste WCAG entre duas cores opacas.
    fn contrast(a: Rgb, b: Rgb) -> f32 {
        let (la, lb) = (relative_luminance(a), relative_luminance(b));
        let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn body_text_passes_wcag_aa_against_its_background() {
        for (name, p) in [
            ("slate", SLATE),
            ("charcoal", CHARCOAL),
            ("frost", FROST),
            ("paper", PAPER),
        ] {
            let ratio = contrast(p.text_primary, p.background);
            assert!(
                ratio >= 4.5,
                "{name}: contraste texto/fundo = {ratio:.2} (mín 4.5)"
            );
        }
    }

    #[test]
    fn secondary_text_passes_wcag_aa_against_its_background() {
        for (name, p) in [
            ("slate", SLATE),
            ("charcoal", CHARCOAL),
            ("frost", FROST),
            ("paper", PAPER),
        ] {
            let ratio = contrast(p.text_secondary, p.background);
            assert!(
                ratio >= 4.5,
                "{name}: contraste secundário = {ratio:.2} (mín 4.5)"
            );
        }
    }

    #[test]
    fn primary_is_distinguishable_from_background() {
        for (name, p) in [
            ("slate", SLATE),
            ("charcoal", CHARCOAL),
            ("frost", FROST),
            ("paper", PAPER),
        ] {
            let ratio = contrast(p.primary, p.background);
            assert!(
                ratio >= 3.0,
                "{name}: destaque/fundo = {ratio:.2} (mín 3.0)"
            );
        }
    }

    #[test]
    fn placeholder_text_stays_readable() {
        // Placeholder é texto de verdade (caminho, "varrendo…", dica de
        // erro), então passa por WCAG AA igual ao secundário. Num design
        // minimalista ele é o que menos pode sumir: é o que explica o que a
        // tela vazia está esperando.
        for (name, p) in [
            ("slate", SLATE),
            ("charcoal", CHARCOAL),
            ("frost", FROST),
            ("paper", PAPER),
        ] {
            let ratio = contrast(p.text_placeholder, p.background);
            assert!(
                ratio >= 4.5,
                "{name}: placeholder/fundo = {ratio:.2} (mín 4.5)"
            );
        }
    }

    #[test]
    fn borders_are_visible_against_the_background() {
        // Borda de 1px é o que dá a hierarquia no design minimalista: se ela
        // some, a interface fica crua sem nenhum outro recurso.
        for (name, p) in [
            ("slate", SLATE),
            ("charcoal", CHARCOAL),
            ("frost", FROST),
            ("paper", PAPER),
        ] {
            let ratio = contrast(p.border, p.background);
            assert!(ratio >= 1.15, "{name}: borda/fundo = {ratio:.2} (mín 1.15)");
        }
    }

    #[test]
    fn accent_text_is_readable_on_the_accent_fill() {
        // Botão primário: fundo = primary, texto = text_inverse.
        for (name, p) in [
            ("slate", SLATE),
            ("charcoal", CHARCOAL),
            ("frost", FROST),
            ("paper", PAPER),
        ] {
            let ratio = contrast(p.text_inverse, p.primary);
            assert!(
                ratio >= 4.5,
                "{name}: texto/botão primário = {ratio:.2} (mín 4.5)"
            );
        }
    }
}
