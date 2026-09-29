//! Faixa de thumbnails: worker único com fila + cache reativo de `ImageHandle`.
//!
//! O worker decodifica (com correção EXIF) para no máximo [`THUMB_MAX`] px;
//! [`ThumbCache::poll`] drena os prontos e publica num `State` — que é o que
//! agenda o re-render da galeria (no egui isso era `request_repaint`).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};

use crate::prelude::*;

use crate::exif::{apply_orientation, read_orientation};
use crate::image_store::image_handle;

/// Maior lado do thumbnail.
pub const THUMB_MAX: u32 = 160;
/// Janela ao redor da seleção mantida em cache/enfileirada.
const THUMB_RADIUS: usize = 25;
/// Teto de handles; além disso, despeja fora da janela.
const THUMB_CAP: usize = 200;
/// Novos jobs por poll (não sufocar a UI).
const JOBS_PER_FRAME: usize = 12;

/// Mapa de thumbs prontos (reativo).
pub type ThumbMap = HashMap<PathBuf, ImageHandle>;

struct ThumbMsg {
    path: PathBuf,
    result: Option<ImageHandle>,
}

fn decode_thumb(path: &PathBuf) -> Option<ImageHandle> {
    let raw = image::ImageReader::open(path).ok()?.decode().ok()?;
    let oriented = apply_orientation(raw, read_orientation(path));
    let thumb = oriented.thumbnail(THUMB_MAX, THUMB_MAX);
    image_handle(&thumb)
}

struct Inner {
    tx: Sender<PathBuf>,
    rx: Receiver<ThumbMsg>,
    queued: HashSet<PathBuf>,
    failed: HashSet<PathBuf>,
}

/// Cache de thumbnails com worker em background.
///
/// `Clone` compartilha o mesmo estado (usado via contexto Freya).
#[derive(Clone)]
pub struct ThumbCache(Rc<RefCell<Inner>>);

impl Default for ThumbCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ThumbCache {
    /// Cria e dispara o worker.
    #[must_use]
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<PathBuf>();
        let (res_tx, res_rx) = mpsc::channel::<ThumbMsg>();
        std::thread::spawn(move || {
            while let Ok(path) = rx.recv() {
                let result = decode_thumb(&path);
                if res_tx.send(ThumbMsg { path, result }).is_err() {
                    break;
                }
            }
        });
        Self(Rc::new(RefCell::new(Inner {
            tx,
            rx: res_rx,
            queued: HashSet::new(),
            failed: HashSet::new(),
        })))
    }

    /// Enfileira candidatos, drena prontos e publica em `cache`.
    ///
    /// Candidatos ordenados por tamanho do arquivo: thumbs de JPGs pequenos
    /// aparecem primeiro; TIFFs gigantes resolvem por último sem bloquear.
    /// Devolve `true` se o `State` mudou.
    pub fn poll(
        &self,
        visible: &[crate::fs_browser::PhotoPath],
        sel: Option<usize>,
        cache: State<ThumbMap>,
    ) -> bool {
        let mut cache = cache;
        let Some(center) = sel.filter(|_| !visible.is_empty()) else {
            return false;
        };
        let (lo, hi) = window_range(visible.len(), center, THUMB_RADIUS);
        let mut changed = false;
        let mut batch: Vec<(PathBuf, Option<ImageHandle>)> = Vec::new();

        {
            let mut inner = self.0.borrow_mut();
            let mut candidates: Vec<PathBuf> = visible[lo..=hi]
                .iter()
                .map(|p| p.path().to_path_buf())
                .filter(|p| {
                    !cache.peek().contains_key(p)
                        && !inner.failed.contains(p)
                        && !inner.queued.contains(p)
                })
                .collect();
            // Baratos primeiro (metadados; falha = por último).
            candidates.sort_by_key(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(u64::MAX));
            for path in candidates.into_iter().take(JOBS_PER_FRAME) {
                if !inner.queued.insert(path.clone()) {
                    continue;
                }
                if inner.tx.send(path).is_err() {
                    inner.queued.clear();
                    break;
                }
                changed = true;
            }
            while let Ok(msg) = inner.rx.try_recv() {
                changed = true;
                inner.queued.remove(&msg.path);
                batch.push((msg.path, msg.result));
            }
        }

        if !batch.is_empty() {
            let mut map = cache.write();
            for (path, handle) in batch {
                match handle {
                    Some(h) => {
                        map.insert(path, h);
                    }
                    None => {
                        map.remove(&path);
                        self.0.borrow_mut().failed.insert(path);
                    }
                }
            }
            if map.len() > THUMB_CAP {
                let keep: HashSet<PathBuf> = visible[lo..=hi]
                    .iter()
                    .map(|p| p.path().to_path_buf())
                    .collect();
                map.retain(|p, _| keep.contains(p));
                self.0.borrow_mut().failed.retain(|p| keep.contains(p));
            }
        }
        changed
    }

    /// Há job enviado e ainda sem resposta?
    ///
    /// O pump usa isto para acordar o próximo frame: a fila só é drenada no
    /// render da raiz, então sem o pulso as miniaturas ficariam paradas.
    #[must_use]
    pub fn in_flight(&self) -> bool {
        !self.0.borrow().queued.is_empty()
    }

    /// Esquece uma foto (ex.: após sobrescrever o arquivo).
    pub fn invalidate(&self, path: &std::path::Path) {
        self.0.borrow_mut().failed.remove(path);
    }

    /// Limpa fila e falhas (troca de pasta/arquivos).
    ///
    /// O `State` é limpo pelo chamador: aqui só cuidamos do não reativo.
    pub fn clear(&self) {
        let mut inner = self.0.borrow_mut();
        inner.queued.clear();
        inner.failed.clear();
    }
}

/// Índices da janela de thumbs (função pura, testável).
#[must_use]
pub fn window_range(len: usize, center: usize, radius: usize) -> (usize, usize) {
    if len == 0 {
        return (0, 0);
    }
    let lo = center.saturating_sub(radius);
    let hi = (center + radius).min(len - 1);
    (lo, hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_clamps_at_edges() {
        assert_eq!(window_range(0, 0, 25), (0, 0));
        assert_eq!(window_range(10, 0, 25), (0, 9));
        assert_eq!(window_range(100, 50, 25), (25, 75));
        assert_eq!(window_range(100, 95, 25), (70, 99));
    }
}
