//! Visualizador central: zoom ancorado no cursor, pan, crop e menu de contexto.

use freya::components::{Button, ContextMenu, Menu, MenuItem};

use crate::icons;
use crate::image_store::LoadState;
use crate::prelude::*;
use crate::ui;

use super::crop::{self, DragKind2};
use super::services::{CropCommand, Services};
use super::state::{self, AppChannel, channel};
use super::toolbar;

/// Cor do overlay de crop (fora da seleção).
const DIM: Color = Color::from_argb(140, 0, 0, 0);
/// Contorno da seleção de crop.
const CROP_STROKE: Color = Color::WHITE;
/// Azul de destaque (borda da foto selecionada na galeria).
pub const ACCENT: Color = Color::from_rgb(10, 132, 255);

#[derive(PartialEq, Clone)]
pub struct Viewer;

impl Viewer {}

impl Component for Viewer {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Services>();
        let maximized = channel(AppChannel::Viewer).read().maximized;
        let load = services.load.read().clone();
        let m = ui::Metrics::new(channel(AppChannel::Config).read().config.ui_scale);

        // Guarda o último erro reportado, para não repetir a escrita (e
        // portanto o re-render) a cada frame.
        let mut last_error = use_state(String::new);

        // Estado local do gesto: só este componente precisa saber.
        let area = use_state(ScreenRect::default);
        let mut crop_rect = use_state(|| None::<ScreenRect>);
        let drag = use_state(|| None::<crop::DragKind>);

        let (zoom, offset, crop_mode, aspect, compare, split) = {
            let radio = channel(AppChannel::Viewer);
            let st = radio.read();
            (
                st.zoom,
                st.offset,
                st.crop_mode,
                st.crop_aspect,
                st.compare,
                st.compare_split,
            )
        };
        let ratio = state::crop_ratio(aspect);

        // Comandos vindos da toolbar (botão Aplicar / troca de proporção).
        // Consome a caixa (escrita, sem assinar) e usa o handle assinado.
        // O `read` assina este componente às mudanças da caixa: sem ele, o
        // "Aplicar" (que só escreve na caixa, sem tocar no radio) nunca
        // acordaria o viewer e o crop pareceria não funcionar.
        let mut viewer = channel(AppChannel::Viewer);
        let _ = services.crop_cmd.read();
        if let Some(cmd) = services.take_crop_cmd() {
            let area_now = *area.read();
            let draw = compute_draw(area_now, services.load.read().display_px(), zoom, offset);
            match cmd {
                CropCommand::Apply => {
                    let rect = crop_rect().map(|r| crop_to_screen(r, area_now));
                    let dims = services.load.read().display_px();
                    state::apply_crop(&mut viewer.write(), &services, draw, dims, rect);
                }
                CropCommand::RefitAspect => {
                    let draw_local = draw_to_local(draw, area_now);
                    crop_rect.set(
                        crop_rect().map(|r| state::refit_crop_to_aspect(r, draw_local, aspect)),
                    );
                }
            }
        }

        // Sair do modo crop (ou trocar de foto) descarta a seleção em curso.
        if !crop_mode && crop_rect().is_some() {
            crop_rect.set(None);
        }

        let body: Element = match &load {
            LoadState::Empty => empty_view(&m, &services),
            LoadState::Loading => rect()
                .expanded()
                .center()
                .child(dim_label(&m, "carregando…"))
                .into_element(),
            LoadState::Failed(e) => {
                // A mensagem também vai para a status bar; escrevemos no
                // canal em vez de durante o render.
                if *last_error.peek() != *e {
                    last_error.set(e.clone());
                    set_error(e.clone());
                }
                failed_card(&m)
            }
            LoadState::Loaded { image: handle, .. } => {
                let dims = load.display_px();
                let draw = compute_draw(*area.read(), dims, zoom, offset);
                let has_original = load.base_image().is_some();
                let original = load.base_image().cloned();

                let on_wheel = move |e: Event<WheelEventData>| {
                    let factor = wheel_factor(&e);
                    if factor == 1.0 {
                        return;
                    }
                    // Zoom ancorado no cursor.
                    let cursor = to_vector(&e.element_location);
                    let center = area.read().center().to_vector();
                    let anchor = cursor - center;
                    state::update(AppChannel::Viewer, |st| {
                        st.zoom = (st.zoom * factor).clamp(state::ZOOM_MIN, state::ZOOM_MAX);
                        let next = anchor + (st.offset - anchor) * factor;
                        // Voltar a 1.0 tem que recentralizar, senão a foto
                        // fica deslocada depois de um zoom-out.
                        st.offset = clamp_pan(*area.read(), dims, st.zoom, next);
                    });
                };

                let on_down = {
                    let crop_rect = crop_rect;
                    let mut drag = drag;
                    move |e: Event<PointerEventData>| {
                        // Tudo em coordenadas de tela: é assim que `draw`,
                        // `area` e o crop são calculados.
                        let at = e.global_location();
                        let p = to_local_point(e.data(), *area.read());
                        let double = EventsCombos::<()>::pressed(at) == PressEventType::Double;
                        if double && !crop_mode {
                            state::update(AppChannel::Viewer, |st| {
                                st.zoom = 1.0;
                                st.offset = Vector2D::new(0.0, 0.0);
                            });
                            return;
                        }
                        if crop_mode {
                            start_crop(crop_rect, drag, p);
                        } else if compare
                            && has_original
                            && hit_divider(p, *area.read(), draw, split)
                        {
                            // Clicar na divisória arrasta a divisória — nunca
                            // faz pan. Sem isso o gesto mais óbvio do
                            // comparador seria competido pelo pan da foto.
                            drag.set(Some(crop::DragKind::Compare));
                        } else if can_pan(*area.read(), dims, zoom) {
                            drag.set(Some(crop::DragKind::Pan(at)));
                        }
                    }
                };

                let on_move = {
                    let crop_rect = crop_rect;
                    let mut drag = drag;
                    let draw_for_compare = draw;
                    move |e: Event<PointerEventData>| {
                        let Some(kind) = drag() else {
                            return;
                        };
                        e.prevent_default();
                        let at = e.global_location();
                        let area_now = *area.read();
                        let p = to_local_point(e.data(), area_now);
                        match kind {
                            crop::DragKind::Pan(last) => {
                                let delta = to_vector(&at) - to_vector(&last);
                                state::update(AppChannel::Viewer, |st| {
                                    st.offset =
                                        clamp_pan(area_now, dims, st.zoom, st.offset + delta);
                                });
                                drag.set(Some(crop::DragKind::Pan(at)));
                            }
                            crop::DragKind::Crop(inner) => {
                                let draw = compute_draw(area_now, dims, zoom, offset);
                                update_crop(crop_rect, inner, p, draw, area_now, ratio);
                            }
                            crop::DragKind::Compare => {
                                let fraction = (at.x as f32 - draw_for_compare.min_x())
                                    / draw_for_compare.width().max(1.0);
                                state::update(AppChannel::Viewer, |st| {
                                    state::set_compare_split(st, fraction);
                                });
                            }
                        }
                    }
                };

                rect()
                    .expanded()
                    .background((16, 16, 18))
                    .overflow(Overflow::Clip)
                    // `draw` é o retângulo que a imagem deve ocupar: sem
                    // dimensioná-lo, zoom e pan não mudariam nada na tela.
                    .child(photo(
                        handle.clone(),
                        draw,
                        *area.read(),
                        None,
                        "Foto exibida",
                    ))
                    // Comparador: a base entra por cima, recortada à esquerda da
                    // divisória. Dois nós com `Clip` valem mais que shader.
                    .maybe(compare && has_original && !crop_mode, {
                        move |el| {
                            let (offset, width) = compare_rect(draw, *area.read(), split);
                            el.child(
                                rect()
                                    .position(Position::new_absolute().left(offset.x).top(offset.y))
                                    .width(Size::px(width.max(0.0)))
                                    .height(Size::px(draw.height()))
                                    .overflow(Overflow::Clip)
                                    .child(photo(
                                        original.clone().expect("base presente"),
                                        draw,
                                        *area.read(),
                                        None,
                                        "Foto original",
                                    )),
                            )
                            .child(compare_handle(
                                offset.x + width,
                                offset.y,
                                draw.height(),
                            ))
                        }
                    })
                    .child(crop_overlay(draw, crop_rect(), crop_mode, *area.read()))
                    .on_wheel(on_wheel)
                    .on_pointer_down(on_down)
                    .on_global_pointer_move(on_move)
                    .on_pointer_leave({
                        let mut drag = drag;
                        move |_| {
                            drag.set(None);
                        }
                    })
                    .on_secondary_down(move |_| {
                        ContextMenu::open_from_down(context_menu());
                    })
                    .into_element()
            }
        };

        rect()
            .expanded()
            .background((16, 16, 18))
            .overflow(Overflow::Clip)
            .on_sized({
                let mut area = area;
                move |e: Event<SizedEventData>| {
                    area.set(e.area);
                }
            })
            .child(body)
            .maybe(maximized, |el| el.child(restore_button(&m)))
    }
}

/// Fator de zoom a partir da roda (mouse + trackpad).
fn wheel_factor(e: &Event<WheelEventData>) -> f32 {
    let mut factor = 1.0;
    if e.delta_y != 0.0 {
        factor *= 1.0 - (e.delta_y * 0.0015) as f32;
    }
    if e.delta_x != 0.0 {
        factor *= 1.0 - (e.delta_x * 0.0015) as f32;
    }
    if (factor - 1.0).abs() < f32::EPSILON {
        return 1.0;
    }
    factor.clamp(0.2, 5.0)
}

/// Cursor **de tela** -> ponto relativo à área do viewer.
///
/// `draw`, `crop` e `area` vivem todos em coordenadas de tela (veem do
/// `on_sized`), mas `element_location` de um evento é relativo ao elemento que
/// tem o handler. Misturar os dois punha o retângulo de crop deslocado — de
/// longe o suficiente para o corte cair fora da foto.
fn to_local_point(e: &PointerEventData, area: ScreenRect) -> Point2D {
    Point2D::new(
        e.global_location().x as f32 - area.min_x(),
        e.global_location().y as f32 - area.min_y(),
    )
}

/// Há algo para arrastar neste zoom?
///
/// O `fit` garante que em zoom 1 a imagem **sempre** cabe na viewport, então
/// um teste só de proporção nunca distinguiria "cabe" de "não cabe": é
/// preciso olhar o zoom também. Antes disso, `can_pan` ignorava o zoom,
/// devolvia `false` para toda foto e o pan nunca começava.
#[must_use]
pub fn can_pan(area: ScreenRect, dims: (u32, u32), zoom: f32) -> bool {
    if area.width() <= 0.0 || area.height() <= 0.0 || dims.0 == 0 || dims.1 == 0 {
        return false;
    }
    let draw = compute_draw(area, dims, zoom, Vector2D::new(0.0, 0.0));
    draw.width() > area.width() + 0.5 || draw.height() > area.height() + 0.5
}

/// Desloca o pan para manter a imagem dentro da viewport.
///
/// Sem o clamp, arrastar até a borda solta a foto e ela some; com ele, a
/// imagem para nas bordas e sempre sobra algo visível.
#[must_use]
pub fn clamp_pan(area: ScreenRect, dims: (u32, u32), zoom: f32, offset: Vector2D) -> Vector2D {
    if !can_pan(area, dims, zoom) {
        return Vector2D::new(0.0, 0.0);
    }
    let draw = compute_draw(area, dims, zoom, offset);
    // Overflow horizontal e vertical, calculados separadamente: se a imagem
    // é mais larga que a janela, o eixo curto não pode deslocar.
    let mut out = Vector2D::new(0.0, 0.0);
    let dx = (draw.width() - area.width()) / 2.0;
    if dx > 0.0 {
        out.x = offset.x.clamp(-dx, dx);
    }
    let dy = (draw.height() - area.height()) / 2.0;
    if dy > 0.0 {
        out.y = offset.y.clamp(-dy, dy);
    }
    out
}

/// Cursor global (f64) -> vetor de layout (f32).
fn to_vector(cursor: &CursorPoint) -> Vector2D {
    Vector2D::new(cursor.x as f32, cursor.y as f32)
}

/// Retângulo desenhado da imagem: fit na área + zoom + pan.
#[must_use]
pub fn compute_draw(area: ScreenRect, dims: (u32, u32), zoom: f32, offset: Vector2D) -> ScreenRect {
    if dims.0 == 0 || dims.1 == 0 || area.width() <= 0.0 || area.height() <= 0.0 {
        return area;
    }
    let fit = (area.width() / dims.0 as f32).min(area.height() / dims.1 as f32);
    crop::centered(
        area.center() + offset,
        dims.0 as f32 * fit * zoom,
        dims.1 as f32 * fit * zoom,
    )
}

/// Inicia/retoma o gesto de crop.
fn start_crop(
    mut crop_rect: State<Option<ScreenRect>>,
    mut drag: State<Option<crop::DragKind>>,
    p: Point2D,
) {
    let kind = match crop_rect() {
        Some(r) => {
            if let Some((handle, anchor)) = crop::hit_handle(&r, p) {
                DragKind2::Resize { handle, anchor }
            } else if r.contains(p) {
                DragKind2::Move {
                    offset: p.to_vector() - r.min().to_vector(),
                }
            } else {
                crop_rect.set(None);
                DragKind2::New { anchor: p }
            }
        }
        None => DragKind2::New { anchor: p },
    };
    drag.set(Some(crop::DragKind::Crop(kind)));
}

/// Atualiza o rect de crop conforme o gesto em andamento.
///
/// O rect é guardado em coordenadas **locais** (origem no canto da área do
/// viewer, para o hit-test casar com o ponteiro); o `draw` chega em
/// coordenadas **de tela**. A interseção e o clamp usam o `draw` convertido
/// para local — sem isso a seleção nascia deslocada e o "Aplicar" recebia um
/// rect fora da foto.
fn update_crop(
    mut crop_rect: State<Option<ScreenRect>>,
    kind: DragKind2,
    p: Point2D,
    draw: ScreenRect,
    area: ScreenRect,
    ratio: Option<f32>,
) {
    let draw_local = draw_to_local(draw, area);
    let next = match kind {
        DragKind2::New { anchor } | DragKind2::Resize { anchor, .. } => Some(
            crop::enforce_aspect(anchor, p, ratio)
                .intersection(&draw_local)
                .unwrap_or(draw_local),
        ),
        DragKind2::Move { offset } => crop_rect().map(|r| {
            let size = r.size;
            // Mantém o rect inteiro dentro da área desenhada da foto.
            let min = (p.to_vector() - offset).clamp(
                draw_local.min().to_vector(),
                draw_local.max().to_vector() - size.to_vector(),
            );
            ScreenRect::new(min.to_point(), size)
        }),
    };
    crop_rect.set(next);
}

/// Overlay do crop: escurece fora, borda e alças (posicionamento global).
fn crop_overlay(
    draw: ScreenRect,
    crop: Option<ScreenRect>,
    crop_mode: bool,
    area: ScreenRect,
) -> impl IntoElement {
    if !crop_mode {
        return rect().position(Position::new_global()).into_element();
    }
    // O rect do crop é local (bate com o hit-test); o overlay é global.
    let Some(cr) = crop.map(|r| crop_to_screen(r, area)) else {
        return rect().position(Position::new_global()).into_element();
    };
    // Nunca desenha uma faixa "de dentro para fora": sem a conversão acima, o
    // `intersection` aqui mascararia o retângulo fora de lugar.
    let cr = cr.intersection(&draw).unwrap_or(draw);

    rect()
        .position(Position::new_global())
        .child(band(draw.min_x(), draw.min_y(), cr.min_x(), draw.max_y()))
        .child(band(cr.max_x(), draw.min_y(), draw.max_x(), draw.max_y()))
        .child(band(cr.min_x(), draw.min_y(), cr.max_x(), cr.min_y()))
        .child(band(cr.min_x(), cr.max_y(), cr.max_x(), draw.max_y()))
        .child(
            rect()
                .position(Position::new_global().left(cr.min_x()).top(cr.min_y()))
                .width(Size::px(cr.width()))
                .height(Size::px(cr.height()))
                .border(Border::new().fill(CROP_STROKE).width(2.)),
        )
        .children(crop::Handle::all().into_iter().map(|h| {
            let p = h.point(&cr);
            rect()
                .position(Position::new_global().left(p.x).top(p.y))
                .width(Size::px(crop::HANDLE_BOX))
                .height(Size::px(crop::HANDLE_BOX))
                .background(CROP_STROKE)
                .border(Border::new().fill(Color::BLACK).width(1.5))
        }))
        .into_element()
}

/// Converte um rect **local ao viewer** (como o estado do crop guarda) para o
/// rect **de tela** que o overlay desenha.
///
/// O crop é guardado em coordenadas locais para o hit-test casar com
/// `element_location`; o overlay usa `Position::Global`, que é de tela. Sem
/// esta conversão, o retângulo de seleção nasceria deslocado pelo canto da
/// janela e as alças não acertariam o mouse.
#[must_use]
pub fn crop_to_screen(rect: ScreenRect, area: ScreenRect) -> ScreenRect {
    ScreenRect::new(
        Point2D::new(rect.min_x() + area.min_x(), rect.min_y() + area.min_y()),
        Size2D::new(rect.width(), rect.height()),
    )
}

/// Inverso de [`crop_to_screen`]: traz um rect de tela para o espaço local do
/// viewer (origem no canto da área). É o que o gesto de crop usa para
/// intersectar a seleção (local) com o desenho da foto (tela).
#[must_use]
pub fn draw_to_local(draw: ScreenRect, area: ScreenRect) -> ScreenRect {
    ScreenRect::new(
        Point2D::new(draw.min_x() - area.min_x(), draw.min_y() - area.min_y()),
        draw.size,
    )
}

/// Nó da foto, posicionado em `draw` dentro da área do viewer.
///
/// Fatorado porque o comparador desenha a imagem duas vezes (original e
/// editada) e as duas têm que ocupar exatamente o mesmo retângulo: se
/// divergirem 1px, o split fica torto.
fn photo(
    handle: ImageHandle,
    draw: ScreenRect,
    area: ScreenRect,
    _border: Option<Color>,
    alt: &'static str,
) -> Element {
    image(handle)
        .width(Size::px(draw.width()))
        .height(Size::px(draw.height()))
        // `draw` e `area` vêm do `on_sized`, ou seja, em coordenadas **de
        // tela**. `Position::Absolute` interpreta `left/top` a partir do pai,
        // então passar a coordenada de tela aqui a somaria duas vezes (uma via
        // pai, outra via o próprio valor) e jogaria a imagem para fora da
        // janela. A diferença entre as duas é o offset relativo correto.
        .position(
            Position::new_absolute()
                .left(draw.min_x() - area.min_x())
                .top(draw.min_y() - area.min_y()),
        )
        .aspect_ratio(AspectRatio::None)
        .sampling_mode(SamplingMode::Mitchell)
        .a11y_alt(alt)
        .into_element()
}

/// Retângulo (relativo à área) da metade original do comparador.
///
/// O pai tem `Position::Absolute` com canto no canto da foto, então o retângulo
/// da metade original é só `(0, 0)` + a fração cortada da largura. O corte é na
/// fração `split` da **largura desenhada da foto**, não da janela.
#[must_use]
pub fn compare_rect(draw: ScreenRect, area: ScreenRect, split: f32) -> (Point2D, f32) {
    let offset = Point2D::new(draw.min_x() - area.min_x(), draw.min_y() - area.min_y());
    (offset, draw.width() * split.clamp(0.0, 1.0))
}

/// Raio de captura da divisória (px de tela), generoso o bastante para o mouse.
pub const DIVIDER_GRAB: f32 = 14.0;

/// O cursor está sobre a divisória? (ponto **local** à área do viewer)
///
/// Só dentro da faixa vertical da foto: na barra de cima ou nas laterais não há
/// divisória para arrastar, e roubar o clique dali só atrapalharia o pan.
#[must_use]
pub fn hit_divider(p: Point2D, area: ScreenRect, draw: ScreenRect, split: f32) -> bool {
    if draw.width() <= 0.0 {
        return false;
    }
    let cut = draw.min_x() - area.min_x() + draw.width() * split.clamp(0.0, 1.0);
    let top = draw.min_y() - area.min_y();
    let within_y = p.y >= top && p.y <= top + draw.height();
    within_y && (p.x - cut).abs() <= DIVIDER_GRAB
}

/// Divisória do comparador: linha vertical + alça arrastável.
///
/// Sem a alça, o usuário teria que acertar uma linha de 2px com o mouse — o
/// mesmo erro de alvo pequeno que já virou bug no star de favorito.
fn compare_handle(cut: f32, top: f32, height: f32) -> Element {
    let knob = 22.0;
    rect()
        .position(Position::new_absolute().left(cut - 1.).top(top))
        .width(Size::px(2.))
        .height(Size::px(height))
        .background(Color::from_argb(220, 255, 255, 255))
        .child(
            rect()
                .position(
                    Position::new_absolute()
                        .left(-(knob / 2.))
                        .top(height / 2. - knob / 2.),
                )
                .width(Size::px(knob))
                .height(Size::px(knob))
                .corner_radius(knob / 2.)
                .background(Color::from_argb(230, 20, 20, 22))
                .border(Border::new().fill(Color::WHITE).width(1.)),
        )
        .into_element()
}

/// Faixa escura do overlay (coordenadas globais).
fn band(left: f32, top: f32, right: f32, bottom: f32) -> impl IntoElement {
    rect()
        .position(Position::new_global().left(left).top(top))
        .width(Size::px((right - left).max(0.0)))
        .height(Size::px((bottom - top).max(0.0)))
        .background(DIM)
}

/// Botão flutuante para sair do modo maximizado.
fn restore_button(m: &ui::Metrics) -> impl IntoElement {
    rect()
        .position(Position::new_global().top(m.gap(2.)).right(m.gap(2.)))
        .child(
            Button::new()
                .filled()
                .corner_radius(m.radius())
                .padding(ui::gaps(m, 1., 2.))
                .on_press(|_| {
                    state::update(AppChannel::Viewer, state::toggle_maximize);
                })
                .child(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(m.gap(1.5))
                        .child(ui::svg(m, "grid-2x2"))
                        .child(ui::text(m, ui::Role::Body, INVERSE, "Restaurar painéis")),
                ),
        )
}

/// Estado vazio: mensagem **e** os dois botões de abrir.
///
/// A mensagem sozinha não resolvia nada: o menu Arquivo é um dropdown pequeno
/// no canto superior, e sem ação visível aqui a primeira impressão era "não
/// abre pasta nenhuma".
fn empty_view(m: &ui::Metrics, services: &Services) -> Element {
    let open_folder = {
        let services = services.clone();
        move |_| toolbar::open_folder_dialog(services.clone())
    };
    let open_files = {
        let services = services.clone();
        move |_| toolbar::open_files_dialog(services.clone())
    };

    rect()
        .expanded()
        .center()
        .child(
            rect()
                .vertical()
                .cross_align(Alignment::Center)
                .spacing(m.gap(2.))
                .child(ui::text(m, ui::Role::Body, DIM_TEXT, "Nenhuma foto aberta"))
                .child(
                    rect()
                        .horizontal()
                        .spacing(m.gap(1.5))
                        .child(
                            Button::new().flat().on_press(open_folder).child(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(m.gap(1.5))
                                    .padding(ui::gaps(m, 1., 2.))
                                    .child(ui::svg(m, "folder"))
                                    .child(ui::text(m, ui::Role::Body, INVERSE, "Abrir pasta…")),
                            ),
                        )
                        .child(
                            Button::new().flat().on_press(open_files).child(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(m.gap(1.5))
                                    .padding(ui::gaps(m, 1., 2.))
                                    .child(ui::svg(m, "file-image"))
                                    .child(ui::text(m, ui::Role::Body, INVERSE, "Abrir arquivos…")),
                            ),
                        ),
                ),
        )
        .into_element()
}

/// Card neutro de falha (o detalhe vai para a barra de status).
fn failed_card(m: &ui::Metrics) -> Element {
    rect()
        .expanded()
        .center()
        .child(
            rect()
                .vertical()
                .spacing(m.gap(2.))
                .cross_align(Alignment::Center)
                .padding(m.gap(5.))
                .corner_radius(m.radius())
                .background(Color::from_rgb(34, 34, 37))
                .border(
                    Border::new()
                        .width(1.)
                        .alignment(BorderAlignment::Inner)
                        .fill(Color::from_argb(90, 128, 128, 136)),
                )
                .child(icons::icon_tinted(
                    "image-off",
                    m.gap(10.),
                    Color::from_rgb(150, 150, 155),
                ))
                .child(ui::text(
                    m,
                    ui::Role::Body,
                    INVERSE,
                    "Não foi possível abrir esta imagem",
                ))
                .child(dim_label(m, "arquivo ilegível, incompleto ou corrompido"))
                .child(dim_label(m, "← → para continuar navegando")),
        )
        .into_element()
}

/// Rótulo em cor secundária sobre fundo escuro.
fn dim_label(m: &ui::Metrics, text: &str) -> impl IntoElement {
    ui::text(m, ui::Role::Small, DIM_TEXT, text.to_owned())
}

/// Cor de texto padrão sobre o fundo escuro do visualizador.
const INVERSE: Color = Color::from_rgb(235, 235, 238);
/// Cor de texto secundária sobre o fundo escuro do visualizador.
const DIM_TEXT: Color = Color::from_rgb(150, 150, 156);

/// Linha divisória dentro do menu de contexto.
fn menu_divider() -> impl IntoElement {
    rect()
        .width(Size::fill())
        .height(Size::px(1.))
        .padding(4.)
        .background(Color::from_argb(60, 128, 128, 136))
}

/// Item de menu com ícone.
fn menu_entry(
    name: &str,
    text: &'static str,
    on_press: EventHandler<Event<PressEventData>>,
) -> impl IntoElement {
    MenuItem::new().on_press(on_press).child(
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(8.)
            .child(icons::icon(name))
            .child(text),
    )
}

/// Menu de contexto da imagem (botão direito).
///
/// Chamado de dentro de um `on_secondary_down`, então não pode usar hooks:
/// os serviços vêm do contexto direto e o estado pela estação.
#[must_use]
pub fn context_menu() -> Menu {
    let services = consume_context::<Services>();
    let path = state::snapshot()
        .current
        .as_ref()
        .map(|c| c.path().to_path_buf());

    let copy_path: EventHandler<Event<PressEventData>> = {
        let path = path.clone();
        move |_| {
            if let Some(p) = path.as_ref() {
                match super::clipboard::copy_text(&p.display().to_string()) {
                    Ok(()) => set_status("Caminho copiado."),
                    Err(e) => set_status(&e),
                }
            }
        }
    }
    .into();
    let copy_image: EventHandler<Event<PressEventData>> = ({
        let services = services.clone();
        move |_| {
            if let Some(img) = services.images.full_image() {
                set_status("Copiando imagem…");
                std::thread::spawn(move || {
                    let res = super::clipboard::copy_image(&img);
                    state::update(AppChannel::Status, |st| {
                        st.status = match res {
                            Ok(()) => String::from("Imagem copiada para a área de transferência."),
                            Err(e) => e,
                        };
                    });
                });
            } else {
                set_status("Imagem ainda carregando.");
            }
        }
    })
    .into();
    let open_default: EventHandler<Event<PressEventData>> = ({
        let path = path.clone();
        move |_| {
            if let Some(p) = path.as_ref()
                && let Err(e) = open::that(p)
            {
                set_status(&format!("Falha ao abrir: {e}"));
            }
        }
    })
    .into();
    let reveal: EventHandler<Event<PressEventData>> = ({
        let path = path.clone();
        move |_| {
            if let Some(p) = path.as_ref()
                && let Err(e) = super::clipboard::reveal_in_folder(p)
            {
                set_status(&e);
            }
        }
    })
    .into();
    let rename: EventHandler<Event<PressEventData>> =
        (move |_| state::update(AppChannel::Dialogs, state::open_rename)).into();
    let delete_trash: EventHandler<Event<PressEventData>> = ({
        let services = services.clone();
        move |_| {
            state::update(AppChannel::Photos, |st| {
                state::delete_current_photo(st, &services);
            });
            ContextMenu::close();
        }
    })
    .into();
    let close: EventHandler<Event<PressEventData>> = (move |_| {
        ContextMenu::close();
    })
    .into();

    Menu::new()
        .child(menu_entry("clipboard", "Copiar caminho", copy_path))
        .child(menu_entry("copy", "Copiar imagem", copy_image))
        .child(menu_divider())
        .child(menu_entry(
            "external-link",
            "Abrir com aplicativo padrão",
            open_default,
        ))
        .child(menu_entry("folder-open", "Mostrar na pasta", reveal))
        .child(menu_divider())
        .child(menu_entry("pencil", "Renomear…", rename))
        .child(menu_entry(
            "trash",
            "Mover para a lixeira (Del)",
            delete_trash,
        ))
        .child(menu_entry("x", "Fechar menu", close))
}

/// Abre o menu de contexto da foto selecionada.
///
/// Usa `open_from_down`: o `ContextMenuViewer` rastreia a posição global do
/// ponteiro, então o menu aparece onde o clique aconteceu. `ContextMenu::open`
/// (sem down) usaria a última posição conhecida e abriria no canto da tela.
pub fn open_context_menu() {
    ContextMenu::open_from_down(context_menu());
}

/// Escreve na barra de status (de dentro de um handler).
fn set_status(note: &str) {
    state::update(AppChannel::Status, |st| st.status = note.to_owned());
}

/// Registra a falha de carregamento na status bar.
fn set_error(note: String) {
    state::update(AppChannel::Status, |st| {
        st.status = format!("Falha ao carregar: {note}")
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Área do viewer em coordenadas de tela.
    fn stage() -> ScreenRect {
        ScreenRect::new(Point2D::new(0., 0.), Size2D::new(800., 600.))
    }

    #[test]
    fn fit_keeps_aspect_and_centers() {
        // 4:3 numa área 4:3: preenche sem sobra.
        let d = compute_draw(stage(), (400, 300), 1.0, Vector2D::new(0., 0.));
        assert!((d.width() - 800.).abs() < 0.01);
        assert!((d.height() - 600.).abs() < 0.01);
        assert!((d.min_x() - 0.).abs() < 0.01);
        assert!((d.min_y() - 0.).abs() < 0.01);
    }

    #[test]
    fn fit_letterboxes_the_long_side() {
        // 1:1 num palco 4:3: a largura limita, sobra embaixo e em cima.
        let d = compute_draw(stage(), (100, 100), 1.0, Vector2D::new(0., 0.));
        assert!((d.width() - 600.).abs() < 0.01);
        assert!((d.height() - 600.).abs() < 0.01);
        assert!((d.center().y - 300.).abs() < 0.01);
        assert!((d.center().x - 400.).abs() < 0.01);
    }

    #[test]
    fn zoom_scales_around_the_center() {
        let a = compute_draw(stage(), (400, 300), 1.0, Vector2D::new(0., 0.));
        let b = compute_draw(stage(), (400, 300), 2.0, Vector2D::new(0., 0.));
        assert!((b.width() - a.width() * 2.0).abs() < 0.01);
        assert!((b.center().x - a.center().x).abs() < 0.01);
        assert!((b.center().y - a.center().y).abs() < 0.01);
    }

    #[test]
    fn pan_shifts_by_the_offset() {
        let a = compute_draw(stage(), (400, 300), 1.0, Vector2D::new(0., 0.));
        let b = compute_draw(stage(), (400, 300), 1.0, Vector2D::new(30., -12.));
        assert!((b.min_x() - (a.min_x() + 30.)).abs() < 0.01);
        assert!((b.min_y() - (a.min_y() - 12.)).abs() < 0.01);
    }

    #[test]
    fn degenerate_inputs_do_not_panic() {
        assert_eq!(
            compute_draw(stage(), (0, 100), 1.0, Vector2D::new(0., 0.)),
            stage()
        );
        let empty = ScreenRect::new(Point2D::new(0., 0.), Size2D::new(0., 0.));
        assert_eq!(
            compute_draw(empty, (100, 100), 1.0, Vector2D::new(0., 0.)),
            empty
        );
    }

    /// Retângulo de viewport já deslocado, como o viewer mediria no `on_sized`.
    fn stage_at(x: f32, y: f32, w: f32, h: f32) -> ScreenRect {
        ScreenRect::new(Point2D::new(x, y), Size2D::new(w, h))
    }

    #[test]
    fn pan_is_disabled_at_fit_zoom() {
        // O fit garante que em zoom 1 tudo cabe: não há o que arrastar.
        let area = stage_at(0., 0., 800., 600.);
        for dims in [
            (400, 300),
            (4000, 300),
            (400, 3000),
            (4000, 4000),
            (100, 9000),
        ] {
            assert!(!can_pan(area, dims, 1.0), "dims {dims:?}");
        }
    }

    #[test]
    fn pan_unlocks_as_soon_as_the_image_overflows() {
        let area = stage_at(0., 0., 800., 600.);
        let dims = (400, 300);
        // 1.0 cabe; 1.2 já estoura a largura.
        assert!(!can_pan(area, dims, 1.0));
        assert!(can_pan(area, dims, 1.2));
        assert!(can_pan(area, dims, 3.0));
    }

    #[test]
    fn pan_unlocks_on_the_axis_that_overflows() {
        // Foto alta e estreita (letterbox): o overflow é vertical.
        let area = stage_at(0., 0., 800., 600.);
        let tall = (300, 4000);
        let zoom = 1.6;
        let draw = compute_draw(area, tall, zoom, Vector2D::new(0., 0.));
        assert!(draw.height() > area.height());
        assert!(draw.width() <= area.width() + 0.5);
        assert!(can_pan(area, tall, zoom));
    }

    #[test]
    fn degenerate_area_never_pans() {
        let empty = stage_at(0., 0., 0., 0.);
        assert!(!can_pan(empty, (400, 300), 4.0));
        assert!(!can_pan(stage_at(0., 0., 800., 600.), (0, 300), 4.0));
    }

    #[test]
    fn clamp_stops_panning_at_the_edge() {
        // No limite do pan, a imagem **cobre** a viewport: nenhuma faixa vazia
        // aparece. Não dá para conter a imagem inteira (ela é maior), só para
        // cobrir — e é isso que impede a foto de sumir.
        let area = stage_at(0., 0., 800., 600.);
        let dims = (4000, 3000);
        let zoom = 4.0;
        let far = Vector2D::new(9999., 9999.);
        let clamped = clamp_pan(area, dims, zoom, far);
        let draw = compute_draw(area, dims, zoom, clamped);

        assert!(draw.min_x() <= area.min_x() + 0.5, "faixa vazia à esquerda");
        assert!(draw.max_x() >= area.max_x() - 0.5, "faixa vazia à direita");
        assert!(draw.min_y() <= area.min_y() + 0.5, "faixa vazia em cima");
        assert!(draw.max_y() >= area.max_y() - 0.5, "faixa vazia embaixo");
    }

    #[test]
    fn clamp_covers_the_viewport_on_both_sides() {
        let area = stage_at(0., 0., 800., 600.);
        let dims = (4000, 4000);
        let zoom = 4.0;
        for far in [
            Vector2D::new(-9999., -9999.),
            Vector2D::new(9999., -9999.),
            Vector2D::new(-9999., 9999.),
            Vector2D::new(9999., 9999.),
        ] {
            let clamped = clamp_pan(area, dims, zoom, far);
            let draw = compute_draw(area, dims, zoom, clamped);
            assert!(
                draw.min_x() <= area.min_x() + 0.5
                    && draw.max_x() >= area.max_x() - 0.5
                    && draw.min_y() <= area.min_y() + 0.5
                    && draw.max_y() >= area.max_y() - 0.5,
                "cobertura falhou em {far:?}: {draw:?}"
            );
        }
    }

    #[test]
    fn clamp_is_idempotent() {
        // Aplicar o clamp de novo não move nada: já está no limite.
        let area = stage_at(0., 0., 800., 600.);
        let dims = (4000, 3000);
        let once = clamp_pan(area, dims, 4.0, Vector2D::new(9999., -9999.));
        let twice = clamp_pan(area, dims, 4.0, once);
        assert!((once.x - twice.x).abs() < 0.01);
        assert!((once.y - twice.y).abs() < 0.01);
    }

    #[test]
    fn clamp_never_pulls_the_image_off_center() {
        // Um offset pequeno demais (imagem quase cabendo) é zerado, para a
        // foto não ficar pendurada num canto.
        let area = stage_at(0., 0., 800., 600.);
        let dims = (400, 300);
        let tiny = Vector2D::new(2., 2.);
        // Em zoom 1 não há pan: volta ao centro.
        assert_eq!(clamp_pan(area, dims, 1.0, tiny), Vector2D::new(0., 0.));
        // Em zoom alto, 2px é um offset válido.
        assert_eq!(clamp_pan(area, dims, 4.0, tiny), tiny);
    }

    #[test]
    fn clamp_recenters_at_fit_zoom() {
        // Em zoom 1 a imagem fica centralizada: offset tem de ser zero.
        let area = stage_at(0., 0., 800., 600.);
        assert_eq!(
            clamp_pan(area, (400, 300), 1.0, Vector2D::new(120., -80.)),
            Vector2D::new(0., 0.)
        );
    }

    #[test]
    fn clamp_is_symmetric() {
        let area = stage_at(0., 0., 800., 600.);
        let dims = (4000, 3000);
        let right = clamp_pan(area, dims, 4.0, Vector2D::new(5000., 0.));
        let left = clamp_pan(area, dims, 4.0, Vector2D::new(-5000., 0.));
        assert_eq!(right.x, -left.x);
    }

    #[test]
    fn clamp_locks_the_axis_that_fits() {
        // Imagem larga e baixa: pode deslizar em X, não em Y.
        let area = stage_at(0., 0., 800., 600.);
        let dims = (4000, 300);
        let c = clamp_pan(area, dims, 2.0, Vector2D::new(500., 500.));
        assert!(c.x.abs() > 0.0, "eixo X deveria liberar: {c:?}");
        assert_eq!(c.y, 0.0, "eixo Y não deveria liberar");
    }

    #[test]
    fn to_local_point_is_relative_to_the_viewer() {
        // Regressão de crop: um cursor de tela precisa virar relativo à área,
        // senão o retângulo nasce deslocado pela posição da janela.
        let area = stage_at(435., 43., 800., 600.);
        let ev = PointerEventData::Mouse(MouseEventData {
            global_location: CursorPoint::new(535., 143.),
            element_location: CursorPoint::new(100., 100.),
            button: Some(MouseButton::Left),
        });
        let p = to_local_point(&ev, area);
        assert!((p.x - 100.).abs() < 0.01);
        assert!((p.y - 100.).abs() < 0.01);
    }

    #[test]
    fn crop_rect_converts_between_local_and_screen() {
        // Regressão de crop: o estado guarda o rect em coordenadas locais (para
        // o hit-test casar com `element_location`), mas o overlay desenha em
        // coordenadas de tela. Sem converter, a seleção nascia deslocada pelo
        // canto da janela e as alças não acertavam o mouse.
        let area = stage_at(435., 43., 800., 600.);
        let local = ScreenRect::new(Point2D::new(100., 50.), Size2D::new(200., 150.));
        let screen = crop_to_screen(local, area);
        assert!((screen.min_x() - 535.).abs() < 0.01);
        assert!((screen.min_y() - 93.).abs() < 0.01);
        // O tamanho não muda na conversão.
        assert!((screen.width() - 200.).abs() < 0.01);
        assert!((screen.height() - 150.).abs() < 0.01);
    }

    #[test]
    fn crop_screen_rect_falls_inside_the_draw_rect() {
        // Fluxo completo do gesto: âncora e ponteiro locais -> seleção que
        // cabe na área desenhada da foto.
        let area = stage_at(0., 0., 800., 600.);
        let dims = (400, 300);
        let draw = compute_draw(area, dims, 1.0, Vector2D::new(0., 0.));
        let anchor = Point2D::new(50., 50.);
        let pointer = Point2D::new(400., 400.);
        let sel = crate::app::crop::enforce_aspect(anchor, pointer, None).intersection(&draw);
        let sel = sel.expect("seleção dentro da foto");
        assert!(sel.min_x() >= draw.min_x() && sel.max_x() <= draw.max_x());
        assert!(sel.min_y() >= draw.min_y() && sel.max_y() <= draw.max_y());
    }

    #[test]
    fn crop_respects_the_requested_aspect_ratio() {
        let anchor = Point2D::new(0., 0.);
        let sel = crate::app::crop::enforce_aspect(anchor, Point2D::new(400., 100.), Some(1.0));
        let ratio = sel.width() / sel.height();
        assert!((ratio - 1.0).abs() < 0.01, "ratio {ratio}");
    }

    #[test]
    fn crop_ignores_degenerate_drags() {
        // Arrasto de tamanho zero não pode gerar divisão por zero.
        let zero = crate::app::crop::enforce_aspect(
            Point2D::new(10., 10.),
            Point2D::new(10., 10.),
            Some(1.0),
        );
        assert!(zero.width() >= 0.0 && zero.height() >= 0.0);
    }

    #[test]
    fn draw_to_local_is_the_inverse_of_crop_to_screen() {
        // O gesto guarda o crop em local e o desenho vive em tela: as duas
        // conversões precisam se anular, senão a interseção mistura espaços.
        let area = stage_at(435., 43., 800., 600.);
        let draw = compute_draw(area, (400, 300), 1.0, Vector2D::new(0., 0.));
        let local = draw_to_local(draw, area);
        // Origem no canto da área, mesmo tamanho.
        assert!((local.min_x() - (draw.min_x() - 435.)).abs() < 0.01);
        assert!((local.min_y() - (draw.min_y() - 43.)).abs() < 0.01);
        assert!((local.width() - draw.width()).abs() < 0.01);
        assert!((local.height() - draw.height()).abs() < 0.01);
        // Volta para tela: identidade.
        let back = crop_to_screen(local, area);
        assert!((back.min_x() - draw.min_x()).abs() < 0.01);
        assert!((back.min_y() - draw.min_y()).abs() < 0.01);
    }

    #[test]
    fn crop_gesture_in_local_space_matches_draw_in_screen_space() {
        // Regressão do bug "crop não funciona": o gesto (local) intersectado
        // com o desenho (tela) sem conversão dava vazio e caía no
        // `unwrap_or(draw)` — guardando coords de tela como se fossem locais.
        // Com a conversão, a seleção local cabe no desenho local.
        let area = stage_at(435., 43., 800., 600.);
        let dims = (400, 300);
        let draw = compute_draw(area, dims, 1.0, Vector2D::new(0., 0.));
        let draw_local = draw_to_local(draw, area);
        let anchor = Point2D::new(50., 50.);
        let pointer = Point2D::new(400., 400.);
        let sel = crate::app::crop::enforce_aspect(anchor, pointer, None)
            .intersection(&draw_local)
            .expect("seleção dentro da foto");
        assert!(sel.min_x() >= draw_local.min_x() && sel.max_x() <= draw_local.max_x());
        assert!(sel.min_y() >= draw_local.min_y() && sel.max_y() <= draw_local.max_y());
        // E convertida para tela, cai dentro do desenho de tela.
        let screen = crop_to_screen(sel, area);
        assert!(screen.min_x() >= draw.min_x() - 0.01 && screen.max_x() <= draw.max_x() + 0.01);
        assert!(screen.min_y() >= draw.min_y() - 0.01 && screen.max_y() <= draw.max_y() + 0.01);
        // O px final é válido (antes era `None` = "seleção muito pequena").
        let out = crate::app::crop::crop_to_preview_px(draw, dims, screen).expect("px válido");
        assert!(out.w > 0 && out.h > 0);
    }

    #[test]
    fn compare_split_cuts_the_photo_not_the_window() {
        // A divisória tem de seguir a foto: com a foto menor que o palco, uma
        // fração da janela deixaria a metade original cortando fora da imagem.
        let area = stage_at(0., 0., 1000., 800.);
        let draw = compute_draw(area, (100, 100), 1.0, Vector2D::new(0., 0.));
        let (offset, width) = compare_rect(draw, area, 0.5);
        assert_eq!(offset.x, draw.min_x() - area.min_x());
        assert!((width - draw.width() * 0.5).abs() < 0.01);
        assert!(width <= draw.width() + 0.01, "nunca passa da foto");
    }

    #[test]
    fn compare_split_follows_the_photo_offset() {
        // Com pan/zoom a foto sai do canto: o recorte precisa acompanhar,
        // senão a metade original aparece deslocada da metade editada.
        let area = stage_at(200., 100., 800., 600.);
        let draw = compute_draw(area, (400, 300), 2.0, Vector2D::new(40., -20.));
        let (offset, width) = compare_rect(draw, area, 0.25);
        assert!((offset.x - (draw.min_x() - 200.)).abs() < 0.01);
        assert!((offset.y - (draw.min_y() - 100.)).abs() < 0.01);
        assert!((width - draw.width() * 0.25).abs() < 0.01);
    }

    #[test]
    fn divider_is_grabbable_only_over_the_photo() {
        let area = stage_at(0., 0., 800., 600.);
        let draw = compute_draw(area, (400, 300), 1.0, Vector2D::new(0., 0.));
        let (_, width) = compare_rect(draw, area, 0.5);
        let offset = Point2D::new(draw.min_x() - area.min_x(), draw.min_y() - area.min_y());
        let cut = offset.x + width;

        // Em cima da divisória, dentro da faixa vertical: pega.
        assert!(hit_divider(Point2D::new(cut, 300.), area, draw, 0.5));
        // Perto o suficiente ainda pega (alvo generoso).
        assert!(hit_divider(
            Point2D::new(cut + DIVIDER_GRAB - 0.5, 300.),
            area,
            draw,
            0.5
        ));
        // Fora do alcance lateral: não pega.
        assert!(!hit_divider(
            Point2D::new(cut + DIVIDER_GRAB + 1., 300.),
            area,
            draw,
            0.5
        ));
        // Acima e abaixo da foto: não pega (senão roubaria o pan das bordas).
        assert!(!hit_divider(Point2D::new(cut, -10.), area, draw, 0.5));
        assert!(!hit_divider(
            Point2D::new(cut, draw.height() + 200.),
            area,
            draw,
            0.5
        ));
    }

    #[test]
    fn divider_never_grabs_on_a_degenerate_photo() {
        let empty = stage_at(0., 0., 0., 0.);
        assert!(!hit_divider(Point2D::new(0., 0.), empty, empty, 0.5));
    }
}
