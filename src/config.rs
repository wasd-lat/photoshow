//! Configuração persistente: favoritas, última pasta e preferências.
//!
//! JSON em `<config_dir>/photoshow/config.json`. Tudo best-effort:
//! falha de IO nunca quebra o app, só volta aos padrões.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Preferências editáveis no menu ⚙ Config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Pastas fixadas (navegação rápida).
    #[serde(default)]
    pub favorites: Vec<PathBuf>,
    /// Última pasta aberta (auto-abrir ao iniciar).
    #[serde(default)]
    pub last_folder: Option<PathBuf>,
    /// Pedir confirmação ao sobrescrever o original.
    #[serde(default = "default_true")]
    pub confirm_overwrite: bool,
    /// Qualidade JPEG ao salvar (1..=100).
    #[serde(default = "default_jpeg_quality")]
    pub jpeg_quality: u8,
    /// Exibir a faixa de thumbnails.
    #[serde(default = "default_true")]
    pub show_filmstrip: bool,
    /// Reabrir a última pasta ao iniciar.
    #[serde(default = "default_true")]
    pub open_last_on_startup: bool,
    /// Respeita `.gitignore`/`.ignore` na varredura (rápido em repos).
    #[serde(default = "default_true")]
    pub respect_gitignore: bool,
    /// Pula arquivos e pastas ocultas na varredura.
    #[serde(default = "default_true")]
    pub skip_hidden: bool,
    /// Pré-carrega vizinhos de até N MB (0 = desativa prefetch).
    #[serde(default = "default_prefetch_mb")]
    pub prefetch_max_mb: u64,
    /// Exibe pastas ocultas (dotfiles) na árvore de navegação.
    #[serde(default)]
    pub show_hidden_folders: bool,
    /// Tema visual (ver `crate::theme::THEMES`).
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Lado do thumbnail da galeria em px (48..=192).
    #[serde(default = "default_thumb_size")]
    pub thumb_size: f32,
    /// Escala global da tipografia (0.75..=1.40; 1.0 = padrão).
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,
    /// Recolhe o navegador lateral (Ctrl+1).
    #[serde(default)]
    pub hide_browser: bool,
    /// Recolhe a galeria inferior (Ctrl+2).
    #[serde(default)]
    pub hide_gallery: bool,
    /// Recolhe a árvore de pastas/favoritas (lado esquerdo, topo).
    #[serde(default)]
    pub hide_tree: bool,
    /// Recolhe a lista de fotos (lado esquerdo, baixo).
    #[serde(default)]
    pub hide_photos: bool,
    /// Recolhe o painel de ajustes (Ctrl+3).
    #[serde(default)]
    pub hide_adjust: bool,
    /// Contorno de foco sempre visível nos botões.
    #[serde(default)]
    pub always_focus_ring: bool,
    /// Critério de ordenação: "name", "date", "size".
    #[serde(default = "default_sort_criteria")]
    pub sort_criteria: String,
    /// Ordenação ascendente?
    #[serde(default = "default_true")]
    pub sort_ascending: bool,
    /// Intervalo do slideshow em segundos (1..=60).
    #[serde(default = "default_slideshow_interval")]
    pub slideshow_interval_secs: u64,
    /// Proporção (%) da largura do navegador no dock (12..=50).
    #[serde(default = "default_dock_browser")]
    pub dock_browser_percent: f32,
    /// Proporção (%) da altura da galeria no dock (8..=50).
    #[serde(default = "default_dock_gallery")]
    pub dock_gallery_percent: f32,
    /// Definições de tags coloridas (1..=8).
    #[serde(default = "default_color_tags")]
    pub color_tags: Vec<ColorTagDef>,
    /// Catálogo global de tags nomeadas para autocomplete/organização.
    #[serde(default)]
    pub named_tags: Vec<String>,
}

/// Definição de etiqueta colorida personalizada pelo usuário.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorTagDef {
    pub id: u8,
    pub name: String,
    pub color_hex: String,
}

impl ColorTagDef {
    pub fn new(id: u8, name: impl Into<String>, hex: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            color_hex: hex.into(),
        }
    }
}

pub fn default_color_tags() -> Vec<ColorTagDef> {
    vec![
        ColorTagDef::new(1, "Vermelho", "#EF4444"),
        ColorTagDef::new(2, "Amarelo", "#F59E0B"),
        ColorTagDef::new(3, "Verde", "#10B981"),
        ColorTagDef::new(4, "Azul", "#3B82F6"),
        ColorTagDef::new(5, "Roxo", "#8B5CF6"),
    ]
}

fn default_sort_criteria() -> String {
    String::from("name")
}

fn default_slideshow_interval() -> u64 {
    3
}

fn default_dock_browser() -> f32 {
    24.0
}

fn default_dock_gallery() -> f32 {
    20.0
}

fn default_true() -> bool {
    true
}

fn default_jpeg_quality() -> u8 {
    90
}

fn default_prefetch_mb() -> u64 {
    64
}

fn default_theme() -> String {
    String::from("slate")
}

fn default_thumb_size() -> f32 {
    88.0
}

fn default_ui_scale() -> f32 {
    1.0
}

/// Reexport dos temas visuais (implementados em `crate::theme`).
#[allow(unused_imports)]
pub use crate::theme::THEMES;

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            favorites: Vec::new(),
            last_folder: None,
            confirm_overwrite: true,
            jpeg_quality: 90,
            show_filmstrip: true,
            open_last_on_startup: true,
            respect_gitignore: true,
            skip_hidden: true,
            prefetch_max_mb: 64,
            show_hidden_folders: false,
            theme: String::from("slate"),
            thumb_size: 88.0,
            ui_scale: 1.0,
            hide_browser: false,
            hide_gallery: false,
            hide_tree: false,
            hide_photos: false,
            hide_adjust: false,
            always_focus_ring: false,
            sort_criteria: String::from("name"),
            sort_ascending: true,
            slideshow_interval_secs: 3,
            dock_browser_percent: 24.0,
            dock_gallery_percent: 20.0,
            color_tags: default_color_tags(),
            named_tags: Vec::new(),
        }
    }
}

/// Menor escala tipográfica aceita (evita texto ilegível).
pub const UI_SCALE_MIN: f32 = 0.75;
/// Maior escala tipográfica aceita (evita estourar a barra).
pub const UI_SCALE_MAX: f32 = 1.40;
/// Passo do ajuste de escala (tecla/slider).
pub const UI_SCALE_STEP: f32 = 0.05;

/// Traz uma escala para a faixa válida, substituindo NaN pelo padrão.
///
/// Feito como função pura e pública para ser testável e para o egui reusar
/// exatamente a mesma regra.
#[must_use]
pub fn sanitize_ui_scale(scale: f32) -> f32 {
    if scale.is_finite() {
        scale.clamp(UI_SCALE_MIN, UI_SCALE_MAX)
    } else {
        1.0
    }
}

impl AppConfig {
    /// Caminho do arquivo de config (None se o SO não informar).
    #[must_use]
    pub fn file_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("dev", "photoshow", "photoshow")
            .map(|d| d.config_dir().join("config.json"))
    }

    /// Carrega do disco ou padrões.
    #[must_use]
    pub fn load() -> Self {
        Self::file_path()
            .and_then(|p| Self::load_from(&p))
            .unwrap_or_default()
    }

    /// Carrega de um caminho explícito (usado em testes).
    /// Se o JSON estiver corrompido, cria backup e recupera com padrões (Config Recovery).
    #[must_use]
    pub fn load_from(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        let parsed = serde_json::from_str::<Self>(&text);
        let mut cfg = match parsed {
            Ok(c) => c,
            Err(_) => {
                // Config corrompida: faz backup para o usuário não perder dados
                let unique = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let backup_path = path.with_extension(format!("corrupt-{unique}.bak"));
                let _ = std::fs::copy(path, backup_path);
                Self::default()
            }
        };
        cfg.jpeg_quality = cfg.jpeg_quality.clamp(1, 100);
        cfg.prefetch_max_mb = cfg.prefetch_max_mb.min(1024);
        cfg.thumb_size = cfg.thumb_size.clamp(48.0, 192.0);
        cfg.ui_scale = sanitize_ui_scale(cfg.ui_scale);
        cfg.dock_browser_percent = cfg.dock_browser_percent.clamp(12.0, 50.0);
        cfg.dock_gallery_percent = cfg.dock_gallery_percent.clamp(8.0, 50.0);
        cfg.slideshow_interval_secs = cfg.slideshow_interval_secs.clamp(1, 60);
        // Remove favoritas que não existem mais.
        cfg.favorites.retain(|p| p.is_dir());
        Some(cfg)
    }

    /// Persiste (cria o diretório se preciso). Erro só vira status na UI.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::file_path().ok_or_else(|| String::from("sem diretório de config"))?;
        self.save_to(&path)
    }

    /// Persiste num caminho explícito de forma atômica (grava em temporário e renomeia).
    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let tmp = path.with_extension(format!("tmp-{unique}"));
        std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
        if let Ok(f) = std::fs::File::open(&tmp) {
            let _ = f.sync_all();
        }
        std::fs::rename(&tmp, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            e.to_string()
        })
    }

    /// Alterna favorito; `true` se fixou, `false` se desafixou.
    pub fn toggle_favorite(&mut self, dir: &Path) -> bool {
        if let Some(i) = self.favorites.iter().position(|f| f == dir) {
            self.favorites.remove(i);
            false
        } else {
            self.favorites.push(dir.to_path_buf());
            true
        }
    }

    /// A pasta está fixada?
    #[must_use]
    pub fn is_favorite(&self, dir: &Path) -> bool {
        self.favorites.iter().any(|f| f == dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_scale_is_clamped_and_nan_safe() {
        assert_eq!(sanitize_ui_scale(0.1), UI_SCALE_MIN);
        assert_eq!(sanitize_ui_scale(9.0), UI_SCALE_MAX);
        assert_eq!(sanitize_ui_scale(f32::NAN), 1.0);
        assert_eq!(sanitize_ui_scale(1.15), 1.15);
    }

    #[test]
    fn old_config_without_new_keys_loads_with_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.json");
        // Config gravado antes de ui_scale/hide_* existirem.
        std::fs::write(&path, r#"{"jpeg_quality":80,"theme":"frost"}"#).expect("write");
        let cfg = AppConfig::load_from(&path).expect("load");
        assert_eq!(cfg.ui_scale, 1.0);
        assert!(!cfg.hide_browser);
        assert!(!cfg.hide_gallery);
        assert!(!cfg.hide_tree);
        assert!(!cfg.hide_photos);
        assert_eq!(cfg.jpeg_quality, 80);
    }

    #[test]
    fn roundtrip_preserves_values() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.json");
        let mut cfg = AppConfig::default();
        cfg.favorites.push(PathBuf::from("/tmp"));
        cfg.jpeg_quality = 77;
        cfg.confirm_overwrite = false;
        cfg.save_to(&path).expect("save");
        let back = AppConfig::load_from(&path).expect("load");
        assert_eq!(back.jpeg_quality, 77);
        assert!(!back.confirm_overwrite);
        // "/tmp" existe, então sobrevive ao retain.
        assert_eq!(back.favorites, vec![PathBuf::from("/tmp")]);
    }

    #[test]
    fn toggle_favorite_pins_and_unpins() {
        let mut cfg = AppConfig::default();
        let dir = Path::new("/fotos");
        assert!(cfg.toggle_favorite(dir));
        assert!(cfg.is_favorite(dir));
        assert!(!cfg.toggle_favorite(dir));
        assert!(!cfg.is_favorite(dir));
    }

    #[test]
    fn corrupt_file_recovers_to_defaults_and_makes_backup() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.json");
        std::fs::write(&path, "{ invalido").expect("write");
        let recovered = AppConfig::load_from(&path).expect("deve recuperar");
        assert_eq!(recovered.jpeg_quality, 90);
        let baks: Vec<_> = std::fs::read_dir(dir.path())
            .expect("read_dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("corrupt-"))
            .collect();
        assert!(!baks.is_empty(), "backup do arquivo corrompido ausente");
    }
}
