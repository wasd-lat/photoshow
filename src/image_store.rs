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
/// Teto de RAM do prefetch (full-res de arquivos grandes pesa GBs).
const PREFETCH_BYTES_CAP: u64 = 512 * 1024 * 1024;

/// Estima a RAM de uma foto decodificada (RGBA, full + display).
fn decoded_bytes(dec: &DecodedPhoto) -> u64 {
    let full = dec.full.width() as u64 * dec.full.height() as u64 * 4;
    let disp = dec.display.width() as u64 * dec.display.height() as u64 * 4;
    full + disp
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
        /// Tamanho em px da imagem exibida (pós-rotate do editor).
        display_px: (u32, u32),
        /// Tamanho em px da full-res.
        full_px: (u32, u32),
    },
    /// Falha (msg para status bar).
    Failed(String),
}

impl LoadState {
    /// Dimensões (w, h) em px do preview exibido.
    #[must_use]
    pub fn display_px(&self) -> (u32, u32) {
        match self {
            Self::Loaded { display_px, .. } => *display_px,
            _ => (0, 0),
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
    result: Option<DecodedPhoto>,
}

struct Inner {
    tx: Sender<LoadMsg>,
    rx: Receiver<LoadMsg>,
    pre_tx: Sender<PrefetchMsg>,
    pre_rx: Receiver<PrefetchMsg>,
    next_id: u64,
    current_id: u64,
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
    prefetch: HashMap<PathBuf, DecodedPhoto>,
    prefetch_order: VecDeque<PathBuf>,
    prefetch_bytes: u64,
    inflight: HashSet<PathBuf>,
    /// Houve mudança que ainda não foi convertida em `LoadState`?
    ///
    /// Sem isto, cada `poll` reconstruiria o `ImageHandle` (e subiria a imagem
    /// de novo para a GPU) a 60 Hz, e o `State` veria um valor novo sempre,
    /// redesenhando o viewer para mostrar exatamente a mesma coisa.
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
            display_px: (0, 0),
            base_px: (0, 0),
            full_px: (0, 0),
            display_img: None,
            base_display: None,
            full: None,
            error: None,
            has_selection: false,
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
        match image_handle(display) {
            Some(image) => LoadState::Loaded {
                image,
                display_px: inner.display_px,
                full_px: inner.full_px,
            },
            None => LoadState::Failed(String::from("falha ao enviar imagem para a GPU")),
        }
    }

    /// Seleciona foto: consome prefetch (instantâneo) ou decodifica em background.
    pub fn select(&self, photo: &crate::fs_browser::PhotoPath) {
        let path = photo.path().to_path_buf();
        let cached = {
            let mut inner = self.0.borrow_mut();
            inner.next_id += 1;
            inner.current_id = inner.next_id;
            let id = inner.current_id;
            inner.has_selection = true;
            inner.display_img = None;
            inner.full = None;
            inner.display_px = (0, 0);
            inner.base_px = (0, 0);
            inner.display_img = None;
            inner.base_display = None;
            inner.full = None;
            inner.error = None;
            inner.publish_dirty = true;
            // Cache de prefetch: caminho quente, sem thread.
            match inner.prefetch.remove(&path) {
                Some(dec) => {
                    inner.prefetch_bytes = inner.prefetch_bytes.saturating_sub(decoded_bytes(&dec));
                    Self::install(&mut inner, dec);
                    None
                }
                None => Some((inner.tx.clone(), id)),
            }
        };
        // Só decodifica em thread quando o prefetch não acertou.
        if let Some((tx, id)) = cached {
            std::thread::spawn(move || {
                let result = decode_photo(&path);
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
                let result = decode_photo(&path).ok();
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
                if let Some(dec) = msg.result {
                    inner.prefetch_bytes += decoded_bytes(&dec);
                    inner.prefetch_order.push_back(msg.path.clone());
                    inner.prefetch.insert(msg.path, dec);
                    // Despeja os mais antigos por contagem E por bytes.
                    while inner.prefetch.len() > PREFETCH_CAP
                        || inner.prefetch_bytes > PREFETCH_BYTES_CAP
                    {
                        if let Some(old) = inner.prefetch_order.pop_front() {
                            if let Some(evicted) = inner.prefetch.remove(&old) {
                                inner.prefetch_bytes =
                                    inner.prefetch_bytes.saturating_sub(decoded_bytes(&evicted));
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
            while let Ok(msg) = inner.rx.try_recv() {
                if msg.id != inner.current_id {
                    continue; // obsoleto: usuário já navegou para outra foto
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
    #[must_use]
    pub fn full_image(&self) -> Option<image::DynamicImage> {
        self.0.borrow().full.clone()
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
}
