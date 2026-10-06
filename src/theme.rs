//! Temas visuais (Freya).
//!
//! Os quatro nomes originais (slate, charcoal, frost, paper) são preservados
//! para não quebrar configs já gravados. Cada um tem uma [`Palette`] com
//! dados puros (testável sem janela) e um mapeamento para a `Theme` do Freya.

use crate::prelude::*;
use freya::components::{ColorsSheet, DARK_COLORS, LIGHT_COLORS, Theme, dark_theme, light_theme};

/// Temas disponíveis (nomes persistidos no config).
pub const THEMES: &[&str] = &[
    "slate",
    "charcoal",
    "frost",
    "paper",
    "tokyo-night",
    "nord",
    "zed-light",
    "zed-dark",
    "gruvbox",
    "nyancat",
];

/// Cor em RGB puro, sem passar pelo `Theme` (para poder testar a palette).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl From<Rgb> for Color {
    fn from(c: Rgb) -> Self {
        Color::from_rgb(c.0, c.1, c.2)
    }
}

impl Rgb {
    /// Converte para [`Color`] do Freya.
    #[must_use]
    pub fn to_color(self) -> Color {
        self.into()
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
// Os dois escuros originais se diferenciam pelo matiz (slate é azulado,
// charcoal é neutro puro) e não só pelo brilho; os claros usam secundário
// escurecido para o texto de apoio passar em WCAG AA com margem.
//
// Nomes originais preservados para não invalidar configs já gravados.

/// Slate: padrão, escuro azulado (o mais claro dos escuros).
pub const SLATE: Palette = Palette {
    primary: Rgb(235, 238, 248),
    secondary: Rgb(170, 176, 196),
    tertiary: Rgb(120, 130, 155),
    background: Rgb(33, 38, 54),
    surface_primary: Rgb(48, 54, 72),
    surface_secondary: Rgb(58, 65, 86),
    surface_tertiary: Rgb(26, 30, 42),
    surface_inverse: Rgb(18, 21, 30),
    surface_inverse_secondary: Rgb(48, 54, 72),
    surface_inverse_tertiary: Rgb(72, 79, 102),
    border: Rgb(72, 79, 102),
    border_focus: Rgb(235, 238, 248),
    text_primary: Rgb(236, 238, 245),
    text_secondary: Rgb(170, 178, 198),
    text_placeholder: Rgb(155, 163, 185),
    text_inverse: Rgb(33, 38, 54),
    text_highlight: Rgb(235, 238, 248),
    focus: Rgb(120, 130, 160),
    active: Rgb(58, 65, 86),
    disabled: Rgb(90, 98, 120),
};

/// Charcoal: neutro puro, quase preto.
pub const CHARCOAL: Palette = Palette {
    primary: Rgb(245, 245, 245),
    secondary: Rgb(178, 178, 178),
    tertiary: Rgb(112, 112, 112),
    background: Rgb(15, 15, 15),
    surface_primary: Rgb(30, 30, 30),
    surface_secondary: Rgb(40, 40, 40),
    surface_tertiary: Rgb(22, 22, 22),
    surface_inverse: Rgb(10, 10, 10),
    surface_inverse_secondary: Rgb(32, 32, 32),
    surface_inverse_tertiary: Rgb(58, 58, 58),
    border: Rgb(48, 48, 48),
    border_focus: Rgb(245, 245, 245),
    text_primary: Rgb(245, 245, 245),
    text_secondary: Rgb(168, 168, 168),
    text_placeholder: Rgb(158, 158, 158),
    text_inverse: Rgb(15, 15, 15),
    text_highlight: Rgb(245, 245, 245),
    focus: Rgb(120, 120, 120),
    active: Rgb(52, 52, 52),
    disabled: Rgb(80, 80, 80),
};

/// Frost: claro, quase branco.
pub const FROST: Palette = Palette {
    primary: Rgb(13, 13, 13),
    secondary: Rgb(110, 110, 116),
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
    text_secondary: Rgb(100, 100, 106),
    text_placeholder: Rgb(100, 100, 106),
    text_inverse: Rgb(252, 252, 253),
    text_highlight: Rgb(13, 13, 13),
    focus: Rgb(150, 150, 154),
    active: Rgb(226, 226, 230),
    disabled: Rgb(196, 196, 200),
};

/// Paper: claro, levemente quente.
pub const PAPER: Palette = Palette {
    primary: Rgb(26, 24, 22),
    secondary: Rgb(116, 112, 106),
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
    text_secondary: Rgb(100, 96, 90),
    text_placeholder: Rgb(100, 96, 90),
    text_inverse: Rgb(251, 250, 248),
    text_highlight: Rgb(26, 24, 22),
    focus: Rgb(154, 150, 144),
    active: Rgb(228, 225, 220),
    disabled: Rgb(198, 195, 190),
};

/// Tokyo Night: azul profundo com acentos neon (azul, roxo, ciano).
pub const TOKYO_NIGHT: Palette = Palette {
    primary: Rgb(122, 162, 247),
    secondary: Rgb(187, 154, 247),
    tertiary: Rgb(125, 207, 255),
    background: Rgb(26, 27, 38),
    surface_primary: Rgb(36, 40, 59),
    surface_secondary: Rgb(46, 51, 75),
    surface_tertiary: Rgb(22, 23, 33),
    surface_inverse: Rgb(15, 16, 24),
    surface_inverse_secondary: Rgb(36, 40, 59),
    surface_inverse_tertiary: Rgb(60, 66, 98),
    border: Rgb(59, 66, 97),
    border_focus: Rgb(122, 162, 247),
    text_primary: Rgb(192, 202, 245),
    text_secondary: Rgb(169, 177, 214),
    text_placeholder: Rgb(135, 145, 185),
    text_inverse: Rgb(26, 27, 38),
    text_highlight: Rgb(122, 162, 247),
    focus: Rgb(122, 162, 247),
    active: Rgb(46, 51, 75),
    disabled: Rgb(86, 95, 137),
};

/// Nord: noite polar azul-acinzentada com acentos gelo (frost) e aurora.
pub const NORD: Palette = Palette {
    primary: Rgb(136, 192, 208),
    secondary: Rgb(129, 161, 193),
    tertiary: Rgb(143, 188, 187),
    background: Rgb(46, 52, 64),
    surface_primary: Rgb(59, 66, 82),
    surface_secondary: Rgb(67, 76, 94),
    surface_tertiary: Rgb(38, 43, 53),
    surface_inverse: Rgb(28, 31, 38),
    surface_inverse_secondary: Rgb(59, 66, 82),
    surface_inverse_tertiary: Rgb(90, 100, 120),
    border: Rgb(76, 86, 106),
    border_focus: Rgb(216, 222, 233),
    text_primary: Rgb(236, 239, 244),
    text_secondary: Rgb(216, 222, 233),
    text_placeholder: Rgb(165, 178, 198),
    text_inverse: Rgb(46, 52, 64),
    text_highlight: Rgb(136, 192, 208),
    focus: Rgb(136, 192, 208),
    active: Rgb(67, 76, 94),
    disabled: Rgb(76, 86, 106),
};

/// Zed Light: claro azulado com acento azul vibrante.
pub const ZED_LIGHT: Palette = Palette {
    primary: Rgb(11, 92, 255),
    secondary: Rgb(70, 110, 180),
    tertiary: Rgb(120, 90, 170),
    background: Rgb(247, 247, 245),
    surface_primary: Rgb(236, 236, 234),
    surface_secondary: Rgb(228, 228, 226),
    surface_tertiary: Rgb(252, 252, 251),
    surface_inverse: Rgb(30, 31, 34),
    surface_inverse_secondary: Rgb(60, 61, 66),
    surface_inverse_tertiary: Rgb(100, 101, 108),
    border: Rgb(220, 220, 218),
    border_focus: Rgb(11, 92, 255),
    text_primary: Rgb(28, 29, 33),
    text_secondary: Rgb(85, 86, 94),
    text_placeholder: Rgb(105, 106, 115),
    text_inverse: Rgb(247, 247, 245),
    text_highlight: Rgb(11, 92, 255),
    focus: Rgb(11, 92, 255),
    active: Rgb(220, 222, 228),
    disabled: Rgb(190, 190, 188),
};

/// Zed Dark: grafite azulado com acento azul claro.
pub const ZED_DARK: Palette = Palette {
    primary: Rgb(96, 150, 255),
    secondary: Rgb(150, 130, 220),
    tertiary: Rgb(90, 200, 200),
    background: Rgb(18, 20, 26),
    surface_primary: Rgb(30, 32, 40),
    surface_secondary: Rgb(38, 41, 51),
    surface_tertiary: Rgb(14, 15, 20),
    surface_inverse: Rgb(11, 12, 15),
    surface_inverse_secondary: Rgb(30, 32, 40),
    surface_inverse_tertiary: Rgb(54, 58, 70),
    border: Rgb(50, 54, 66),
    border_focus: Rgb(130, 175, 255),
    text_primary: Rgb(228, 230, 236),
    text_secondary: Rgb(158, 164, 180),
    text_placeholder: Rgb(140, 146, 162),
    text_inverse: Rgb(18, 20, 26),
    text_highlight: Rgb(130, 175, 255),
    focus: Rgb(90, 140, 255),
    active: Rgb(38, 41, 51),
    disabled: Rgb(80, 86, 100),
};

/// Gruvbox: marrom quente retrô com acentos laranja, amarelo e verde.
pub const GRUVBOX: Palette = Palette {
    primary: Rgb(254, 128, 25),
    secondary: Rgb(250, 189, 47),
    tertiary: Rgb(142, 192, 124),
    background: Rgb(40, 40, 40),
    surface_primary: Rgb(60, 56, 54),
    surface_secondary: Rgb(80, 73, 69),
    surface_tertiary: Rgb(29, 32, 33),
    surface_inverse: Rgb(24, 24, 24),
    surface_inverse_secondary: Rgb(60, 56, 54),
    surface_inverse_tertiary: Rgb(102, 92, 84),
    border: Rgb(102, 92, 84),
    border_focus: Rgb(250, 189, 47),
    text_primary: Rgb(235, 219, 178),
    text_secondary: Rgb(213, 196, 161),
    text_placeholder: Rgb(189, 174, 147),
    text_inverse: Rgb(40, 40, 40),
    text_highlight: Rgb(254, 128, 25),
    focus: Rgb(254, 128, 25),
    active: Rgb(80, 73, 69),
    disabled: Rgb(146, 131, 116),
};

/// Nyancat: extremamente colorido — roxo espacial com rosa, ciano e amarelo
/// do arco-íris da Nyan Cat.
pub const NYANCAT: Palette = Palette {
    primary: Rgb(255, 51, 153),
    secondary: Rgb(64, 224, 255),
    tertiary: Rgb(255, 214, 0),
    background: Rgb(26, 15, 46),
    surface_primary: Rgb(42, 26, 72),
    surface_secondary: Rgb(56, 34, 96),
    surface_tertiary: Rgb(19, 11, 34),
    surface_inverse: Rgb(13, 7, 24),
    surface_inverse_secondary: Rgb(42, 26, 72),
    surface_inverse_tertiary: Rgb(80, 50, 130),
    border: Rgb(150, 80, 180),
    border_focus: Rgb(0, 255, 255),
    text_primary: Rgb(255, 255, 255),
    text_secondary: Rgb(255, 214, 235),
    text_placeholder: Rgb(205, 240, 255),
    text_inverse: Rgb(26, 15, 46),
    text_highlight: Rgb(255, 214, 0),
    focus: Rgb(255, 51, 153),
    active: Rgb(56, 34, 96),
    disabled: Rgb(130, 110, 150),
};

/// Paleta pelo nome salvo no config; desconhecido cai em `slate`.
#[must_use]
pub fn palette(name: &str) -> Palette {
    match name {
        "charcoal" => CHARCOAL,
        "frost" => FROST,
        "paper" => PAPER,
        "tokyo-night" => TOKYO_NIGHT,
        "nord" => NORD,
        "zed-light" => ZED_LIGHT,
        "zed-dark" => ZED_DARK,
        "gruvbox" => GRUVBOX,
        "nyancat" => NYANCAT,
        _ => SLATE,
    }
}

/// O tema é escuro?
#[must_use]
pub fn is_dark(name: &str) -> bool {
    matches!(
        name,
        "slate" | "charcoal" | "" | "tokyo-night" | "nord" | "zed-dark" | "gruvbox" | "nyancat"
    )
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
        assert!(!known.is_empty());
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
        let all = [
            SLATE,
            CHARCOAL,
            FROST,
            PAPER,
            TOKYO_NIGHT,
            NORD,
            ZED_LIGHT,
            ZED_DARK,
            GRUVBOX,
            NYANCAT,
        ];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b, "paletas repetidas");
            }
        }
    }

    #[test]
    fn light_and_dark_are_classified() {
        for name in [
            "slate",
            "charcoal",
            "tokyo-night",
            "nord",
            "zed-dark",
            "gruvbox",
            "nyancat",
        ] {
            assert!(is_dark(name), "{name} deveria ser escuro");
        }
        for name in ["frost", "paper", "zed-light"] {
            assert!(!is_dark(name), "{name} deveria ser claro");
        }
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
        for name in THEMES {
            let p = palette(name);
            let ratio = contrast(p.text_primary, p.background);
            assert!(
                ratio >= 4.5,
                "{name}: contraste texto/fundo = {ratio:.2} (mín 4.5)"
            );
        }
    }

    #[test]
    fn secondary_text_passes_wcag_aa_against_its_background() {
        for name in THEMES {
            let p = palette(name);
            let ratio = contrast(p.text_secondary, p.background);
            assert!(
                ratio >= 4.5,
                "{name}: contraste secundário = {ratio:.2} (mín 4.5)"
            );
        }
    }

    #[test]
    fn primary_is_distinguishable_from_background() {
        for name in THEMES {
            let p = palette(name);
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
        for name in THEMES {
            let p = palette(name);
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
        for name in THEMES {
            let p = palette(name);
            let ratio = contrast(p.border, p.background);
            assert!(ratio >= 1.15, "{name}: borda/fundo = {ratio:.2} (mín 1.15)");
        }
    }

    #[test]
    fn accent_text_is_readable_on_the_accent_fill() {
        // Botão primário: fundo = primary, texto = text_inverse.
        for name in THEMES {
            let p = palette(name);
            let ratio = contrast(p.text_inverse, p.primary);
            assert!(
                ratio >= 4.5,
                "{name}: texto/botão primário = {ratio:.2} (mín 4.5)"
            );
        }
    }

    #[test]
    fn secondary_text_passes_wcag_aa_on_surfaces_too() {
        // Regressão de legibilidade nos temas claros: o texto de apoio não
        // vive só sobre o fundo da janela — ele aparece sobre as superfícies
        // (toolbar, listas, cards). Cinza translúcido sobre branco some.
        for name in THEMES {
            let p = palette(name);
            for (surface, sname) in [
                (p.surface_primary, "surface_primary"),
                (p.surface_tertiary, "surface_tertiary"),
            ] {
                let ratio = contrast(p.text_secondary, surface);
                assert!(
                    ratio >= 4.5,
                    "{name}: secundário/{sname} = {ratio:.2} (mín 4.5)"
                );
            }
        }
    }

    #[test]
    fn dark_themes_are_visually_distinct_from_each_other() {
        // Regressão: os escuros eram quase idênticos (só brilho). Cada par de
        // temas escuros precisa diferir no matiz ou no brilho de forma
        // perceptível — medido pela distância euclidiana no RGB do fundo.
        let darks = [
            ("slate", SLATE),
            ("charcoal", CHARCOAL),
            ("tokyo-night", TOKYO_NIGHT),
            ("nord", NORD),
            ("zed-dark", ZED_DARK),
            ("gruvbox", GRUVBOX),
            ("nyancat", NYANCAT),
        ];
        for (i, (na, a)) in darks.iter().enumerate() {
            for (nb, b) in &darks[i + 1..] {
                let d = dist(a.background, b.background);
                assert!(
                    d >= 12.0,
                    "{na} x {nb}: fundos indistinguíveis (dist {d:.1})"
                );
            }
        }
    }

    /// Distância euclidiana entre duas cores (0..441).
    fn dist(a: Rgb, b: Rgb) -> f32 {
        let (dr, dg, db) = (
            a.0 as f32 - b.0 as f32,
            a.1 as f32 - b.1 as f32,
            a.2 as f32 - b.2 as f32,
        );
        (dr * dr + dg * dg + db * db).sqrt()
    }
}
