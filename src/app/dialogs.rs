//! Modais: renomear (F2) e configurações.
//!
//! Regra do Freya que molda este arquivo: hooks (`use_a11y`, `use_consume`)
//! só podem ser chamados no topo do `render`. Por isso os ids e contextos são
//! obtidos **antes** de qualquer `.maybe(...)`, e as linhas são montadas como
//! `Element` sempre, para dentro ou fora do `Popup`.

use freya::components::{
    Button, Input, MenuItem, Popup, PopupButtons, PopupContent, PopupTitle, Select, Slider, Switch,
};

use crate::config::AppConfig;
use crate::prelude::*;

use super::services::Services;
use super::state::{self, AppChannel, channel};

/// Callback de um toggle das preferências.
type ToggleHandler = EventHandler<()>;

#[derive(PartialEq, Clone)]
pub struct RenameDialog;

impl RenameDialog {}

impl Component for RenameDialog {
    fn render(&self) -> impl IntoElement {
        // Hooks primeiro, incondicionalmente.
        let services = use_consume::<Services>();
        let initial = channel(AppChannel::Dialogs).read().rename_buf.clone();
        let name = use_state(|| initial);

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
                    .child(weak("Novo nome (mantenha a extensão)."))
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

        let open = channel(AppChannel::Dialogs).read().rename_open;
        Popup::new().maybe(open, |popup| popup.child(body))
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
        // Hooks primeiro, incondicionalmente.
        let services = use_consume::<Services>();
        let switch_id = use_a11y();
        let theme_state = use_consume::<State<freya::components::Theme>>();
        let cfg = channel(AppChannel::Config).read().config.clone();

        let close: EventHandler<Event<PressEventData>> = (move |_| close_settings()).into();

        let body = rect()
            .vertical()
            .spacing(8.)
            .child(PopupTitle::new(String::from("Configurações")))
            .child(PopupContent::new().child(toggle_row(
                switch_id,
                "Confirmar ao sobrescrever",
                cfg.confirm_overwrite,
                flip(|c| &mut c.confirm_overwrite),
            )))
            .child(PopupContent::new().child(toggle_row(
                switch_id,
                "Exibir faixa de thumbnails",
                cfg.show_filmstrip,
                flip(|c| &mut c.show_filmstrip),
            )))
            .child(PopupContent::new().child(toggle_row(
                switch_id,
                "Reabrir última pasta ao iniciar",
                cfg.open_last_on_startup,
                flip(|c| &mut c.open_last_on_startup),
            )))
            .child(PopupContent::new().child(weak("Varredura (pastas grandes):")))
            .child(
                PopupContent::new().child(toggle_row(
                    switch_id,
                    "Respeitar .gitignore",
                    cfg.respect_gitignore,
                    ({
                        let services = services.clone();
                        move |_| set_scan_opt(&services, Some(!cfg.respect_gitignore), None)
                    })
                    .into(),
                )),
            )
            .child(
                PopupContent::new().child(toggle_row(
                    switch_id,
                    "Pular pastas/arquivos ocultos",
                    cfg.skip_hidden,
                    ({
                        let services = services.clone();
                        move |_| set_scan_opt(&services, None, Some(!cfg.skip_hidden))
                    })
                    .into(),
                )),
            )
            .child(PopupContent::new().child(toggle_row(
                switch_id,
                "Exibir pastas ocultas na árvore",
                cfg.show_hidden_folders,
                flip(|c| &mut c.show_hidden_folders),
            )))
            .child(PopupContent::new().child(jpeg_quality_row(cfg.jpeg_quality)))
            .child(PopupContent::new().child(prefetch_row(cfg.prefetch_max_mb)))
            .child(PopupContent::new().child(theme_row(theme_state, cfg.theme.clone())))
            .child(
                PopupContent::new()
                    .child(weak(&format!("{} pasta(s) fixada(s)", cfg.favorites.len()))),
            )
            .child(
                PopupButtons::new().child(Button::new().filled().on_press(close).child("Fechar")),
            );

        let open = channel(AppChannel::Dialogs).read().settings_open;
        Popup::new().maybe(open, |popup| popup.child(body))
    }
}

/// Inverte um booleano das preferências e persiste.
fn flip(f: fn(&mut AppConfig) -> &mut bool) -> ToggleHandler {
    (move |_| {
        set(|c| {
            let slot = f(c);
            *slot = !*slot;
        })
    })
    .into()
}

/// Linha com `Switch` e rótulo. O id de acessibilidade vem de fora, para não
/// chamar hook dentro de um `.maybe(...)`.
fn toggle_row(
    a11y_id: AccessibilityId,
    text: &'static str,
    value: bool,
    on_toggle: ToggleHandler,
) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(10.)
        .child(
            Switch::new()
                .key(a11y_id)
                .toggled(value)
                .on_toggle(on_toggle),
        )
        .child(label().text(text))
}

/// Slider da qualidade JPEG (50..=100).
fn jpeg_quality_row(value: u8) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .vertical()
        .spacing(2.)
        .child(format!("Qualidade JPEG: {value}"))
        .child(
            Slider::new(move |v: f64| {
                set(|c| c.jpeg_quality = (v as u8).clamp(1, 100));
            })
            .value(((value as f32 - 50.0) / 50.0 * 100.0).clamp(0.0, 100.0) as f64)
            .scroll_enabled(false),
        )
}

/// Slider do teto de prefetch em MB (0..=256).
fn prefetch_row(value: u64) -> impl IntoElement {
    let mb = value.min(256);
    rect()
        .width(Size::fill())
        .vertical()
        .spacing(2.)
        .child(format!("Prefetch até (MB, 0 = off): {mb}"))
        .child(
            Slider::new(move |v: f64| {
                set(|c| c.prefetch_max_mb = (v.max(0.0) as u64).min(1024));
            })
            .value(mb as f64 / 256.0 * 100.0)
            .scroll_enabled(false),
        )
}

/// Persiste um campo das preferências (chamado de um handler).
fn set(f: impl FnOnce(&mut AppConfig)) {
    state::update(AppChannel::Config, |st| {
        f(&mut st.config);
        st.config.save().ok();
    });
}

/// Troca uma opção de varredura e revarre preservando a foto atual.
fn set_scan_opt(services: &Services, gitignore: Option<bool>, hidden: Option<bool>) {
    state::update(AppChannel::Config, |st| {
        if let Some(v) = gitignore {
            st.config.respect_gitignore = v;
        }
        if let Some(v) = hidden {
            st.config.skip_hidden = v;
        }
        st.config.save().ok();
    });
    state::update(AppChannel::Photos, |st| {
        let Some(dir) = st.current_dir.clone() else {
            return;
        };
        let preserve = st.current.as_ref().map(|c| c.path().to_path_buf());
        state::start_scan(services, st, dir, preserve);
    });
}

/// Dropdown de tema (aplica na hora).
fn theme_row(theme_state: State<freya::components::Theme>, current: String) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(10.)
        .child(label().text("Tema:"))
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

/// Texto secundário dentro dos popups.
fn weak(text: &str) -> impl IntoElement {
    label()
        .font_size(13.0)
        .color(Color::from_argb(118, 110, 110, 110))
        .text(text.to_owned())
}
