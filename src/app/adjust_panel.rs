//! Painel de ajustes: histograma RGB + sliders de exposição, contraste,
//! saturação e temperatura.
//!
//! Fica no dock como painel ao lado do visualizador, recolhível com `Ctrl+3`.
//! É um [`Component`] — e não uma função solta — porque precisa de hooks, e
//! hooks mudam de índice se o elemento for montado condicionalmente (o Freya
//! aborta o render nesse caso).
//!
//! O painel **não guarda estado de edição**: lê e escreve no `EditorStack`, que
//! é a fonte da verdade. É o que faz o `Ctrl+Z` funcionar nos sliders sem
//! nenhum código a mais aqui.

use freya::components::{Button, Slider};

use crate::adjust::BINS;
use crate::adjust::Histogram;
use crate::image_store::LoadState;
use crate::prelude::*;
use crate::ui;

use super::services::Services;
use super::state::{self, AdjustField, AppChannel, channel};

/// Altura do gráfico do histograma (px de tela, antes da escala da UI).
const HIST_HEIGHT: f32 = 54.0;
/// Resolução do slider do Freya (ele entrega 0..=100).
const SLIDER_MAX: f64 = 100.0;

#[derive(PartialEq, Clone)]
pub struct AdjustPanel;

impl Component for AdjustPanel {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Services>();
        let edit = channel(AppChannel::Edit);
        let cfg = channel(AppChannel::Config).read().config.clone();
        let m = ui::Metrics::new(cfg.ui_scale);
        let pal = crate::theme::palette(&cfg.theme);
        let dim = pal.text_secondary.to_color();
        let ink = pal.text_primary.to_color();

        let load = services.load.read().clone();
        let histogram = load.histogram();
        let adjust = edit.read().editor.state().adjust;
        let has_photo = !matches!(load, LoadState::Empty);

        rect()
            .width(Size::fill())
            .height(Size::fill())
            .vertical()
            .padding(ui::gaps(&m, 2., 2.))
            .spacing(m.gap(1.25))
            .child(ui::section(&m, dim, "Ajustes"))
            .child(histogram_view(&m, &histogram))
            .child(clip_row(&m, &histogram, ink))
            .children({
                let ctx = SliderCtx {
                    m: &m,
                    adjust,
                    ink,
                    dim,
                    services: &services,
                };
                let rows: Vec<Element> = state::ADJUST_SLIDERS
                    .iter()
                    .map(|(label, field, max)| slider_row(&ctx, label, *field, *max))
                    .collect();
                rows
            })
            .child(
                Button::new()
                    .flat()
                    .width(Size::fill())
                    .enabled(!adjust.is_clean())
                    .corner_radius(m.radius_sm())
                    .padding(ui::gaps(&m, 0.5, 1.5))
                    .on_press({
                        let services = services.clone();
                        move |_| {
                            state::update(AppChannel::Edit, |st| {
                                st.editor.reset_adjust();
                                state::refresh_preview(st, &services);
                            });
                        }
                    })
                    .child(ui::text(&m, ui::Role::Body, ink, "Zerar ajustes")),
            )
            .maybe(!has_photo, |el| {
                el.child(ui::faint(&m, dim, "Abra uma foto para ajustar."))
            })
    }
}

/// Gráfico do histograma: uma coluna por faixa, três canais sobrepostos.
///
/// Sem `canvas`/shader de propósito: `rect` é o que a UI já usa em todo o
/// resto, e 64 colunas × 3 canais são 192 nós por frame — abaixo do que pesa.
/// Um shader seria mais bonito e bem mais caro de manter.
///
/// As colunas são posicionadas em `Position::Global` (como o overlay de crop
/// e o gráfico já fazem no viewer): altura percentual exigiria o eixo do pai
/// em `flex`, e aí a largura deixaria de casar com a faixa.
fn histogram_view(m: &ui::Metrics, h: &Histogram) -> impl IntoElement {
    let peak = h.peak().max(1);
    let width = 100.0 / BINS as f32;
    let height = HIST_HEIGHT * m.scale();

    rect()
        .width(Size::fill())
        .height(Size::px(height))
        .background(Color::from_argb(24, 128, 128, 136))
        .corner_radius(m.radius_sm())
        .overflow(Overflow::Clip)
        .child(
            rect()
                .position(Position::new_global())
                .width(Size::px(0.))
                .height(Size::px(0.)),
        )
        .children((0..BINS).map(|i| {
            let left = i as f32 * width;
            let bar = |count: u32, color: Color| {
                let h = bar_height(count, peak) * height;
                rect()
                    .position(Position::new_global().left(left).bottom(0.))
                    .width(Size::px(width))
                    .height(Size::px(h))
                    .background(color)
                    .into_element()
            };
            // B embaixo, G no meio, R por cima: a mistura dá a leitura de
            // "onde está a cor" sem três gráficos lado a lado.
            let r = bar(h.r[i], Color::from_rgb(235, 85, 85));
            let g = bar(h.g[i], Color::from_rgb(70, 205, 105));
            let b = bar(h.b[i], Color::from_rgb(70, 110, 245));
            rect().child(b).child(g).child(r)
        }))
}

/// Altura de uma barra: proporcional ao pico, mas nunca zero.
///
/// Sem o piso, o canal com pouquíssimos pixels desapareceria do gráfico
/// inteiro — e é justamente um canal com pouca contagem que diz "algo está
/// estourado/travado aqui".
#[must_use]
pub fn bar_height(count: u32, peak: u32) -> f32 {
    if count == 0 {
        return 0.0;
    }
    ((count as f32) / (peak.max(1) as f32)).clamp(0.05, 1.0)
}

/// Aviso de pixels estourados/travados.
///
/// Só aparece quando há clipping de verdade: um contador sempre visível em zero
/// vira decoração, e o ponto do histograma é avisar **antes** de salvar.
fn clip_row(m: &ui::Metrics, h: &Histogram, _ink: Color) -> impl IntoElement {
    let note = clip_note(h);
    rect().width(Size::fill()).maybe(!note.is_empty(), |el| {
        el.child(ui::text(
            m,
            ui::Role::Small,
            Color::from_rgb(240, 170, 60),
            format!("Clipping: {note}"),
        ))
    })
}

/// Texto de clipping: só lista o lado que de fato estourou.
#[must_use]
pub fn clip_note(h: &Histogram) -> String {
    let mut parts = Vec::new();
    if h.blowout > 0 {
        parts.push(format!("{} estourado(s)", h.blowout));
    }
    if h.shadow > 0 {
        parts.push(format!("{} sem detalhe", h.shadow));
    }
    parts.join(" · ")
}

/// Contexto de um slider: o painel repete os mesmos valores em 4 linhas.
struct SliderCtx<'a> {
    m: &'a ui::Metrics,
    adjust: crate::adjust::Adjust,
    ink: Color,
    dim: Color,
    services: &'a Services,
}

/// Uma linha: rótulo + slider + valor numérico.
fn slider_row(ctx: &SliderCtx<'_>, label: &'static str, field: AdjustField, max: f32) -> Element {
    let m = ctx.m;
    let value = field.read(&ctx.adjust);
    let text = format_value(field, value);
    let services = ctx.services.clone();

    rect()
        .width(Size::fill())
        .vertical()
        .spacing(2.)
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .main_align(Alignment::SpaceBetween)
                .child(ui::text(m, ui::Role::Body, ctx.ink, label))
                .child(ui::text(m, ui::Role::Small, ctx.dim, text)),
        )
        .child(
            Slider::new(move |v: f64| {
                let mapped = slider_to_value(v, max);
                let services = services.clone();
                state::update(AppChannel::Edit, |st| {
                    st.editor.tweak(|adj| field.write(adj, mapped));
                    state::refresh_preview(st, &services);
                });
            })
            .value(value_to_slider(value, max))
            .scroll_enabled(false),
        )
        .into_element()
}

/// Converte a posição do slider (0..=100, bipolar) para o valor do ajuste.
///
/// Bipolar de propósito: o centro é o ajuste neutro, então zerar é um clique
/// no meio do slider em vez de caçar um botão "0".
#[must_use]
pub fn slider_to_value(raw: f64, max: f32) -> f32 {
    let t = (raw / SLIDER_MAX) as f32 * 2.0 - 1.0;
    t.clamp(-1.0, 1.0) * max
}

/// Inverso de [`slider_to_value`].
#[must_use]
pub fn value_to_slider(value: f32, max: f32) -> f64 {
    let t = (value / max).clamp(-1.0, 1.0) as f64;
    ((t + 1.0) * 0.5 * SLIDER_MAX).clamp(0.0, SLIDER_MAX)
}

/// Texto do valor, com a unidade de cada ajuste.
#[must_use]
pub fn format_value(field: AdjustField, v: f32) -> String {
    match field {
        AdjustField::Exposure => format!("{:+.2} EV", v),
        AdjustField::Saturation => format!("{:+.0}%", v * 100.0),
        _ => format!("{:+.2}", v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_adjust_field_has_a_slider() {
        // Sem isto, um campo novo no `Adjust` mas não no painel fica
        // inalcançável — e ninguém nota até alguém reportar.
        let fields: Vec<AdjustField> = state::ADJUST_SLIDERS.iter().map(|(_, f, _)| *f).collect();
        for expected in [
            AdjustField::Exposure,
            AdjustField::Contrast,
            AdjustField::Saturation,
            AdjustField::Temperature,
        ] {
            assert!(fields.contains(&expected), "falta slider para {expected:?}");
        }
        assert_eq!(fields.len(), 4, "campo novo sem slider?");
    }

    #[test]
    fn field_read_write_roundtrip() {
        let mut adj = crate::adjust::Adjust::default();
        for (_, field, max) in state::ADJUST_SLIDERS {
            let max = *max;
            field.write(&mut adj, max / 2.0);
            assert_eq!(field.read(&adj), max / 2.0);
        }
        assert!(!adj.is_clean());
    }

    #[test]
    fn slider_ends_map_to_the_field_limits() {
        // O `Slider` entrega 0..=100; as pontas têm de ser exatamente -max/+max
        // e o centro, zero — senão o ajuste não vai a zero nem no meio.
        for (_, _, max) in state::ADJUST_SLIDERS {
            let max = *max;
            assert!((slider_to_value(0.0, max) - -max).abs() < 1e-4);
            assert!((slider_to_value(SLIDER_MAX, max) - max).abs() < 1e-4);
            assert!(slider_to_value(SLIDER_MAX / 2.0, max).abs() < 1e-4);
        }
    }

    #[test]
    fn slider_position_and_value_are_inverses() {
        for (_, _, max) in state::ADJUST_SLIDERS {
            let max = *max;
            for step in 0..=10 {
                let raw = f64::from(step) * 10.0;
                let value = slider_to_value(raw, max);
                let back = value_to_slider(value, max);
                assert!((back - raw).abs() < 1e-3, "{step} -> {value} -> {back}");
            }
        }
    }

    #[test]
    fn out_of_range_slider_input_is_clamped_not_wrapped() {
        // Arraste além da track não pode inverter o sinal do ajuste.
        assert!(slider_to_value(-40.0, 1.0) <= 0.0);
        assert!(slider_to_value(140.0, 1.0) >= 0.0);
        assert_eq!(slider_to_value(500.0, 1.0), 1.0);
    }

    #[test]
    fn exposure_shows_ev_and_saturation_shows_percent() {
        assert_eq!(format_value(AdjustField::Exposure, 1.0), "+1.00 EV");
        assert_eq!(format_value(AdjustField::Exposure, -0.5), "-0.50 EV");
        assert_eq!(format_value(AdjustField::Saturation, 1.0), "+100%");
        assert_eq!(format_value(AdjustField::Saturation, -1.0), "-100%");
        assert_eq!(format_value(AdjustField::Contrast, 0.25), "+0.25");
    }

    #[test]
    fn bar_height_never_hides_a_channel_that_has_data() {
        // 1 pixel contra um pico de 1M ainda precisa ter barra visível.
        let small = bar_height(1, 1_000_000);
        assert!(small >= 0.05, "barra invisível: {small}");
        assert_eq!(bar_height(0, 1_000_000), 0.0, "vazio é zero, não 5%");
        assert_eq!(bar_height(100, 100), 1.0);
    }

    #[test]
    fn clipping_note_lists_only_what_actually_clipped() {
        let mut h = Histogram::default();
        assert!(clip_note(&h).is_empty(), "sem clipping, sem nota");
        h.blowout = 12;
        let high = clip_note(&h);
        assert!(high.contains("12"));
        assert!(!high.contains("detalhe"));
        h.shadow = 3;
        let both = clip_note(&h);
        assert!(both.contains('·'), "os dois lados entram: {both}");
    }
}
