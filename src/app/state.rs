//! Estado global do app (Freya Radio) e as transições de estado.
//!
//! O estado é dividido em canais para que cada painel re-renderize só quando
//! o que ele mostra muda:
//!
//! | Canal     | Quem re-renderiza           | Conteúdo                        |
//! |-----------|-----------------------------|---------------------------------|
//! | `Photos`  | navegador, galeria, viewer  | lista, seleção, árvore, filtros |
//! | `Viewer`  | visualizador                | zoom, pan, crop, maximizado     |
//! | `Edit`    | visualizador, barra de edição| pilha não-destrutiva            |
//! | `Status`  | barra de status             | mensagens, scan, save           |
//! | `Config`  | preferências                | `AppConfig`                     |
//! | `Dialogs` | modais                      | abrir/fechar                     |
//!
//! O que não é UI (cache de imagens, filas de decode) vive em
//! [`super::services::Services`], injetado via contexto.

use std::path::{Path, PathBuf};

use freya::prelude::consume_context;
use freya::radio::{Radio, RadioChannel, RadioStation, use_radio};
use torin::geometry::Vector2D;

use crate::config::AppConfig;
use crate::editor::EditorStack;
use crate::fs_browser::{self, PhotoPath, ScanOptions};
use crate::prelude::ScreenRect;

use super::services::Services;

/// Proporções de crop: rótulo + (w, h). `None` = livre.
pub const ASPECT_OPTIONS: &[(&str, Option<(u32, u32)>)] = &[
    ("Livre", None),
    ("1:1", Some((1, 1))),
    ("4:3", Some((4, 3))),
    ("3:2", Some((3, 2))),
    ("16:9", Some((16, 9))),
    ("9:16", Some((9, 16))),
];

/// Opções do filtro de formato (dropdown da toolbar).
pub const FORMAT_FILTERS: &[&str] = &["Todas", "JPG", "PNG", "WebP", "TIFF", "BMP", "GIF"];

/// Zoom mínimo/máximo (multiplicador do fit).
pub const ZOOM_MIN: f32 = 0.1;
/// Teto de zoom.
pub const ZOOM_MAX: f32 = 20.0;

/// Nó da árvore de pastas (filhos carregados sob demanda).
#[derive(Debug, Clone, PartialEq)]
pub struct DirNode {
    /// Caminho do diretório.
    pub path: PathBuf,
    /// Filhos já lidos (`None` = ainda não carregados).
    pub children: Option<Vec<DirNode>>,
    /// Está expandido na UI?
    pub expanded: bool,
}

impl DirNode {
    /// Cria um nó fechado, sem filhos.
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            children: None,
            expanded: false,
        }
    }

    /// Nome de exibição do diretório.
    #[must_use]
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.display().to_string())
    }
}

/// Estado completo da aplicação (dados puros, sem recursos de GPU/thread).
#[derive(Clone)]
pub struct AppState {
    /// Preferências persistidas.
    pub config: AppConfig,
    /// Todas as fotos da pasta.
    pub photos: Vec<PhotoPath>,
    /// Fotos visíveis pelo filtro de formato atual.
    pub visible: Vec<PhotoPath>,
    /// Índice selecionado em `visible`.
    pub sel: Option<usize>,
    /// Foto atualmente aberta.
    pub current: Option<PhotoPath>,
    /// Raiz da árvore de navegação.
    pub tree: Option<DirNode>,
    /// Pasta aberta.
    pub current_dir: Option<PathBuf>,
    /// Pasta em varredura (`None` = ocioso).
    pub scanning: Option<PathBuf>,
    /// Geração do scan: descarta resultados obsoletos.
    pub scan_seq: u64,
    /// Foto a preservar quando o scan atual terminar.
    pub preserve_on_scan: Option<PathBuf>,

    /// Pilha de edição não-destrutiva.
    pub editor: EditorStack,
    /// Multiplicador de zoom sobre o fit.
    pub zoom: f32,
    /// Deslocamento de pan em px de tela.
    pub offset: Vector2D,
    /// Modo crop ativo?
    pub crop_mode: bool,
    /// Índice em [`ASPECT_OPTIONS`].
    pub crop_aspect: usize,

    /// Janela em fullscreen?
    pub fullscreen: bool,
    /// Visualizador maximizado?
    pub maximized: bool,
    /// Índice em [`FORMAT_FILTERS`].
    pub format_filter: usize,

    /// Mensagem da barra de status.
    pub status: String,
    /// Salvar em andamento?
    pub saving: bool,
    /// Filtro aplicado (evita recomputar `visible` à toa).
    pub applied_filter: usize,

    /// Modal de renomear aberto?
    pub rename_open: bool,
    /// Modal de configurações aberto?
    pub settings_open: bool,
    /// Texto digitado no modal de renomear.
    pub rename_buf: String,
}

/// Canais do estado global.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub enum AppChannel {
    /// Lista de fotos, seleção, árvore e filtros.
    Photos,
    /// Zoom, pan, crop e modos de janela.
    Viewer,
    /// Pilha de edição.
    Edit,
    /// Mensagens de status, scan e save.
    Status,
    /// Preferências.
    Config,
    /// Modais.
    Dialogs,
}

impl RadioChannel<AppState> for AppChannel {
    fn derive_channel(self, _state: &AppState) -> Vec<Self> {
        match self {
            // Editar muda o botão "Salvar" (toolbar) e o preview (viewer).
            AppChannel::Edit => vec![self, AppChannel::Viewer],
            // Abrir/fechar modais muda o que a UI consome.
            AppChannel::Dialogs => vec![self, AppChannel::Photos, AppChannel::Viewer],
            _ => vec![self],
        }
    }
}

impl AppState {
    /// Estado inicial a partir de uma config carregada.
    #[must_use]
    pub fn from_config(config: AppConfig) -> Self {
        Self {
            config,
            photos: Vec::new(),
            visible: Vec::new(),
            sel: None,
            current: None,
            tree: None,
            current_dir: None,
            scanning: None,
            scan_seq: 0,
            preserve_on_scan: None,
            editor: EditorStack::new(),
            zoom: 1.0,
            offset: Vector2D::new(0.0, 0.0),
            crop_mode: false,
            crop_aspect: 0,
            fullscreen: false,
            maximized: false,
            format_filter: 0,
            status: String::from("Abra uma pasta ou fixe uma favorita para começar."),
            saving: false,
            // `usize::MAX` força o primeiro `apply_filter_if_changed`.
            applied_filter: usize::MAX,
            rename_open: false,
            settings_open: false,
            rename_buf: String::new(),
        }
    }
}

/// Handle de um canal do estado global, **assinando** o componente atual.
///
/// É um hook: só pode ser chamado no topo de `render`. Dentro de handlers ou
/// tasks use [`station`] / [`update`].
#[must_use]
pub fn channel(c: AppChannel) -> Radio<AppState, AppChannel> {
    use_radio(c)
}

/// A estação de estado global, sem hook.
///
/// `use_radio` é um hook e não pode ser chamado de um event handler (aí
/// `current_run == 0` e ele realocaria um slot a cada clique). A estação vive
/// no contexto e dá o mesmo acesso, então é isto que os handlers usam.
#[must_use]
pub fn station() -> RadioStation<AppState, AppChannel> {
    consume_context::<RadioStation<AppState, AppChannel>>()
}

/// Lê o estado sem assinar o componente (para handlers e tasks).
#[must_use]
pub fn snapshot() -> AppState {
    station().peek().clone()
}

/// Escreve num canal a partir de um handler, sem passar por hook.
pub fn update(c: AppChannel, f: impl FnOnce(&mut AppState)) {
    let mut station = station();
    let mut guard = station.write_channel(c);
    f(&mut guard);
}

// --- Predicados de UI ---

/// Foto passa no filtro de formato? Grupos: JPG=jpg+jpeg, TIFF=tiff+tif.
#[must_use]
pub fn matches_filter(photo: &PhotoPath, filter: &str) -> bool {
    if filter == "Todas" {
        return true;
    }
    let ext = photo
        .path()
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match filter {
        "JPG" => ext == "jpg" || ext == "jpeg",
        "TIFF" => ext == "tiff" || ext == "tif",
        other => ext == other.to_ascii_lowercase(),
    }
}

/// Proporção de crop atual (`None` = livre).
#[must_use]
pub fn crop_ratio(index: usize) -> Option<f32> {
    ASPECT_OPTIONS
        .get(index)
        .and_then(|(_, r)| *r)
        .map(|(w, h)| w as f32 / h as f32)
}

/// Rótulo da proporção de crop atual.
#[must_use]
pub fn crop_ratio_name(index: usize) -> &'static str {
    ASPECT_OPTIONS.get(index).map_or("Livre", |(n, _)| *n)
}

/// Posição/total para a barra de status.
#[must_use]
pub fn position_label(state: &AppState) -> String {
    match state.sel {
        Some(i) => format!("{}/{}", i + 1, state.visible.len()),
        None => String::from("0/0"),
    }
}

// --- Árvore de pastas ---

/// Opções de varredura a partir das preferências.
#[must_use]
pub fn scan_opts(config: &AppConfig) -> ScanOptions {
    ScanOptions {
        respect_gitignore: config.respect_gitignore,
        skip_hidden: config.skip_hidden,
    }
}

/// Subpastas para a árvore, respeitando "exibir ocultas".
#[must_use]
pub fn child_dirs(dir: &Path, show_hidden: bool) -> Vec<DirNode> {
    fs_browser::list_subdirs(dir)
        .into_iter()
        .filter(|p| show_hidden || !fs_browser::is_hidden(p))
        .map(DirNode::new)
        .collect()
}

/// Limpa o cache de filhos da árvore (rebuild lazy no próximo render).
/// Mantém os flags de expandido; só descarta as listas já lidas.
pub fn clear_tree_cache(node: &mut DirNode) {
    if let Some(kids) = node.children.as_mut() {
        for kid in kids.iter_mut() {
            clear_tree_cache(kid);
        }
    }
    node.children = None;
}

/// Alterna expandido do n-ésimo nó (pré-ordem).
pub fn toggle_node(root: &mut DirNode, target: usize) {
    let mut counter = 0;
    toggle_inner(root, target, &mut counter);
}

fn toggle_inner(node: &mut DirNode, target: usize, counter: &mut usize) -> bool {
    if *counter == target {
        node.expanded = !node.expanded;
        return true;
    }
    *counter += 1;
    if node.expanded
        && let Some(kids) = node.children.as_mut()
    {
        for kid in kids.iter_mut() {
            if toggle_inner(kid, target, counter) {
                return true;
            }
        }
    }
    false
}

// --- Transições quetocam disco (precisam de `Services`) ---

/// Dispara um scan de pasta em background e marca "varrendo" na status bar.
pub fn start_scan(
    services: &Services,
    state: &mut AppState,
    dir: PathBuf,
    preserve: Option<PathBuf>,
) {
    state.scan_seq += 1;
    state.scanning = Some(dir.clone());
    state.preserve_on_scan = preserve;
    state.status = format!("Varrendo {}…", dir.display());
    let opts = scan_opts(&state.config);
    services.start_scan(dir, opts, state.scan_seq);
}

/// Define a pasta raiz (favorita, diálogo ou startup) e varre.
pub fn open_dir_path(services: &Services, state: &mut AppState, dir: PathBuf) {
    let show_hidden = state.config.show_hidden_folders;
    let mut root = DirNode::new(dir.clone());
    root.expanded = true;
    root.children = Some(child_dirs(&dir, show_hidden));
    state.tree = Some(root);
    state.current_dir = Some(dir.clone());
    state.config.last_folder = Some(dir.clone());
    start_scan(services, state, dir, None);
}

/// Carrega as fotos de uma subpasta escolhida na árvore.
pub fn open_subdir(services: &Services, state: &mut AppState, dir: &Path) {
    state.current_dir = Some(dir.to_path_buf());
    start_scan(services, state, dir.to_path_buf(), None);
}

/// Aplica o resultado de um scan recém-concluído; `true` se mudou algo.
pub fn apply_scan_result(
    state: &mut AppState,
    services: &Services,
    seq: u64,
    result: fs_browser::ScanResult,
) -> bool {
    if seq != state.scan_seq {
        return false; // obsoleto: o usuário abriu outra pasta no meio
    }
    state.scanning = None;
    let dir_label = result.dir.display().to_string();
    if result.photos.is_empty() {
        state.status = format!(
            "Nenhuma imagem em {} ({} arquivos verificados).",
            dir_label, result.files_seen
        );
        replace_photos(state, services, Vec::new());
        return true;
    }
    state.status = format!(
        "{} fotos em {} ({} arquivos verificados).",
        result.photos.len(),
        dir_label,
        result.files_seen
    );
    // Preserva a seleção no rescan (troca de opção de varredura).
    let preserve = state.preserve_on_scan.take().and_then(|p| {
        result
            .photos
            .iter()
            .position(|ph| ph.path() == p)
            .map(|i| (i, p))
    });
    replace_photos(state, services, result.photos);
    if let Some((i, _)) = preserve
        && let Some(photo) = state.visible.get(i).cloned()
    {
        select_photo(state, services, i, photo);
    }
    true
}

// --- Transições puras (só estado) ---

/// Troca a lista de fotos (limpa caches e seleção).
pub fn replace_photos(state: &mut AppState, services: &Services, photos: Vec<PhotoPath>) {
    state.photos = photos;
    services.reset();
    state.editor.clear();
    exit_crop_mode(state);
    // Força o refresh do filtro.
    state.applied_filter = usize::MAX;
    apply_filter_if_changed(state, services);
}

/// Recomputa `visible` se o filtro mudou; preserva a foto atual.
pub fn apply_filter_if_changed(state: &mut AppState, services: &Services) -> bool {
    if state.format_filter == state.applied_filter {
        return false;
    }
    state.applied_filter = state.format_filter;
    let filter = FORMAT_FILTERS
        .get(state.format_filter)
        .copied()
        .unwrap_or("Todas");
    state.visible = state
        .photos
        .iter()
        .filter(|p| matches_filter(p, filter))
        .cloned()
        .collect();
    // Preserva a seleção se a foto continua visível.
    if let Some(cur) = &state.current
        && let Some(i) = state.visible.iter().position(|p| p == cur)
    {
        state.sel = Some(i);
        return true;
    }
    state.sel = None;
    state.current = None;
    if let Some(first) = state.visible.first().cloned() {
        select_photo(state, services, 0, first);
    }
    true
}

/// Seleciona a foto do índice, reiniciando edição e transformação.
pub fn select_photo(state: &mut AppState, services: &Services, index: usize, photo: PhotoPath) {
    state.sel = Some(index);
    state.current = Some(photo.clone());
    services.images.select(&photo);
    state.editor.clear();
    exit_crop_mode(state);
    state.zoom = 1.0;
    state.offset = Vector2D::new(0.0, 0.0);
}

/// Avança/retrocede `delta` fotos visíveis.
pub fn step(state: &mut AppState, services: &Services, delta: isize) {
    if state.visible.is_empty() {
        return;
    }
    let cur = state.sel.unwrap_or(0) as isize;
    let next = (cur + delta).clamp(0, state.visible.len() as isize - 1) as usize;
    if Some(next) != state.sel
        && let Some(photo) = state.visible.get(next).cloned()
    {
        select_photo(state, services, next, photo);
    }
}

/// Aplica a edição atual à imagem de preview (ou restaura a base).
pub fn refresh_preview(state: &AppState, services: &Services) {
    if state.editor.is_dirty() {
        let edit = state.editor.state();
        services.images.apply_preview(Some(&edit), services.load);
    } else {
        services.images.apply_preview(None, services.load);
    }
}

/// Gira 90°; `cw` = horário.
pub fn rotate(state: &mut AppState, services: &Services, cw: bool) {
    let Some(base) = services.images.display_base_dims() else {
        return;
    };
    if cw {
        state.editor.rotate_cw(base);
    } else {
        state.editor.rotate_ccw(base);
    }
    refresh_preview(state, services);
}

/// Desfaz a última edição.
pub fn undo(state: &mut AppState, services: &Services) -> bool {
    if !state.editor.undo() {
        return false;
    }
    refresh_preview(state, services);
    true
}

/// Refaz a última edição desfeita.
pub fn redo(state: &mut AppState, services: &Services) -> bool {
    if !state.editor.redo() {
        return false;
    }
    refresh_preview(state, services);
    true
}

/// Descarta todas as edições.
pub fn reset_edits(state: &mut AppState, services: &Services) {
    state.editor.clear();
    exit_crop_mode(state);
    services.images.apply_preview(None, services.load);
}

/// Sai do modo crop.
pub fn exit_crop_mode(state: &mut AppState) {
    state.crop_mode = false;
}

/// Entra/sai do modo crop.
pub fn toggle_crop_mode(state: &mut AppState) {
    if state.crop_mode {
        exit_crop_mode(state);
    } else {
        state.crop_mode = true;
        state.status =
            String::from("Arraste: novo · dentro: mover · alças: redimensionar · Enter aplica.");
    }
}

/// Reaplica a proporção a um rect de crop já desenhado na tela.
///
/// `draw` é o retângulo da imagem na tela; a âncora é o centro do rect atual.
#[must_use]
pub fn refit_crop_to_aspect(rect: ScreenRect, draw: ScreenRect, aspect: usize) -> ScreenRect {
    let Some(ratio) = crop_ratio(aspect) else {
        return rect;
    };
    super::crop::refit_aspect(rect, draw, Some(ratio))
}

/// Aplica o crop da tela ao preview (não grava no disco).
///
/// `draw`/`dims` descrevem a imagem exibida; `rect` é a seleção em coordenadas
/// de tela. Devolve `Some` quando o crop entrou na pilha de edição.
pub fn apply_crop(
    state: &mut AppState,
    services: &Services,
    draw: ScreenRect,
    dims: (u32, u32),
    rect: Option<ScreenRect>,
) -> bool {
    let converted = rect.and_then(|r| super::crop::crop_to_preview_px(draw, dims, r));
    match converted {
        Some(c) => {
            state.editor.set_crop(Some(c));
            refresh_preview(state, services);
            exit_crop_mode(state);
            state.status = String::from("Crop aplicado no preview — Salvar para gravar.");
            true
        }
        None => {
            state.status = String::from("Seleção de crop muito pequena.");
            false
        }
    }
}

/// Maximiza/restaura o visualizador.
pub fn toggle_maximize(state: &mut AppState) {
    state.maximized = !state.maximized;
    if state.maximized {
        state.status = String::from("Visualizador maximizado — F9 ou o botão flutuante restaura.");
    } else {
        state.status = String::from("Painéis restaurados.");
    }
}

/// Abre o modal de renomear, preenchendo com o nome atual.
pub fn open_rename(state: &mut AppState) {
    if let Some(cur) = &state.current {
        state.rename_buf = cur.display_name();
        state.rename_open = true;
    }
}

/// Valida e aplica o novo nome; devolve o caminho final em caso de sucesso.
pub fn apply_rename(
    state: &mut AppState,
    services: &Services,
    new_name: &str,
) -> Result<PathBuf, String> {
    let Some(cur) = state.current.clone() else {
        state.rename_open = false;
        return Err(String::from("Nenhuma foto selecionada."));
    };
    let old_path = cur.path().to_path_buf();
    let new_name = new_name.trim().to_owned();
    let dest = old_path
        .parent()
        .map(|p| p.join(&new_name))
        .unwrap_or_else(|| PathBuf::from(&new_name));
    // Valida a extensão antes de tocar no disco.
    if PhotoPath::new(dest).is_none() {
        state.status = String::from("Use um nome com extensão de imagem (.jpg, .png, …).");
        return Err(state.status.clone());
    }
    let dest = fs_browser::rename_photo(&old_path, &new_name)?;
    let new_photo = PhotoPath::new(dest.clone()).expect("extensão validada");
    for list in [&mut state.photos, &mut state.visible] {
        for p in list.iter_mut() {
            if p.path() == old_path {
                *p = new_photo.clone();
            }
        }
    }
    state.current = Some(new_photo);
    services.thumbs.invalidate(&old_path);
    state.status = format!("Renomeado para {}", dest.display());
    Ok(dest)
}

/// Alterna favorito; a config é persistida pelo chamador.
pub fn toggle_favorite(state: &mut AppState, dir: &Path) -> bool {
    let pinned = state.config.toggle_favorite(dir);
    if pinned {
        state.status = format!("Pasta fixada: {}", dir.display());
    } else {
        state.status = String::from("Pasta desafixada.");
    }
    pinned
}

/// Tamanho do thumbnail da galeria.
#[must_use]
pub fn thumb_size(state: &AppState) -> f32 {
    state.config.thumb_size
}

/// Ajusta o tamanho do thumbnail (persistido).
pub fn set_thumb_size(state: &mut AppState, size: f32) -> bool {
    let clamped = size.clamp(48.0, 192.0);
    if (clamped - state.config.thumb_size).abs() < f32::EPSILON {
        return false;
    }
    state.config.thumb_size = clamped;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::crop;
    use crate::prelude::{Point2D, ScreenRect, Size2D};

    fn photo(name: &str) -> PhotoPath {
        PhotoPath::new(PathBuf::from(name)).expect("extensão válida")
    }

    fn state_with(photos: Vec<PhotoPath>) -> AppState {
        let mut state = AppState::from_config(AppConfig::default());
        state.photos = photos.clone();
        state.visible = photos;
        state
    }

    #[test]
    fn filter_groups_extensions() {
        let jpg = photo("a.JPG");
        let jpeg = photo("b.jpeg");
        let png = photo("c.png");
        assert!(matches_filter(&jpg, "Todas"));
        assert!(matches_filter(&jpg, "JPG"));
        assert!(matches_filter(&jpeg, "JPG"));
        assert!(!matches_filter(&png, "JPG"));
        assert!(matches_filter(&png, "PNG"));
    }

    #[test]
    fn crop_ratio_lookup() {
        assert_eq!(crop_ratio(0), None);
        assert_eq!(crop_ratio(1), Some(1.0));
        let sixteen_nine = crop_ratio(4).expect("16:9");
        assert!((sixteen_nine - 16.0 / 9.0).abs() < 0.0001);
        assert_eq!(crop_ratio_name(0), "Livre");
        assert_eq!(crop_ratio_name(99), "Livre");
    }

    #[test]
    fn position_label_tracks_selection() {
        let mut state = state_with(vec![photo("a.png"), photo("b.png")]);
        assert_eq!(position_label(&state), "0/0");
        state.sel = Some(1);
        assert_eq!(position_label(&state), "2/2");
    }

    #[test]
    fn thumb_size_is_clamped_and_detects_change() {
        let mut state = AppState::from_config(AppConfig::default());
        assert!(set_thumb_size(&mut state, 10.0));
        assert_eq!(thumb_size(&state), 48.0);
        assert!(!set_thumb_size(&mut state, 48.0));
        assert!(set_thumb_size(&mut state, 500.0));
        assert_eq!(thumb_size(&state), 192.0);
    }

    #[test]
    fn reset_edits_clears_everything() {
        let mut state = AppState::from_config(AppConfig::default());
        state.editor.rotate_cw((100, 80));
        state.crop_mode = true;
        assert!(state.editor.is_dirty());
        state.editor.clear();
        exit_crop_mode(&mut state);
        assert!(!state.editor.is_dirty());
        assert!(!state.crop_mode);
    }

    #[test]
    fn apply_crop_rejects_a_selection_below_the_minimum() {
        // Pré-condição do viewer: seleção pequena demais nunca vira `CropRect`.
        let draw = ScreenRect::new(Point2D::new(0., 0.), Size2D::new(800., 600.));
        let tiny = ScreenRect::new(Point2D::new(100., 100.), Size2D::new(2., 2.));
        assert!(crop::crop_to_preview_px(draw, (800, 600), tiny).is_none());
        // E a seleção em cima da foto inteira vira o rect completo.
        let whole = crop::crop_to_preview_px(draw, (800, 600), draw).expect("válida");
        assert_eq!((whole.w, whole.h), (800, 600));
    }

    #[test]
    fn toggle_favorite_reports_state() {
        let mut state = AppState::from_config(AppConfig::default());
        let dir = Path::new("/fotos");
        assert!(toggle_favorite(&mut state, dir));
        assert!(state.config.is_favorite(dir));
        assert!(!toggle_favorite(&mut state, dir));
    }

    #[test]
    fn tree_toggle_walks_preorder() {
        let mut root = DirNode::new(PathBuf::from("/a"));
        root.expanded = true;
        root.children = Some(vec![
            DirNode::new(PathBuf::from("/a/b")),
            DirNode::new(PathBuf::from("/a/c")),
        ]);
        // Pré-ordem: 0 = raiz, 1 = /a/b, 2 = /a/c.
        toggle_node(&mut root, 2);
        assert!(root.children.as_ref().expect("filhos")[1].expanded);
        toggle_node(&mut root, 2);
        assert!(!root.children.as_ref().expect("filhos")[1].expanded);
        // O índice 0 é a própria raiz.
        toggle_node(&mut root, 0);
        assert!(!root.expanded);
    }

    #[test]
    fn preorder_skips_collapsed_subtrees() {
        let mut root = DirNode::new(PathBuf::from("/a"));
        root.children = Some(vec![DirNode::new(PathBuf::from("/a/b"))]);
        // Raiz fechada: /a/b não aparece no índice, logo toggle(0) é a raiz.
        toggle_node(&mut root, 0);
        assert!(root.expanded);
    }

    #[test]
    fn clear_tree_cache_keeps_expanded_flags() {
        let mut root = DirNode::new(PathBuf::from("/a"));
        root.expanded = true;
        let mut kid = DirNode::new(PathBuf::from("/a/b"));
        kid.expanded = true;
        root.children = Some(vec![kid]);
        clear_tree_cache(&mut root);
        assert!(root.children.is_none());
        assert!(root.expanded);
    }
}
