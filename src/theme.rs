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

/// Slate: padrão, azul-acinzentado escuro.
pub const SLATE: Palette = Palette {
    primary: Rgb(10, 132, 255),
    secondary: Rgb(120, 170, 235),
    tertiary: Rgb(40, 96, 170),
    background: Rgb(28, 30, 34),
    surface_primary: Rgb(56, 60, 68),
    surface_secondary: Rgb(68, 73, 82),
    surface_tertiary: Rgb(44, 47, 53),
    surface_inverse: Rgb(220, 224, 230),
    surface_inverse_secondary: Rgb(200, 205, 212),
    surface_inverse_tertiary: Rgb(180, 186, 194),
    border: Rgb(74, 79, 88),
    border_focus: Rgb(10, 132, 255),
    text_primary: Rgb(236, 239, 243),
    text_secondary: Rgb(160, 166, 176),
    text_placeholder: Rgb(120, 126, 136),
    text_inverse: Rgb(20, 22, 26),
    text_highlight: Rgb(10, 132, 255),
    focus: Rgb(60, 132, 255),
    active: Rgb(90, 96, 106),
    disabled: Rgb(90, 90, 90),
};

/// Charcoal: neutro quase preto, sem viés azul.
pub const CHARCOAL: Palette = Palette {
    primary: Rgb(214, 214, 214),
    secondary: Rgb(150, 150, 150),
    tertiary: Rgb(96, 96, 96),
    background: Rgb(24, 24, 24),
    surface_primary: Rgb(48, 48, 48),
    surface_secondary: Rgb(58, 58, 58),
    surface_tertiary: Rgb(38, 38, 38),
    surface_inverse: Rgb(228, 228, 228),
    surface_inverse_secondary: Rgb(208, 208, 208),
    surface_inverse_tertiary: Rgb(188, 188, 188),
    border: Rgb(66, 66, 66),
    border_focus: Rgb(214, 214, 214),
    text_primary: Rgb(240, 240, 240),
    text_secondary: Rgb(166, 166, 166),
    text_placeholder: Rgb(126, 126, 126),
    text_inverse: Rgb(18, 18, 18),
    text_highlight: Rgb(214, 214, 214),
    focus: Rgb(128, 128, 128),
    active: Rgb(86, 86, 86),
    disabled: Rgb(88, 88, 88),
};

/// Frost: claro, levemente azulado.
pub const FROST: Palette = Palette {
    primary: Rgb(20, 110, 210),
    secondary: Rgb(150, 185, 230),
    tertiary: Rgb(30, 70, 140),
    background: Rgb(245, 248, 252),
    surface_primary: Rgb(228, 234, 242),
    surface_secondary: Rgb(238, 242, 248),
    surface_tertiary: Rgb(250, 251, 253),
    surface_inverse: Rgb(52, 58, 68),
    surface_inverse_secondary: Rgb(72, 78, 88),
    surface_inverse_tertiary: Rgb(96, 102, 112),
    border: Rgb(206, 214, 226),
    border_focus: Rgb(20, 110, 210),
    text_primary: Rgb(22, 28, 38),
    text_secondary: Rgb(88, 98, 112),
    text_placeholder: Rgb(140, 150, 164),
    text_inverse: Rgb(250, 251, 253),
    text_highlight: Rgb(20, 110, 210),
    focus: Rgb(135, 175, 230),
    active: Rgb(214, 222, 234),
    disabled: Rgb(206, 214, 226),
};

/// Paper: claro, quente (creme).
pub const PAPER: Palette = Palette {
    primary: Rgb(180, 110, 30),
    secondary: Rgb(226, 190, 150),
    tertiary: Rgb(140, 80, 20),
    background: Rgb(250, 247, 241),
    surface_primary: Rgb(238, 232, 222),
    surface_secondary: Rgb(245, 240, 232),
    surface_tertiary: Rgb(252, 250, 246),
    surface_inverse: Rgb(58, 50, 42),
    surface_inverse_secondary: Rgb(80, 70, 60),
    surface_inverse_tertiary: Rgb(104, 94, 82),
    border: Rgb(220, 212, 200),
    border_focus: Rgb(180, 110, 30),
    text_primary: Rgb(38, 32, 26),
    text_secondary: Rgb(108, 96, 82),
    text_placeholder: Rgb(160, 148, 132),
    text_inverse: Rgb(252, 250, 246),
    text_highlight: Rgb(180, 110, 30),
    focus: Rgb(214, 178, 130),
    active: Rgb(230, 222, 210),
    disabled: Rgb(220, 212, 200),
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
}
