//! Teste de integração do pipeline completo, sem janela.
//!
//! Cobre o caminho que a UI exercita a cada interação: varrer pasta ->
//! decodificar -> aplicar edição -> assar -> gravar -> reler. É a mesma
//! sequência de `image_store`, `editor` e `services`, sem o Freya.

use std::path::{Path, PathBuf};

use photoshow::{
    editor::{CropRect, EditorStack, apply_to_image, bake, save_baked},
    exif,
    fs_browser::{self, PhotoPath, ScanOptions, has_supported_extension, scan_blocking},
    image_store::{DISPLAY_MAX_DIM, decode_photo, image_handle},
};

/// Foto de teste gerada em disco, com dimensões conhecidas.
fn write_photo(dir: &Path, name: &str, w: u32, h: u32) -> PathBuf {
    let img = image::RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
    });
    let path = dir.join(name);
    img.save(&path).expect("gravar png de teste");
    path
}

/// Pasta com algumas fotos e um arquivo que não é imagem.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    write_photo(dir.path(), "a_um.png", 640, 480);
    write_photo(dir.path(), "b_dois.JPG", 320, 240);
    write_photo(dir.path(), "c_tres.webp", 200, 200);
    // Um arquivo que não é imagem: a varredura deve inspecionar e descartar.
    std::fs::write(dir.path().join("notas.txt"), b"nao sou imagem").expect("escrever txt");
    dir
}

fn scan_opts() -> ScanOptions {
    ScanOptions {
        respect_gitignore: true,
        skip_hidden: true,
    }
}

#[test]
fn scan_finds_only_supported_images_sorted_by_name() {
    let dir = fixture();
    let result = scan_blocking(dir.path().to_path_buf(), scan_opts());

    let names: Vec<String> = result.photos.iter().map(|p| p.display_name()).collect();
    assert_eq!(names, vec!["a_um.png", "b_dois.JPG", "c_tres.webp"]);
    // O .txt foi inspecionado, mas não entrou na lista.
    assert_eq!(result.files_seen, 4);
    for photo in &result.photos {
        assert!(has_supported_extension(photo.path()));
    }
}

#[test]
fn loose_files_are_filtered_and_sorted() {
    let photos = fs_browser::filter_loose_files(vec![
        PathBuf::from("/x/c.png"),
        PathBuf::from("/x/a.txt"),
        PathBuf::from("/x/b.jpg"),
    ]);
    let names: Vec<String> = photos.iter().map(|p| p.display_name()).collect();
    assert_eq!(names, vec!["b.jpg", "c.png"]);
}

#[test]
fn decode_gives_full_and_display_versions() {
    let dir = fixture();
    let path = write_photo(dir.path(), "grande.png", 3000, 2000);

    let dec = decode_photo(&path).expect("decodificar");
    assert_eq!(dec.full_size, (3000, 2000));
    // A versão de display respeita o teto.
    assert!(
        dec.display.width().max(dec.display.height()) <= DISPLAY_MAX_DIM,
        "display não foi reduzido: {:?}",
        (dec.display.width(), dec.display.height())
    );
    // Full-res preservada para o bake.
    assert_eq!((dec.full.width(), dec.full.height()), (3000, 2000));
}

#[test]
fn decoded_image_becomes_a_freya_handle() {
    let dir = fixture();
    let path = write_photo(dir.path(), "pequena.png", 32, 24);
    let dec = decode_photo(&path).expect("decodificar");

    let handle = image_handle(&dec.display).expect("handle");
    assert_eq!((handle.image.width(), handle.image.height()), (32, 24));
}

#[test]
fn editing_a_photo_preserves_full_resolution_for_the_bake() {
    let dir = fixture();
    let path = write_photo(dir.path(), "editar.png", 800, 600);
    let dec = decode_photo(&path).expect("decodificar");

    let mut editor = EditorStack::new();
    editor.rotate_cw((dec.display.width(), dec.display.height()));
    let edit = editor.state();
    assert_eq!(edit.rot, 1);

    // O preview gira; o bake usa a full-res e o mesmo fator de escala.
    let preview = apply_to_image(&dec.display, &edit);
    assert_eq!((preview.width(), preview.height()), (600, 800));

    let baked = bake(
        &dec.full,
        (dec.display.width(), dec.display.height()),
        &edit,
    );
    assert_eq!((baked.width(), baked.height()), (600, 800));
}

#[test]
fn crop_scales_from_display_to_full_resolution() {
    let dir = fixture();
    let path = write_photo(dir.path(), "crop.png", 800, 600);
    let dec = decode_photo(&path).expect("decodificar");

    // Display e full são 1:1 aqui, então o crop é o mesmo retângulo.
    let mut editor = EditorStack::new();
    editor.set_crop(Some(CropRect {
        x: 100,
        y: 50,
        w: 200,
        h: 150,
    }));
    let baked = bake(
        &dec.full,
        (dec.display.width(), dec.display.height()),
        &editor.state(),
    );
    assert_eq!((baked.width(), baked.height()), (200, 150));
}

#[test]
fn undo_and_redo_roundtrip_through_the_stack() {
    let mut editor = EditorStack::new();
    editor.rotate_cw((100, 80));
    assert_eq!(editor.state().rot, 1);
    assert!(editor.undo());
    assert!(editor.state().is_clean());
    assert!(editor.can_redo());
    assert!(editor.redo());
    assert_eq!(editor.state().rot, 1);
}

#[test]
fn saving_overwrites_and_reloads_with_the_edit_applied() {
    let dir = fixture();
    let path = write_photo(dir.path(), "salvar.png", 400, 300);
    let dec = decode_photo(&path).expect("decodificar");

    let mut editor = EditorStack::new();
    editor.rotate_ccw((dec.display.width(), dec.display.height()));

    let baked = bake(
        &dec.full,
        (dec.display.width(), dec.display.height()),
        &editor.state(),
    );
    save_baked(&baked, &path, 92).expect("salvar");

    // Relendo do disco: a foto agora está girada de fato.
    let after = decode_photo(&path).expect("reler");
    assert_eq!((after.full_size.0, after.full_size.1), (300, 400));
}

#[test]
fn saving_in_every_supported_format_roundtrips() {
    let dir = fixture();
    let source = write_photo(dir.path(), "fonte.png", 120, 90);
    let dec = decode_photo(&source).expect("decodificar");
    let clean = EditorStack::new().state();

    for ext in ["png", "jpg", "bmp", "gif", "tiff", "webp"] {
        let out = dir.path().join(format!("saida.{ext}"));
        let baked = bake(
            &dec.full,
            (dec.display.width(), dec.display.height()),
            &clean,
        );
        save_baked(&baked, &out, 88).unwrap_or_else(|e| panic!("salvar {ext}: {e}"));
        assert!(out.exists(), "{ext} não foi criado");
        let reread = decode_photo(&out).unwrap_or_else(|e| panic!("reler {ext}: {e}"));
        assert_eq!(reread.full_size, (120, 90), "{ext} perdeu as dimensões");
    }
}

#[test]
fn missing_orientation_falls_back_to_identity() {
    // Arquivo sem EXIF: a orientação precisa ser 1, sem quebrar o decode.
    let dir = fixture();
    let path = write_photo(dir.path(), "sem_exif.png", 40, 20);
    assert_eq!(exif::read_orientation(&path), 1);
    let dec = decode_photo(&path).expect("decodificar");
    assert_eq!(dec.full_size, (40, 20));
}

#[test]
fn rename_photo_rejects_unsafe_names() {
    let dir = fixture();
    let src = dir.path().join("original.png");
    std::fs::write(&src, b"x").expect("escrever");

    // Move de verdade...
    let moved = fs_browser::rename_photo(&src, "novo.png").expect("renomear");
    assert!(moved.exists());
    assert!(!src.exists());

    // ...e recusa nome vazio, com barra e nome repetido.
    assert!(fs_browser::rename_photo(&moved, "").is_err());
    assert!(fs_browser::rename_photo(&moved, "   ").is_err());
    assert!(fs_browser::rename_photo(&moved, "a/b.png").is_err());
    assert!(fs_browser::rename_photo(&moved, r"a\b.png").is_err());
    assert!(fs_browser::rename_photo(&moved, "novo.png").is_err());

    // Nome já existente também é recusado.
    std::fs::write(dir.path().join("ocupado.png"), b"y").expect("escrever");
    assert!(fs_browser::rename_photo(&moved, "ocupado.png").is_err());
}

#[test]
fn the_extension_gate_is_what_blocks_non_images() {
    // `rename_photo` só valida o caminho; quem barra extensão inválida é a
    // camada de app, via `PhotoPath`. Este é o portão que ela usa.
    assert!(PhotoPath::new(PathBuf::from("/x/novo.png")).is_some());
    assert!(PhotoPath::new(PathBuf::from("/x/ruim.txt")).is_none());
}

#[test]
fn photo_path_rejects_unsupported_extensions() {
    assert!(PhotoPath::new(PathBuf::from("/x/foto.png")).is_some());
    assert!(PhotoPath::new(PathBuf::from("/x/foto.JPEG")).is_some());
    assert!(PhotoPath::new(PathBuf::from("/x/foto.gif")).is_some());
    assert!(PhotoPath::new(PathBuf::from("/x/foto.pdf")).is_none());
    assert!(PhotoPath::new(PathBuf::from("/x/sem_ext")).is_none());
}
