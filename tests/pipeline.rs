//! Teste de integração do pipeline completo, sem janela.
//!
//! Cobre o caminho que a UI exercita a cada interação: varrer pasta ->
//! decodificar -> aplicar edição -> assar -> gravar -> reler. É a mesma
//! sequência de `image_store`, `editor` e `services`, sem o Freya.

use std::path::{Path, PathBuf};

use ::exif;
use photoshow::{
    editor::{CropRect, EditorStack, apply_to_image, bake, save_baked},
    exif as app_exif,
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

/// Escreve uma imagem de teste em `path`, deixando o formato para a extensão.
fn write_image(path: &Path, w: u32, h: u32) {
    let img = image::RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
    });
    img.save(path).expect("gravar imagem de teste");
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
    save_baked(&baked, &path, 92, None).expect("salvar");

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
        save_baked(&baked, &out, 88, None).unwrap_or_else(|e| panic!("salvar {ext}: {e}"));
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
    assert_eq!(app_exif::read_orientation(&path), 1);
    let dec = decode_photo(&path).expect("decodificar");
    assert_eq!(dec.full_size, (40, 20));
}

#[test]
fn saving_over_the_original_keeps_the_camera_metadata() {
    // O bug do 0.1: `bake()` reconstrói a imagem do zero, então salvar
    // sobrescrevendo apagava câmera/data/exposição — metadado que o
    // photographer não tem como recuperar.
    let dir = fixture();
    let path = dir.path().join("camera.jpg");
    write_photo_with_exif(&path, 60, 40);
    assert_eq!(
        app_exif::read_orientation(&path),
        6,
        "fixture sem Orientation=6"
    );

    let dec = decode_photo(&path).expect("decodificar");
    let mut editor = EditorStack::new();
    editor.tweak(|a| a.exposure = 0.5);
    let baked = bake(
        &dec.full,
        (dec.display.width(), dec.display.height()),
        &editor.state(),
    );

    // `source` = o próprio destino: é assim que o "Sobrescrever" chama.
    save_baked(&baked, &path, 90, Some(&path)).expect("salvar");

    let meta = photoshow::exif::ExifMeta::read_from(&path).expect("EXIF preservado");
    assert!(
        meta.to_tiff_block().is_some(),
        "o arquivo salvo ficou sem nenhum EXIF"
    );
    assert_eq!(
        model_of(&path),
        Some(String::from("Photoshow TestCam")),
        "o modelo da câmera não sobreviveu ao save"
    );
    // Orientation vai a 1 porque os pixels salvos já estão orientados.
    assert_eq!(
        app_exif::read_orientation(&path),
        1,
        "orientation deveria ser 1"
    );
}

#[test]
fn the_saved_orientation_does_not_rotate_the_photo_twice() {
    // Fecha o cicloOrientation→pixels→save→reload: se a tag antiga voltasse
    // junto, quem reabrisse o arquivo veria a foto girada de novo.
    let dir = fixture();
    let path = dir.path().join("giro.jpg");
    write_photo_with_exif(&path, 60, 40);
    let dec = decode_photo(&path).expect("decodificar");
    // Orientation=6: o decode já entrega a imagem de pé (40x60).
    assert_eq!(dec.full_size, (40, 60));

    save_baked(&dec.full, &path, 90, Some(&path)).expect("salvar");
    let after = decode_photo(&path).expect("reabrir");
    assert_eq!(after.full_size, (40, 60), "a foto girou ao reabrir");
}

#[test]
fn a_failed_save_leaves_the_original_untouched() {
    // A garantia do save atômico: um destino que não pode ser escrito não pode
    // custar o conteúdo antigo. Gravar direto (o jeito anterior) truncaria.
    let dir = fixture();
    let path = dir.path().join("protegida.jpg");
    write_image(&path, 32, 16);
    let before = std::fs::read(&path).expect("ler original");

    // Diretório inexistente = destino impossível, sem tocar no original.
    let impossible = dir.path().join("inexistente").join("x.jpg");
    let result = save_baked(&decoded_sample(32, 16), &impossible, 90, None);
    assert!(result.is_err(), "deveria falhar");

    assert_eq!(
        std::fs::read(&path).expect("ler"),
        before,
        "o original foi alterado por um save que falhou"
    );
}

#[test]
fn saving_leaves_no_temporary_file_behind() {
    // O temporário precisa sumir: um `.foto.png.photoshow-...` na pasta
    // apareceria na varredura de fotos do próprio app.
    let dir = fixture();
    let path = dir.path().join("limpa.png");
    write_image(&path, 24, 18);

    save_baked(&decoded_sample(24, 18), &path, 90, None).expect("salvar");

    let leftovers: Vec<String> = std::fs::read_dir(dir.path())
        .expect("listar")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains("photoshow-") && n.ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "temporários órfãos: {leftovers:?}");
}

/// JPEG com EXIF de câmera, `Orientation=6` e dimensões declaradas.
fn write_photo_with_exif(path: &Path, w: u32, h: u32) {
    write_image(path, w, h);
    let fields = vec![
        exif::Field {
            tag: exif::Tag::Orientation,
            ifd_num: exif::In::PRIMARY,
            value: exif::Value::Short(vec![6]),
        },
        exif::Field {
            tag: exif::Tag::Model,
            ifd_num: exif::In::PRIMARY,
            value: exif::Value::Ascii(vec![b"Photoshow TestCam".to_vec()]),
        },
        exif::Field {
            tag: exif::Tag::PixelXDimension,
            ifd_num: exif::In::PRIMARY,
            value: exif::Value::Long(vec![w]),
        },
        exif::Field {
            tag: exif::Tag::PixelYDimension,
            ifd_num: exif::In::PRIMARY,
            value: exif::Value::Long(vec![h]),
        },
    ];
    let mut writer = exif::experimental::Writer::new();
    for f in &fields {
        writer.push_field(f);
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    writer.write(&mut buf, true).expect("serializar exif");
    let block = buf.into_inner();
    assert!(
        photoshow::exif::embed(path, path, &block),
        "não consegui montar o fixture com EXIF"
    );
}

/// Lê o modelo da câmera de um arquivo.
fn model_of(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let ex = exif::Reader::new().read_from_container(&mut reader).ok()?;
    let field = ex.get_field(exif::Tag::Model, exif::In::PRIMARY)?;
    let text = match &field.value {
        exif::Value::Ascii(vec) => vec
            .first()
            .and_then(|b| std::str::from_utf8(b).ok())
            .map(|s| s.trim_matches('\0').trim().to_owned()),
        _ => Some(field.display_value().to_string()),
    };
    let text = text?.trim_matches('"').trim().to_owned();
    (!text.is_empty()).then_some(text)
}

/// Imagem em memória de `w`x`h`, para testar gravação sem passar por decode.
fn decoded_sample(w: u32, h: u32) -> image::DynamicImage {
    image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 90])
    }))
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

#[test]
fn rotating_twice_returns_to_the_original_geometry() {
    // Regressão: `ImageStore` guardava só a imagem já transformada, então a
    // 2ª rotação derivava. Aqui, o round-trip de duas rotações tem que voltar
    // exatamente à imagem original — o que prova que a base não foi perdida.
    let dir = fixture();
    let path = write_photo(dir.path(), "giro.png", 400, 200);
    let dec = decode_photo(&path).expect("decodificar");
    let base = dec.display.clone();

    let mut editor = EditorStack::new();
    let dims = (base.width(), base.height());

    editor.rotate_cw(dims);
    let once = apply_to_image(&base, &editor.state());
    assert_eq!(
        (once.width(), once.height()),
        (dims.1, dims.0),
        "1ª rotação deveria trocar w/h"
    );

    editor.rotate_cw(dims);
    let twice = apply_to_image(&base, &editor.state());
    assert_eq!(
        (twice.width(), twice.height()),
        (dims.0, dims.1),
        "2ª rotação deveria voltar às dimensões originais"
    );
    assert_eq!(editor.state().rot, 2);
}

#[test]
fn four_rotations_are_pixel_identical_to_the_original() {
    // Quatro rotações de 90° são a identidade: byte a byte, não só em tamanho.
    let dir = fixture();
    let path = write_photo(dir.path(), "identidade.png", 64, 48);
    let dec = decode_photo(&path).expect("decodificar");
    let base = dec.display.clone();
    let dims = (base.width(), base.height());

    let mut editor = EditorStack::new();
    for _ in 0..4 {
        editor.rotate_cw(dims);
    }
    let back = apply_to_image(&base, &editor.state());
    assert_eq!(back, base, "4 rotações não voltaram ao original");
}

#[test]
fn rotating_twice_keeps_a_previously_set_crop() {
    // O crop gira junto com a foto. Se a rotação usasse as dims já
    // transformadas, a 2ª rotação mandaria o crop para fora da imagem.
    let dir = fixture();
    let path = write_photo(dir.path(), "crop_giro.png", 400, 200);
    let dec = decode_photo(&path).expect("decodificar");
    let dims = (dec.display.width(), dec.display.height());

    let mut editor = EditorStack::new();
    let crop = CropRect {
        x: 10,
        y: 20,
        w: 100,
        h: 50,
    };
    editor.set_crop(Some(crop));
    editor.rotate_cw(dims);
    editor.rotate_cw(dims);

    let final_crop = editor.state().crop.expect("crop preservado");
    // Após meia volta, o rect original (10,20,100,50) na imagem 400x200 volta
    // para a mesma posição relativa — o Bake tem de caber na imagem.
    let baked = bake(&dec.full, dims, &editor.state());
    assert!(
        (baked.width() > 0 && baked.height() > 0),
        "bake produziu imagem inválida: {final_crop:?}"
    );
    assert!(baked.width() <= dec.full.width());
    assert!(baked.height() <= dec.full.height());
}

#[test]
fn undo_after_rotation_restores_the_exact_original_pixels() {
    let dir = fixture();
    let path = write_photo(dir.path(), "undo_giro.png", 40, 20);
    let dec = decode_photo(&path).expect("decodificar");
    let base = dec.display.clone();

    let mut editor = EditorStack::new();
    editor.rotate_cw((base.width(), base.height()));
    assert_ne!(apply_to_image(&base, &editor.state()), base);

    assert!(editor.undo());
    assert_eq!(
        apply_to_image(&base, &editor.state()),
        base,
        "undo não devolveu a imagem original"
    );
}

#[test]
fn adjust_reaches_the_saved_file_not_only_the_preview() {
    // O ponto do 0.4: preview e save passam pelo MESMO código. Se o ajuste
    // aparecesse na tela e não no arquivo, o usuário perderia a edição ao
    // salvar — e o teste é o que garante que isso não volta.
    let dir = fixture();
    let path = write_photo(dir.path(), "ajuste.png", 80, 60);
    let dec = decode_photo(&path).expect("decodificar");
    let dims = (dec.display.width(), dec.display.height());

    let mut editor = EditorStack::new();
    editor.tweak(|a| a.exposure = 2.0);
    assert!(editor.state().adjust.exposure > 0.0);

    let baked = bake(&dec.full, dims, &editor.state());
    save_baked(&baked, &path, 92, None).expect("salvar");

    // Relendo do disco: os pixels têm de estar mais claros.
    let after = decode_photo(&path).expect("reler");
    let before_px = dec.full.to_rgb8().get_pixel(10, 10).0;
    let after_px = after.full.to_rgb8().get_pixel(10, 10).0;
    assert!(
        after_px[0] > before_px[0],
        "exposição não chegou ao arquivo: {before_px:?} -> {after_px:?}"
    );
}

#[test]
fn preview_and_bake_agree_on_the_geometry_after_an_adjust() {
    // Ajustes são por pixel, crop é por retângulo: aplicar o crop depois do
    // ajuste não pode trocar a geometria nem o tamanho.
    let dir = fixture();
    let path = write_photo(dir.path(), "combo.png", 200, 100);
    let dec = decode_photo(&path).expect("decodificar");
    let dims = (dec.display.width(), dec.display.height());

    let mut editor = EditorStack::new();
    editor.set_crop(Some(CropRect {
        x: 10,
        y: 10,
        w: 80,
        h: 50,
    }));
    editor.tweak(|a| {
        a.exposure = 1.0;
        a.saturation = 0.5;
    });

    let preview = apply_to_image(&dec.display, &editor.state());
    let baked = bake(&dec.full, dims, &editor.state());
    // Mesma proporção entre preview e arquivo, mesmo com o ajuste no meio.
    let pw = preview.width() as f64 / preview.height() as f64;
    let bw = baked.width() as f64 / baked.height() as f64;
    assert!(
        (pw - bw).abs() < 0.01,
        "preview {pw:.3} x bake {bw:.3}: geometrias divergentes"
    );
}

#[test]
fn undo_after_a_slider_drag_jumps_back_the_whole_drag() {
    // Regressão de UX: arrastar um slider dispara dezenas de eventos. Se cada
    // um virasse um passo de undo, o Ctrl+Z voltaria 1% por vez.
    let dir = fixture();
    let _ = dir;
    let mut editor = EditorStack::new();
    for i in 0..40 {
        editor.tweak(|a| a.exposure = i as f32 * 0.05);
    }
    assert!(editor.state().adjust.exposure > 0.0);

    // Um único undo volta ao neutro.
    assert!(editor.undo());
    assert!(editor.state().adjust.is_clean(),);
}

#[test]
fn undo_does_not_swallow_a_previous_rotation() {
    // A fusão do arrasto do slider só pode juntar ajustes **entre si**: se ela
    // juntasse com a rotação, o Ctrl+Z depois de um ajuste perderia o
    // enquadramento do usuário.
    let mut editor = EditorStack::new();
    editor.rotate_cw((100, 50));
    assert_eq!(editor.state().rot, 1);
    editor.tweak(|a| a.contrast = 0.5);

    assert!(editor.undo());
    assert_eq!(editor.state().rot, 1, "a rotação não pode sumir no undo");
    assert!(editor.state().adjust.is_clean());

    assert!(editor.undo());
    assert_eq!(editor.state().rot, 0, "o segundo undo desfaz a rotação");
}

#[test]
fn compare_base_keeps_the_geometry_and_drops_only_the_color() {
    // Regressão do comparador: se a metade "antes" fosse desenhada sem a
    // rotação ao lado de uma preview girada, as duas metades ficariam
    // desalinhadas e a comparação mentiria. Ela tem que preservar rot/crop e
    // zerar só a cor.
    let dir = fixture();
    let path = write_photo(dir.path(), "compara.png", 200, 100);
    let dec = decode_photo(&path).expect("decodificar");
    let base = dec.display.clone();

    let mut editor = EditorStack::new();
    editor.rotate_cw((base.width(), base.height()));
    editor.tweak(|a| a.exposure = 2.0);

    let preview = apply_to_image(&base, &editor.state());
    let compare = apply_to_image(
        &base,
        &photoshow::editor::EditorState {
            adjust: Default::default(),
            ..editor.state()
        },
    );

    assert_eq!(
        (compare.width(), compare.height()),
        (preview.width(), preview.height()),
        "as metades precisam ter a mesma geometria para o split fechar"
    );
    // E a comparação é realmente sobre a cor: os pixels diferem.
    assert_ne!(
        compare.to_rgba8().as_raw(),
        preview.to_rgba8().as_raw(),
        "o comparador precisa mostrar a diferença de cor"
    );
}

#[test]
fn zeroing_the_adjustments_keeps_the_crop_and_the_rotation() {
    // "Zerar ajustes" e "Reset" são intenções diferentes: o primeiro joga fora
    // a cor, não o enquadramento.
    let mut editor = EditorStack::new();
    editor.rotate_cw((100, 50));
    editor.set_crop(Some(CropRect {
        x: 5,
        y: 5,
        w: 20,
        h: 10,
    }));
    editor.tweak(|a| a.temperature = 0.8);

    editor.reset_adjust();
    let st = editor.state();
    assert!(st.adjust.is_clean());
    assert_eq!(st.rot, 1, "a rotação tem de sobreviver");
    assert_eq!(
        st.crop,
        Some(CropRect {
            x: 5,
            y: 5,
            w: 20,
            h: 10
        })
    );
    assert!(
        editor.is_dirty(),
        "ainda há edição: o crop e a rotação contam"
    );
}

#[test]
fn histogram_reports_clipping_caused_by_the_adjustment_itself() {
    // É para isso que o overlay existe: o usuário precisa ver que o +EV
    // estourou antes de salvar, não depois de perder o detalhe.
    // 200, não 250: 250 já é exatamente o limiar de clipping (`CLIP_HIGH = 250`),
    // então a imagem começaria marcada como estourada antes do ajuste.
    let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        32,
        32,
        image::Rgb([200, 200, 200]),
    ));
    let clean = photoshow::adjust::histogram(img.to_rgba8().as_raw(), 1);
    assert_eq!(clean.blowout, 0, "nada estourado antes do ajuste");

    let mut editor = EditorStack::new();
    editor.tweak(|a| a.exposure = 3.0);
    let boosted = apply_to_image(&img, &editor.state());
    let after = photoshow::adjust::histogram(boosted.to_rgba8().as_raw(), 1);
    assert!(
        after.blowout > 0,
        "o ajuste estourou e o histograma não avisou"
    );
}

#[test]
fn sidecar_stores_ratings_colors_and_tags() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut sidecar = photoshow::fs_browser::FolderSidecar::load_for_dir(dir.path());
    assert_eq!(sidecar.get_rating("foto1.jpg"), 0);
    assert_eq!(sidecar.get_color("foto1.jpg"), 0);
    assert!(!sidecar.has_tag("foto1.jpg", "férias"));

    sidecar.set_rating("foto1.jpg".to_string(), 5);
    sidecar.set_color("foto1.jpg".to_string(), 2);
    sidecar.add_tag("foto1.jpg".to_string(), "Férias".to_string());
    sidecar.add_tag("foto1.jpg".to_string(), "Praia".to_string());
    sidecar.save_for_dir(dir.path()).expect("save sidecar");

    let loaded = photoshow::fs_browser::FolderSidecar::load_for_dir(dir.path());
    assert_eq!(loaded.get_rating("foto1.jpg"), 5);
    assert_eq!(loaded.get_color("foto1.jpg"), 2);
    assert!(loaded.has_tag("foto1.jpg", "férias"));
    assert!(loaded.has_tag("foto1.jpg", "praia"));
    assert_eq!(loaded.get_tags("foto1.jpg"), &["férias", "praia"]);

    let mut modified = loaded;
    modified.remove_tag("foto1.jpg", "férias");
    assert!(!modified.has_tag("foto1.jpg", "férias"));
    assert!(modified.has_tag("foto1.jpg", "praia"));
}

#[test]
fn batch_processing_transforms_and_reports_files() {
    let dir = fixture();
    let dest_dir = dir.path().join("saida_lote");
    std::fs::create_dir_all(&dest_dir).expect("mkdir");

    let p1 = write_photo(dir.path(), "lote1.png", 200, 100);
    let p2 = write_photo(dir.path(), "lote2.jpg", 150, 150);

    let photos = vec![
        photoshow::fs_browser::PhotoPath::new(p1).expect("photo1"),
        photoshow::fs_browser::PhotoPath::new(p2).expect("photo2"),
    ];

    let cfg = photoshow::batch::BatchConfig {
        rotate_cw: 1, // 90°
        max_dim: Some(80),
        format: String::from("jpg"),
        jpeg_quality: 85,
        name_pattern: String::from("saida_{i}"),
        dest_dir: dest_dir.clone(),
    };

    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (tx, rx) = std::sync::mpsc::channel();
    photoshow::batch::run_batch(photos, cfg, cancel, move |prog| {
        let _ = tx.send(prog);
    });

    let mut last = photoshow::batch::BatchProgress::default();
    while let Ok(prog) = rx.recv() {
        let done = prog.finished;
        last = prog;
        if done {
            break;
        }
    }

    assert!(last.finished, "batch não finalizou");
    assert_eq!(last.successes, 2, "deveria ter processado 2 fotos");
    assert!(
        last.failures.is_empty(),
        "houve falhas: {:?}",
        last.failures
    );

    // Valida que os arquivos transformados foram criados
    let out1 = dest_dir.join("saida_001.jpg");
    let out2 = dest_dir.join("saida_002.jpg");
    assert!(out1.exists(), "saida_001.jpg ausente");
    assert!(out2.exists(), "saida_002.jpg ausente");

    let dec1 = photoshow::image_store::decode_photo(&out1).expect("decode out1");
    // Original 200x100 rotacionado 90° vira 100x200, reduzido com max 80 vira 40x80
    assert!(dec1.full_size.0 <= 80 && dec1.full_size.1 <= 80);
}
