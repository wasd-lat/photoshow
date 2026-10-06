//! Regressão do menu Arquivo: o clique chega ao item, o menu fecha e o
//! trabalho em background **sobrevive** ao menu fechar.
//!
//! Este é o bug que fazia "abrir pasta" e "abrir arquivos" não abrirem nada.
//! Diagnóstico: no Freya, `spawn` amarra a task ao escopo do componente que
//! criou o handler, e o runner cancela as tasks do escopo quando ele
//! desmonta. Todo item do menu fecha o menu no mesmo clique
//! (`open.set(false)`), então a task do diálogo nativo morria antes de o
//! `rfd` aparecer. A correção é `app::services::background` (= `spawn_forever`).
//!
//! O teste monta a estrutura **real** — [`photoshow::ui::Dropdown`] com um
//! painel opaco e uma entrada `flat()` que fecha o menu e depois sobe a task,
//! como o item "Abrir pasta…" — e clica de verdade.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use freya::components::Button;
use freya::prelude::*;
use freya_testing::prelude::*;

use photoshow::app::services::background;
use photoshow::ui::Dropdown;

/// Contador de cliques no item + sinal de "o trabalho de fundo rodou".
#[derive(Clone, Default, PartialEq)]
struct Probe {
    item_presses: Rc<Cell<usize>>,
    background_done: Rc<Cell<bool>>,
}

/// Réplica do menu Arquivo: gatilho que alterna, entrada que fecha e trabalha.
#[derive(PartialEq, Clone)]
struct FileMenu {
    probe: Probe,
}

impl Component for FileMenu {
    fn render(&self) -> impl IntoElement {
        let mut open = use_state(|| true);
        let probe = self.probe.clone();

        let item = Button::new()
            .flat()
            .expanded()
            .padding(8.)
            .on_press(move |_| {
                // Ordem real da barra: fecha o menu e só depois dispara o
                // trabalho. É este `set` que desmonta a entrada no frame
                // seguinte.
                open.set(false);
                probe.item_presses.set(probe.item_presses.get() + 1);
                let done = probe.background_done.clone();
                background(async move {
                    timer(Duration::from_millis(1)).await;
                    done.set(true);
                });
            })
            .child("Abrir pasta…");

        // O painel precisa de fundo: no hit-testing do Freya um nó opaco
        // "captura" o clique e impede que nós abaixo dele o recebam, então um
        // painel transparente deixaria a entrada morta.
        let items = rect()
            .width(Size::px(210.))
            .background(Color::from_rgb(30, 30, 32))
            .border(Border::new().width(1.).fill(Color::from_rgb(80, 80, 90)))
            .padding(4.)
            .child(item);

        Dropdown::new(
            open(),
            Button::new()
                .flat()
                .child("Arquivo")
                .on_press(move |_| open.toggle()),
            items,
            move |_| open.set(false),
        )
    }
}

fn app(probe: Probe) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .child(FileMenu { probe })
}

/// Centro do menor nó que contém o ponto: o alvo real do clique.
///
/// Usado em vez de uma coordenada fixa porque o menu é posicionado em
/// coordenadas *globais* e o alvo depende da medição do texto.
fn deepest_node_center(test: &TestingRunner, x: f64, y: f64) -> (f64, f64) {
    let areas: Vec<_> = test
        .find_many(|node, _| Some(node.layout().area))
        .into_iter()
        .filter(|a| {
            a.min().x <= x as f32
                && a.min().y <= y as f32
                && a.max().x >= x as f32
                && a.max().y >= y as f32
        })
        .collect();
    let smallest = areas
        .iter()
        .min_by_key(|a| (a.size.width * a.size.height) as i64)
        .expect("nenhum nó sob o ponto");
    (
        (smallest.min().x + smallest.size.width / 2.0) as f64,
        (smallest.min().y + smallest.size.height / 2.0) as f64,
    )
}

fn launch(probe: Probe) -> TestingRunner {
    let mut test = launch_test({
        let probe = probe.clone();
        move || app(probe.clone())
    });
    test.poll(Duration::from_millis(1), Duration::from_millis(30));
    test
}

#[test]
fn clicking_a_menu_item_presses_it_and_closes_the_menu() {
    let probe = Probe::default();
    let mut test = launch(probe.clone());

    let target = deepest_node_center(&test, 25.0, 60.0);
    test.click_cursor(target);
    test.poll(Duration::from_millis(1), Duration::from_millis(30));
    assert_eq!(probe.item_presses.get(), 1, "o clique não chegou ao item");

    // Menu fechado: a entrada saiu da árvore, então clicar ali não dispara nada.
    test.click_cursor(target);
    test.poll(Duration::from_millis(1), Duration::from_millis(30));
    assert_eq!(
        probe.item_presses.get(),
        1,
        "o menu não fechou após o clique"
    );
}

#[test]
fn clicking_outside_the_menu_closes_it() {
    let probe = Probe::default();
    let mut test = launch(probe.clone());

    // Longe do menu: o backdrop transparente recebe o clique e fecha.
    test.click_cursor((300.0, 300.0));
    test.poll(Duration::from_millis(1), Duration::from_millis(30));

    let target = deepest_node_center(&test, 25.0, 60.0);
    test.click_cursor(target);
    test.poll(Duration::from_millis(1), Duration::from_millis(30));
    assert_eq!(
        probe.item_presses.get(),
        0,
        "o clique fora não fechou o menu (backdrop não recebeu o evento)"
    );
}

#[test]
fn background_work_from_a_menu_item_survives_the_menu_closing() {
    let probe = Probe::default();
    let mut test = launch(probe.clone());

    let target = deepest_node_center(&test, 25.0, 60.0);
    test.click_cursor(target);
    // A task só é escalada se sobreviver ao unmount do menu.
    test.poll(Duration::from_millis(1), Duration::from_millis(60));

    assert_eq!(probe.item_presses.get(), 1);
    assert!(
        probe.background_done.get(),
        "a task de background morreu junto com o menu: use `services::background` (spawn_forever), não `spawn`"
    );
}

/// O Viewer monta de verdade e aceita o decode no meio.
#[derive(PartialEq, Clone)]
struct ViewerHarness {
    photo_path: std::path::PathBuf,
    seen_loaded: std::rc::Rc<std::cell::Cell<bool>>,
}

impl Component for ViewerHarness {
    fn render(&self) -> impl IntoElement {
        use freya::radio::use_init_radio_station;
        use photoshow::app::services::Services;
        use photoshow::app::state::{AppChannel, AppState};
        use photoshow::config::AppConfig;
        use photoshow::fs_browser::PhotoPath;

        use_init_radio_station::<AppState, AppChannel>(|| {
            AppState::from_config(AppConfig::default())
        });
        // `Services::new()` cria `State`: só funciona DENTRO de um escopo
        // Freya ativo, nunca solto no corpo do `#[test]`.
        let services = use_hook(Services::new);
        use_provide_context(|| services.clone());
        let mut selected = use_state(|| false);
        if !*selected.peek()
            && let Some(photo) = PhotoPath::new(self.photo_path.clone())
        {
            selected.set(true);
            services.images.select(&photo);
        }
        // No headless testing os timers de background rodam em lock-step com
        // o `poll`, mas a thread de decode roda no SO: um `tick()` explícito
        // no render garante que o canal é drenado a cada frame do teste.
        services.tick();
        if services.load.read().display_px() != (0, 0) {
            self.seen_loaded.set(true);
        }
        photoshow::app::viewer::Viewer
    }
}

#[test]
fn viewer_survives_the_empty_to_loaded_transition() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("hook.png");
    image::RgbImage::new(32, 24).save(&path).expect("fixture");

    let seen_loaded = std::rc::Rc::new(std::cell::Cell::new(false));
    let mut runner = launch_test({
        let path = path.clone();
        let seen_loaded = seen_loaded.clone();
        move || ViewerHarness {
            photo_path: path.clone(),
            seen_loaded: seen_loaded.clone(),
        }
    });
    // O decode roda em background e o pump troca `LoadState::Empty` para
    // `Loaded` no mesmo componente: um hook dentro do braço `Loaded` mudaria
    // a contagem de hooks entre frames e o Freya abortaria aqui.
    for _ in 0..60 {
        runner.poll(Duration::from_millis(10), Duration::from_millis(50));
    }
    assert!(
        seen_loaded.get(),
        "a foto nunca carregou: o teste não exercitou o braço Loaded"
    );
}
