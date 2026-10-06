//! Cache de imagens: decode em background -> `ImageHandle` do Freya.
//!
//! O domínio não conhece a GUI: quem converte RGBA em `ImageHandle` é
//! [`image_handle`], e quem agenda o re-render é o *pump* em
//! `crate::app::services` (o equivalente do `request_repaint` do egui).
//!
//! Fluxo: [`ImageStore::select`] dispara uma thread que decodifica (com
//! correção EXIF) e reduz para display (max [`DISPLAY_MAX_DIM`] px); o
//! resultado volta por `mpsc` com id de geração — obsoletos são descartados.
//! [`ImageStore::poll`] publica o resultado num `State` reativo.
//!
//! extras:
//! - [`ImageStore::ensure_prefetched`]: decodifica vizinhos (±2) em
//!   background; `select` consome o cache e vira instantâneo (cap 4).
//! - [`ImageStore::apply_preview`]: aplica o [`EditorState`] na imagem de
//!   display (preview de rotate/crop) e publica o novo `State`.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};

use crate::prelude::*;
use thiserror::Error;

use crate::editor::EditorState;
use crate::exif::{apply_orientation, read_orientation};

/// Maior lado (px) da versão de display. Full-res fica em `full`.
pub const DISPLAY_MAX_DIM: u32 = 2048;
/// Vizinhos pré-decodificados para cada lado.
const PREFETCH_RADIUS: isize = 2;
const PREFETCH_CAP: usize = 4;
/// Teto de RAM do prefetch (medido em bytes reais decodificados na RAM - PS-P1-04).
const PREFETCH_BYTES_CAP: u64 = 256 * 1024 * 1024;

/// Versão leve para prefetch: apenas a imagem reduzida e dimensões full.
/// Não guarda o buffer full-resolution na RAM durante o prefetch (PS-P1-03).
#[derive(Debug, Clone)]
pub struct PrefetchedPhoto {
    /// Versão de display orientada.
    pub display: image::DynamicImage,
    /// Dimensões originais da foto.
    pub full_size: (u32, u32),
}

impl PrefetchedPhoto {
    /// Bytes reais do buffer RGBA decodificado de display em RAM (PS-P1-04).
    #[must_use]
    pub fn ram_bytes(&self) -> u64 {
        self.display.width() as u64 * self.display.height() as u64 * 4
    }
}

/// Erros de carregamento de foto.
#[derive(Debug, Error)]
pub enum LoadError {
    /// Falha de IO ao ler o arquivo.
    #[error("io: {0}")]
    Io(String),
    /// Falha ao decodificar (formato/arquivo corrompido).
    #[error("decode: {0}")]
    Decode(String),
}

impl From<image::ImageError> for LoadError {
    fn from(e: image::ImageError) -> Self {
        Self::Decode(e.to_string())
    }
}

/// Foto decodificada: full-res corrigida + versão de display (orientada).
#[derive(Debug)]
pub struct DecodedPhoto {
    /// Imagem original corrigida (para o bake no save).
    pub full: image::DynamicImage,
    /// Versão reduzida orientada (para display e preview do editor).
    pub display: image::DynamicImage,
    /// Dimensões (w, h) da full-res.
    pub full_size: (u32, u32),
}

/// Decodifica + corrige EXIF + reduz. Função pura para facilitar teste.
pub fn decode_photo(path: &Path) -> Result<DecodedPhoto, LoadError> {
    let reader = image::ImageReader::open(path).map_err(|e| LoadError::Io(e.to_string()))?;
    let raw = reader.decode()?;
    let full = apply_orientation(raw, read_orientation(path));
    let full_size = (full.width(), full.height());
    let display = if full.width().max(full.height()) > DISPLAY_MAX_DIM {
        full.thumbnail(DISPLAY_MAX_DIM, DISPLAY_MAX_DIM)
    } else {
        full.clone()
    };
    Ok(DecodedPhoto {
        full,
        display,
        full_size,
    })
}

/// Decodifica apenas a versão de exibição e dimensões para prefetch leve (PS-P1-03).
pub fn decode_for_display(path: &Path) -> Result<PrefetchedPhoto, LoadError> {
    let reader = image::ImageReader::open(path).map_err(|e| LoadError::Io(e.to_string()))?;
    let raw = reader.decode()?;
    let oriented = apply_orientation(raw, read_orientation(path));
    let full_size = (oriented.width(), oriented.height());
    let display = if oriented.width().max(oriented.height()) > DISPLAY_MAX_DIM {
        oriented.thumbnail(DISPLAY_MAX_DIM, DISPLAY_MAX_DIM)
    } else {
        oriented
    };
    Ok(PrefetchedPhoto { display, full_size })
}

/// Converte uma imagem do crate `image` no `ImageHandle` do Freya.
///
/// RGBA não pré-multiplicado: é o que o Skia consome direto, sem cópia extra.
#[must_use]
pub fn image_handle(img: &image::DynamicImage) -> Option<ImageHandle> {
    let rgba = img.to_rgba8();
    ImageHandle::from_rgba(
        rgba.width(),
        rgba.height(),
        Bytes::from(rgba.into_raw()),
        AlphaType::Unpremul,
    )
}

/// Converte para `ImageHandle` **e** devolve o histograma do mesmo buffer.
///
/// O painel de ajustes precisa dos dois, e as duas coisas saem do mesmo RGBA:
/// calcular o histograma de uma segunda vez custaria um passe completo sobre
/// até 2048px de lado a cada ajuste de slider, sem acrescentar nada.
#[must_use]
pub fn image_handle_with_histogram(
    img: &image::DynamicImage,
    sample: usize,
) -> Option<(ImageHandle, crate::adjust::Histogram)> {
    let rgba = img.to_rgba8();
    let histogram = crate::adjust::histogram(rgba.as_raw(), sample);
    let handle = ImageHandle::from_rgba(
        rgba.width(),
        rgba.height(),
        Bytes::from(rgba.into_raw()),
        AlphaType::Unpremul,
    )?;
    Some((handle, histogram))
}

/// Passo da amostragem do histograma, em pixels de lado.
///
/// Alvo: ~100k amostras por passe. É o que mantém o painel responsivo a cada
/// movimento de slider numa foto grande, e ainda assim dá ~1000 amostras por
/// faixa (64) — mais que suficiente para a forma do gráfico.
pub const HISTOGRAM_TARGET_PX: u64 = 100_000;

/// Passo de amostragem para uma imagem de `dims` (nunca zero).
#[must_use]
pub fn histogram_sample(dims: (u32, u32)) -> usize {
    let total = u64::from(dims.0) * u64::from(dims.1);
    if total <= HISTOGRAM_TARGET_PX {
        return 1;
    }
    (total / HISTOGRAM_TARGET_PX) as usize
}

/// Estado reativo do carregamento atual.
#[derive(Clone, PartialEq, Default)]
pub enum LoadState {
    /// Nada selecionado.
    #[default]
    Empty,
    /// Decodificando em background.
    Loading,
    /// Pronta para exibir.
    Loaded {
        /// Handle atual (base ou preview do editor).
        image: ImageHandle,
        /// Versão **sem nenhum ajuste**, só quando o comparador está ligado.
        ///
        /// É `None` no caso comum de propósito: publicá-lo sempre custaria um
        /// upload para a GPU por foto a mais, sem nenhum uso.
        base_image: Option<ImageHandle>,
        /// Tamanho em px da imagem exibida (pós-rotate do editor).
        display_px: (u32, u32),
        /// Tamanho em px da full-res.
        full_px: (u32, u32),
        /// Histograma do que está sendo exibido; vazio enquanto não carregou.
        ///
        /// `Box` de propósito: são 3 × 64 contadores (768 bytes), o que estouraria
        /// o tamanho de cada variante do `LoadState` — e esse estado é copiado a
        /// cada `read()` de quatro componentes por frame.
        histogram: Box<crate::adjust::Histogram>,
    },
    /// Falha (msg para status bar).
    Failed(String),
}

impl LoadState {
    /// A versão original está disponível (comparador ligado)?
    #[must_use]
    pub fn base_image(&self) -> Option<&ImageHandle> {
        match self {
            Self::Loaded { base_image, .. } => base_image.as_ref(),
            _ => None,
        }
    }

    /// Dimensões (w, h) em px do preview exibido.
    #[must_use]
    pub fn display_px(&self) -> (u32, u32) {
        match self {
            Self::Loaded { display_px, .. } => *display_px,
            _ => (0, 0),
        }
    }

    /// Histograma do que está sendo exibido; vazio enquanto não carregou.
    #[must_use]
    pub fn histogram(&self) -> crate::adjust::Histogram {
        match self {
            Self::Loaded { histogram, .. } => **histogram,
            _ => crate::adjust::Histogram::default(),
        }
    }
}

struct LoadMsg {
    id: u64,
    path: PathBuf,
    result: Result<DecodedPhoto, LoadError>,
}

struct PrefetchMsg {
    path: PathBuf,
    result: Option<PrefetchedPhoto>,
}

struct Inner {
    tx: Sender<LoadMsg>,
    rx: Receiver<LoadMsg>,
    pre_tx: Sender<PrefetchMsg>,
    pre_rx: Receiver<PrefetchMsg>,
    next_id: u64,
    current_id: u64,
    /// Cancelamento atômico de decodes obsoletos (PS-P1-05).
    current_gen: std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// Caminho da foto atualmente ativa (para lazy decode de full_image).
    current_path: Option<PathBuf>,
    display_px: (u32, u32),
    /// Dimensões de `display_img` **antes** de qualquer edição.
    ///
    /// `apply_preview` reescreve `display_img` com a imagem já transformada,
    /// então esta é a única cópia estável da geometria base — a que rotate,
    /// crop e bake precisam para não derivarem a cada clique.
    base_px: (u32, u32),
    full_px: (u32, u32),
    display_img: Option<image::DynamicImage>,
    /// Cópia de `display_img` ainda sem edição, para desfazer o preview.
    base_display: Option<image::DynamicImage>,
    full: Option<image::DynamicImage>,
    error: Option<String>,
    has_selection: bool,
    /// Comparador ligado? Publica a imagem-base ao lado do preview.
    compare_on: bool,
    /// Geometria (rot + crop) do preview atual, usada na metade "antes".
    compare_geom: crate::editor::EditorState,
    prefetch: HashMap<PathBuf, PrefetchedPhoto>,
    prefetch_order: VecDeque<PathBuf>,
    prefetch_bytes: u64,
    inflight: HashSet<PathBuf>,
    /// Houve mudança que ainda não foi convertida em `LoadState`?
    publish_dirty: bool,
}

/// Guarda seleção + imagem decodificada; alimenta o editor e o viewer.
///
/// `Clone` compartilha o mesmo estado (usado via contexto Freya).
#[derive(Clone)]
pub struct ImageStore(Rc<RefCell<Inner>>);

impl Default for ImageStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageStore {
    /// Cria vazio.
    #[must_use]
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let (pre_tx, pre_rx) = mpsc::channel();
        Self(Rc::new(RefCell::new(Inner {
            tx,
            rx,
            pre_tx,
            pre_rx,
            next_id: 0,
            current_id: 0,
            current_gen: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            current_path: None,
            display_px: (0, 0),
            base_px: (0, 0),
            full_px: (0, 0),
            display_img: None,
            base_display: None,
            full: None,
            error: None,
            has_selection: false,
            compare_on: false,
            compare_geom: crate::editor::EditorState::clean(),
            prefetch: HashMap::new(),
            prefetch_order: VecDeque::new(),
            prefetch_bytes: 0,
            inflight: HashSet::new(),
            publish_dirty: true,
        })))
    }

    fn install(inner: &mut Inner, dec: DecodedPhoto) {
        inner.full_px = dec.full_size;
        let base = dec.display.clone();
        inner.display_px = (base.width(), base.height());
        inner.base_px = (base.width(), base.height());
        inner.base_display = Some(dec.display);
        inner.display_img = Some(base);
        inner.full = Some(dec.full);
        inner.publish_dirty = true;
    }

    fn install_prefetched(inner: &mut Inner, pref: PrefetchedPhoto) {
        inner.full_px = pref.full_size;
        let base = pref.display.clone();
        inner.display_px = (base.width(), base.height());
        inner.base_px = (base.width(), base.height());
        inner.base_display = Some(pref.display);
        inner.display_img = Some(base);
        inner.full = None; // Full-res é lazy decode (PS-P1-03)
        inner.publish_dirty = true;
    }

    /// Publica o estado atual a partir das imagens em memória.
    ///
    /// `None` quando nada mudou desde o último poll: reconstruir o
    /// `ImageHandle` custa uma cópia para a GPU, e não pode acontecer a cada
    /// tique do pump.
    fn publish(inner: &mut Inner) -> Option<LoadState> {
        if !inner.publish_dirty {
            return None;
        }
        inner.publish_dirty = false;
        Some(Self::build_state(inner))
    }

    fn build_state(inner: &Inner) -> LoadState {
        if !inner.has_selection {
            return LoadState::Empty;
        }
        if let Some(e) = &inner.error {
            return LoadState::Failed(e.clone());
        }
        let Some(display) = inner.display_img.as_ref() else {
            return LoadState::Loading;
        };
        // Amostragem em grade: o histograma é uma tendência, não uma contagem.
        // Pegar 1 em cada N pixels mantém a forma e corta o custo proporcional
        // à área — sem ele, um ajuste de slider numa foto de 24 MP custaria
        // dezenas de milhões de leituras por vez.
        match image_handle_with_histogram(display, histogram_sample(inner.display_px)) {
            Some((image, histogram)) => {
                // Comparador: a metade "antes" só vai para a GPU quando o
                // comparador está ligado. Ela mantém a **geometria** do preview
                // (rot/crop) e zera só a **cor** — por isso usa `compare_base`:
                // desenhar a base sem girar ao lado de uma preview girada
                // desalinharia as duas metades, e a comparação mentiria.
                let base_image = if inner.compare_on {
                    inner
                        .base_display
                        .as_ref()
                        .and_then(|b| Self::compare_base(b, inner.compare_geom))
                } else {
                    None
                };
                LoadState::Loaded {
                    image,
                    base_image,
                    display_px: inner.display_px,
                    full_px: inner.full_px,
                    histogram: Box::new(histogram),
                }
            }
            None => LoadState::Failed(String::from("falha ao enviar imagem para a GPU")),
        }
    }

    /// Metade "antes" do comparador: mesma geometria, cor neutra.
    ///
    /// `geom` é a geometria (rot + crop) do preview atual; os ajustes vão de
    /// fora porque o que se quer comparar é a **cor**, não o enquadramento.
    fn compare_base(
        base: &image::DynamicImage,
        geom: crate::editor::EditorState,
    ) -> Option<ImageHandle> {
        image_handle(&crate::editor::apply_to_image(
            base,
            &crate::editor::EditorState {
                adjust: Default::default(),
                ..geom
            },
        ))
    }

    /// Seleciona foto: consome prefetch (instantâneo) ou decodifica em background.
    pub fn select(&self, photo: &crate::fs_browser::PhotoPath) {
        let path = photo.path().to_path_buf();
        let (cached, cur_gen) = {
            let mut inner = self.0.borrow_mut();
            inner.next_id += 1;
            inner.current_id = inner.next_id;
            let id = inner.current_id;
            inner
                .current_gen
                .store(id, std::sync::atomic::Ordering::Relaxed);
            let cur_gen = inner.current_gen.clone();
            inner.current_path = Some(path.clone());
            inner.has_selection = true;
            inner.display_img = None;
            inner.full = None;
            inner.display_px = (0, 0);
            inner.base_px = (0, 0);
            inner.base_display = None;
            inner.error = None;
            inner.publish_dirty = true;
            // Cache de prefetch: caminho quente, sem thread.
            let cached_op = match inner.prefetch.remove(&path) {
                Some(pref) => {
                    inner.prefetch_bytes = inner.prefetch_bytes.saturating_sub(pref.ram_bytes());
                    Self::install_prefetched(&mut inner, pref);
                    None
                }
                None => Some((inner.tx.clone(), id)),
            };
            (cached_op, cur_gen)
        };
        // Só decodifica em thread quando o prefetch não acertou.
        if let Some((tx, id)) = cached {
            std::thread::spawn(move || {
                // PS-P1-05: Se uma seleção mais recente já ocorreu, cancela imediatamente!
                if cur_gen.load(std::sync::atomic::Ordering::Relaxed) != id {
                    return;
                }
                let result = decode_photo(&path);
                if cur_gen.load(std::sync::atomic::Ordering::Relaxed) != id {
                    return;
                }
                let _ = tx.send(LoadMsg { id, path, result });
            });
        }
    }

    /// Garante prefetch dos vizinhos [center-R, center+R].
    /// `max_file_bytes`: pula arquivos maiores (0 = prefetch desativado).
    pub fn ensure_prefetched(
        &self,
        photos: &[crate::fs_browser::PhotoPath],
        center: usize,
        max_file_bytes: u64,
    ) {
        if photos.is_empty() || max_file_bytes == 0 {
            return;
        }
        for d in -PREFETCH_RADIUS..=PREFETCH_RADIUS {
            if d == 0 {
                continue;
            }
            let Some(i) = center.checked_add_signed(d).filter(|i| *i < photos.len()) else {
                continue;
            };
            let path = photos[i].path().to_path_buf();
            let tx = {
                let mut inner = self.0.borrow_mut();
                if inner.prefetch.contains_key(&path) || !inner.inflight.insert(path.clone()) {
                    continue;
                }
                // Guarda barato: arquivo gigante nem entra na fila de decode.
                let small_enough = std::fs::metadata(&path)
                    .map(|m| m.len() <= max_file_bytes)
                    .unwrap_or(true);
                if !small_enough {
                    inner.inflight.remove(&path);
                    continue;
                }
                inner.pre_tx.clone()
            };
            std::thread::spawn(move || {
                // PS-P1-03: decode leve sem reter full-res
                let result = decode_for_display(&path).ok();
                let _ = tx.send(PrefetchMsg { path, result });
            });
        }
    }

    /// Drena resultados (principal + prefetch) e publica em `load`.
    ///
    /// Devolve `true` se o estado reativo mudou (o `State` já foi notificado,
    /// o que agenda o próximo render no Freya).
    pub fn poll(&self, load: State<LoadState>) -> bool {
        let mut changed = false;
        {
            let mut inner = self.0.borrow_mut();
            while let Ok(msg) = inner.pre_rx.try_recv() {
                changed = true;
                inner.inflight.remove(&msg.path);
                if let Some(pref) = msg.result {
                    inner.prefetch_bytes += pref.ram_bytes();
                    inner.prefetch_order.push_back(msg.path.clone());
                    inner.prefetch.insert(msg.path, pref);
                    // Despeja os mais antigos por contagem E por bytes reais de RAM (PS-P1-04).
                    while inner.prefetch.len() > PREFETCH_CAP
                        || inner.prefetch_bytes > PREFETCH_BYTES_CAP
                    {
                        if let Some(old) = inner.prefetch_order.pop_front() {
                            if let Some(evicted) = inner.prefetch.remove(&old) {
                                inner.prefetch_bytes =
                                    inner.prefetch_bytes.saturating_sub(evicted.ram_bytes());
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
            while let Ok(msg) = inner.rx.try_recv() {
                if msg.id != inner.current_id {
                    continue; // obsoleto: usuário já navegou para outra foto (PS-P1-05)
                }
                changed = true;
                match msg.result {
                    Ok(dec) => Self::install(&mut inner, dec),
                    Err(e) => {
                        inner.error = Some(format!("{}: {e}", msg.path.display()));
                        inner.publish_dirty = true;
                    }
                }
            }
        }

        let mut load = load;
        if let Some(next) = Self::publish(&mut self.0.borrow_mut())
            && *load.peek() != next
        {
            load.set(next);
            changed = true;
        }
        changed
    }

    /// Reconstrói a imagem aplicando o estado do editor e publica o resultado.
    ///
    /// `edit = None` volta à imagem base. Cobre rotate, crop, undo, redo e
    /// reset por um único caminho — o mesmo `bake` é usado no save.
    pub fn apply_preview(&self, edit: Option<&EditorState>, load: State<LoadState>) {
        {
            let mut inner = self.0.borrow_mut();
            // Sempre a partir de `base_display`, nunca do preview anterior:
            // aplicar duas vezes sobre o resultado anterior compunha as
            // transformações e o crop saía do lugar depois do 2º clique.
            let Some(base) = inner.base_display.clone() else {
                return;
            };
            // A geometria do comparador acompanha o preview: se o usuário girar
            // ou recortar com o comparador ligado, a metade "antes" tem que
            // girar junto, ou as metades ficam desalinhadas.
            inner.compare_geom = edit.copied().unwrap_or_else(EditorState::clean);
            let shown = match edit {
                Some(edit) => crate::editor::apply_to_image(&base, edit),
                None => base,
            };
            inner.display_px = (shown.width(), shown.height());
            inner.display_img = Some(shown);
            inner.publish_dirty = true;
        }
        self.poll(load);
    }

    /// Dimensões da imagem de display **antes** de qualquer edição.
    ///
    /// Não pode ler `display_img`: `apply_preview` sobrescreve esse campo com
    /// a imagem já rotacionada/cropada, e a 2ª rotação acabaria rotacionando o
    /// crop contra as dimensões erradas (a imagem "desalinha" a partir do
    /// segundo clique). Guarda a base à parte, em `base_px`.
    #[must_use]
    pub fn display_base_dims(&self) -> Option<(u32, u32)> {
        let inner = self.0.borrow();
        let (w, h) = inner.base_px;
        (w > 0 && h > 0).then_some((w, h))
    }

    /// Cópia da full-res para o thread de salvamento.
    /// Se a foto veio do prefetch leve, carrega a full-res sob demanda (PS-P1-03).
    #[must_use]
    pub fn full_image(&self) -> Option<image::DynamicImage> {
        let mut inner = self.0.borrow_mut();
        if let Some(ref full) = inner.full {
            return Some(full.clone());
        }
        if let Some(ref path) = inner.current_path
            && let Ok(dec) = decode_photo(path)
        {
            inner.full = Some(dec.full.clone());
            return Some(dec.full);
        }
        inner.display_img.clone()
    }

    /// Liga/desliga o comparador.
    ///
    /// Só o flag muda aqui: a imagem-base entra no próximo `publish`, porque
    /// subir outra textura para a GPU é caro e não deve acontecer num toggle
    /// que o usuário pode ter apertado por engano.
    pub fn set_compare(&self, on: bool) {
        let mut inner = self.0.borrow_mut();
        if inner.compare_on == on {
            return;
        }
        inner.compare_on = on;
        inner.publish_dirty = true;
    }

    /// Limpa prefetch (troca de pasta/arquivos).
    pub fn clear_prefetch(&self) {
        let mut inner = self.0.borrow_mut();
        inner.prefetch.clear();
        inner.prefetch_order.clear();
        inner.prefetch_bytes = 0;
        inner.inflight.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_png(dir: &Path, name: &str, w: u32, h: u32) -> PathBuf {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        let path = dir.join(name);
        img.save(&path).expect("save png");
        path
    }

    #[test]
    fn decode_small_image_keeps_size() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_test_png(dir.path(), "s.png", 32, 24);
        let dec = decode_photo(&path).expect("decode");
        assert_eq!(dec.full_size, (32, 24));
        assert_eq!((dec.display.width(), dec.display.height()), (32, 24));
    }

    #[test]
    fn decode_large_image_downscales_display() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_test_png(dir.path(), "big.png", 3000, 2000);
        let dec = decode_photo(&path).expect("decode");
        assert_eq!(dec.full_size, (3000, 2000));
        assert!(dec.display.width().max(dec.display.height()) <= DISPLAY_MAX_DIM);
    }

    #[test]
    fn decode_missing_file_errors() {
        let err = decode_photo(Path::new("/nao/existe/foto.png")).expect_err("deveria falhar");
        assert!(matches!(err, LoadError::Io(_)));
    }

    #[test]
    fn image_handle_roundtrips_dimensions() {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::new(8, 4));
        let handle = image_handle(&img).expect("handle");
        assert_eq!((handle.image.width(), handle.image.height()), (8, 4));
    }

    #[test]
    fn histogram_comes_with_the_handle_not_a_second_decode() {
        // Uma chamada, uma imagem, os dois resultados: é o que garante o
        // "sem passe extra" do painel de ajustes.
        // Cinza médio, não preto: `RgbImage::new` produz (0,0,0), que é
        // justamente "sombra travada" — o contador makeria clipping à toa.
        let img =
            image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(8, 4, image::Rgb([128; 3])));
        let (_, h) = image_handle_with_histogram(&img, 1).expect("handle + hist");
        assert_eq!(h.total(), 32);
        assert!(!h.has_clipping());
        assert_eq!(h.blowout, 0);
        assert_eq!(h.shadow, 0);
    }

    #[test]
    fn histogram_sample_scales_with_the_area_but_never_zero() {
        assert_eq!(histogram_sample((320, 240)), 1);
        assert_eq!(histogram_sample((0, 0)), 1);
        // 24 MP: ~24 amostras por passe, ~100k amostra total.
        let big = histogram_sample((6000, 4000));
        assert!(big >= 200, "amostragem agressiva, veio {big}");
        let sampled = (u64::from(6000u32) * u64::from(4000u32)) / big as u64;
        assert!(
            sampled <= HISTOGRAM_TARGET_PX * 2,
            "passe ainda caro: {sampled}"
        );
    }

    /// Instala uma imagem de `w`×`h` como se tivesse sido decodificada.
    fn install_sized(store: &ImageStore, w: u32, h: u32) {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::new(w, h));
        let mut inner = store.0.borrow_mut();
        ImageStore::install(
            &mut inner,
            DecodedPhoto {
                display: img.clone(),
                full: img,
                full_size: (w, h),
            },
        );
    }

    /// Instala a partir de um arquivo real (usa o decoder de verdade).
    fn install_file(store: &ImageStore, path: &Path) {
        let dec = decode_photo(path).expect("decode");
        let mut inner = store.0.borrow_mut();
        ImageStore::install(&mut inner, dec);
    }

    /// Recria o preview sem passar pelo `State` do Freya.
    ///
    /// `apply_preview` chama `poll`, que escreve no `State` — e `State::write`
    /// exige um contexto Freya ativo. Fora da janela (nos testes) isso estoura,
    /// então esta cópia da mesma lógica roda direto no `Inner`.
    fn preview(inner: &mut Inner, edit: Option<&crate::editor::EditorState>) {
        let Some(base) = inner.base_display.clone() else {
            return;
        };
        let shown = match edit {
            Some(edit) => crate::editor::apply_to_image(&base, edit),
            None => base,
        };
        inner.display_px = (shown.width(), shown.height());
        inner.display_img = Some(shown);
        inner.publish_dirty = true;
    }

    #[test]
    fn base_dims_survive_repeated_rotation() {
        // Regressão: `apply_preview` sobrescreve `display_img` com a imagem já
        // rotacionada. Ler as dims de lá fazia a 2ª rotação derivar, e a partir
        // daí o crop saía do lugar a cada clique.
        let dir = tempfile::tempdir().expect("tempdir");
        let _ = write_test_png(dir.path(), "a.png", 4, 2);
        let store = ImageStore::new();
        install_sized(&store, 400, 200);
        assert_eq!(store.display_base_dims(), Some((400, 200)));

        let mut editor = crate::editor::EditorStack::default();

        // 1ª rotação: retrato.
        editor.rotate_cw(store.display_base_dims().expect("base"));
        preview(&mut store.0.borrow_mut(), Some(&editor.state()));
        assert_eq!(store.display_base_dims(), Some((400, 200)));

        // 2ª rotação: volta ao tamanho original, e a base continua 400×200.
        editor.rotate_cw(store.display_base_dims().expect("base"));
        preview(&mut store.0.borrow_mut(), Some(&editor.state()));
        assert_eq!(store.display_base_dims(), Some((400, 200)));

        // Duas rotações horário = meia volta, que é o estado intermediário esperado
        // (e `rot` acumula, não satura).
        assert_eq!(editor.state().rot, 2);
    }

    #[test]
    fn preview_always_starts_from_the_unedited_image() {
        // Aplicar o mesmo preview duas vezes não pode acumular transformação.
        let dir = tempfile::tempdir().expect("tempdir");
        let _ = write_test_png(dir.path(), "b.png", 4, 2);
        let store = ImageStore::new();
        install_sized(&store, 400, 200);

        let mut editor = crate::editor::EditorStack::default();
        editor.rotate_cw((400, 200));
        let once = editor.state();

        preview(&mut store.0.borrow_mut(), Some(&once));
        let first = store.display_base_dims();
        // Segundo `apply_preview` com o MESMO estado tem que dar o mesmo
        // resultado — não o resultado rotacionado duas vezes.
        preview(&mut store.0.borrow_mut(), Some(&once));
        assert_eq!(store.display_base_dims(), first);
        assert_eq!(store.display_base_dims(), Some((400, 200)));
    }

    #[test]
    fn preview_without_edit_restores_the_base_image() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _ = write_test_png(dir.path(), "c.png", 4, 2);
        let store = ImageStore::new();
        install_sized(&store, 400, 200);

        let mut editor = crate::editor::EditorStack::default();
        editor.rotate_cw((400, 200));
        preview(&mut store.0.borrow_mut(), Some(&editor.state()));
        // Reset: `edit = None` precisa devolver a imagem original.
        preview(&mut store.0.borrow_mut(), None);
        assert_eq!(store.display_base_dims(), Some((400, 200)));
        assert_eq!(
            store
                .0
                .borrow()
                .display_img
                .as_ref()
                .map(|d| (d.width(), d.height())),
            Some((400, 200))
        );
    }

    #[test]
    fn changing_photo_resets_base_dims() {
        // Trocar de foto não pode herdar as dims da anterior: senão o crop da
        // foto nova nasceria com a geometria da antiga.
        let dir = tempfile::tempdir().expect("tempdir");
        let a = write_test_png(dir.path(), "d1.png", 4, 2);
        let b = write_test_png(dir.path(), "d2.png", 3, 5);
        let store = ImageStore::new();

        install_file(&store, &a);
        assert_eq!(store.display_base_dims(), Some((4, 2)));

        store.select(&crate::fs_browser::PhotoPath::new(b.clone()).expect("photo b"));
        install_file(&store, &b);
        assert_eq!(store.display_base_dims(), Some((3, 5)));
    }

    #[test]
    fn compare_only_publishes_the_base_handle_when_requested() {
        let store = ImageStore::new();
        install_sized(&store, 40, 20);
        // `install` sozinho não marca seleção: quem faz isso é `select`.
        store.0.borrow_mut().has_selection = true;

        // Desligado: sem base na GPU.
        let off = ImageStore::publish(&mut store.0.borrow_mut()).expect("state");
        assert!(off.base_image().is_none());

        // Ligado: a imagem base passa a vir junto.
        store.set_compare(true);
        let on = ImageStore::publish(&mut store.0.borrow_mut()).expect("state");
        assert!(on.base_image().is_some());

        // E desliga de novo, sem deixar rastro.
        store.set_compare(false);
        let back = ImageStore::publish(&mut store.0.borrow_mut()).expect("state");
        assert!(back.base_image().is_none());
    }
}
