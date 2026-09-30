//! O que abrir a partir de caminhos externos (argv, `%F` do `.desktop`).
//!
//! A regra é a mesma para o `argv` do arranque e para os diálogos nativos, e
//! está isolada aqui em duas partes: [`Target`]/[`resolve`] são **puras** (o
//! que abrir, dada uma lista de caminhos) e [`open_target`] é o **efeito**
//! (aplica no estado). Assim a decisão tem teste sem UI e o efeito fica num
//! lugar só — antes o `argv` era simplesmente ignorado e a barra de Ferramentas
//! tinha uma cópia da lógica.

use std::path::PathBuf;

use crate::app::services::Services;
use crate::app::state::{self, AppChannel};
use crate::fs_browser::{self, PhotoPath};

/// O que o app deve abrir, já interpretado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// Uma pasta: varre recursivamente e mostra tudo que achar.
    Folder(PathBuf),
    /// Uma pasta **e** a foto a selecionar nela.
    ///
    /// É o que o "Abrir com" do gerenciador de arquivos entrega: uma foto
    /// solta. Abrir a pasta dela (em vez de uma lista de um item só) é o que o
    /// usuário espera — ele quer navegar as vizinhas em seguida.
    Selection {
        /// Pasta a abrir.
        dir: PathBuf,
        /// Foto a deixar selecionada.
        photo: PathBuf,
    },
    /// Arquivos soltos: sem árvore de pastas, só a lista passada.
    Files(Vec<PhotoPath>),
}

/// Interpreta uma lista de caminhos vindos de fora.
///
/// Regras, em ordem:
/// 1. Nenhum caminho → `None` (o app decide se reabre a última pasta).
/// 2. Uma pasta → [`Target::Folder`].
/// 3. Uma foto que existe → [`Target::Selection`] com a pasta dela.
/// 4. Vários caminhos → [`Target::Files`], já filtrados por extensão.
///
/// Uma foto que **não** existe em disco ainda é tratada como arquivo solto
/// (a extensão decide: `~/foto.png` é foto, `~/notas.txt` não é) — assim um
/// caminho colado à mão mostra o que der para mostrar, em vez de nada.
#[must_use]
pub fn resolve(paths: &[PathBuf]) -> Option<Target> {
    let usable: Vec<&PathBuf> = paths.iter().filter(|p| !p.as_os_str().is_empty()).collect();
    if usable.is_empty() {
        return None;
    }
    // Uma pasta sozinha é um "abrir pasta".
    if usable.len() == 1 && usable[0].is_dir() {
        return Some(Target::Folder(usable[0].clone()));
    }
    // Uma foto existente sozinha: abre a pasta dela com a foto selecionada.
    if usable.len() == 1
        && let Some(photo) = usable[0]
            .is_file()
            .then(|| usable[0].clone())
            .filter(|p| PhotoPath::new(p.clone()).is_some())
        && let Some(dir) = usable[0].parent().filter(|d| d.is_dir())
    {
        return Some(Target::Selection {
            dir: dir.to_path_buf(),
            photo,
        });
    }
    let photos = fs_browser::filter_loose_files(usable.iter().map(|p| (*p).clone()).collect());
    (!photos.is_empty()).then_some(Target::Files(photos))
}

/// Aplica o alvo no estado global.
pub fn open_target(services: &Services, target: Target) {
    match target {
        Target::Folder(dir) => state::update(AppChannel::Photos, |st| {
            state::open_dir_path(services, st, dir);
        }),
        Target::Selection { dir, photo } => state::update(AppChannel::Photos, |st| {
            state::open_dir_and_select(services, st, dir, photo);
        }),
        Target::Files(photos) => state::update(AppChannel::Photos, |st| {
            st.tree = None;
            st.current_dir = None;
            st.status = format!("{} arquivos soltos", photos.len());
            state::replace_photos(st, services, photos);
        }),
    }
}

/// Abre o que `paths` pedir; `false` quando não há nada para abrir.
pub fn open_paths(services: &Services, paths: &[PathBuf]) -> bool {
    let Some(target) = resolve(paths) else {
        return false;
    };
    open_target(services, target);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_to_open_is_none() {
        assert_eq!(resolve(&[]), None);
        assert_eq!(resolve(&[PathBuf::new()]), None);
    }

    #[test]
    fn a_real_directory_becomes_a_folder_target() {
        let dir = std::env::temp_dir();
        assert_eq!(
            resolve(std::slice::from_ref(&dir)),
            Some(Target::Folder(dir))
        );
    }

    #[test]
    fn loose_files_are_filtered_by_extension() {
        let paths = vec![
            PathBuf::from("/tmp/b.png"),
            PathBuf::from("/tmp/a.JPG"),
            PathBuf::from("/tmp/notas.txt"),
        ];
        let Some(Target::Files(photos)) = resolve(&paths) else {
            panic!("esperava arquivos soltos");
        };
        let names: Vec<String> = photos.iter().map(|p| p.display_name()).collect();
        assert_eq!(names, vec!["a.JPG".to_string(), "b.png".to_string()]);
    }

    #[test]
    fn only_unsupported_extensions_is_none() {
        assert_eq!(resolve(&[PathBuf::from("/tmp/notas.txt")]), None);
    }

    #[test]
    fn a_directory_wins_only_when_it_is_alone() {
        let dir = std::env::temp_dir();
        let paths = vec![dir.clone(), PathBuf::from("/tmp/a.png")];
        let Some(Target::Files(photos)) = resolve(&paths) else {
            panic!("mistura de pasta e arquivo não é Target::Folder");
        };
        assert_eq!(photos.len(), 1);
    }

    #[test]
    fn one_existing_photo_opens_its_folder_with_the_photo_selected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let photo = dir.path().join("z_aberta.png");
        std::fs::write(&photo, b"nao precisa ser um png de verdade").expect("escrever");

        assert_eq!(
            resolve(std::slice::from_ref(&photo)),
            Some(Target::Selection {
                dir: dir.path().to_path_buf(),
                photo,
            })
        );
    }

    #[test]
    fn a_photo_that_does_not_exist_stays_a_loose_file() {
        let missing = PathBuf::from("/tmp/nao_existe_42.png");
        let Some(Target::Files(photos)) = resolve(std::slice::from_ref(&missing)) else {
            panic!("foto inexistente deve virar arquivo solto");
        };
        assert_eq!(photos.len(), 1);
        assert_eq!(photos[0].path(), missing);
    }
}
