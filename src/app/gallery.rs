//! Galeria de miniaturas: grade fluida que se adapta à largura do painel.

use freya::components::{Button, ScrollView, Slider};

use crate::fs_browser::PhotoPath;
use crate::icons;
use crate::prelude::*;
use crate::thumbs::ThumbMap;

use super::services::Services;
use super::state::{self, AppChannel, AppState, channel};

/// Espaçamento entre células da grade.
const GAP: f32 = 8.0;
/// Quantas fotos ficam em volta da seleção (evita travar o frame).
const WINDOW: usize = 300;
/// Menor/maior lado do thumbnail (mesmo cap do worker de decode).
const CELL_MIN: f32 = 48.0;
const CELL_MAX: f32 = 192.0;

#[derive(PartialEq, Clone)]
pub struct Gallery;

impl Gallery {}

impl Component for Gallery {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Services>();
        let photos = channel(AppChannel::Photos);
        let config = channel(AppChannel::Config);
        let snapshot = photos.read().clone();
        let enabled = config.read().config.show_filmstrip;
        let cell = state::thumb_size(&config.read());

        if !enabled {
            return rect()
                .expanded()
                .padding(10.)
                .child(weak("Galeria desativada — ative em Config."));
        }
        if snapshot.visible.is_empty() {
            return rect().expanded().padding(10.).child(weak("Nenhuma foto."));
        }

        // Enfileira/drena miniaturas da janela em torno da seleção.
        // O `pump_thumbs` fica no render da raiz (ver `app::refresh_thumbs`):
        // escrever no `State` daqui dentro não agendaria o próximo frame.
        let thumbs = services.thumb_map.read().clone();

        rect()
            .expanded()
            .vertical()
            .padding(8.)
            .spacing(6.)
            .child(size_controls(cell))
            .child(ScrollView::new().expanded().child(grid(
                snapshot.visible,
                snapshot.sel,
                thumbs,
                cell,
                photos,
                services,
            )))
    }
}

/// Cabeçalho: `−` slider `＋` e o valor em px.
fn size_controls(cell: f32) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(8.)
        .child(weak("Tamanho:"))
        .child(
            Button::new()
                .flat()
                .compact()
                .on_press(move |_| set_thumb_size(cell - 16.0))
                .child(icons::icon("minus")),
        )
        .child(
            Slider::new(move |v: f64| {
                set_thumb_size(CELL_MIN + (v as f32 / 100.0) * (CELL_MAX - CELL_MIN));
            })
            .value(((cell - CELL_MIN) / (CELL_MAX - CELL_MIN) * 100.0) as f64)
            .size(Size::px(160.))
            .scroll_enabled(false),
        )
        .child(
            Button::new()
                .flat()
                .compact()
                .on_press(move |_| set_thumb_size(cell + 16.0))
                .child(icons::icon("plus")),
        )
        .child(weak(&format!("{cell:.0}px")))
}

/// Aplica e persiste o tamanho do thumbnail.
fn set_thumb_size(size: f32) {
    // Fora do render: `channel` é hook, e handler não pode chamar hook.
    state::update(AppChannel::Config, |st| {
        if state::set_thumb_size(st, size) {
            st.config.save().ok();
        }
    });
}

/// Texto secundário.
fn weak(text: &str) -> impl IntoElement {
    label()
        .font_size(13.0)
        .color((120, 120, 128))
        .text(text.to_owned())
}

/// Grade fluida: `wrap` deixa as colunas se adaptarem à largura do painel.
fn grid(
    photos: Vec<PhotoPath>,
    sel: Option<usize>,
    thumbs: ThumbMap,
    cell: f32,
    photos_radio: Radio<AppState, AppChannel>,
    services: Services,
) -> impl IntoElement {
    let (lo, hi) = match sel {
        Some(s) => (
            s.saturating_sub(WINDOW),
            (s + WINDOW).min(photos.len().saturating_sub(1)),
        ),
        None => (0, photos.len().saturating_sub(1).min(WINDOW)),
    };
    let slice: Vec<(usize, PhotoPath)> = photos[lo..=hi]
        .iter()
        .cloned()
        .enumerate()
        .map(|(k, p)| (lo + k, p))
        .collect();

    rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::wrap_spacing(GAP))
        .children(slice.into_iter().map(|(index, photo)| {
            let handle = thumbs.get(photo.path()).cloned();
            cell_view(
                index,
                photo,
                handle,
                sel,
                cell,
                photos_radio,
                services.clone(),
            )
        }))
}

/// Uma célula da grade: thumb (ou placeholder) + borda de seleção.
#[allow(clippy::too_many_arguments)]
fn cell_view(
    index: usize,
    photo: PhotoPath,
    handle: Option<ImageHandle>,
    sel: Option<usize>,
    cell: f32,
    photos: Radio<AppState, AppChannel>,
    services: Services,
) -> Element {
    let selected = Some(index) == sel;
    let path = photo.path().to_path_buf();
    let photos_click = photos;
    let services_select = services.clone();

    // Duplo-clique numa miniatura maximiza o Visualizador (F9 restaura).
    let on_press = move |e: Event<PointerEventData>| {
        let double = EventsCombos::<()>::pressed(e.global_location()) == PressEventType::Double;
        let mut radio = photos_click;
        let mut st = radio.write();
        let idx = st
            .visible
            .iter()
            .position(|p| p.path() == path)
            .unwrap_or(index);
        state::select_photo(&mut st, &services_select, idx, photo.clone());
        // Duplo-clique maximiza o Visualizador (restaura com F9).
        if double {
            state::toggle_maximize(&mut st);
        }
    };
    let on_press: EventHandler<Event<PointerEventData>> = on_press.into();

    let on_menu: EventHandler<Event<PressEventData>> = (move |_| {
        super::viewer::open_context_menu();
    })
    .into();

    rect()
        .key(index)
        .width(Size::px(cell))
        .height(Size::px(cell))
        .corner_radius(6.)
        .overflow(Overflow::Clip)
        .background((42, 42, 46))
        .maybe_child(handle.map(|h| {
            // Célula quadrada, foto 4:3: sem centralizar sobraria uma faixa
            // morta embaixo de toda miniatura.
            image(h)
                .width(Size::px(cell))
                .height(Size::px(cell))
                .aspect_ratio(AspectRatio::Min)
                .image_cover(ImageCover::Center)
                .sampling_mode(SamplingMode::Mitchell)
        }))
        .maybe(selected, |el| {
            el.border(Border::new().fill(super::viewer::ACCENT).width(2.5))
        })
        .on_pointer_press(on_press)
        .on_secondary_down(on_menu)
        .into_element()
}
