//! Barra superior: menu Arquivo, ferramentas de edição e filtro de formato.

use freya::components::{Button, Select, Tooltip, TooltipContainer};

use crate::prelude::*;
use crate::ui;

use super::services::{CropCommand, Services};
use super::state::{self, AppChannel, AppState, channel};
use super::window;

/// Callback de um item de menu (`EventHandler` é `Copy`, closure não é).
type Press = EventHandler<Event<PressEventData>>;

#[derive(PartialEq, Clone)]
pub struct Toolbar;

impl Toolbar {}

impl Component for Toolbar {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Services>();
        let photos = channel(AppChannel::Photos);
        let edit = channel(AppChannel::Edit);
        let mut viewer = channel(AppChannel::Viewer);
        let status = channel(AppChannel::Status);
        let mut dialogs = channel(AppChannel::Dialogs);

        let has_photo = photos.read().current.is_some();
        let dirty = edit.read().editor.is_dirty();
        let saving = status.read().saving;
        let m = ui::Metrics::new(channel(AppChannel::Config).read().config.ui_scale);

        rect()
            .width(Size::fill())
            .horizontal()
            .main_align(Alignment::SpaceBetween)
            .cross_align(Alignment::Center)
            .padding(ui::gaps_of(m, 1., 1.5))
            .background(Color::from_argb(24, 255, 255, 255))
            .border(
                Border::new()
                    .width(1.)
                    .alignment(BorderAlignment::Inner)
                    .fill(Color::from_argb(90, 128, 128, 136)),
            )
            .child(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(m.gap(1.5))
                    .child(file_menu(&m, photos, edit, services.clone(), dirty, saving))
                    .maybe(has_photo, |el| {
                        el.child(vrule(&m)).child(edit_bar(
                            &m,
                            edit,
                            viewer,
                            services.clone(),
                            dirty,
                        ))
                    }),
            )
            .child(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(m.gap(1.5))
                    .child(format_filter(&m, photos, services.clone()))
                    .child(ui::icon_button(
                        &m,
                        "panel-left",
                        "Ocultar/mostrar navegador (Ctrl+1)",
                        Button::new().flat().on_press(move |_| {
                            state::update(AppChannel::Config, |st| {
                                st.config.hide_browser = !st.config.hide_browser;
                                st.config.save().ok();
                            })
                        }),
                    ))
                    .child(ui::icon_button(
                        &m,
                        "panel-bottom",
                        "Ocultar/mostrar galeria (Ctrl+2)",
                        Button::new().flat().on_press(move |_| {
                            state::update(AppChannel::Config, |st| {
                                st.config.hide_gallery = !st.config.hide_gallery;
                                st.config.save().ok();
                            })
                        }),
                    ))
                    .child(ui::icon_button(
                        &m,
                        "fullscreen",
                        "Fullscreen (F11)",
                        Button::new().flat().on_press(move |_| {
                            let next = !viewer.read().fullscreen;
                            viewer.write().fullscreen = next;
                            window::set_fullscreen(next);
                        }),
                    ))
                    .child(ui::icon_button(
                        &m,
                        "settings",
                        "Configurações",
                        Button::new().flat().on_press(move |_| {
                            dialogs.write().settings_open = true;
                        }),
                    )),
            )
    }
}

/// Linha divisória vertical.
fn vrule(m: &ui::Metrics) -> impl IntoElement {
    rect()
        .width(Size::px(1.))
        .height(Size::px(m.gap(5.)))
        .background(Color::from_argb(90, 128, 128, 136))
}

/// Botão com ícone + rótulo, embrulhado em tooltip.
fn tool_button(
    m: &ui::Metrics,
    name: &str,
    text: &'static str,
    tip: &'static str,
    button: Button,
) -> impl IntoElement {
    TooltipContainer::new(Tooltip::new_text(tip)).child(
        button
            .corner_radius(m.radius_sm())
            .padding(ui::gaps(m, 0.5, 1.5))
            .child(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(m.gap(1.5))
                    .child(ui::svg(m, name))
                    .child(ui::text(m, ui::Role::Body, TEXT, text)),
            ),
    )
}

/// Cor do texto padrão da barra (o tema é sempre escuro na barra).
const TEXT: Color = Color::from_rgb(240, 240, 243);

/// Menu Arquivo: abrir, salvar, salvar como, renomear, restaurar layout.
///
/// Usa [`ui::Dropdown`] em vez do `Menu` do Freya: o `Menu` entra no fluxo de
/// layout e empurra a barra inteira para baixo quando abre — o que fazia o
/// clique acertar o item errado e "Abrir pasta…" não abrir nada.
fn file_menu(
    m: &ui::Metrics,
    photos: Radio<AppState, AppChannel>,
    edit: Radio<AppState, AppChannel>,
    services: Services,
    dirty: bool,
    saving: bool,
) -> impl IntoElement {
    let mut open = use_state(|| false);

    let open_folder: Press = {
        let services = services.clone();
        move |_| open_folder_dialog(services.clone())
    }
    .into();
    let open_files: Press = {
        let services = services.clone();
        move |_| open_files_dialog(services.clone())
    }
    .into();
    let do_save: Press = {
        let services = services.clone();
        move |_| save_overwrite(services.clone(), photos, edit)
    }
    .into();
    let do_save_as: Press = {
        let services = services.clone();
        move |_| save_as_dialog(services.clone(), photos, edit)
    }
    .into();
    let do_rename: Press = (move |_| state::update(AppChannel::Dialogs, state::open_rename)).into();
    let do_restore: Press = (move |_| {
        state::update(AppChannel::Viewer, |st| st.maximized = false);
        state::update(AppChannel::Config, |st| {
            st.config.hide_browser = false;
            st.config.hide_gallery = false;
            st.config.save().ok();
        });
        state::update(AppChannel::Status, |st| {
            st.status = String::from("Layout padrão restaurado.");
        });
    })
    .into();

    // Só os *itens* vão para o Dropdown: ele monta o `Menu` em volta. Passar
    // um `Menu` pronto aqui embrulharia menu em menu, e o menu interno
    // ( Layer::Overlay, sem itens clicáveis no lugar certo) engole o externo.
    let items = rect()
        .width(Size::fill())
        .child(menu_entry(m, "folder", "Abrir pasta…", open_folder))
        .child(menu_entry(m, "file-image", "Abrir arquivos…", open_files))
        .child(menu_divider(m))
        .maybe(dirty && !saving, |el| {
            el.child(menu_entry(m, "save", "Salvar", do_save))
        })
        .maybe(!saving, |el| {
            el.child(menu_entry(m, "save", "Salvar como…", do_save_as))
        })
        .maybe(!saving, |el| {
            el.child(menu_entry(m, "pencil", "Renomear… (F2)", do_rename))
        })
        .child(menu_divider(m))
        .child(menu_entry(m, "grid-2x2", "Restaurar layout", do_restore));

    let trigger = Button::new()
        .flat()
        .corner_radius(m.radius_sm())
        .padding(ui::gaps(m, 1., 2.))
        .on_press(move |_| open.toggle())
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(m.gap(1.5))
                .child(ui::svg(m, "folder"))
                .child(ui::text(m, ui::Role::Body, TEXT, "Arquivo")),
        );

    let close: EventHandler<()> = (move |_| open.set(false)).into();

    ui::Dropdown::new(open(), trigger, items, close)
}

/// Item de menu com ícone.
///
/// Cada item é um alvo de clique do tamanho inteiro do item — o `MenuItem` do
/// Freya só dá clique no texto, e um item de menu clicável só no glifo é
/// exatamente o tipo de coisa que faz "Abrir pasta…" parecer quebrado.
fn menu_entry(
    m: &ui::Metrics,
    name: &str,
    text: &'static str,
    on_press: Press,
) -> impl IntoElement {
    let press: EventHandler<Event<PressEventData>> = on_press;
    TooltipContainer::new(Tooltip::new_text(text)).child(
        Button::new()
            .flat()
            .expanded()
            // Alvo de clique alto, via padding (o `Button` do Freya não tem
            // `min_height`); 1.0 vertical já dá ~26px, acima do mínimo de 24.
            .padding(ui::gaps(m, 1., 2.))
            .corner_radius(m.radius_sm())
            .on_press(press)
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(m.gap(2.))
                    .child(ui::svg(m, name))
                    .child(ui::text(m, ui::Role::Body, TEXT, text)),
            ),
    )
}

/// Linha divisória dentro do menu.
fn menu_divider(m: &ui::Metrics) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .height(Size::px(1.))
        .padding(m.gap(1.))
        .background(Color::from_argb(70, 128, 128, 136))
}

/// Ferramentas de edição: rotate, crop, proporção, undo/redo, reset.
fn edit_bar(
    m: &ui::Metrics,
    edit: Radio<AppState, AppChannel>,
    viewer: Radio<AppState, AppChannel>,
    services: Services,
    dirty: bool,
) -> impl IntoElement {
    let (crop_mode, can_undo, can_redo) = {
        let v = viewer.read();
        let e = edit.read();
        (v.crop_mode, e.editor.can_undo(), e.editor.can_redo())
    };

    rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(m.gap(1.))
        .child(tool_button(
            m,
            "rotate-ccw",
            "90°",
            "Rotacionar anti-horário",
            Button::new().flat().on_press({
                let services = services.clone();
                move |_| {
                    state::update(AppChannel::Edit, |st| {
                        state::rotate(st, &services, false);
                    });
                }
            }),
        ))
        .child(tool_button(
            m,
            "rotate-cw",
            "90°",
            "Rotacionar horário",
            Button::new().flat().on_press({
                let services = services.clone();
                move |_| {
                    state::update(AppChannel::Edit, |st| {
                        state::rotate(st, &services, true);
                    });
                }
            }),
        ))
        .child(vrule(m))
        .child(tool_button(
            m,
            "crop",
            if crop_mode { "Crop… (ativo)" } else { "Crop" },
            "Arraste para recortar, alças redimensionam, Enter aplica",
            Button::new().flat().on_press(move |_| {
                state::update(AppChannel::Viewer, state::toggle_crop_mode);
            }),
        ))
        // O rótulo "Proporção:" custaria ~80px e era o que fazia a barra
        // estourar a largura da janela; o próprio dropdown ("Livre", "16:9")
        // já diz a que se refere, e o tooltip confirma.
        .child(
            TooltipContainer::new(Tooltip::new_text("Proporção do recorte"))
                .child(aspect_select(viewer, services.clone())),
        )
        .maybe(crop_mode, |el| {
            el.child(tool_button(
                m,
                "check",
                "Aplicar",
                "Aplica o recorte (Enter)",
                Button::new().flat().on_press({
                    let services = services.clone();
                    move |_| services.request_crop(CropCommand::Apply)
                }),
            ))
        })
        .child(vrule(m))
        .child(ui::icon_button(
            m,
            "undo",
            "Desfazer (Ctrl+Z)",
            Button::new().flat().enabled(can_undo).on_press({
                let services = services.clone();
                move |_| {
                    state::update(AppChannel::Edit, |st| {
                        state::undo(st, &services);
                    });
                }
            }),
        ))
        .child(ui::icon_button(
            m,
            "redo",
            "Refazer (Ctrl+Shift+Z)",
            Button::new().flat().enabled(can_redo).on_press({
                let services = services.clone();
                move |_| {
                    state::update(AppChannel::Edit, |st| {
                        state::redo(st, &services);
                    });
                }
            }),
        ))
        .child(
            // Mesmo padding do `tool_button`, senão "Reset" fica com altura
            // diferente dos vizinhos e desalinha a barra.
            Button::new()
                .flat()
                .enabled(dirty)
                .corner_radius(m.radius_sm())
                .padding(ui::gaps(m, 0.5, 1.5))
                .on_press({
                    let services = services.clone();
                    move |_| {
                        state::update(AppChannel::Edit, |st| {
                            state::reset_edits(st, &services);
                        });
                    }
                })
                .child(ui::text(m, ui::Role::Body, TEXT, "Reset")),
        )
        .maybe(dirty, |el| {
            // Ponto em vez de "• editado": ~10px em vez de ~60, e o texto
            // continua disponível no tooltip.
            el.child(
                TooltipContainer::new(Tooltip::new_text("Foto editada — ainda não salva"))
                    .child(label().color(Color::from_rgb(255, 200, 0)).text("•")),
            )
        })
}

/// Dropdown de proporção do crop.
fn aspect_select(viewer: Radio<AppState, AppChannel>, services: Services) -> impl IntoElement {
    let selected = viewer.read().crop_aspect;
    let ratio_name = state::crop_ratio_name(selected);

    Select::new()
        .selected_item(ratio_name)
        .children(
            state::ASPECT_OPTIONS
                .iter()
                .enumerate()
                .map(|(i, (name, _))| {
                    let on_press: Press = {
                        let mut viewer = viewer;
                        let services = services.clone();
                        move |_| {
                            viewer.write().crop_aspect = i;
                            services.request_crop(CropCommand::RefitAspect);
                        }
                    }
                    .into();
                    MenuItem::new()
                        .selected(i == selected)
                        .on_press(on_press)
                        .child(*name)
                }),
        )
}

/// Dropdown de filtro de formato.
fn format_filter(
    m: &ui::Metrics,
    photos: Radio<AppState, AppChannel>,
    services: Services,
) -> impl IntoElement {
    let selected = photos.read().format_filter;
    let current = state::FORMAT_FILTERS
        .get(selected)
        .copied()
        .unwrap_or("Todas");

    rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(m.gap(1.5))
        .child(ui::text(m, ui::Role::Body, TEXT, "Formato:"))
        .child(Select::new().selected_item(current).children(
            state::FORMAT_FILTERS.iter().enumerate().map(|(i, name)| {
                let on_press: Press = {
                    let services = services.clone();
                    move |_| {
                        state::update(AppChannel::Photos, |st| {
                            st.format_filter = i;
                            state::apply_filter_if_changed(st, &services);
                        });
                    }
                }
                .into();
                MenuItem::new()
                    .selected(i == selected)
                    .on_press(on_press)
                    .child(*name)
            }),
        ))
}

/// Diálogo nativo de pasta (roda fora da thread de UI).
fn open_folder_dialog(services: Services) {
    spawn(async move {
        let picked = thread(|| rfd::FileDialog::new().pick_folder()).await;
        if let Some(dir) = picked {
            state::update(AppChannel::Photos, |st| {
                state::open_dir_path(&services, st, dir);
            });
        }
    });
}

/// Diálogo nativo de arquivos soltos.
fn open_files_dialog(services: Services) {
    spawn(async move {
        let files = thread(|| {
            rfd::FileDialog::new()
                .add_filter(
                    "Imagens",
                    &["jpg", "jpeg", "png", "webp", "tiff", "tif", "bmp", "gif"],
                )
                .pick_files()
        })
        .await
        .unwrap_or_default();
        let photos = crate::fs_browser::filter_loose_files(files);
        if photos.is_empty() {
            state::update(AppChannel::Status, |st| {
                st.status = String::from("Nenhuma imagem válida selecionada.");
            });
            return;
        }
        state::update(AppChannel::Photos, |st| {
            st.status = format!("{} arquivos soltos", photos.len());
            st.tree = None;
            st.current_dir = None;
            state::replace_photos(st, &services, photos);
        });
    });
}

/// Coleta o que o save precisa: destino, full-res e dims de display.
fn collect_save_input(
    photos: Radio<AppState, AppChannel>,
    services: &Services,
) -> (
    Option<std::path::PathBuf>,
    Option<image::DynamicImage>,
    Option<(u32, u32)>,
) {
    let dest = photos
        .read()
        .current
        .as_ref()
        .map(|c| c.path().to_path_buf());
    (
        dest,
        services.images.full_image(),
        services.images.display_base_dims(),
    )
}

/// Salva sobrescrevendo o original, com confirmação opcional.
fn save_overwrite(
    services: Services,
    photos: Radio<AppState, AppChannel>,
    edit: Radio<AppState, AppChannel>,
) {
    let (dest, full, base) = collect_save_input(photos, &services);
    let (Some(dest), Some(full), Some(base)) = (dest, full, base) else {
        state::update(AppChannel::Status, |st| {
            st.status = String::from("Nada para salvar.");
        });
        return;
    };
    let editor_state = edit.read().editor.state();
    let config = state::snapshot().config;

    spawn(async move {
        if config.confirm_overwrite {
            let name = dest
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let answer = thread(move || {
                rfd::MessageDialog::new()
                    .set_title("Sobrescrever original?")
                    .set_description(format!("{name} será substituído pela versão editada."))
                    .set_buttons(rfd::MessageButtons::YesNo)
                    .show()
            })
            .await;
            if answer != rfd::MessageDialogResult::Yes {
                return;
            }
        }
        state::update(AppChannel::Status, |st| {
            st.saving = true;
            st.status = String::from("Salvando…");
        });
        services.start_save(full, base, editor_state, dest, config.jpeg_quality, true);
    });
}

/// Diálogo "Salvar como…".
fn save_as_dialog(
    services: Services,
    photos: Radio<AppState, AppChannel>,
    edit: Radio<AppState, AppChannel>,
) {
    let (_, full, base) = collect_save_input(photos, &services);
    let name = photos
        .read()
        .current
        .as_ref()
        .map(|c| c.display_name())
        .unwrap_or_else(|| String::from("foto.png"));

    spawn(async move {
        let (Some(full), Some(base)) = (full, base) else {
            state::update(AppChannel::Status, |st| {
                st.status = String::from("Nada para salvar.");
            });
            return;
        };
        let picked = thread(move || {
            rfd::FileDialog::new()
                .set_file_name(&name)
                .add_filter("JPEG", &["jpg", "jpeg"])
                .add_filter("PNG", &["png"])
                .add_filter("WebP", &["webp"])
                .add_filter("TIFF", &["tiff", "tif"])
                .add_filter("BMP", &["bmp"])
                .save_file()
        })
        .await;
        let Some(dest) = picked else {
            return;
        };
        let editor_state = edit.read().editor.state();
        let quality = state::snapshot().config.jpeg_quality;
        state::update(AppChannel::Status, |st| {
            st.saving = true;
            st.status = String::from("Salvando…");
        });
        services.start_save(full, base, editor_state, dest, quality, false);
    });
}
