//! Raiz do app: layout, atalhos globais e composição dos painéis.
//!
//! O `app.rs` original (egui) tinha 2.116 linhas com UI, estado, IO e
//! atalhos misturados. Aqui a UI vira componentes e o estado vai para o
//! Freya Radio (`state.rs`), mantendo as mesmas funcionalidades.

pub mod browser;
pub mod clipboard;
pub mod crop;
pub mod dialogs;
pub mod gallery;
pub mod services;
pub mod shortcuts;
pub mod state;
pub mod statusbar;
pub mod toolbar;
pub mod viewer;
pub mod window;

use crate::prelude::*;
use freya::radio::use_init_radio_station;

use crate::config::AppConfig;
use crate::theme;

use services::Services;
use state::{AppChannel, AppState, channel};

/// Raiz da aplicação.
pub fn app() -> impl IntoElement {
    // Config e tema vêm do disco uma única vez, no arranque.
    let cfg = AppConfig::load();
    use_init_radio_station::<AppState, AppChannel>(move || AppState::from_config(cfg.clone()));
    use_init_theme(|| theme::build(&AppConfig::load().theme));

    let services = use_hook(Services::new);
    use_provide_context(|| services.clone());

    // Reabre a última pasta para navegação imediata.
    let startup = channel(AppChannel::Photos).read().current_dir.clone();
    if startup.is_none() {
        reopen_last_folder(&services);
    }

    // Um frame pode encontrar resultados de threads que terminaram antes dele.
    services.subscribe_results();
    services.tick();
    apply_async_results(&services);
    refresh_thumbs(&services);
    prefetch_neighbors(&services);

    let fullscreen = channel(AppChannel::Viewer).read().fullscreen;

    rect()
        .expanded()
        .theme_background()
        .theme_color()
        // No eixo principal, `Size::fill()` come *tudo* que sobra a partir dos
        // irmãos anteriores. Um painel `fill` no meio empurraria o rodapé para
        // fora da janela; `Content::flex` + `Size::flex(1)` divide o que sobra
        // entre os filhos flex e é o que dá a altura certa ao dock.
        .content(Content::flex())
        .on_global_key_down(shortcuts::global_shortcuts)
        // O viewer de menu de contexto é um overlay: precisa receber eventos
        // globais, mas não pode ocupar espaço no fluxo do layout.
        .child(
            rect()
                .width(Size::px(0.))
                .height(Size::px(0.))
                .child(ContextMenuViewer::new()),
        )
        .child(dialogs::RenameDialog)
        .child(dialogs::SettingsDialog)
        .maybe(!fullscreen, |el| {
            el.child(toolbar::Toolbar)
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::flex(1.))
                        .child(Panels),
                )
                .child(statusbar::StatusBar)
        })
        .maybe(fullscreen, |el| el.child(viewer::Viewer))
}

/// Dock: Navegador | Visualizador / Miniaturas (tudo redimensionável).
///
/// É um [`Component`] (e não uma função solta) para que o `channel` do
/// estado global caia no escopo dele: chamado de dentro do `.maybe()` acima,
/// um hook em escopo da raiz mudaria de índice conforme o fullscreen liga e
/// desliga, e o Freya aborta o render.
#[derive(PartialEq, Clone)]
struct Panels;

impl Component for Panels {
    fn render(&self) -> impl IntoElement {
        let maximized = channel(AppChannel::Viewer).read().maximized;
        let cfg = channel(AppChannel::Config);
        let (hide_browser, hide_gallery) = {
            let c = cfg.read();
            (c.config.hide_browser, c.config.hide_gallery)
        };

        // Maximizado, navegador oculto ou galeria oculta viram layouts
        // diferentes — e não "painel com 0%". `ResizablePanel` continua
        // desenhando a alça de arraste, então um painel de 0% deixaria um
        // sulco de 4px e o conteúdo ainda poderia ser mirado.
        if maximized {
            return only(viewer::Viewer);
        }

        let vertical = ResizableContainer::new()
            .direction(Direction::Vertical)
            .panel(
                ResizablePanel::new(PanelSize::percent(80.))
                    .min_size(20.)
                    .child(viewer::Viewer),
            )
            .panel(
                ResizablePanel::new(PanelSize::percent(20.))
                    .min_size(8.)
                    .child(gallery::Gallery),
            );

        match (hide_browser, hide_gallery) {
            (true, true) => only(viewer::Viewer),
            (true, false) => ResizableContainer::new()
                .direction(Direction::Horizontal)
                .panel(
                    ResizablePanel::new(PanelSize::percent(80.))
                        .min_size(20.)
                        .child(viewer::Viewer),
                )
                .panel(
                    ResizablePanel::new(PanelSize::percent(20.))
                        .min_size(8.)
                        .child(gallery::Gallery),
                )
                .into_element(),
            (false, true) => ResizableContainer::new()
                .direction(Direction::Horizontal)
                .panel(
                    ResizablePanel::new(PanelSize::percent(24.))
                        .min_size(12.)
                        .child(browser::Browser),
                )
                .panel(ResizablePanel::new(PanelSize::percent(76.)).child(viewer::Viewer))
                .into_element(),
            (false, false) => ResizableContainer::new()
                .direction(Direction::Horizontal)
                .panel(
                    ResizablePanel::new(PanelSize::percent(24.))
                        .min_size(12.)
                        .child(browser::Browser),
                )
                .panel(ResizablePanel::new(PanelSize::percent(76.)).child(vertical))
                .into_element(),
        }
    }
}

/// Dock de um painel só (sem alça de arraste sobrando).
fn only(child: impl IntoElement) -> Element {
    ResizableContainer::new()
        .direction(Direction::Horizontal)
        .panel(ResizablePanel::new(PanelSize::percent(100.)).child(child))
        .into_element()
}

/// Reabre a última pasta salva, se a opção estiver ligada.
///
/// É o que faz o app já abrir com conteúdo na vez seguinte.
fn reopen_last_folder(services: &Services) {
    let config = state::snapshot().config;
    if !config.open_last_on_startup {
        return;
    }
    let Some(dir) = config.last_folder.filter(|d| d.is_dir()) else {
        return;
    };
    state::update(AppChannel::Photos, |st| {
        state::open_dir_path(services, st, dir);
    });
}

/// Enfileira/drena as miniaturas da janela em torno da seleção.
///
/// Vive no render da raiz (e não na galeria) porque um `State` escrito de
/// dentro do próprio render que o assina não agenda o próximo frame: as
/// células ficariam vazias para sempre.
fn refresh_thumbs(services: &Services) {
    let photos = channel(AppChannel::Photos);
    let st = photos.read();
    services.pump_thumbs(&st.visible, st.sel);
}

/// Pré-decodifica as vizinhas da foto aberta (navegação por setas sem espera).
///
/// Só enfileira: o decode acontece em thread e volta pelo pump, respeitando
/// o teto de memória e o limite de MB configurados.
fn prefetch_neighbors(services: &Services) {
    let (visible, sel, max_mb) = {
        let photos = channel(AppChannel::Photos);
        let st = photos.read();
        let config = channel(AppChannel::Config);
        (
            st.visible.clone(),
            st.sel,
            config.read().config.prefetch_max_mb,
        )
    };
    let Some(center) = sel else {
        return;
    };
    services
        .images
        .ensure_prefetched(&visible, center, max_mb.saturating_mul(1024 * 1024));
}

/// Consome resultados de scan/save e aplica ao estado (uma vez por render).
///
/// Só o que chegou conta: um frame sem resultado novo não gasta re-render.
/// Usa [`state::update`] (e não `channel`) de propósito — `channel` é hook e
/// esta função é condicional, o que mudaria a contagem de hooks do `render`
/// da raiz e faria o Freya abortar no frame seguinte.
fn apply_async_results(services: &Services) {
    if let Some(outcome) = services.take_scan() {
        state::update(AppChannel::Photos, |st| {
            state::apply_scan_result(st, services, outcome.seq, outcome.result);
        });
    }
    if let Some(outcome) = services.take_save() {
        state::update(AppChannel::Status, |st| {
            st.saving = false;
            st.status = outcome.note.clone();
        });
        if let Some(path) = outcome.reload {
            services.thumbs.invalidate(&path);
            state::update(AppChannel::Photos, |st| {
                state::reset_edits(st, services);
                let Some(idx) = st.sel else { return };
                let Some(photo) = st.visible.get(idx).cloned() else {
                    return;
                };
                state::select_photo(st, services, idx, photo);
            });
        }
    }
}
