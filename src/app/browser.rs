//! Painel esquerdo: favoritas, árvore de pastas e lista de fotos.

use crate::prelude::*;
use freya::components::{
    Button, PanelSize, ResizableContainer, ResizablePanel, ScrollView, VirtualItem,
    VirtualScrollView,
};
use freya::radio::Radio;

use crate::fs_browser::PhotoPath;
use crate::ui;

use super::services::Services;
use super::state::{self, AppChannel, AppState, channel};

/// Padding vertical de cada linha da lista de fotos.
const ROW_PAD: f32 = 3.0;
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
        let m = ui::Metrics::new(config.read().config.ui_scale);
        let favorites = config.read().config.favorites.clone();
        let scanning = status.read().scanning.is_some();
        let hide_tree = config.read().config.hide_tree;
        let hide_photos = config.read().config.hide_photos;
        // Cores de texto vêm da paleta: o painel usa o fundo do tema, então
        // um cinza claro fixo sumia nos temas claros.
        let theme_name = config.read().config.theme.clone();
        let pal = crate::theme::palette(&theme_name);
        let colors = PanelColors {
            text: pal.text_primary.to_color(),
            sel_bg: selection_bg(&theme_name),
            faint: pal.text_secondary.to_color(),
        };
        let faint_c = colors.faint;
        // A altura da linha deriva da tipografia: com fonte maior, uma altura
        // fixa faria o texto vazar para a linha de baixo.
        let row = row_height(&m);

        // Árvore (favoritas + pasta atual) — pode ser recolhida independentemente.
        let tree_section = rect().width(Size::fill()).child(
            ScrollView::new()
                .width(Size::fill())
                .height(Size::fill())
                .child(
                    rect()
                        .width(Size::fill())
                        .padding(ui::gaps(&m, 2., 2.))
                        .spacing(m.gap(1.5))
                        .child(ui::section(&m, faint_c, "Favoritas"))
                        .maybe(favorites.is_empty(), |el| {
                            el.child(ui::faint(&m, faint_c, "Nenhuma pasta fixada."))
                        })
                        .children(
                            favorites
                                .into_iter()
                                .map(|dir| favorite_row(&m, dir, colors, photos, services.clone())),
                        ),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .padding(ui::gaps(&m, 2., 2.))
                        .spacing(m.gap(1.5))
                        .child(ui::section(&m, faint_c, "Pasta atual"))
                        .maybe(snapshot.tree.is_none(), |el| {
                            el.child(ui::faint(&m, faint_c, "Nenhuma pasta aberta."))
                        })
                        .maybe_child(snapshot.tree.clone().map(|root| {
                            current_folder(&m, root, colors, photos, config, services.clone())
                        })),
                ),
        );

        // Lista de fotos — pode ser recolhida independentemente.
        let photos_section = rect()
            .width(Size::fill())
            .vertical()
            .padding(ui::gaps(&m, 2., 2.))
            .spacing(m.gap(1.))
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(m.gap(1.5))
                    .child(ui::text(
                        &m,
                        ui::Role::Small,
                        faint_c,
                        format!("Fotos ({})", snapshot.visible.len()),
                    ))
                    .maybe(scanning, |el| el.child(ui::faint(&m, faint_c, "varrendo…"))),
            )
            .child(photo_list(
                &m,
                row,
                snapshot.visible,
                snapshot.sel,
                colors,
                services,
            ));

        rect().width(Size::fill()).expanded().child({
            // Se ambas recolhidas, não mostra nada (evita alça vazia).
            if hide_tree && hide_photos {
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .into_element()
            } else if hide_tree {
                // Só lista de fotos.
                ResizableContainer::new()
                    .direction(Direction::Vertical)
                    .panel(
                        ResizablePanel::new(PanelSize::percent(100.))
                            .min_size(20.)
                            .child(photos_section),
                    )
                    .into_element()
            } else if hide_photos {
                // Só árvore.
                ResizableContainer::new()
                    .direction(Direction::Vertical)
                    .panel(
                        ResizablePanel::new(PanelSize::percent(100.))
                            .min_size(20.)
                            .child(tree_section),
                    )
                    .into_element()
            } else {
                // Ambas visíveis: divisível.
                ResizableContainer::new()
                    .direction(Direction::Vertical)
                    .panel(
                        ResizablePanel::new(PanelSize::percent(TREE_PERCENT))
                            .min_size(8.)
                            .child(tree_section),
                    )
                    .panel(
                        ResizablePanel::new(PanelSize::percent(100. - TREE_PERCENT))
                            .min_size(8.)
                            .child(photos_section),
                    )
                    .into_element()
            }
        })
    }
}

/// Cores de texto/realce do painel, resolvidas da paleta uma vez no `render`.
#[derive(Clone, Copy)]
struct PanelColors {
    /// Nomes de pastas e fotos.
    text: Color,
    /// Fundo da linha selecionada.
    sel_bg: Color,
    /// Texto de apoio ("Nenhuma pasta…", contadores, cabeçalhos).
    faint: Color,
}

/// Linha de uma pasta favorita: nome + botão de desafixar.
fn favorite_row(
    m: &ui::Metrics,
    dir: std::path::PathBuf,
    colors: PanelColors,
    photos: Radio<AppState, AppChannel>,
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
        .spacing(m.gap(1.5))
        .corner_radius(m.radius_sm())
        .maybe(active, |el| el.background(colors.sel_bg))
        .child(ui::svg(m, "folder"))
        .child(
            Button::new()
                .flat()
                .expanded()
                .on_press(move |_| {
                    state::update(AppChannel::Photos, |st| {
                        state::open_dir_path(&services, st, for_open.clone());
                    });
                })
                .child(ui::text(m, ui::Role::Body, colors.text, name)),
        )
        .child(ui::icon_button(
            m,
            "x",
            "Desafixar",
            Button::new().flat().on_press(move |_| {
                state::update(AppChannel::Config, |st| {
                    state::toggle_favorite(st, &for_unpin);
                    st.config.save().ok();
                });
            }),
        ))
}

/// Altura de uma linha da lista: fonte + entrelinha + padding, arredondado.
fn row_height(m: &ui::Metrics) -> f32 {
    let line = m.font(ui::Role::Body) * 1.35;
    (line + ROW_PAD * 2.0 * m.scale()).ceil()
}

/// Fundo do realce de seleção: visível no tema vigente.
/// Sobre fundo escuro um véu branco funciona; sobre fundo claro ele some,
/// então lá o véu é escuro.
fn selection_bg(theme: &str) -> Color {
    if crate::theme::is_dark(theme) {
        Color::from_argb(40, 255, 255, 255)
    } else {
        Color::from_argb(28, 0, 0, 0)
    }
}

/// Cabeçalho da pasta atual: nome + fixar/desafixar, e a árvore abaixo.
fn current_folder(
    m: &ui::Metrics,
    root: state::DirNode,
    colors: PanelColors,
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
        .spacing(m.gap(1.))
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(m.gap(1.5))
                .child(ui::svg(m, "folder-open"))
                .child(ui::text(m, ui::Role::Body, colors.text, root.name()))
                .child(
                    // O alvo do star tem área de clique mínima (24px). O
                    // `compact()` antigo dava 14px, e um alvo de 14px é
                    // difícil de acertar — era o star "que não funcionava".
                    ui::icon_button_colored(
                        m,
                        "star",
                        pinned.then_some(Color::from_rgb(255, 200, 0)),
                        if pinned {
                            "Desafixar esta pasta"
                        } else {
                            "Fixar esta pasta"
                        },
                        Button::new().flat().on_press(move |_| {
                            state::update(AppChannel::Config, |st| {
                                let Some(dir) = st.current_dir.clone() else {
                                    return;
                                };
                                state::toggle_favorite(st, &dir);
                                st.config.save().ok();
                            });
                        }),
                    ),
                ),
        )
        .child(
            ScrollView::new()
                .width(Size::fill())
                .height(Size::fill())
                .child(tree_rows(m, tree, colors, current, photos, config, services)),
        )
}

/// Lista recursiva de nós da árvore (pré-ordem, indentação por profundidade).
fn tree_rows(
    m: &ui::Metrics,
    node: state::DirNode,
    colors: PanelColors,
    current: Option<std::path::PathBuf>,
    _photos: Radio<AppState, AppChannel>,
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
        .child(tree_row(m, node, colors, current.clone(), services.clone()))
        .maybe(expanded, |el| {
            el.children(
                kids.into_iter().map(|kid| {
                    tree_rows(m, kid, colors, current.clone(), _photos, config, services.clone())
                }),
            )
        })
}

/// Uma linha da árvore: careta de expansão + nome da pasta.
fn tree_row(
    m: &ui::Metrics,
    node: state::DirNode,
    colors: PanelColors,
    current: Option<std::path::PathBuf>,
    services: Services,
) -> impl IntoElement {
    let path = node.path.clone();
    let active = current.as_deref() == Some(path.as_path());
    let has_kids = node.children.is_some() || !state::child_dirs(&path, true).is_empty();
    let expanded = node.expanded;
    let toggle_path = path.clone();
    let open_path = path;
    let caret = m.icon();

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(m.gap(1.))
        .corner_radius(m.radius_sm())
        .maybe(active, |el| el.background(colors.sel_bg))
        .child(if has_kids {
            Button::new()
                .flat()
                .padding(m.gap(0.5))
                .corner_radius(m.radius_sm())
                .on_press(move |_| {
                    state::update(AppChannel::Photos, |st| {
                        let Some(root) = st.tree.as_mut() else {
                            return;
                        };
                        if let Some(index) = preorder_index(root, &toggle_path) {
                            state::toggle_node(root, index);
                        }
                    });
                })
                .child(ui::svg(
                    m,
                    if expanded {
                        "chevron-down"
                    } else {
                        "chevron-right"
                    },
                ))
                .into_element()
        } else {
            rect().width(Size::px(caret + m.gap(1.))).into_element()
        })
        .child(
            Button::new()
                .flat()
                .expanded()
                .padding(ui::gaps(m, 0., 1.))
                .corner_radius(m.radius_sm())
                .on_press(move |_| {
                    state::update(AppChannel::Photos, |st| {
                        state::open_subdir(&services, st, &open_path);
                    });
                })
                .child(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(m.gap(1.5))
                        .child(ui::svg(m, "folder"))
                        .child(ui::text(m, ui::Role::Body, colors.text, node.name())),
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
    m: &ui::Metrics,
    row: f32,
    visible: Vec<PhotoPath>,
    sel: Option<usize>,
    colors: PanelColors,
    services: Services,
) -> impl IntoElement {
    if visible.is_empty() {
        return ui::faint(m, colors.faint, "Nenhuma foto.").into_element();
    }
    let count = visible.len();
    // `m` é `&Metrics` e a closure do VirtualScrollView é `move`: copiamos o
    // valor (ele é `Copy`) para não emprestar o parâmetro do caller.
    let m = *m;

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
                .padding(ui::gaps_of(m, ROW_PAD, 1.5))
                .overflow(Overflow::Clip)
                .corner_radius(m.radius_sm())
                .maybe(selected, |el| el.background(colors.sel_bg))
                .child(ui::text_one_line(
                    &m,
                    ui::Role::Body,
                    colors.text,
                    photo.display_name(),
                ))
                .on_press(move |_| {
                    state::update(AppChannel::Photos, |st| {
                        let idx = st
                            .visible
                            .iter()
                            .position(|p| p.path() == path)
                            .unwrap_or(index);
                        state::select_photo(st, &services, idx, photo.clone());
                    });
                })
                .into_element()
        },
    )
    .length(count)
    .item_size(row)
    .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_height_grows_with_the_font_scale() {
        let small = row_height(&ui::Metrics::new(0.75));
        let normal = row_height(&ui::Metrics::new(1.0));
        let big = row_height(&ui::Metrics::new(1.4));
        assert!(small < normal, "{small} !< {normal}");
        assert!(normal < big, "{normal} !< {big}");
    }

    #[test]
    fn row_height_fits_one_line_of_body_text() {
        // A linha precisa ser mais alta que a caixa da fonte, senão o texto
        // invade a linha seguinte.
        for scale in [0.75, 1.0, 1.2, 1.4] {
            let m = ui::Metrics::new(scale);
            assert!(
                row_height(&m) >= m.font(ui::Role::Body),
                "escala {scale}: {} < {}",
                row_height(&m),
                m.font(ui::Role::Body)
            );
        }
    }

    #[test]
    fn tree_percent_leaves_room_for_the_photo_list() {
        // A árvore e a lista dividem o painel por `flex`. Testar o par inteiro
        // (e não duas constantes isoladas) pega o erro que importa: um dia em
        // que alguém mudar `TREE_PERCENT` para 70 e a lista virar um sulco.
        let split = (TREE_PERCENT, 100.0 - TREE_PERCENT);
        assert!(split.0 >= 15.0 && split.0 <= 50.0, "árvore: {}", split.0);
        assert!(
            split.1 >= split.0,
            "a lista precisa de mais espaço: {split:?}"
        );
    }
}
