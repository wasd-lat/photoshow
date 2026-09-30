//! Barra superior: menu Arquivo, ferramentas de edição e filtro de formato.

use freya::components::{Button, Select, Tooltip, TooltipContainer};

use crate::prelude::*;
use crate::ui;

use super::services::{CropCommand, Services, background};
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
        let cfg = channel(AppChannel::Config).read().config.clone();

        let p = crate::theme::palette(&cfg.theme);
        let text_color = p.text_primary.to_color();

        let has_photo = photos.read().current.is_some();
        let dirty = edit.read().editor.is_dirty();
        let saving = status.read().saving;
        let m = ui::Metrics::new(cfg.ui_scale);

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
                    .child(file_menu(
                        &m,
                        &p,
                        photos,
                        edit,
                        services.clone(),
                        dirty,
                        saving,
                    ))
                    .maybe(has_photo, |el| {
                        el.child(vrule(&m)).child(edit_bar(
                            &m,
                            text_color,
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
                    .child(format_filter(&m, text_color, photos, services.clone()))
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
                            state::update(AppChannel::Dialogs, |st| st.settings_open = true);
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
    text_color: Color,
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
                    .child(ui::text(m, ui::Role::Body, text_color, text)),
            ),
    )
}

/// Menu Arquivo: abrir, salvar, salvar como, renomear, restaurar layout.
///
/// Usa [`ui::Dropdown`] ancorado fora do fluxo: não empurra a barra de ferramentas
/// e fecha ao clicar fora ou pressionar Escape.
fn file_menu(
    m: &ui::Metrics,
    palette: &crate::theme::Palette,
    photos: Radio<AppState, AppChannel>,
    edit: Radio<AppState, AppChannel>,
    services: Services,
    dirty: bool,
    saving: bool,
) -> impl IntoElement {
    let mut open = use_state(|| false);
    let text_color = palette.text_primary.to_color();

    let open_folder = {
        let services = services.clone();
        let mut open = open;
        move |_| {
            open.set(false);
            open_folder_dialog(services.clone());
        }
    };
    let open_files = {
        let services = services.clone();
        let mut open = open;
        move |_| {
            open.set(false);
            open_files_dialog(services.clone());
        }
    };
    let do_save = {
        let services = services.clone();
        let mut open = open;
        move |_| {
            open.set(false);
            save_overwrite(services.clone(), photos, edit);
        }
    };
    let do_save_as = {
        let services = services.clone();
        let mut open = open;
        move |_| {
            open.set(false);
            save_as_dialog(services.clone(), photos, edit);
        }
    };
    let do_rename = {
        let mut open = open;
        move |_| {
            open.set(false);
            state::update(AppChannel::Dialogs, state::open_rename);
        }
    };
    let do_restore = {
        let mut open = open;
        move |_| {
            open.set(false);
            state::update(AppChannel::Viewer, |st| st.maximized = false);
            state::update(AppChannel::Config, |st| {
                st.config.hide_browser = false;
                st.config.hide_gallery = false;
                st.config.save().ok();
            });
            state::update(AppChannel::Status, |st| {
                st.status = String::from("Layout padrão restaurado.");
            });
        }
    };

    let border_color = palette.border.to_color();
    let surface_color = palette.surface_primary.to_color();

    let items = rect()
        .width(Size::px((210.0f32 * m.scale()).clamp(180.0f32, 260.0f32)))
        .background(surface_color)
        .border(Border::new().width(1.).fill(border_color))
        .corner_radius(m.radius())
        .shadow(
            Shadow::new()
                .blur(14.)
                .color(Color::from_argb(80, 0, 0, 0))
                .y(4.),
        )
        .padding(ui::gaps(m, 1., 1.))
        .spacing(m.gap(0.5))
        .child(menu_entry(
            m,
            text_color,
            "folder",
            "Abrir pasta…",
            open_folder,
        ))
        .child(menu_entry(
            m,
            text_color,
            "file-image",
            "Abrir arquivos…",
            open_files,
        ))
        .child(menu_divider(m, border_color))
        .maybe(dirty && !saving, |el| {
            el.child(menu_entry(m, text_color, "save", "Salvar", do_save))
        })
        .maybe(!saving, |el| {
            el.child(menu_entry(
                m,
                text_color,
                "save",
                "Salvar como…",
                do_save_as,
            ))
        })
        .maybe(!saving, |el| {
            el.child(menu_entry(
                m,
                text_color,
                "pencil",
                "Renomear… (F2)",
                do_rename,
            ))
        })
        .child(menu_divider(m, border_color))
        .child(menu_entry(
            m,
            text_color,
            "grid-2x2",
            "Restaurar layout",
            do_restore,
        ));

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
                .child(ui::text(m, ui::Role::Body, text_color, "Arquivo")),
        );

    let close: EventHandler<()> = (move |_| open.set(false)).into();

    ui::Dropdown::new(open(), trigger, items, close)
}

/// Item de menu com ícone e texto.
fn menu_entry(
    m: &ui::Metrics,
    text_color: Color,
    name: &str,
    text: &'static str,
    on_press: impl Into<EventHandler<Event<PressEventData>>>,
) -> impl IntoElement {
    Button::new()
        .flat()
        .expanded()
        .padding(ui::gaps(m, 1., 2.))
        .corner_radius(m.radius_sm())
        .on_press(on_press)
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(m.gap(2.))
                .child(ui::svg(m, name))
                .child(ui::text(m, ui::Role::Body, text_color, text)),
        )
}

/// Linha divisória dentro do menu.
fn menu_divider(m: &ui::Metrics, color: Color) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .height(Size::px(1.))
        .padding(m.gap(0.5))
        .background(color)
}

/// Ferramentas de edição: rotate, crop, proporção, undo/redo, reset.
fn edit_bar(
    m: &ui::Metrics,
    text_color: Color,
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
            text_color,
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
            text_color,
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
            text_color,
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
                text_color,
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
                .child(ui::text(m, ui::Role::Body, text_color, "Reset")),
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
    text_color: Color,
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
        .child(ui::text(m, ui::Role::Body, text_color, "Formato:"))
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
pub fn open_folder_dialog(services: Services) {
    // `background` (task da raiz), e não `spawn`: o menu que disparou isto é
    // desmontado no mesmo clique (`open.set(false)`) e uma task escopada seria
    // cancelada junto — o diálogo nunca chegaria a aparecer.
    background(async move {
        let picked = thread(|| rfd::FileDialog::new().pick_folder()).await;
        let Some(dir) = picked else { return };
        crate::cli::open_target(&services, crate::cli::Target::Folder(dir));
    });
}

/// Diálogo nativo de arquivos.
///
/// Um arquivo só abre a pasta dele com a foto já selecionada (é o que o
/// usuário espera de "Abrir arquivos…"); vários viram lista solta.
pub fn open_files_dialog(services: Services) {
    background(async move {
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
        if files.is_empty() {
            return; // cancelado: não é erro
        }
        if files.len() == 1 {
            let Some(target) = crate::cli::resolve(&files) else {
                no_valid_image();
                return;
            };
            crate::cli::open_target(&services, target);
            return;
        }
        let photos = crate::fs_browser::filter_loose_files(files);
        if photos.is_empty() {
            no_valid_image();
            return;
        }
        crate::cli::open_target(&services, crate::cli::Target::Files(photos));
    });
}

/// Aviso na status bar: o usuário escolheu algo que não é imagem.
fn no_valid_image() {
    state::update(AppChannel::Status, |st| {
        st.status = String::from("Nenhuma imagem válida selecionada.");
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

    background(async move {
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

    background(async move {
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
