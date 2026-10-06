//! Ajustes globais de cor: exposição, contraste, saturação, temperatura.
//!
//! Tudo aqui é **função pura** e não conhece a GUI: o preview (display) e o
//! `bake` (full-res) chamam exatamente [`adjust_pixels`], então o que se vê na
//! tela é o que vai para o disco. É o requisito do plano (seção 26, 0.4):
//! mesma semântica matemática nas duas resoluções.
//!
//! ## Ordem das operações (fixa, e não é arbitrária)
//!
//! ```text
//! exposição  -> temperatura -> contraste -> saturação
//! ```
//!
//! Exposição vem primeiro porque multiplica em **linear**, antes de qualquer
//! curva: é o único ajuste em que a ordem muda o resultado. Temperatura
//! (balanço de canais) vem antes do contraste, senão o contraste empurra o
//! imbalance para os extremos e ele deixa de parecer "quente/frio".
//! Saturação por último, sobre a luminância já corrigida.
//!
//! ## Espaço de cor
//!
//! Exposição opera em **linear** (sRGB invertido): um `+1 EV` dobra a luz, como
//! na física. Temperatura, contraste e saturação trabalham em **gamma** (o sRGB
//! codificado), que é onde o olho percebe cor — é assim que todo slider de
//! câmera sempre funcionou. Misturar os dois numa curva só daria resultados
//! criativos, não previsíveis.
//!
//! ponytail: nenhum ajuste é *local* (máscaras, pinça de branco, curva por
//! canal). O conjunto é fechado de propósito. Ao abrir para ajustes locais,
//! trocar [`adjust_pixels`] por um shader/Skia e manter [`Adjust`] como estado.

/// Ajuste global de cor, em `-1..=1` (exposição vai além: em EV).
///
/// Default = tudo neutro. Deriva `Copy` de propósito: o `EditorState` tira
/// snapshots dela a cada operação, e é por isso que o undo continua barato.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Adjust {
    /// Exposição em **stops** (`1.0` = +1 EV = dobra a luz). `-3..=3`.
    pub exposure: f32,
    /// Contraste em torno do cinza médio. `-1..=1`.
    pub contrast: f32,
    /// Saturação: `-1` = cinza puro, `1` = +100%. `-1..=1`.
    pub saturation: f32,
    /// Temperatura: `-1` = frio (azulado), `1` = quente (avermelhado). `-1..=1`.
    pub temperature: f32,
}

impl Adjust {
    /// Nenhum ajuste ativo?
    ///
    /// Compara por `EPSILON`, não por `==`: slider entrega `f32` e um arrasto
    /// de 1px produz diferença de 1e-7 — sem isso o `EditorStack::commit`
    /// empilharia histórico a cada pixel de mouse.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        [
            self.exposure,
            self.contrast,
            self.saturation,
            self.temperature,
        ]
        .iter()
        .all(|v| v.abs() < EPSILON)
    }

    /// Igual a [`Adjust::is_clean`], tolerante a ruído de `f32`.
    ///
    /// O `EditorStack` compara estados para decidir se há o que commitar;
    /// com `==` direto, dois valores que deveriam ser o mesmo ajuste
    /// (mesmo slider lido em frames diferentes) seriam estados distintos.
    #[must_use]
    pub fn eq_approx(&self, other: &Self) -> bool {
        let close = |a: f32, b: f32| (a - b).abs() < EPSILON;
        close(self.exposure, other.exposure)
            && close(self.contrast, other.contrast)
            && close(self.saturation, other.saturation)
            && close(self.temperature, other.temperature)
    }

    /// Trás cada campo para a faixa válida (chamado ao gravar no editor).
    #[must_use]
    pub fn clamped(&self) -> Self {
        Self {
            exposure: self.exposure.clamp(-EXPOSURE_MAX, EXPOSURE_MAX),
            contrast: self.contrast.clamp(-1., 1.),
            saturation: self.saturation.clamp(-1., 1.),
            temperature: self.temperature.clamp(-1., 1.),
        }
    }
}

/// Tolerância de comparação entre ajustes.
pub const EPSILON: f32 = 1e-4;
/// Teto de exposição em stops (|EV|): além disso a imagem vira ruído.
pub const EXPOSURE_MAX: f32 = 4.0;
/// Ganho do canal vermelho por unidade de temperatura.
const TEMP_R: f32 = 0.12;
/// Ganho do canal azul por unidade de temperatura (sinal oposto ao vermelho).
const TEMP_B: f32 = 0.10;
/// Cinza médio em gamma sRGB (0.5 codificado) — pivô do contraste.
const CONTRAST_PIVOT: f32 = 0.5;
/// Peso do contraste na forma de curva (`1.0` = suave, `2.0` = forte).
const CONTRAST_CURVE: f32 = 1.0;

/// sRGB codificado (0..=1) -> linear (0..=1).
///
/// Curva oficial da especificação sRGB, não um `powf(2.2)`: a diferença é
/// pequena nas sombras, que é justamente onde um ajuste de exposição de
/// -2 EV passa a importar.
#[must_use]
pub fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear -> sRGB codificado (0..=1).
#[must_use]
pub fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// Aplica o ajuste a um pixel RGB em gamma sRGB (`0..=1`), devolvendo RGB.
///
/// A exposição já entra convertedida para linear por quem chama (esta função
/// opera só em gamma). É a mesma ordem documentada no módulo.
#[must_use]
pub fn adjust_rgb(rgb: [f32; 3], adj: &Adjust) -> [f32; 3] {
    let mut c = rgb;

    // Temperatura: escala de canais antes de qualquer curva. Vale mais quente
    // = vermelho sobe, azul desce (e o inverso no frio).
    let t = adj.temperature;
    c[0] *= 1.0 + TEMP_R * t;
    c[2] *= 1.0 - TEMP_B * t;

    // Contraste em torno do cinza médio. `c > 0` estica, `c < 0` achata;
    // ambos saturam em 0 e 1, então o clamp no fim basta.
    if adj.contrast.abs() > EPSILON {
        let k = 1.0 + adj.contrast * CONTRAST_CURVE;
        for v in c.iter_mut() {
            *v = (*v - CONTRAST_PIVOT) * k + CONTRAST_PIVOT;
        }
    }

    // Saturação: mistura com a luminância Rec.709 (a que o olho usa para
    // "brilho"). `saturation = 0` dá o cinza puro.
    if adj.saturation.abs() > EPSILON {
        let luma = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        let k = 1.0 + adj.saturation;
        for v in c.iter_mut() {
            *v = luma + (*v - luma) * k;
        }
    }

    c
}

/// Aplica o ajuste a um buffer RGBA in-place.
///
/// Caminho único do preview **e** do save (exigência do plano §26): nenhuma das
/// duas resolutions tem versão própria da matemática. Faz o ganho de exposição
/// em linear e devolve o buffer em gamma, como a GPU espera.
pub fn adjust_pixels(buf: &mut [u8], adj: &Adjust) {
    if adj.is_clean() {
        return;
    }
    // Evolução é multiplicativa e não pode ser feita em gamma: dobrar o valor
    // codificado não dobra a luz perceived.
    if adj.exposure.abs() > EPSILON {
        let gain = 2f32.powf(adj.exposure);
        for px in buf.as_chunks_mut::<4>().0 {
            for ch in px.iter_mut().take(3) {
                let v = srgb_to_linear(f32::from(*ch) / 255.0) * gain;
                *ch = to_u8(linear_to_srgb(v));
            }
        }
    }

    let rest = Adjust {
        exposure: 0.0,
        ..*adj
    };
    if rest.is_clean() {
        return;
    }
    for px in buf.as_chunks_mut::<4>().0 {
        let rgb = adjust_rgb(
            [
                f32::from(px[0]) / 255.0,
                f32::from(px[1]) / 255.0,
                f32::from(px[2]) / 255.0,
            ],
            &rest,
        );
        px[0] = to_u8(rgb[0]);
        px[1] = to_u8(rgb[1]);
        px[2] = to_u8(rgb[2]);
        // Alpha passa intacto: um ajuste de cor não tem por que transparentizar
        // a foto nem crear bordas onde não havia.
    }
}

fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Número de faixas do histograma.
pub const BINS: usize = 64;
/// Faixa inicial do histograma (evita um pico Huge nas sombras puras).
pub const BINS_CLIP: u32 = 100;
/// Nível de estourado (clip) que conta como "estourado".
pub const CLIP_HIGH: u32 = 250;
/// Nível de estourado que conta como "travado no preto".
pub const CLIP_LOW: u32 = 5;

/// Histograma RGB + contagem de pixels estourados/travados.
///
/// Feito para o overlay do viewer: `blowout`/`shadow` são o que o usuário
/// precisa saber **antes** de salvar, porque só aí a perda é irreversível.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Histogram {
    /// Contagem por canal, `BINS` faixas de largura igual.
    pub r: [u32; BINS],
    /// Contagem do canal verde.
    pub g: [u32; BINS],
    /// Contagem do canal azul.
    pub b: [u32; BINS],
    /// Pixels com algum canal >= [`CLIP_HIGH`] (luz estourada).
    pub blowout: u64,
    /// Pixels com **todos** os canais <= [`CLIP_LOW`] (preto travado).
    pub shadow: u64,
}

impl Default for Histogram {
    fn default() -> Self {
        Self {
            r: [0; BINS],
            g: [0; BINS],
            b: [0; BINS],
            blowout: 0,
            shadow: 0,
        }
    }
}

impl Histogram {
    /// Total de pixels contados (usa o canal vermelho como referência).
    #[must_use]
    pub fn total(&self) -> u64 {
        self.r.iter().map(|&c| u64::from(c)).sum()
    }

    /// Maior contagem de qualquer faixa: é a altura máxima do gráfico.
    ///
    /// Normalizar pelo pico (e não pelo total) mantém a forma visível em fotos
    /// com muita área chapada — normalizar pelo total achataria tudo contra a
    /// base e o histograma viraria uma linha.
    #[must_use]
    pub fn peak(&self) -> u32 {
        self.r
            .iter()
            .chain(self.g.iter())
            .chain(self.b.iter())
            .copied()
            .max()
            .unwrap_or(0)
    }

    /// Há pixels estourados ou travados? (o overlay usa para não mostrar 0%)
    #[must_use]
    pub fn has_clipping(&self) -> bool {
        self.blowout > 0 || self.shadow > 0
    }
}

/// Calcula o histograma de um buffer RGBA.
///
/// `sample` > 1 pula pixels (amostragem em grade): o histograma é uma tendência,
/// não uma contagem — usar todos os pixels de uma foto de 24 MP a cada
/// ajuste de slider é desperdício visível.
#[must_use]
pub fn histogram(buf: &[u8], sample: usize) -> Histogram {
    let sample = sample.max(1);
    let mut h = Histogram::default();
    for px in buf.as_chunks::<4>().0.iter().step_by(sample) {
        h.r[bin_of(px[0])] += 1;
        h.g[bin_of(px[1])] += 1;
        h.b[bin_of(px[2])] += 1;
        // Clipping conta **pixel**, não canal: um pixel estourado vale 1, não 3.
        // Somar por canal inflaria a leitura e o número não bateria com "há
        // N pixels perdidos" — que é a pergunta que o usuário faz.
        let high = px[0] >= CLIP_HIGH as u8 || px[1] >= CLIP_HIGH as u8 || px[2] >= CLIP_HIGH as u8;
        let low = px[0] <= CLIP_LOW as u8 && px[1] <= CLIP_LOW as u8 && px[2] <= CLIP_LOW as u8;
        h.blowout += u64::from(high);
        h.shadow += u64::from(low);
    }
    h
}

/// Índice da faixa de um byte (0..=255) -> `0..BINS`.
#[must_use]
pub fn bin_of(value: u8) -> usize {
    let v = u32::from(value);
    let span = 255 - BINS_CLIP;
    if v <= BINS_CLIP {
        0
    } else {
        (((v - BINS_CLIP) * (BINS as u32 - 1)) / span) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::apply_to_image;

    fn gray(level: u8) -> image::DynamicImage {
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(4, 4, image::Rgb([level; 3])))
    }

    fn px(img: &image::DynamicImage) -> [u8; 3] {
        let rgb = img.to_rgb8();
        let p = rgb.get_pixel(0, 0).0;
        [p[0], p[1], p[2]]
    }

    #[test]
    fn clean_adjust_is_the_identity() {
        let img = gray(120);
        assert_eq!(px(&apply_to_image(&img, &Default::default())), px(&img));
    }

    #[test]
    fn positive_exposure_brightens_and_negative_darkens() {
        let img = gray(120);
        let up = apply_to_image(
            &img,
            &crate::editor::EditorState {
                adjust: Adjust {
                    exposure: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let down = apply_to_image(
            &img,
            &crate::editor::EditorState {
                adjust: Adjust {
                    exposure: -1.0,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        // +1 EV dobra a luz: em gamma isso é bem mais que 2x.
        assert!(px(&up)[0] > px(&img)[0], "+EV deve clarear");
        assert!(px(&down)[0] < px(&img)[0], "-EV deve escurecer");
    }

    #[test]
    fn plus_and_minus_one_ev_cancel_out() {
        let img = gray(90);
        let both = apply_to_image(
            &img,
            &crate::editor::EditorState {
                adjust: Adjust {
                    exposure: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let back = apply_to_image(
            &both,
            &crate::editor::EditorState {
                adjust: Adjust {
                    exposure: -1.0,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        // Tollerance de 1 nível: ida e volta em gamma não fecha em bits.
        assert!(
            px(&back)[0].abs_diff(px(&img)[0]) <= 2,
            "{} -> {} -> {}",
            px(&img)[0],
            px(&both)[0],
            px(&back)[0]
        );
    }

    #[test]
    fn exposure_extremes_stay_inside_the_byte_range() {
        // Um `as u8` de valor fora de 0..=255 daria wrap (255 -> 0) e a foto
        // viraria ruído preto-e-branco. O clamp é o que evita isso.
        for ev in [-EXPOSURE_MAX, EXPOSURE_MAX] {
            for start in [0u8, 1, 128, 254, 255] {
                let mut buf = [start; 4];
                adjust_pixels(
                    &mut buf,
                    &Adjust {
                        exposure: ev,
                        ..Default::default()
                    },
                );
                // `u8` não tem como "passar" de 255: o teste é o wrap.
                assert_eq!(
                    buf[0],
                    wrap_free(start, ev),
                    "wrap em EV {ev} a partir de {start}"
                );
            }
        }
    }

    /// Valor esperado do branco/pretos após o ajuste, calculado à mão.
    fn wrap_free(start: u8, ev: f32) -> u8 {
        let v = srgb_to_linear(f32::from(start) / 255.0) * 2f32.powf(ev);
        to_u8(linear_to_srgb(v))
    }

    #[test]
    fn full_negative_saturation_is_pure_gray() {
        // Um pixel colorido tem de virar cinza: sem croma, os três canais
        // convergem para a luminância.
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            2,
            2,
            image::Rgb([200, 40, 90]),
        ));
        let out = apply_to_image(
            &img,
            &crate::editor::EditorState {
                adjust: Adjust {
                    saturation: -1.0,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let [r, g, b] = px(&out);
        assert_eq!(r, g, "R/G devem convergir");
        assert_eq!(g, b, "G/B devem convergir");
    }

    #[test]
    fn warm_pushes_red_up_and_blue_down() {
        let img = gray(128);
        let out = apply_to_image(
            &img,
            &crate::editor::EditorState {
                adjust: Adjust {
                    temperature: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let [r, _g, b] = px(&out);
        assert!(r > 128, "vermelho deveria subir, veio {r}");
        assert!(b < 128, "azul deveria descer, veio {b}");
    }

    #[test]
    fn cold_is_the_mirror_of_warm() {
        let img = gray(128);
        let cold = apply_to_image(
            &img,
            &crate::editor::EditorState {
                adjust: Adjust {
                    temperature: -1.0,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        assert!(px(&cold)[2] > 128, "azul deveria subir no frio");
        assert!(px(&cold)[0] < 128);
    }

    #[test]
    fn contrast_pushes_mid_gray_apart() {
        let img = gray(128);
        let hard = apply_to_image(
            &img,
            &crate::editor::EditorState {
                adjust: Adjust {
                    contrast: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        // 0.5 codificado é o pivô: meio-cinza exato não deve se mover.
        assert!(
            px(&hard)[0].abs_diff(128) <= 2,
            "pivô foi deslocado: {}",
            px(&hard)[0]
        );
        let dark = apply_to_image(
            &img,
            &crate::editor::EditorState {
                adjust: Adjust {
                    contrast: -1.0,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        assert!(px(&dark)[0] > 100, "cinza médio deveria ir pro meio");
    }

    #[test]
    fn srgb_and_linear_roundtrip() {
        for v in [0u8, 1, 45, 128, 200, 254, 255] {
            let f = f32::from(v) / 255.0;
            let back = to_u8(linear_to_srgb(srgb_to_linear(f)));
            assert!(back.abs_diff(v) <= 1, "{v} -> {back}");
        }
    }

    #[test]
    fn linear_is_darker_than_gamma_in_the_midtones() {
        // 0.5 codificado está bem acima de 0.5 linear: é por isso que a
        // exposição precisa inverter a curva em vez de só multiplicar.
        assert!(srgb_to_linear(0.5) < 0.5);
        assert!(srgb_to_linear(1.0) > 0.99);
        assert!(srgb_to_linear(0.0) < 0.001);
    }

    #[test]
    fn alpha_is_never_touched() {
        let mut buf = [10u8, 20, 30, 77];
        let adj = Adjust {
            exposure: 2.0,
            contrast: 0.5,
            saturation: 0.5,
            temperature: 0.5,
        };
        adjust_pixels(&mut buf, &adj);
        assert_eq!(buf[3], 77, "alpha não pode virar transparente");
    }

    #[test]
    fn clamped_keeps_every_field_in_range() {
        let wild = Adjust {
            exposure: 99.0,
            contrast: -42.0,
            saturation: 7.0,
            temperature: -30.0,
        }
        .clamped();
        assert!(wild.exposure.abs() <= EXPOSURE_MAX);
        assert!(wild.contrast.abs() <= 1.0);
        assert!(wild.saturation.abs() <= 1.0);
        assert!(wild.temperature.abs() <= 1.0);
    }

    #[test]
    fn eq_approx_tolerates_slider_jitter() {
        let a = Adjust {
            contrast: 0.5,
            ..Default::default()
        };
        let b = Adjust {
            contrast: 0.5 + 1e-7,
            ..Default::default()
        };
        assert!(a.eq_approx(&b), "ruído de f32 não pode gerar histórico");
        let c = Adjust {
            contrast: 0.6,
            ..Default::default()
        };
        assert!(!a.eq_approx(&c));
    }

    #[test]
    fn histogram_counts_every_channel() {
        let buf = [
            255, 0, 0, 255, // R estourado
            0, 255, 0, 255, // G estourado
            0, 0, 255, 255, // B estourado
        ];
        let h = histogram(&buf, 1);
        assert_eq!(h.r[BINS - 1], 1, "vermelho na última faixa");
        assert_eq!(h.g[BINS - 1], 1, "verde na última faixa");
        assert_eq!(h.b[BINS - 1], 1, "azul na última faixa");
        assert_eq!(h.total(), 3);
        assert_eq!(h.blowout, 3);
        assert_eq!(h.shadow, 0);
        assert!(h.has_clipping());
    }

    #[test]
    fn histogram_detects_crushed_shadows() {
        // Só conta preto travado quando **todos** os canais estão no chão: um
        // pixel (0, 200, 0) é verde saturado, não sombra perdida.
        let buf = [0, 0, 0, 255, 3, 3, 3, 255, 0, 200, 0, 255];
        let h = histogram(&buf, 1);
        assert_eq!(h.shadow, 2, "dois pixels pretos de verdade");
        assert_eq!(h.blowout, 0);
    }

    #[test]
    fn histogram_sampling_divides_the_work_not_the_result_shape() {
        // 4 pixels, `step_by(2)` entra em 2 deles: os de índice par.
        let buf = [
            10, 10, 10, 255, 10, 10, 10, 255, 240, 240, 240, 255, 240, 240, 240, 255,
        ];
        let all = histogram(&buf, 1);
        let half = histogram(&buf, 2);
        assert_eq!(half.total(), all.total() / 2);
        assert!(half.peak() > 0);
        // Só o pixel de índice 2 é claro (240): o 3º não entra na amostra.
        assert_eq!(half.r[bin_of(240)], 1);
        assert_eq!(half.r[bin_of(10)], 1);
    }

    #[test]
    fn sample_zero_would_panic_and_must_not() {
        let buf = [128u8, 128, 128, 255];
        assert!(histogram(&buf, 0).total() > 0);
    }

    #[test]
    fn bins_cover_the_whole_byte_range() {
        assert_eq!(bin_of(0), 0);
        assert_eq!(bin_of(255), BINS - 1);
        // Monotônico: sem buracos entre o início e o fim.
        for v in 0..=255u16 {
            let a = bin_of(v as u8);
            let b = bin_of((v + 1).min(255) as u8);
            assert!(b >= a, "desmonotônico em {v}: {a} -> {b}");
        }
    }

    #[test]
    fn peak_is_the_tallest_bar_of_any_channel() {
        let mut h = Histogram::default();
        h.g[10] = 7;
        h.b[3] = 12;
        assert_eq!(h.peak(), 12);
    }

    #[test]
    fn empty_histogram_is_zero_not_a_division_error() {
        let h = histogram(&[], 1);
        assert_eq!(h.total(), 0);
        assert_eq!(h.peak(), 0);
        assert!(!h.has_clipping());
    }
}
