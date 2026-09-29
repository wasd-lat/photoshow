//! Painel esquerdo: favoritas, árvore de pastas e lista de fotos.

use crate::prelude::*;
use freya::components::{Button, ScrollView, VirtualItem, VirtualScrollView};
use freya::radio::Radio;

use crate::fs_browser::PhotoPath;
use crate::icons;

use super::services::Services;
use super::state::{self, AppChannel, AppState, channel};

/// Altura de cada linha da lista de fotos.
const ROW: f32 = 24.0;
/// Proporção da altura do painel devote à árvore (o resto é a lista de fotos).
const TREE_PERCENT: f32 = 34.0;

#[derive(PartialEq, Clone)]
pub struct Browser;

impl Browser {}

impl Component for Browser {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Services>();
        let photos = channel(AppChannel::Photos);
        let config = channel(AppChannel::Config);
        let status = channel(AppChannel::Status);

        let snapshot = photos.read().clone();
        let favorites = config.read().config.favorites.clone();
        let scanning = status.read().scanning.is_some();

        rect()
            .width(Size::fill())
            .expanded()
            .vertical()
            // Mesma armadilha do dock: `fill` no eixo principal come tudo que
            // sobra depois dele. Árvore e lista se dividem por `flex`.
            .content(Content::flex())
            .child(
                ScrollView::new()
                    .width(Size::fill())
                    .height(Size::flex(TREE_PERCENT))
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding(8.)
                            .spacing(6.)
                            .child(section_header("Favoritas"))
                            .maybe(favorites.is_empty(), |el| {
                                el.child(weak("Nenhuma pasta fixada."))
                            })
                            .children(
                                favorites
                                    .into_iter()
                                    .map(|dir| favorite_row(dir, photos, config, services.clone())),
                            ),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding(8.)
                            .spacing(6.)
                            .child(section_header("Pasta atual"))
                            .maybe(snapshot.tree.is_none(), |el| {
                                el.child(weak("Nenhuma pasta aberta."))
                            })
                            .maybe_child(snapshot.tree.clone().map(|root| {
                                current_folder(root, photos, config, services.clone())
                            })),
                    ),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(100. - TREE_PERCENT))
                    .vertical()
                    .padding(8.)
                    .spacing(4.)
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(6.)
                            .child(label().text(format!("Fotos ({})", snapshot.visible.len())))
                            .maybe(scanning, |el| el.child(weak("varrendo…"))),
                    )
                    .child(photo_list(snapshot.visible, snapshot.sel, photos, services)),
            )
    }
}

/// Cabeçalho de seção estilo Finder: versalete cinza, compacto.
fn section_header(text: &str) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .padding(Gaps::new(4., 0., 0., 0.))
        .child(
            label()
                .font_size(12.0)
                .color(Color::from_argb(120, 0, 0, 0))
                .text(text.to_uppercase()),
        )
}

/// Texto secundário.
fn weak(text: &str) -> impl IntoElement {
    label()
        .font_size(13.0)
        .color(Color::from_argb(120, 0, 0, 0))
        .text(text.to_owned())
}

/// Linha de uma pasta favorita: nome + botão de desafixar.
fn favorite_row(
    dir: std::path::PathBuf,
    photos: Radio<AppState, AppChannel>,
    config: Radio<AppState, AppChannel>,
    services: Services,
) -> impl IntoElement {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.display().to_string());
    let active = photos.read().current_dir.as_deref() == Some(dir.as_path());
    let for_open = dir.clone();
    let for_unpin = dir;

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(6.)
        .corner_radius(6.)
        .maybe(active, |el| {
            el.background(Color::from_argb(40, 10, 132, 255))
        })
        .child(icons::icon("folder"))
        .child(
            Button::new()
                .flat()
                .expanded()
                .on_press(move |_| {
                    let mut radio = photos;
                    let mut st = radio.write();
                    state::open_dir_path(&services, &mut st, for_open.clone());
                })
                .child(name),
        )
        .child(
            Button::new()
                .flat()
                .compact()
                .on_press(move |_| {
                    let mut radio = config;
                    let mut st = radio.write();
                    state::toggle_favorite(&mut st, &for_unpin);
                    st.config.save().ok();
                })
                .child(icons::icon("x")),
        )
}

/// Cabeçalho da pasta atual: nome + fixar/desafixar, e a árvore abaixo.
fn current_folder(
    root: state::DirNode,
    photos: Radio<AppState, AppChannel>,
    config: Radio<AppState, AppChannel>,
    services: Services,
) -> impl IntoElement {
    let current = photos.read().current_dir.clone();
    let pinned = current
        .as_ref()
        .map(|d| config.read().config.is_favorite(d))
        .unwrap_or(false);
    let tree = root.clone();

    rect()
        .width(Size::fill())
        .spacing(4.)
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(6.)
                .child(icons::icon("folder-open"))
                .child(root.name())
                .child(
                    Button::new()
                        .flat()
                        .compact()
                        .on_press(move |_| {
                            let mut radio = config;
                            let mut st = radio.write();
                            let Some(dir) = st.current_dir.clone() else {
                                return;
                            };
                            state::toggle_favorite(&mut st, &dir);
                            st.config.save().ok();
                        })
                        .child(icons::icon_tinted(
                            "star",
                            14.0,
                            if pinned {
                                Color::from_rgb(255, 200, 0)
                            } else {
                                Color::from_argb(255, 115, 115, 128)
                            },
                        )),
                ),
        )
        .child(
            ScrollView::new()
                .width(Size::fill())
                .height(Size::fill())
                .child(tree_rows(tree, current, photos, config, services)),
        )
}

/// Lista recursiva de nós da árvore (pré-ordem, indentação por profundidade).
fn tree_rows(
    node: state::DirNode,
    current: Option<std::path::PathBuf>,
    photos: Radio<AppState, AppChannel>,
    config: Radio<AppState, AppChannel>,
    services: Services,
) -> impl IntoElement {
    let show_hidden = config.read().config.show_hidden_folders;
    let expanded = node.expanded;
    let mut kids = node
        .children
        .clone()
        .unwrap_or_else(|| state::child_dirs(&node.path, show_hidden));
    kids.sort_by_key(|k| k.name().to_lowercase());

    rect()
        .width(Size::fill())
        .child(tree_row(node, current.clone(), photos, services.clone()))
        .maybe(expanded, |el| {
            el.children(
                kids.into_iter()
                    .map(|kid| tree_rows(kid, current.clone(), photos, config, services.clone())),
            )
        })
}

/// Uma linha da árvore: careta de expansão + nome da pasta.
fn tree_row(
    node: state::DirNode,
    current: Option<std::path::PathBuf>,
    photos: Radio<AppState, AppChannel>,
    services: Services,
) -> impl IntoElement {
    let path = node.path.clone();
    let active = current.as_deref() == Some(path.as_path());
    let has_kids = node.children.is_some() || !state::child_dirs(&path, true).is_empty();
    let expanded = node.expanded;
    let toggle_path = path.clone();
    let open_path = path;

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(4.)
        .corner_radius(6.)
        .maybe(active, |el| {
            el.background(Color::from_argb(40, 10, 132, 255))
        })
        .child(if has_kids {
            Button::new()
                .flat()
                .compact()
                .on_press(move |_| {
                    let mut radio = photos;
                    let mut st = radio.write();
                    let Some(root) = st.tree.as_mut() else {
                        return;
                    };
                    if let Some(index) = preorder_index(root, &toggle_path) {
                        state::toggle_node(root, index);
                    }
                })
                .child(icons::icon(if expanded {
                    "chevron-down"
                } else {
                    "chevron-right"
                }))
                .into_element()
        } else {
            rect().width(Size::px(20.)).into_element()
        })
        .child(
            Button::new()
                .flat()
                .expanded()
                .on_press(move |_| {
                    let mut radio = photos;
                    let mut st = radio.write();
                    state::open_subdir(&services, &mut st, &open_path);
                })
                .child(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(6.)
                        .child(icons::icon("folder"))
                        .child(node.name()),
                ),
        )
}

/// Índice de pré-ordem de um nó (para o toggle por caminho).
fn preorder_index(root: &state::DirNode, target: &std::path::Path) -> Option<usize> {
    let mut counter = 0usize;
    walk_index(root, target, &mut counter)
}

fn walk_index(
    node: &state::DirNode,
    target: &std::path::Path,
    counter: &mut usize,
) -> Option<usize> {
    let mine = *counter;
    *counter += 1;
    if node.path == target {
        return Some(mine);
    }
    if node.expanded
        && let Some(kids) = node.children.as_ref()
    {
        for kid in kids {
            if let Some(found) = walk_index(kid, target, counter) {
                return Some(found);
            }
        }
    }
    None
}

/// Lista virtualizada de fotos: 50k itens custam ~40 linhas.
///
/// A seleção vai dentro do `builder_data` de propósito: o `PartialEq` do
/// `VirtualScrollView` compara só `builder_data`, `item_size`, `length` e o
/// layout. Capturada só na closure, uma troca de seleção com a mesma lista
/// faria o componente parecer "igual" e o realce ficaria congelado na foto
/// anterior.
fn photo_list(
    visible: Vec<PhotoPath>,
    sel: Option<usize>,
    photos: Radio<AppState, AppChannel>,
    services: Services,
) -> impl IntoElement {
    if visible.is_empty() {
        return weak("Nenhuma foto.").into_element();
    }
    let count = visible.len();

    VirtualScrollView::new_with_data(
        (visible, sel),
        move |item: VirtualItem, data: &(Vec<PhotoPath>, Option<usize>)| {
            let (list, sel) = data;
            let Some(photo) = list.get(item.index) else {
                return rect()
                    .key(item.index)
                    .height(Size::px(item.size))
                    .into_element();
            };
            let selected = Some(item.index) == *sel;
            let path = photo.path().to_path_buf();
            let services = services.clone();
            let photo = photo.clone();
            let index = item.index;
            rect()
                .key(item.index)
                .width(Size::fill())
                .height(Size::px(item.size))
                .horizontal()
                .cross_align(Alignment::Center)
                .padding(3.)
                .corner_radius(6.)
                .background(Color::from_argb(30, 0, 0, 0))
                .maybe(selected, |el| {
                    el.background(Color::from_argb(60, 10, 132, 255))
                })
                .child(photo.display_name())
                .on_press(move |_| {
                    let mut radio = photos;
                    let mut st = radio.write();
                    let idx = st
                        .visible
                        .iter()
                        .position(|p| p.path() == path)
                        .unwrap_or(index);
                    state::select_photo(&mut st, &services, idx, photo.clone());
                })
                .into_element()
        },
    )
    .length(count)
    .item_size(ROW)
    .into_element()
}
