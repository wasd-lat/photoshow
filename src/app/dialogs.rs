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
