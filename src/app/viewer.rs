//! Visualizador central: zoom ancorado no cursor, pan, crop e menu de contexto.

use freya::components::{Button, ContextMenu, Menu, MenuItem};

use crate::icons;
use crate::image_store::LoadState;
use crate::prelude::*;

use super::crop::{self, DragKind2};
use super::services::{CropCommand, Services};
use super::state::{self, AppChannel, channel};

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

        // Guarda o último erro reportado, para não repetir a escrita (e
        // portanto o re-render) a cada frame.
        let mut last_error = use_state(String::new);

        // Estado local do gesto: só este componente precisa saber.
        let area = use_state(ScreenRect::default);
        let mut crop_rect = use_state(|| None::<ScreenRect>);
        let drag = use_state(|| None::<crop::DragKind>);

        let (zoom, offset, crop_mode, aspect) = {
            let radio = channel(AppChannel::Viewer);
            let st = radio.read();
            (st.zoom, st.offset, st.crop_mode, st.crop_aspect)
        };
        let ratio = state::crop_ratio(aspect);

        // Comandos vindos da toolbar (botão Aplicar / troca de proporção).
        // Consome a caixa (escrita, sem assinar) e usa o handle assinado.
        let mut viewer = channel(AppChannel::Viewer);
        if let Some(cmd) = services.take_crop_cmd() {
            let draw = compute_draw(
                *area.read(),
                services.load.read().display_px(),
                zoom,
                offset,
            );
            match cmd {
                CropCommand::Apply => {
                    let rect = crop_rect();
                    let dims = services.load.read().display_px();
                    state::apply_crop(&mut viewer.write(), &services, draw, dims, rect);
                }
                CropCommand::RefitAspect => {
                    crop_rect
                        .set(crop_rect().map(|r| state::refit_crop_to_aspect(r, draw, aspect)));
                }
            }
        }

        // Sair do modo crop (ou trocar de foto) descarta a seleção em curso.
        if !crop_mode && crop_rect().is_some() {
            crop_rect.set(None);
        }

        let body: Element = match &load {
            LoadState::Empty => placeholder(
                "Nenhuma foto — abra uma pasta ou fixe uma favorita. (F11 = fullscreen)",
            ),
            LoadState::Loading => rect()
                .expanded()
                .center()
                .child(dim_label("carregando…"))
                .into_element(),
            LoadState::Failed(e) => {
                // A mensagem também vai para a status bar; escrevemos no
                // canal em vez de durante o render.
                if *last_error.peek() != *e {
                    last_error.set(e.clone());
                    set_error(e.clone());
                }
                failed_card()
            }
            LoadState::Loaded { image: handle, .. } => {
                let dims = load.display_px();
                let draw = compute_draw(*area.read(), dims, zoom, offset);

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
                        st.offset = anchor + (st.offset - anchor) * factor;
                    });
                };

                let on_down = {
                    let crop_rect = crop_rect;
                    let mut drag = drag;
                    move |e: Event<PointerEventData>| {
                        let p = to_point(&e.element_location());
                        let draw = compute_draw(*area.read(), dims, zoom, offset);
                        let double = EventsCombos::<()>::pressed(e.global_location())
                            == PressEventType::Double;
                        if double && !crop_mode {
                            state::update(AppChannel::Viewer, |st| {
                                st.zoom = 1.0;
                                st.offset = Vector2D::new(0.0, 0.0);
                            });
                            return;
                        }
                        if crop_mode {
                            start_crop(crop_rect, drag, p);
                        } else {
                            drag.set(Some(crop::DragKind::Pan(e.element_location())));
                        }
                        let _ = draw;
                    }
                };

                let on_move = {
                    let crop_rect = crop_rect;
                    let mut drag = drag;
                    move |e: Event<PointerEventData>| {
                        let Some(kind) = drag() else {
                            return;
                        };
                        e.prevent_default();
                        let p = to_point(&e.element_location());
                        let draw = compute_draw(*area.read(), dims, zoom, offset);
                        match kind {
                            crop::DragKind::Pan(last) => {
                                let delta = to_vector(&e.element_location()) - to_vector(&last);
                                state::update(AppChannel::Viewer, |st| st.offset += delta);
                                drag.set(Some(crop::DragKind::Pan(e.element_location())));
                            }
                            crop::DragKind::Crop(inner) => {
                                update_crop(crop_rect, inner, p, draw, ratio);
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
                    .child(
                        image(handle.clone())
                            .width(Size::px(draw.width()))
                            .height(Size::px(draw.height()))
                            .offset_x(draw.min_x() - area.read().min_x())
                            .offset_y(draw.min_y() - area.read().min_y())
                            .aspect_ratio(AspectRatio::None)
                            .sampling_mode(SamplingMode::Mitchell)
                            .a11y_alt("Foto exibida"),
                    )
                    .child(crop_overlay(draw, crop_rect(), crop_mode))
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
            .maybe(maximized, |el| el.child(restore_button()))
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

/// Cursor global (f64) -> ponto de layout (f32).
fn to_point(cursor: &CursorPoint) -> Point2D {
    Point2D::new(cursor.x as f32, cursor.y as f32)
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
fn update_crop(
    mut crop_rect: State<Option<ScreenRect>>,
    kind: DragKind2,
    p: Point2D,
    draw: ScreenRect,
    ratio: Option<f32>,
) {
    let next = match kind {
        DragKind2::New { anchor } | DragKind2::Resize { anchor, .. } => Some(
            crop::enforce_aspect(anchor, p, ratio)
                .intersection(&draw)
                .unwrap_or(draw),
        ),
        DragKind2::Move { offset } => crop_rect().map(|r| {
            let size = r.size;
            // Mantém o rect inteiro dentro da área desenhada da foto.
            let min = (p.to_vector() - offset).clamp(
                draw.min().to_vector(),
                draw.max().to_vector() - size.to_vector(),
            );
            ScreenRect::new(min.to_point(), size)
        }),
    };
    crop_rect.set(next);
}

/// Overlay do crop: escurece fora, borda e alças (posicionamento global).
fn crop_overlay(draw: ScreenRect, crop: Option<ScreenRect>, crop_mode: bool) -> impl IntoElement {
    if !crop_mode {
        return rect().position(Position::new_global()).into_element();
    }
    let Some(cr) = crop.map(|r| r.intersection(&draw).unwrap_or(draw)) else {
        return rect().position(Position::new_global()).into_element();
    };

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

/// Faixa escura do overlay (coordenadas globais).
fn band(left: f32, top: f32, right: f32, bottom: f32) -> impl IntoElement {
    rect()
        .position(Position::new_global().left(left).top(top))
        .width(Size::px((right - left).max(0.0)))
        .height(Size::px((bottom - top).max(0.0)))
        .background(DIM)
}

/// Botão flutuante para sair do modo maximizado.
fn restore_button() -> impl IntoElement {
    rect()
        .position(Position::new_global().top(8.).right(8.))
        .child(
            Button::new()
                .filled()
                .compact()
                .on_press(|_| {
                    state::update(AppChannel::Viewer, state::toggle_maximize);
                })
                .child(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(5.)
                        .child(icons::icon("grid-2x2"))
                        .child("Restaurar painéis"),
                ),
        )
}

/// Mensagem central quando não há foto.
fn placeholder(text: &'static str) -> Element {
    rect()
        .expanded()
        .center()
        .child(label().color((220, 220, 220)).text(text))
        .into_element()
}

/// Card neutro de falha (o detalhe vai para a barra de status).
fn failed_card() -> Element {
    rect()
        .expanded()
        .center()
        .child(
            rect()
                .vertical()
                .spacing(8.)
                .cross_align(Alignment::Center)
                .padding(20.)
                .corner_radius(12.)
                .background((32, 32, 36))
                .child(icons::icon_tinted(
                    "image-off",
                    40.0,
                    Color::from_rgb(150, 150, 155),
                ))
                .child(
                    label()
                        .color((235, 235, 235))
                        .text("Não foi possível abrir esta imagem"),
                )
                .child(dim_label("arquivo ilegível, incompleto ou corrompido"))
                .child(dim_label("← → para continuar navegando")),
        )
        .into_element()
}

/// Rótulo em cor secundária sobre fundo escuro.
fn dim_label(text: &str) -> impl IntoElement {
    label()
        .font_size(13.0)
        .color((160, 160, 165))
        .text(text.to_owned())
}

/// Linha divisória dentro do menu de contexto.
fn menu_divider() -> impl IntoElement {
    rect()
        .width(Size::fill())
        .height(Size::px(1.))
        .padding(4.)
        .background(Color::from_argb(30, 0, 0, 0))
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
    let copy_image: EventHandler<Event<PressEventData>> = (move |_| match services
        .images
        .full_image()
        .map(|img| super::clipboard::copy_image(&img))
    {
        Some(Ok(())) => set_status("Imagem copiada."),
        Some(Err(e)) => set_status(&e),
        None => set_status("Imagem ainda carregando."),
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
        .child(menu_entry("x", "Fechar menu", close))
}

/// Abre o menu de contexto da foto selecionada.
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
}
