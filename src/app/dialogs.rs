//! Modais: renomear (F2) e configurações.
//!
//! Regra do Freya que molda este arquivo: hooks (`use_a11y`, `use_consume`)
//! só podem ser chamados no topo do `render`. Por isso os ids e contextos são
//! obtidos **antes** de qualquer `.maybe(...)`, e as linhas são montadas como
//! `Element` sempre, para dentro ou fora do `Popup`.

use freya::components::{
    Button, Input, MenuItem, Popup, PopupButtons, PopupContent, PopupTitle, Select, Slider, Switch,
};

use crate::config::{AppConfig, UI_SCALE_MAX, UI_SCALE_MIN};
use crate::prelude::*;
use crate::ui;

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
        // O modal respeita a escala global: se o texto da app é grande, o
        // diálogo grande é legível — o inverso deixaria os botões ilegíveis.
        let m = ui::Metrics::new(cfg.ui_scale);

        let close: EventHandler<Event<PressEventData>> = (move |_| close_settings()).into();

        let body = rect()
            .vertical()
            .spacing(m.gap(2.))
            .child(PopupTitle::new(String::from("Configurações")))
            .child(PopupContent::new().child(toggle_row(
                &m,
                switch_id,
                "Confirmar ao sobrescrever",
                cfg.confirm_overwrite,
                flip(|c| &mut c.confirm_overwrite),
            )))
            .child(PopupContent::new().child(toggle_row(
                &m,
                switch_id,
                "Exibir faixa de thumbnails",
                cfg.show_filmstrip,
                flip(|c| &mut c.show_filmstrip),
            )))
            .child(PopupContent::new().child(toggle_row(
                &m,
                switch_id,
                "Reabrir última pasta ao iniciar",
                cfg.open_last_on_startup,
                flip(|c| &mut c.open_last_on_startup),
            )))
            .child(PopupContent::new().child(weak("Varredura (pastas grandes):")))
            .child(PopupContent::new().child(toggle_row(
                &m,
                switch_id,
                "Respeitar .gitignore",
                cfg.respect_gitignore,
                flip_rescan(&services, |c| &mut c.respect_gitignore),
            )))
            .child(PopupContent::new().child(toggle_row(
                &m,
                switch_id,
                "Pular pastas/arquivos ocultos",
                cfg.skip_hidden,
                flip_rescan(&services, |c| &mut c.skip_hidden),
            )))
            .child(PopupContent::new().child(toggle_row(
                &m,
                switch_id,
                "Exibir pastas ocultas na árvore",
                cfg.show_hidden_folders,
                flip(|c| &mut c.show_hidden_folders),
            )))
            .child(PopupContent::new().child(weak("Aparência")))
            .child(PopupContent::new().child(theme_row(&m, theme_state, cfg.theme.clone())))
            .child(PopupContent::new().child(ui_scale_row(&m, cfg.ui_scale)))
            .child(PopupContent::new().child(weak("Imagens")))
            .child(PopupContent::new().child(jpeg_quality_row(&m, cfg.jpeg_quality)))
            .child(PopupContent::new().child(prefetch_row(&m, cfg.prefetch_max_mb)))
            .child(PopupContent::new().child(weak("Painéis")))
            .child(PopupContent::new().child(toggle_row(
                &m,
                switch_id,
                "Mostrar navegador (Ctrl+1)",
                !cfg.hide_browser,
                flip(|c| &mut c.hide_browser).invert(),
            )))
            .child(PopupContent::new().child(toggle_row(
                &m,
                switch_id,
                "Mostrar galeria (Ctrl+2)",
                !cfg.hide_gallery,
                flip(|c| &mut c.hide_gallery).invert(),
            )))
            .child(weak("Varredura (pastas grandes):"))
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
///
/// [`Toggle::invert`] devolve o oposto sem tocar no config: usado nos toggles
/// de "mostrar painel", em que o switch ligado é `!hide_*`.
fn flip(f: fn(&mut AppConfig) -> &mut bool) -> Toggle {
    Toggle {
        inner: f,
        invert: false,
        rescan: None,
    }
}

/// Toggle que também revarre a pasta atual depois de mudar.
///
/// Mudar `.gitignore` ou "ocultos" só tem efeito no próximo scan, então o
/// handler não pode ser o `flip` simples: precisa revarre preservando a foto
/// selecionada.
fn flip_rescan(services: &Services, f: fn(&mut AppConfig) -> &mut bool) -> Toggle {
    Toggle {
        inner: f,
        invert: false,
        rescan: Some(services.clone()),
    }
}

/// Um toggle de `AppConfig` com a polaridade escolhida.
struct Toggle {
    inner: fn(&mut AppConfig) -> &mut bool,
    invert: bool,
    /// Quando `Some`, revarre a pasta atual depois de gravar.
    rescan: Option<Services>,
}

impl Toggle {
    /// Devolve a mesma toggle com a polaridade invertida.
    #[must_use]
    fn invert(mut self) -> Self {
        self.invert = !self.invert;
        self
    }

    /// Handler pronto para o `Switch`.
    ///
    /// Inverter e depois negar dá o valor oposto, que é o que "Mostrar painel"
    /// precisa: o switch é ligado quando `hide_*` é falso.
    #[must_use]
    fn into_handler(self) -> ToggleHandler {
        let Toggle {
            inner,
            invert,
            rescan,
        } = self;
        (move |_| {
            set(|c| {
                let slot = inner(c);
                *slot = !(*slot) ^ invert;
            });
            if let Some(services) = &rescan {
                rescan_current(services);
            }
        })
        .into()
    }
}

/// Linha com `Switch` e rótulo. O id de acessibilidade vem de fora, para não
/// chamar hook dentro de um `.maybe(...)`.
fn toggle_row(
    m: &ui::Metrics,
    a11y_id: AccessibilityId,
    text: &'static str,
    value: bool,
    on_toggle: Toggle,
) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(m.gap(2.5))
        .child(
            Switch::new()
                .key(a11y_id)
                .toggled(value)
                .on_toggle(on_toggle.into_handler()),
        )
        .child(ui::text(
            m,
            ui::Role::Body,
            Color::from_rgb(20, 20, 22),
            text,
        ))
}

/// Escala tipográfica: slider + valor + passo de teclado.
///
/// Fica no modal de configurações porque é uma preferência de leitura, não
/// um estado de visualização: quem troca uma vez quer que valha sempre.
fn ui_scale_row(m: &ui::Metrics, value: f32) -> impl IntoElement {
    let pct = (value * 100.0).round() as u32;
    // `f64` de ponta a ponta: o `Slider` entrega `f64` e converte `f32` no meio
    // só criaria erro de tipo onde não há erro de conta.
    let min = f64::from(UI_SCALE_MIN);
    let span = f64::from(UI_SCALE_MAX - UI_SCALE_MIN);
    rect()
        .width(Size::fill())
        .vertical()
        .spacing(2.)
        .child(ui::text(
            m,
            ui::Role::Body,
            Color::from_rgb(20, 20, 22),
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
            Color::from_argb(150, 90, 90, 96),
            format!(
                "Atalhos: Ctrl+= aumenta, Ctrl+- diminui ({}%–{}%)",
                (UI_SCALE_MIN * 100.0) as u32,
                (UI_SCALE_MAX * 100.0) as u32
            ),
        ))
}

/// Slider da qualidade JPEG (50..=100).
fn jpeg_quality_row(m: &ui::Metrics, value: u8) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .vertical()
        .spacing(2.)
        .child(ui::text(
            m,
            ui::Role::Body,
            Color::from_rgb(20, 20, 22),
            format!("Qualidade JPEG: {value}"),
        ))
        .child(
            Slider::new(move |v: f64| {
                set(|c| c.jpeg_quality = (v as u8).clamp(1, 100));
            })
            .value(((value as f32 - 50.0) / 50.0 * 100.0).clamp(0.0, 100.0) as f64)
            .scroll_enabled(false),
        )
}

/// Slider do teto de prefetch em MB (0..=256).
fn prefetch_row(m: &ui::Metrics, value: u64) -> impl IntoElement {
    let mb = value.min(256);
    rect()
        .width(Size::fill())
        .vertical()
        .spacing(2.)
        .child(ui::text(
            m,
            ui::Role::Body,
            Color::from_rgb(20, 20, 22),
            format!("Prefetch até (MB, 0 = off): {mb}"),
        ))
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

/// Revarre a pasta atual preservando a foto selecionada.
///
/// As opções de varredura só entram em vigor no próximo scan, então mudar
/// `.gitignore` sem isto só surtiria efeito na próxima vez que o app abrisse.
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
) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(m.gap(2.5))
        .child(ui::text(
            m,
            ui::Role::Body,
            Color::from_rgb(20, 20, 22),
            "Tema:",
        ))
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
        .font_size(11.5)
        .color(Color::from_argb(150, 90, 90, 96))
        .text(text.to_owned())
}
