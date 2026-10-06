//! Modais: renomear (F2) e configurações.
//!
//! Regra do Freya que molda este arquivo: hooks (`use_consume`, `use_state`)
//! só podem ser chamados no topo de `render`. Por isso os canais e contextos
//! são obtidos incondicionalmente no início.

use freya::components::{
    Button, Input, MenuItem, Popup, PopupButtons, PopupContent, PopupTitle, ScrollView, Select,
    Slider, Switch,
};

use crate::config::{AppConfig, UI_SCALE_MAX, UI_SCALE_MIN};
use crate::prelude::*;
use crate::ui;

use super::services::Services;
use super::state::{self, AppChannel, channel};

#[derive(PartialEq, Clone)]
pub struct RenameDialog;

impl RenameDialog {}

impl Component for RenameDialog {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Services>();
        let dialogs = channel(AppChannel::Dialogs);
        let initial = dialogs.read().rename_buf.clone();
        let open = dialogs.read().rename_open;
        let name = use_state(|| initial.clone());

        let cfg = channel(AppChannel::Config).read().config.clone();
        let p = crate::theme::palette(&cfg.theme);
        let text_secondary = p.text_secondary.to_color();

        use_side_effect({
            let initial = initial.clone();
            let mut name = name;
            move || {
                name.set_if_modified(initial.clone());
            }
        });

        let ok: EventHandler<Event<PressEventData>> = {
            let services = services.clone();
            move |_| {
                let text = name.peek().clone();
                commit(&services, &text);
            }
        }
        .into();
        let cancel: EventHandler<Event<PressEventData>> = (move |_| close_rename()).into();

        let body = rect()
            .vertical()
            .spacing(10.)
            .child(PopupTitle::new(String::from("Renomear")))
            .child(
                PopupContent::new()
                    .child(
                        label()
                            .font_size(12.)
                            .color(text_secondary)
                            .text("Novo nome (mantenha a extensão)."),
                    )
                    .child(
                        Input::new(name)
                            .width(Size::px(320.))
                            .auto_focus(true)
                            .on_submit({
                                let services = services.clone();
                                move |text: String| commit(&services, &text)
                            }),
                    ),
            )
            .child(
                PopupButtons::new()
                    .child(Button::new().filled().on_press(ok).child("OK"))
                    .child(Button::new().outline().on_press(cancel).child("Cancelar")),
            );

        Popup::new()
            .on_close_request(move |_| close_rename())
            .maybe(open, |popup| popup.child(body))
    }
}

/// Aplica o novo nome e fecha o modal (sucesso ou erro).
fn commit(services: &Services, text: &str) {
    state::update(AppChannel::Dialogs, |st| {
        let _ = state::apply_rename(st, services, text);
        st.rename_open = false;
    });
}

/// Fecha o modal de renomear sem salvar.
fn close_rename() {
    state::update(AppChannel::Dialogs, |st| st.rename_open = false);
}

/// Fecha o modal de configurações.
fn close_settings() {
    state::update(AppChannel::Dialogs, |st| st.settings_open = false);
}

#[derive(PartialEq, Clone)]
pub struct SettingsDialog;

impl SettingsDialog {}

impl Component for SettingsDialog {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Services>();
        let theme_state = use_consume::<State<freya::components::Theme>>();
        let cfg = channel(AppChannel::Config).read().config.clone();
        let dialogs = channel(AppChannel::Dialogs);
        let open = dialogs.read().settings_open;

        let m = ui::Metrics::new(cfg.ui_scale);
        let p = crate::theme::palette(&cfg.theme);
        let text_primary = p.text_primary.to_color();
        let text_secondary = p.text_secondary.to_color();
        let border_color = p.border.to_color();

        let close: EventHandler<Event<PressEventData>> = (move |_| close_settings()).into();

        let scroll_height = (420.0f32 * m.scale()).clamp(300.0f32, 520.0f32);

        let body = rect()
            .vertical()
            .spacing(m.gap(1.5))
            .child(PopupTitle::new(String::from("Configurações")))
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(scroll_height))
                    .child(
                        ScrollView::new().child(
                            rect()
                                .width(Size::fill())
                                .padding(ui::gaps(&m, 1., 1.5))
                                .spacing(m.gap(2.))
                                .child(section_header(&m, "COMPORTAMENTO", text_secondary))
                                .child(toggle_row(
                                    &m,
                                    "Confirmar ao sobrescrever",
                                    cfg.confirm_overwrite,
                                    text_primary,
                                    flip(|c| &mut c.confirm_overwrite),
                                ))
                                .child(toggle_row(
                                    &m,
                                    "Exibir faixa de thumbnails",
                                    cfg.show_filmstrip,
                                    text_primary,
                                    flip(|c| &mut c.show_filmstrip),
                                ))
                                .child(toggle_row(
                                    &m,
                                    "Reabrir última pasta ao iniciar",
                                    cfg.open_last_on_startup,
                                    text_primary,
                                    flip(|c| &mut c.open_last_on_startup),
                                ))
                                .child(dialog_sep(border_color))
                                .child(section_header(
                                    &m,
                                    "VARREDURA (PASTAS GRANDES)",
                                    text_secondary,
                                ))
                                .child(toggle_row(
                                    &m,
                                    "Respeitar .gitignore",
                                    cfg.respect_gitignore,
                                    text_primary,
                                    flip_rescan(&services, |c| &mut c.respect_gitignore),
                                ))
                                .child(toggle_row(
                                    &m,
                                    "Pular pastas/arquivos ocultos",
                                    cfg.skip_hidden,
                                    text_primary,
                                    flip_rescan(&services, |c| &mut c.skip_hidden),
                                ))
                                .child(toggle_row(
                                    &m,
                                    "Exibir pastas ocultas na árvore",
                                    cfg.show_hidden_folders,
                                    text_primary,
                                    flip(|c| &mut c.show_hidden_folders),
                                ))
                                .child(dialog_sep(border_color))
                                .child(section_header(&m, "APARÊNCIA", text_secondary))
                                .child(theme_row(&m, theme_state, cfg.theme.clone(), text_primary))
                                .child(ui_scale_row(&m, cfg.ui_scale, text_primary, text_secondary))
                                .child(dialog_sep(border_color))
                                .child(section_header(&m, "PAINÉIS", text_secondary))
                                .child(toggle_row(
                                    &m,
                                    "Mostrar navegador (Ctrl+1)",
                                    !cfg.hide_browser,
                                    text_primary,
                                    flip(|c| &mut c.hide_browser),
                                ))
                                .child(toggle_row(
                                    &m,
                                    "Mostrar galeria (Ctrl+2)",
                                    !cfg.hide_gallery,
                                    text_primary,
                                    flip(|c| &mut c.hide_gallery),
                                ))
                                .child(dialog_sep(border_color))
                                .child(section_header(&m, "IMAGENS E PERFORMANCE", text_secondary))
                                .child(jpeg_quality_row(&m, cfg.jpeg_quality, text_primary))
                                .child(prefetch_row(&m, cfg.prefetch_max_mb, text_primary))
                                .child(dialog_sep(border_color))
                                .child(section_header(&m, "PASTAS FIXADAS", text_secondary))
                                .child(ui::text(
                                    &m,
                                    ui::Role::Body,
                                    text_secondary,
                                    format!("{} pasta(s) fixada(s)", cfg.favorites.len()),
                                )),
                        ),
                    ),
            )
            .child(
                PopupButtons::new().child(Button::new().filled().on_press(close).child("Fechar")),
            );

        Popup::new()
            .on_close_request(move |_| close_settings())
            .maybe(open, |popup| popup.child(body))
    }
}

/// Cabeçalho miúdo de categoria.
fn section_header(m: &ui::Metrics, title: &'static str, color: Color) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .padding(ui::gaps(m, 0.5, 0.))
        .child(ui::text(m, ui::Role::Section, color, title))
}

/// Linha divisória sutil.
fn dialog_sep(color: Color) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .height(Size::px(1.))
        .background(color)
}

/// Ação de alternar um booleano do AppConfig.
#[derive(Clone)]
struct Toggle {
    inner: fn(&mut AppConfig) -> &mut bool,
    rescan: Option<Services>,
}

impl Toggle {
    fn run(&self) {
        let inner = self.inner;
        set(|c| {
            let slot = inner(c);
            *slot = !*slot;
        });
        if let Some(services) = &self.rescan {
            rescan_current(services);
        }
    }
}

/// Inverte um booleano das preferências e persiste.
fn flip(f: fn(&mut AppConfig) -> &mut bool) -> Toggle {
    Toggle {
        inner: f,
        rescan: None,
    }
}

/// Toggle que também revarre a pasta atual depois de mudar.
fn flip_rescan(services: &Services, f: fn(&mut AppConfig) -> &mut bool) -> Toggle {
    Toggle {
        inner: f,
        rescan: Some(services.clone()),
    }
}

/// Linha com `Switch` e rótulo clicável.
fn toggle_row(
    m: &ui::Metrics,
    text: &'static str,
    value: bool,
    text_color: Color,
    toggle: Toggle,
) -> impl IntoElement {
    let t_switch = toggle.clone();
    let t_label = toggle;
    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(m.gap(2.5))
        .child(
            Switch::new()
                .toggled(value)
                .on_toggle(move |_| t_switch.run()),
        )
        .child(
            rect()
                .on_press(move |e: Event<PressEventData>| {
                    e.stop_propagation();
                    t_label.run();
                })
                .child(ui::text(m, ui::Role::Body, text_color, text)),
        )
}

/// Escala tipográfica: slider + valor + passo de teclado.
fn ui_scale_row(
    m: &ui::Metrics,
    value: f32,
    text_color: Color,
    hint_color: Color,
) -> impl IntoElement {
    let pct = (value * 100.0).round() as u32;
    let min = f64::from(UI_SCALE_MIN);
    let span = f64::from(UI_SCALE_MAX - UI_SCALE_MIN);
    rect()
        .width(Size::fill())
        .vertical()
        .spacing(2.)
        .child(ui::text(
            m,
            ui::Role::Body,
            text_color,
            format!("Tamanho do texto: {pct}%"),
        ))
        .child(
            Slider::new(move |v: f64| {
                set(|c| {
                    c.ui_scale = crate::config::sanitize_ui_scale((min + v / 100.0 * span) as f32);
                });
            })
            .value((f64::from(value) - min) / span * 100.0)
            .scroll_enabled(false),
        )
        .child(ui::text(
            m,
            ui::Role::Small,
            hint_color,
            format!(
                "Atalhos: Ctrl+= aumenta, Ctrl+- diminui ({}%–{}%)",
                (UI_SCALE_MIN * 100.0) as u32,
                (UI_SCALE_MAX * 100.0) as u32
            ),
        ))
}

/// Slider da qualidade JPEG (1..=100).
fn jpeg_quality_row(m: &ui::Metrics, value: u8, text_color: Color) -> impl IntoElement {
    let q = value.clamp(1, 100);
    rect()
        .width(Size::fill())
        .vertical()
        .spacing(2.)
        .child(ui::text(
            m,
            ui::Role::Body,
            text_color,
            format!("Qualidade JPEG: {q}%"),
        ))
        .child(
            Slider::new(move |v: f64| {
                set(|c| c.jpeg_quality = (v.round() as u8).clamp(1, 100));
            })
            .value(f64::from(q))
            .scroll_enabled(false),
        )
}

/// Slider do teto de prefetch em MB (0..=256).
fn prefetch_row(m: &ui::Metrics, value: u64, text_color: Color) -> impl IntoElement {
    let mb = value.min(256);
    rect()
        .width(Size::fill())
        .vertical()
        .spacing(2.)
        .child(ui::text(
            m,
            ui::Role::Body,
            text_color,
            format!("Prefetch até (MB, 0 = off): {mb}"),
        ))
        .child(
            Slider::new(move |v: f64| {
                set(|c| c.prefetch_max_mb = (v / 100.0 * 256.0).round() as u64);
            })
            .value(mb as f64 / 256.0 * 100.0)
            .scroll_enabled(false),
        )
}

/// Persiste um campo das preferências.
fn set(f: impl FnOnce(&mut AppConfig)) {
    state::update(AppChannel::Config, |st| {
        f(&mut st.config);
        st.config.save().ok();
    });
}

/// Revarre a pasta atual preservando a foto selecionada.
fn rescan_current(services: &Services) {
    state::update(AppChannel::Photos, |st| {
        let Some(dir) = st.current_dir.clone() else {
            return;
        };
        let preserve = st.current.as_ref().map(|c| c.path().to_path_buf());
        state::start_scan(services, st, dir, preserve);
    });
}

/// Dropdown de tema (aplica na hora).
fn theme_row(
    m: &ui::Metrics,
    theme_state: State<freya::components::Theme>,
    current: String,
    text_color: Color,
) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(m.gap(2.5))
        .child(ui::text(m, ui::Role::Body, text_color, "Tema:"))
        .child(Select::new().selected_item(current.as_str()).children(
            crate::theme::THEMES.iter().map(|name| {
                let name: &str = name;
                let on_press: EventHandler<Event<PressEventData>> = {
                    let mut theme_state = theme_state;
                    move |_| {
                        state::update(AppChannel::Config, |st| {
                            st.config.theme = name.to_owned();
                            st.config.save().ok();
                        });
                        theme_state.set(crate::theme::build(name));
                    }
                }
                .into();
                MenuItem::new()
                    .selected(name == current)
                    .on_press(on_press)
                    .child(name)
            }),
        ))
}

/// Fecha o modal de ajuda.
fn close_help() {
    state::update(AppChannel::Dialogs, |st| st.help_open = false);
}

/// Fecha o modal de metadados EXIF.
fn close_exif() {
    state::update(AppChannel::Dialogs, |st| st.exif_open = false);
}

#[derive(PartialEq, Clone)]
pub struct HelpDialog;

impl HelpDialog {}

impl Component for HelpDialog {
    fn render(&self) -> impl IntoElement {
        let dialogs = channel(AppChannel::Dialogs);
        let open = dialogs.read().help_open;
        let cfg = channel(AppChannel::Config).read().config.clone();

        let m = ui::Metrics::new(cfg.ui_scale);
        let p = crate::theme::palette(&cfg.theme);
        let text_primary = p.text_primary.to_color();
        let text_secondary = p.text_secondary.to_color();

        let close: EventHandler<Event<PressEventData>> = (move |_| close_help()).into();
        let scroll_height = (400.0f32 * m.scale()).clamp(280.0f32, 500.0f32);

        let shortcuts_list: &[(&str, &str)] = &[
            ("← / →", "Foto anterior / próxima"),
            ("+ / - / 0", "Zoom in / out / resetar"),
            ("F11", "Tela cheia"),
            ("F9", "Maximizar visualizador"),
            ("Espaço", "Iniciar / pausar slideshow"),
            ("Ctrl+1", "Ocultar / mostrar navegador"),
            ("Ctrl+2", "Ocultar / mostrar galeria"),
            ("Ctrl+3", "Ocultar / mostrar ajustes"),
            ("Ctrl+0", "Restaurar layout de painéis"),
            ("Ctrl+B", "Comparar com original"),
            ("Ctrl+I", "Metadados EXIF da foto"),
            ("Ctrl+O", "Abrir arquivos"),
            ("Ctrl+Shift+O", "Abrir pasta"),
            ("Ctrl+Z / Ctrl+Y", "Desfazer / refazer"),
            ("F2", "Renomear arquivo"),
            ("Enter", "Aplicar recorte (crop)"),
            ("1 .. 5", "Avaliar foto (estrelas)"),
            ("Esc", "Fechar modal / sair de fullscreen"),
            ("F1 / ?", "Esta janela de atalhos"),
        ];

        let body = rect()
            .vertical()
            .spacing(m.gap(1.5))
            .child(PopupTitle::new(String::from("Atalhos de Teclado")))
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(scroll_height))
                    .child(
                        ScrollView::new().child(
                            rect()
                                .width(Size::fill())
                                .padding(ui::gaps(&m, 1., 1.5))
                                .spacing(m.gap(1.5))
                                .children(shortcuts_list.iter().map(|(key, desc)| {
                                    rect()
                                        .width(Size::fill())
                                        .horizontal()
                                        .main_align(Alignment::SpaceBetween)
                                        .cross_align(Alignment::Center)
                                        .padding(ui::gaps(&m, 0.5, 1.))
                                        .border(
                                            Border::new()
                                                .width(1.)
                                                .alignment(BorderAlignment::Inner)
                                                .fill(Color::from_argb(30, 128, 128, 136)),
                                        )
                                        .corner_radius(m.radius_sm())
                                        .child(
                                            rect()
                                                .padding(ui::gaps(&m, 0.2, 0.8))
                                                .background(Color::from_argb(35, 128, 128, 136))
                                                .corner_radius(m.radius_sm())
                                                .child(ui::text(
                                                    &m,
                                                    ui::Role::Small,
                                                    text_primary,
                                                    *key,
                                                )),
                                        )
                                        .child(ui::text(&m, ui::Role::Body, text_secondary, *desc))
                                })),
                        ),
                    ),
            )
            .child(
                PopupButtons::new().child(Button::new().filled().on_press(close).child("Fechar")),
            );

        Popup::new()
            .on_close_request(move |_| close_help())
            .maybe(open, |popup| popup.child(body))
    }
}

#[derive(PartialEq, Clone)]
pub struct ExifDialog;

impl ExifDialog {}

impl Component for ExifDialog {
    fn render(&self) -> impl IntoElement {
        let photos = channel(AppChannel::Photos);
        let dialogs = channel(AppChannel::Dialogs);
        let open = dialogs.read().exif_open;
        let cfg = channel(AppChannel::Config).read().config.clone();

        let current = photos.read().current.clone();
        let details = photos.read().exif_details.clone();
        let sidecar = photos.read().sidecar.clone();

        let m = ui::Metrics::new(cfg.ui_scale);
        let p = crate::theme::palette(&cfg.theme);
        let text_primary = p.text_primary.to_color();
        let text_secondary = p.text_secondary.to_color();

        let close: EventHandler<Event<PressEventData>> = (move |_| close_exif()).into();

        let file_name = current
            .as_ref()
            .map(|c| c.display_name())
            .unwrap_or_else(|| String::from("Nenhuma foto selecionada"));

        let rating = current
            .as_ref()
            .map(|c| sidecar.get_rating(&c.display_name()))
            .unwrap_or(0);

        let rating_str = if rating > 0 {
            format!("{} de 5 estrelas", "★".repeat(rating as usize))
        } else {
            String::from("Sem avaliação")
        };

        let mut rows: Vec<(&str, String)> = Vec::new();
        rows.push(("Arquivo", file_name));
        rows.push(("Avaliação", rating_str));

        if let Some(d) = details {
            if let Some(cam) = d.camera_model.or(d.camera_make) {
                rows.push(("Câmera", cam));
            }
            if let Some(lens) = d.lens_model {
                rows.push(("Lente", lens));
            }
            if let Some(dt) = d.date_time {
                rows.push(("Data / Hora", dt));
            }
            if let Some((w, h)) = d.dimensions {
                rows.push(("Dimensões", format!("{w} × {h} px")));
            }
            if let Some(exp) = d.exposure_time {
                rows.push(("Exposição", exp));
            }
            if let Some(f) = d.f_number {
                rows.push(("Abertura", f));
            }
            if let Some(iso) = d.iso {
                rows.push(("ISO", iso));
            }
            if let Some(fl) = d.focal_length {
                rows.push(("Distância Focal", fl));
            }
        }

        let body = rect()
            .vertical()
            .spacing(m.gap(1.5))
            .child(PopupTitle::new(String::from("Metadados da Imagem")))
            .child(
                rect()
                    .width(Size::fill())
                    .padding(ui::gaps(&m, 1., 1.5))
                    .spacing(m.gap(1.5))
                    .children(rows.into_iter().map(|(label, val)| {
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .padding(ui::gaps(&m, 0.4, 0.8))
                            .child(ui::text(&m, ui::Role::Body, text_secondary, label))
                            .child(ui::text(&m, ui::Role::Body, text_primary, val))
                    })),
            )
            .child(
                PopupButtons::new().child(Button::new().filled().on_press(close).child("Fechar")),
            );

        Popup::new()
            .on_close_request(move |_| close_exif())
            .maybe(open, |popup| popup.child(body))
    }
}

/// Fecha o modal de lote.
fn close_batch() {
    state::update(AppChannel::Dialogs, |st| st.batch_open = false);
}

#[derive(PartialEq, Clone)]
pub struct BatchDialog;

impl BatchDialog {}

impl Component for BatchDialog {
    fn render(&self) -> impl IntoElement {
        let photos = channel(AppChannel::Photos);
        let dialogs = channel(AppChannel::Dialogs);
        let open = dialogs.read().batch_open;
        let progress = dialogs.read().batch_progress.clone();
        let cfg = channel(AppChannel::Config).read().config.clone();

        let m = ui::Metrics::new(cfg.ui_scale);
        let p = crate::theme::palette(&cfg.theme);
        let text_primary = p.text_primary.to_color();
        let text_secondary = p.text_secondary.to_color();

        let dest_state = use_state(|| {
            photos
                .read()
                .current_dir
                .clone()
                .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        });
        let pattern_state = use_state(|| String::from("foto_{i}"));
        let cancel_flag =
            use_hook(|| std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));

        let is_running = progress.as_ref().map(|pr| !pr.finished).unwrap_or(false);

        let close: EventHandler<Event<PressEventData>> = (move |_| close_batch()).into();

        let start: EventHandler<Event<PressEventData>> = {
            let photos_list = photos.read().visible.clone();
            let dest_dir = dest_state.read().clone();
            let pattern = pattern_state.read().clone();
            let cancel = cancel_flag.clone();

            move |_| {
                cancel.store(false, std::sync::atomic::Ordering::Relaxed);
                let config = crate::batch::BatchConfig {
                    rotate_cw: 0,
                    max_dim: Some(1920),
                    format: String::from("jpg"),
                    jpeg_quality: 90,
                    name_pattern: pattern.clone(),
                    dest_dir: dest_dir.clone(),
                };
                let cancel_handle = cancel.clone();
                crate::batch::run_batch(photos_list.clone(), config, cancel_handle, move |prog| {
                    state::update(AppChannel::Dialogs, |st| {
                        st.batch_progress = Some(prog);
                    });
                });
            }
        }
        .into();

        let cancel_press: EventHandler<Event<PressEventData>> = {
            let cancel = cancel_flag.clone();
            move |_| {
                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
        .into();

        let choose_folder: EventHandler<Event<PressEventData>> = {
            let mut dest_state = dest_state;
            move |_| {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    dest_state.set(folder);
                }
            }
        }
        .into();

        let body = rect()
            .vertical()
            .spacing(m.gap(1.5))
            .child(PopupTitle::new(String::from("Processamento em Lote (0.5)")))
            .child(
                rect()
                    .width(Size::fill())
                    .padding(ui::gaps(&m, 1., 1.5))
                    .spacing(m.gap(1.5))
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .child(ui::text(&m, ui::Role::Body, text_secondary, "Destino:"))
                            .child(
                                Button::new()
                                    .outline()
                                    .on_press(choose_folder)
                                    .child(dest_state.read().display().to_string()),
                            ),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .child(ui::text(
                                &m,
                                ui::Role::Body,
                                text_secondary,
                                "Padrão de nome:",
                            ))
                            .child(Input::new(pattern_state).width(Size::px(180.))),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .child(ui::text(
                                &m,
                                ui::Role::Body,
                                text_secondary,
                                "Fotos na fila:",
                            ))
                            .child(ui::text(
                                &m,
                                ui::Role::Body,
                                text_primary,
                                format!("{} fotos", photos.read().visible.len()),
                            )),
                    )
                    .maybe_child(progress.as_ref().map(|prog| {
                        let text = if prog.finished {
                            if prog.cancelled {
                                String::from("Cancelado pelo usuário.")
                            } else {
                                format!(
                                    "Concluído: {} fotos salvas, {} falhas.",
                                    prog.successes,
                                    prog.failures.len()
                                )
                            }
                        } else {
                            format!(
                                "Processando {}/{} ({})",
                                prog.current, prog.total, prog.current_file
                            )
                        };
                        rect()
                            .width(Size::fill())
                            .padding(ui::gaps(&m, 0.5, 1.))
                            .background(Color::from_argb(35, 128, 128, 136))
                            .corner_radius(m.radius_sm())
                            .child(ui::text(&m, ui::Role::Small, text_primary, text))
                    })),
            )
            .child(
                PopupButtons::new()
                    .maybe(!is_running, |btns| {
                        btns.child(Button::new().filled().on_press(start).child("Iniciar Lote"))
                            .child(Button::new().outline().on_press(close).child("Fechar"))
                    })
                    .maybe(is_running, |btns| {
                        btns.child(
                            Button::new()
                                .filled()
                                .on_press(cancel_press)
                                .child("Cancelar"),
                        )
                    }),
            );

        Popup::new()
            .on_close_request(move |_| close_batch())
            .maybe(open, |popup| popup.child(body))
    }
}
