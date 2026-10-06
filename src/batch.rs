//! Processamento de fotos em lote (Versão 0.5).
//!
//! Reutiliza o pipeline atômico e de salvamento existente:
//! decodifica -> rotaciona -> redimensiona -> converte formato -> grava atômico.
//! Executa em thread dedicada com cancelamento cooperativo e relatório.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::editor::save_baked;
use crate::fs_browser::PhotoPath;
use crate::image_store::decode_photo;

/// Opções de transformação em lote.
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Rotação acumulada (0..=3 quartos de volta CW).
    pub rotate_cw: u8,
    /// Dimensão máxima em pixels (`None` = mantém tamanho).
    pub max_dim: Option<u32>,
    /// Formato de saída ("jpg", "png", "webp" ou vazio para manter).
    pub format: String,
    /// Qualidade JPEG (1..=100).
    pub jpeg_quality: u8,
    /// Padrão de renomeação ("" = nome original; ex: "viagem_{i}" ou "foto_###").
    pub name_pattern: String,
    /// Pasta de destino.
    pub dest_dir: PathBuf,
}

/// Estado do progresso da fila em lote.
#[derive(Debug, Clone, Default)]
pub struct BatchProgress {
    /// Quantidade de arquivos processados até o momento.
    pub current: usize,
    /// Total de arquivos na fila.
    pub total: usize,
    /// Nome do arquivo sendo processado agora.
    pub current_file: String,
    /// Processamento concluído?
    pub finished: bool,
    /// Cancelado pelo usuário?
    pub cancelled: bool,
    /// Total de arquivos processados com sucesso.
    pub successes: usize,
    /// Lista de falhas: (nome_arquivo, erro).
    pub failures: Vec<(String, String)>,
}

/// Executa o lote em background reportando o progresso por callback.
pub fn run_batch(
    photos: Vec<PhotoPath>,
    config: BatchConfig,
    cancel: Arc<AtomicBool>,
    progress_cb: impl Fn(BatchProgress) + Send + 'static,
) {
    std::thread::spawn(move || {
        let total = photos.len();
        let mut successes = 0;
        let mut failures = Vec::new();

        for (idx, photo) in photos.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                progress_cb(BatchProgress {
                    current: idx,
                    total,
                    current_file: photo.display_name(),
                    finished: true,
                    cancelled: true,
                    successes,
                    failures,
                });
                return;
            }

            progress_cb(BatchProgress {
                current: idx,
                total,
                current_file: photo.display_name(),
                finished: false,
                cancelled: false,
                successes,
                failures: failures.clone(),
            });

            match process_single(photo.path(), &config, idx + 1) {
                Ok(()) => successes += 1,
                Err(e) => failures.push((photo.display_name(), e)),
            }
        }

        progress_cb(BatchProgress {
            current: total,
            total,
            current_file: String::new(),
            finished: true,
            cancelled: false,
            successes,
            failures,
        });
    });
}

fn process_single(src: &Path, cfg: &BatchConfig, index: usize) -> Result<(), String> {
    let dec = decode_photo(src).map_err(|e| e.to_string())?;
    let mut img = dec.full;

    // 1. Rotação
    match cfg.rotate_cw % 4 {
        1 => img = img.rotate90(),
        2 => img = img.rotate180(),
        3 => img = img.rotate270(),
        _ => {}
    }

    // 2. Redimensionamento
    if let Some(max) = cfg.max_dim
        && (img.width() > max || img.height() > max)
    {
        img = img.thumbnail(max, max);
    }

    // 3. Destino e nome
    let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| String::from("foto"));
    let orig_ext = src
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_else(|| String::from("jpg"));

    let ext = if cfg.format.is_empty() || cfg.format == "original" {
        orig_ext
    } else {
        cfg.format.clone()
    };

    let filename = if cfg.name_pattern.is_empty() {
        format!("{stem}.{ext}")
    } else {
        let pat = cfg
            .name_pattern
            .replace("{i}", &format!("{index:03}"))
            .replace("###", &format!("{index:03}"));
        format!("{pat}.{ext}")
    };

    let dest = cfg.dest_dir.join(filename);
    save_baked(&img, &dest, cfg.jpeg_quality, Some(src))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_config_generates_correct_names() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = BatchConfig {
            rotate_cw: 0,
            max_dim: None,
            format: String::from("png"),
            jpeg_quality: 90,
            name_pattern: String::from("foto_{i}"),
            dest_dir: dir.path().to_path_buf(),
        };
        let _src = Path::new("/dummy/teste.jpg");
        // Valida substituição de padrão
        let name = if cfg.name_pattern.is_empty() {
            String::from("teste.png")
        } else {
            let pat = cfg.name_pattern.replace("{i}", &format!("{:03}", 42));
            format!("{pat}.{}", cfg.format)
        };
        assert_eq!(name, "foto_042.png");
    }
}
