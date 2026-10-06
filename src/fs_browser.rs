//! Descoberta de arquivos de imagem: pastas e arquivos soltos.
//!
//! Tipos de domínio (anti-primitivo): [`PhotoPath`] em vez de `String` solta.

use std::path::{Path, PathBuf};

/// Extensões suportadas no MVP (minúsculas, sem ponto).
pub const SUPPORTED_EXTENSIONS: &[&str] =
    &["jpg", "jpeg", "png", "webp", "tiff", "tif", "bmp", "gif"];

/// Caminho de foto validado pela extensão.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PhotoPath(PathBuf);

impl PhotoPath {
    /// Constrói a partir de qualquer caminho; aceita só extensões suportadas.
    pub fn new(path: PathBuf) -> Option<Self> {
        has_supported_extension(&path).then_some(Self(path))
    }

    /// Caminho interno.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Nome do arquivo para exibição (fallback: caminho completo).
    #[must_use]
    pub fn display_name(&self) -> String {
        self.0
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.0.to_string_lossy().into_owned())
    }

    /// Data de modificação (fallback: Unix Epoch).
    #[must_use]
    pub fn modified_time(&self) -> std::time::SystemTime {
        std::fs::metadata(&self.0)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    }

    /// Tamanho do arquivo em bytes (fallback: 0).
    #[must_use]
    pub fn file_size(&self) -> u64 {
        std::fs::metadata(&self.0).map(|m| m.len()).unwrap_or(0)
    }
}

/// Critérios de ordenação das fotos (0.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SortCriteria {
    #[default]
    Name,
    Date,
    Size,
}

impl SortCriteria {
    pub const ALL: &[(&'static str, Self)] = &[
        ("Nome", Self::Name),
        ("Data", Self::Date),
        ("Tamanho", Self::Size),
    ];

    #[must_use]
    pub fn from_str_name(s: &str) -> Self {
        match s {
            "date" => Self::Date,
            "size" => Self::Size,
            _ => Self::Name,
        }
    }

    #[must_use]
    pub fn as_str_name(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Date => "date",
            Self::Size => "size",
        }
    }
}

/// Ordena fotos segundo o critério e direção especificados.
pub fn sort_photos(photos: &mut [PhotoPath], criteria: SortCriteria, ascending: bool) {
    match criteria {
        SortCriteria::Name => {
            photos.sort_by_cached_key(|p| p.display_name().to_lowercase());
        }
        SortCriteria::Date => {
            photos.sort_by_cached_key(|p| p.modified_time());
        }
        SortCriteria::Size => {
            photos.sort_by_cached_key(|p| p.file_size());
        }
    }
    if !ascending {
        photos.reverse();
    }
}

/// Guarda classificações (estrelas 1..=5) persistidas no sidecar `.photoshow.json` da pasta (0.2).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct FolderSidecar {
    #[serde(default)]
    pub ratings: std::collections::HashMap<String, u8>,
    #[serde(default)]
    pub colors: std::collections::HashMap<String, u8>,
    #[serde(default)]
    pub tags: std::collections::HashMap<String, Vec<String>>,
}

impl FolderSidecar {
    #[must_use]
    pub fn load_for_dir(dir: &Path) -> Self {
        let sidecar_path = dir.join(".photoshow.json");
        std::fs::read_to_string(sidecar_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_for_dir(&self, dir: &Path) -> Result<(), String> {
        let sidecar_path = dir.join(".photoshow.json");
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let tmp = dir.join(format!(".photoshow.json.tmp-{unique}"));
        std::fs::write(&tmp, content).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, sidecar_path).map_err(|e| e.to_string())
    }

    #[must_use]
    pub fn get_rating(&self, photo_name: &str) -> u8 {
        self.ratings.get(photo_name).copied().unwrap_or(0)
    }

    pub fn set_rating(&mut self, photo_name: String, rating: u8) {
        if rating == 0 {
            self.ratings.remove(&photo_name);
        } else {
            self.ratings.insert(photo_name, rating.clamp(1, 5));
        }
    }

    #[must_use]
    pub fn get_color(&self, photo_name: &str) -> u8 {
        self.colors.get(photo_name).copied().unwrap_or(0)
    }

    pub fn set_color(&mut self, photo_name: String, color_id: u8) {
        if color_id == 0 {
            self.colors.remove(&photo_name);
        } else {
            self.colors.insert(photo_name, color_id);
        }
    }

    #[must_use]
    pub fn get_tags(&self, photo_name: &str) -> &[String] {
        self.tags.get(photo_name).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn add_tag(&mut self, photo_name: String, tag: String) {
        let tag = tag.trim().to_lowercase();
        if tag.is_empty() {
            return;
        }
        let list = self.tags.entry(photo_name).or_default();
        if !list.contains(&tag) {
            list.push(tag);
        }
    }

    pub fn remove_tag(&mut self, photo_name: &str, tag: &str) {
        let tag = tag.trim().to_lowercase();
        if let Some(list) = self.tags.get_mut(photo_name) {
            list.retain(|t| t != &tag);
            if list.is_empty() {
                self.tags.remove(photo_name);
            }
        }
    }

    #[must_use]
    pub fn has_tag(&self, photo_name: &str, tag: &str) -> bool {
        let tag = tag.trim().to_lowercase();
        self.tags
            .get(photo_name)
            .map(|list| list.iter().any(|t| t == &tag))
            .unwrap_or(false)
    }
}

/// Verifica extensão (case-insensitive).
#[must_use]
pub fn has_supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Opções da varredura (espelham as preferências do menu de ajustes).
#[derive(Debug, Clone, Copy)]
pub struct ScanOptions {
    /// Respeita `.gitignore`/`.ignore` (vale fora de repo git também).
    pub respect_gitignore: bool,
    /// Pula arquivos e pastas ocultos (dotfiles).
    pub skip_hidden: bool,
}

/// Resultado de uma varredura (lista vazia = nenhuma imagem, não erro).
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// Pasta varrida.
    pub dir: PathBuf,
    /// Fotos ordenadas por nome.
    pub photos: Vec<PhotoPath>,
    /// Arquivos inspecionados (para o status "N verificados").
    pub files_seen: u64,
}

fn is_skipped_dir(entry: &ignore::DirEntry) -> bool {
    entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
        && entry.file_name().to_string_lossy() == ".git"
}

fn walk_photos(dir: &Path, opts: ScanOptions) -> (Vec<PhotoPath>, u64) {
    let mut builder = ignore::WalkBuilder::new(dir);
    builder
        .hidden(opts.skip_hidden)
        .git_ignore(opts.respect_gitignore)
        .git_global(opts.respect_gitignore)
        .git_exclude(opts.respect_gitignore)
        .parents(opts.respect_gitignore)
        .require_git(false)
        .follow_links(false)
        .filter_entry(|e| !is_skipped_dir(e));
    let mut photos = Vec::new();
    let mut files_seen = 0u64;
    for entry in builder.build().filter_map(Result::ok) {
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        files_seen += 1;
        if let Some(p) = PhotoPath::new(entry.path().to_path_buf()) {
            photos.push(p);
        }
    }
    sort_photos(&mut photos, SortCriteria::Name, true);
    (photos, files_seen)
}

/// Varredura de pasta (bloqueante: chamar de uma thread de worker).
///
/// Pula `.git` sempre, ocultas e `gitignore` conforme as opções (rápido em
/// árvores com milhares de arquivos não-imagem).
#[must_use]
pub fn scan_blocking(dir: PathBuf, opts: ScanOptions) -> ScanResult {
    let (photos, files_seen) = walk_photos(&dir, opts);
    ScanResult {
        dir,
        photos,
        files_seen,
    }
}

/// Filtra uma lista solta de arquivos (diálogo nativo) para fotos válidas.
#[must_use]
pub fn filter_loose_files(paths: Vec<PathBuf>) -> Vec<PhotoPath> {
    let mut photos: Vec<PhotoPath> = paths.into_iter().filter_map(PhotoPath::new).collect();
    sort_photos(&mut photos, SortCriteria::Name, true);
    photos
}

/// Subpastas diretas ordenadas por nome (ignora erros de permissão).
#[must_use]
pub fn list_subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort_by_key(|a| dir_name(a).to_lowercase());
    dirs
}

fn dir_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.display().to_string())
}

/// Pasta/arquivo oculto (nome começa com ponto)?
#[must_use]
pub fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .map(|n| n.to_string_lossy().starts_with('.'))
        .unwrap_or(false)
}

/// Move uma foto para a lixeira do sistema operacional de forma segura.
pub fn delete_to_trash(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|e| format!("lixeira: {e}"))
}

/// Renomeia uma foto dentro da mesma pasta. Erro em texto para a status bar.
pub fn rename_photo(path: &Path, new_name: &str) -> Result<PathBuf, String> {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err(String::from("Nome vazio."));
    }
    if new_name.contains(['/', '\\']) {
        return Err(String::from("Nome não pode conter barras."));
    }
    let Some(parent) = path.parent() else {
        return Err(String::from("Pasta de origem inválida."));
    };
    let dest = parent.join(new_name);
    if dest == path {
        return Err(String::from("Nome igual ao atual."));
    }
    if dest.exists() {
        return Err(String::from("Já existe um arquivo com esse nome."));
    }
    std::fs::rename(path, &dest).map_err(|e| format!("Falha ao renomear: {e}"))?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn accepts_supported_extensions_case_insensitive() {
        assert!(has_supported_extension(Path::new("foto.JPG")));
        assert!(has_supported_extension(Path::new("img.WebP")));
        assert!(!has_supported_extension(Path::new("doc.pdf")));
        assert!(!has_supported_extension(Path::new("sem_extensao")));
    }

    fn scan_opts() -> ScanOptions {
        ScanOptions {
            respect_gitignore: true,
            skip_hidden: true,
        }
    }

    #[test]
    fn scan_lists_and_sorts_photos() {
        let dir = tempfile::tempdir().expect("tempdir");
        for name in ["b.png", "a.JPG", "nota.txt", "c.gif"] {
            fs::write(dir.path().join(name), b"x").expect("write");
        }
        let res = scan_blocking(dir.path().to_path_buf(), scan_opts());
        assert_eq!(res.files_seen, 4);
        let names: Vec<_> = res.photos.iter().map(|p| p.display_name()).collect();
        assert_eq!(names, vec!["a.JPG", "b.png", "c.gif"]);
    }

    #[test]
    fn scan_empty_dir_returns_empty_vec() {
        let dir = tempfile::tempdir().expect("tempdir");
        let res = scan_blocking(dir.path().to_path_buf(), scan_opts());
        assert!(res.photos.is_empty());
    }

    #[test]
    fn scan_respects_gitignore_and_hidden() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).expect("mkdir");
        fs::write(sub.join(".gitignore"), b"*.png\n").expect("write");
        fs::write(sub.join("a.png"), b"x").expect("write");
        fs::write(sub.join("b.jpg"), b"x").expect("write");
        fs::create_dir(sub.join(".hidden")).expect("mkdir");
        fs::write(sub.join(".hidden").join("c.jpg"), b"x").expect("write");

        let res = scan_blocking(dir.path().to_path_buf(), scan_opts());
        let names: Vec<_> = res.photos.iter().map(|p| p.display_name()).collect();
        assert_eq!(names, vec!["b.jpg"]);

        let no_opts = ScanOptions {
            respect_gitignore: false,
            skip_hidden: false,
        };
        let res = scan_blocking(dir.path().to_path_buf(), no_opts);
        let names: Vec<_> = res.photos.iter().map(|p| p.display_name()).collect();
        assert_eq!(names, vec!["a.png", "b.jpg", "c.jpg"]);
    }

    #[test]
    fn scan_always_skips_git_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git = dir.path().join(".git");
        fs::create_dir(&git).expect("mkdir");
        fs::write(git.join("x.jpg"), b"x").expect("write");
        let no_opts = ScanOptions {
            respect_gitignore: false,
            skip_hidden: false,
        };
        let res = scan_blocking(dir.path().to_path_buf(), no_opts);
        assert!(res.photos.is_empty());
    }

    #[test]
    fn filter_loose_files_drops_unsupported() {
        let photos = filter_loose_files(vec![
            PathBuf::from("/x/foto.png"),
            PathBuf::from("/x/texto.txt"),
        ]);
        assert_eq!(photos.len(), 1);
        assert_eq!(photos[0].display_name(), "foto.png");
    }

    #[test]
    fn list_subdirs_returns_sorted_dirs_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::create_dir(dir.path().join("b_sub")).expect("mkdir");
        fs::create_dir(dir.path().join("a_sub")).expect("mkdir");
        fs::write(dir.path().join("foto.png"), b"x").expect("write");
        let subs = list_subdirs(dir.path());
        let names: Vec<_> = subs
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["a_sub", "b_sub"]);
    }

    #[test]
    fn rename_moves_file_and_rejects_conflicts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("foto.png");
        fs::write(&src, b"x").expect("write");
        let dest = rename_photo(&src, "nova.png").expect("rename");
        assert!(dest.exists());
        assert!(!src.exists());
        fs::write(dir.path().join("outra.png"), b"y").expect("write");
        assert!(rename_photo(&dest, "outra.png").is_err());
        assert!(rename_photo(&dest, "").is_err());
        assert!(rename_photo(&dest, "a/b.png").is_err());
    }

    #[test]
    fn hidden_detection_uses_dot_prefix() {
        assert!(is_hidden(Path::new("/a/.config")));
        assert!(!is_hidden(Path::new("/a/fotos")));
    }
}
