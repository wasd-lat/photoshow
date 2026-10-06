//! Metadados EXIF: ler orientação, e **preservar** o resto ao salvar.
//!
//! Duas responsabilidades que costumam ficar no mesmo lugar porque dependem
//! do mesmo arquivo:
//!
//! - [`read_orientation`] / [`apply_orientation`]: a foto de celular precisa
//!   aparecer de pé.
//! - [`embed`]: ao salvar, câmera, data, exposição e GPS **não podem sumir**.
//!   O `bake()` do editor reconstrói a imagem do zero, então sem reescrever
//!   os metadados o "Salvar" apaga tudo que a câmera gravou.
//!
//! ## Por que a Orientation vira 1
//!
//! Os pixels salvos já estão **fisicamente** orientados (`decode_photo`
//! aplicou a correção). Se o `Orientation=6` fosse copiado, o próximo
//! visualizador giraria a foto de novo — e ela apareceria torta. A tag é
//! normalizada para identidade, que é a única leitura consistente: pixels
//! orientados + `Orientation=1`.
//!
//! ## Dimensões acompanham o recorte
//!
//! As tags `PixelXDimension`, `PixelYDimension`, `ImageWidth` e `ImageLength`
//! precisam refletir o tamanho da imagem resultante, e não o original.
//!
//! ## Formatos
//!
//! O EXIF é reescrito para JPEG e WebP. TIFF é o próprio container EXIF e
//! exigiria reescrever os IFDs (o `Writer` deste crate não faz isso), então é
//! explicitamente **fora** — PNG/BMP/GIF também não têm onde guardar.
//! Metadado perdido é melhor que arquivo corrompido: `embed` só age onde sabe
//! agir.

use std::io::BufReader;
use std::path::Path;

use exif::experimental::Writer as ExifWriter;

/// Identificador do segmento APP1 do EXIF (`"Exif\0\0"`).
const EXIF_ID: [u8; 6] = [0x45, 0x78, 0x69, 0x66, 0x00, 0x00];
/// Marcador APP1 do JPEG.
const JPEG_APP1: u8 = 0xe1;

/// Lê o Orientation (1..=8); qualquer falha retorna 1 (identidade).
#[must_use]
pub fn read_orientation(path: &Path) -> u16 {
    let Ok(file) = std::fs::File::open(path) else {
        return 1;
    };
    let mut reader = BufReader::new(file);
    let Ok(exif) = exif::Reader::new().read_from_container(&mut reader) else {
        return 1;
    };
    match exif
        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
    {
        Some(v @ 1..=8) => v as u16,
        _ => 1,
    }
}

/// Aplica a correção de orientação, retornando a imagem exibível.
#[must_use]
pub fn apply_orientation(img: image::DynamicImage, orientation: u16) -> image::DynamicImage {
    match orientation {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        7 => img.rotate90().flipv(),
        8 => img.rotate270(),
        _ => img,
    }
}

/// Metadados EXIF a preservar, prontos para a nova imagem.
#[derive(Debug, Clone, Default)]
pub struct ExifMeta {
    fields: Vec<exif::Field>,
}

impl ExifMeta {
    /// Lê os metadados de `source`.
    ///
    /// `None` quando o arquivo não tem EXIF (PNG, BMP, arquivo sem câmera).
    #[must_use]
    pub fn read_from(source: &Path) -> Option<Self> {
        let file = std::fs::File::open(source).ok()?;
        let mut reader = BufReader::new(file);
        let src = exif::Reader::new().read_from_container(&mut reader).ok()?;
        let fields: Vec<exif::Field> = src.fields().cloned().collect();
        if fields.is_empty() {
            return None;
        }
        Some(Self { fields })
    }

    /// Reescreve os campos para os pixels resultantes de `width`x`height`.
    ///
    /// - `Orientation` vira 1 (identidade).
    /// - Dimensões atualizadas se existirem no cabeçalho.
    /// - Remove campos que apontavam para posições inválidas no arquivo antigo.
    #[must_use]
    pub fn rebased(mut self, width: u32, height: u32) -> Self {
        for f in &mut self.fields {
            match f.tag {
                exif::Tag::Orientation => {
                    f.value = exif::Value::Short(vec![1]);
                }
                exif::Tag::PixelXDimension | exif::Tag::ImageWidth => {
                    f.value = u32_to_value(width);
                }
                exif::Tag::PixelYDimension | exif::Tag::ImageLength => {
                    f.value = u32_to_value(height);
                }
                _ => {}
            }
        }

        // Descarta tags de offsets internos/tiras que dependiam do arquivo original.
        self.fields.retain(|f| !is_offset_or_strip(f.tag));

        // Assegura que Orientation=1 esteja presente
        if !self.fields.iter().any(|f| f.tag == exif::Tag::Orientation) {
            self.fields.push(exif::Field {
                tag: exif::Tag::Orientation,
                ifd_num: exif::In::PRIMARY,
                value: exif::Value::Short(vec![1]),
            });
        }
        self
    }

    /// Serializa os campos em um bloco TIFF (conteúdo para container).
    #[must_use]
    pub fn to_tiff_block(&self) -> Option<Vec<u8>> {
        if self.fields.is_empty() {
            return None;
        }
        let mut writer = ExifWriter::new();
        for f in &self.fields {
            writer.push_field(f);
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        writer.write(&mut buf, true).ok()?;
        Some(buf.into_inner())
    }
}

fn u32_to_value(v: u32) -> exif::Value {
    if v <= u32::from(u16::MAX) {
        exif::Value::Short(vec![v as u16])
    } else {
        exif::Value::Long(vec![v])
    }
}

fn is_offset_or_strip(tag: exif::Tag) -> bool {
    use exif::Tag;
    matches!(
        tag,
        Tag::StripOffsets
            | Tag::StripByteCounts
            | Tag::RowsPerStrip
            | Tag::TileOffsets
            | Tag::TileByteCounts
            | Tag::JPEGInterchangeFormat
            | Tag::JPEGInterchangeFormatLength
    )
}

/// Embute o bloco TIFF EXIF no arquivo `target`, escrevendo por cima dele.
///
/// `target` tem que ser um arquivo **ainda não publicado** (o temporário do
/// save atômico), não a foto do usuário: a escrita é direta e uma falha aqui
/// deixaria o alvo truncado. Devolve `false` se o container não comportar
/// EXIF, se o arquivo não for reconhecido ou se a escrita falhar.
pub fn embed(target: &Path, kind_hint: &Path, block: &[u8]) -> bool {
    let Ok(data) = std::fs::read(target) else {
        return false;
    };
    let patched = match kind_of(kind_hint) {
        Kind::Jpeg => inject_jpeg(&data, block),
        Kind::WebP => inject_webp(&data, block),
        Kind::Unsupported => return false,
    };
    match patched {
        Some(next) if next.len() > data.len() => std::fs::write(target, next).is_ok(),
        _ => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Jpeg,
    WebP,
    Unsupported,
}

fn kind_of(path: &Path) -> Kind {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jpg" | "jpeg") => Kind::Jpeg,
        Some("webp") => Kind::WebP,
        _ => Kind::Unsupported,
    }
}

fn inject_jpeg(data: &[u8], block: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 2 || data[0] != 0xff || data[1] != 0xd8 {
        return None;
    }
    let payload_len = EXIF_ID.len().checked_add(block.len())?;
    if payload_len + 2 > usize::from(u16::MAX) {
        return None;
    }
    let mut out = Vec::with_capacity(data.len() + payload_len + 4);
    out.extend_from_slice(&data[..2]);
    out.push(0xff);
    out.push(JPEG_APP1);
    out.extend_from_slice(&((payload_len + 2) as u16).to_be_bytes());
    out.extend_from_slice(&EXIF_ID);
    out.extend_from_slice(block);
    out.extend_from_slice(&data[2..]);
    Some(out)
}

fn inject_webp(data: &[u8], block: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"WEBP" {
        return None;
    }
    let old_size = u32::from_le_bytes(data[4..8].try_into().ok()?) as usize;
    if old_size + 8 > data.len() {
        return None;
    }
    let pad = block.len() % 4;
    let chunk_size = block.len() + pad;

    let mut out = Vec::with_capacity(data.len() + 8 + chunk_size);
    out.extend_from_slice(&data[..4]);
    out.extend_from_slice(&((old_size + 8 + chunk_size) as u32).to_le_bytes());
    out.extend_from_slice(&data[8..12]);
    out.extend_from_slice(b"EXIF");
    out.extend_from_slice(&(block.len() as u32).to_le_bytes());
    out.extend_from_slice(block);
    out.extend(std::iter::repeat_n(0u8, pad));
    out.extend_from_slice(&data[12..]);
    Some(out)
}

/// Metadados fotográficos extraídos para o painel de informações (0.2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExifDetails {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub date_time: Option<String>,
    pub exposure_time: Option<String>,
    pub f_number: Option<String>,
    pub iso: Option<String>,
    pub focal_length: Option<String>,
    pub dimensions: Option<(u32, u32)>,
}

impl ExifDetails {
    /// Extrai os metadados fotográficos do arquivo.
    #[must_use]
    pub fn read_from(path: &Path) -> Option<Self> {
        let file = std::fs::File::open(path).ok()?;
        let mut reader = BufReader::new(file);
        let src = exif::Reader::new().read_from_container(&mut reader).ok()?;

        let get_str = |tag: exif::Tag| {
            src.get_field(tag, exif::In::PRIMARY)
                .map(|f| f.display_value().to_string().trim().to_owned())
                .filter(|s| !s.is_empty())
        };

        let camera_make = get_str(exif::Tag::Make);
        let camera_model = get_str(exif::Tag::Model);
        let lens_model = get_str(exif::Tag::LensModel);
        let date_time =
            get_str(exif::Tag::DateTimeOriginal).or_else(|| get_str(exif::Tag::DateTime));
        let exposure_time = get_str(exif::Tag::ExposureTime);
        let f_number = get_str(exif::Tag::FNumber);
        let iso =
            get_str(exif::Tag::PhotographicSensitivity).or_else(|| get_str(exif::Tag::ISOSpeed));
        let focal_length = get_str(exif::Tag::FocalLength);

        let w = src
            .get_field(exif::Tag::PixelXDimension, exif::In::PRIMARY)
            .or_else(|| src.get_field(exif::Tag::ImageWidth, exif::In::PRIMARY))
            .and_then(|f| f.value.get_uint(0));
        let h = src
            .get_field(exif::Tag::PixelYDimension, exif::In::PRIMARY)
            .or_else(|| src.get_field(exif::Tag::ImageLength, exif::In::PRIMARY))
            .and_then(|f| f.value.get_uint(0));
        let dimensions = match (w, h) {
            (Some(w), Some(h)) if w > 0 && h > 0 => Some((w, h)),
            _ => None,
        };

        Some(Self {
            camera_make,
            camera_model,
            lens_model,
            date_time,
            exposure_time,
            f_number,
            iso,
            focal_length,
            dimensions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, Rgb, RgbImage};

    fn labeled() -> DynamicImage {
        let img = RgbImage::from_fn(3, 2, |x, y| Rgb([(y * 3 + x + 1) as u8, 0, 0]));
        DynamicImage::ImageRgb8(img)
    }

    fn grid(img: &DynamicImage) -> (u32, u32, Vec<u8>) {
        let rgb = img.to_rgb8();
        let px = rgb.pixels().map(|p| p[0]).collect();
        (rgb.width(), rgb.height(), px)
    }

    fn jpeg_with_fields(path: &Path, fields: Vec<exif::Field>) {
        labeled().save(path).expect("salvar jpeg de teste");
        let data = std::fs::read(path).expect("ler");
        let mut writer = ExifWriter::new();
        for f in &fields {
            writer.push_field(f);
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        writer.write(&mut buf, true).expect("serializar exif");
        let patched = inject_jpeg(&data, &buf.into_inner()).expect("injetar app1");
        std::fs::write(path, patched).expect("gravar");
    }

    fn jpeg_with_orientation(path: &Path, orientation: u16) {
        jpeg_with_fields(
            path,
            vec![exif::Field {
                tag: exif::Tag::Orientation,
                ifd_num: exif::In::PRIMARY,
                value: exif::Value::Short(vec![orientation]),
            }],
        );
    }

    #[test]
    fn orientations_map_correctly() {
        let cases: &[(u16, u32, u32, &[u8])] = &[
            (1, 3, 2, &[1, 2, 3, 4, 5, 6]),
            (2, 3, 2, &[3, 2, 1, 6, 5, 4]),
            (3, 3, 2, &[6, 5, 4, 3, 2, 1]),
            (4, 3, 2, &[4, 5, 6, 1, 2, 3]),
            (5, 2, 3, &[1, 4, 2, 5, 3, 6]),
            (6, 2, 3, &[4, 1, 5, 2, 6, 3]),
            (7, 2, 3, &[6, 3, 5, 2, 4, 1]),
            (8, 2, 3, &[3, 6, 2, 5, 1, 4]),
            (0, 3, 2, &[1, 2, 3, 4, 5, 6]),
            (9, 3, 2, &[1, 2, 3, 4, 5, 6]),
        ];
        for (ori, w, h, labels) in cases {
            let (gw, gh, px) = grid(&apply_orientation(labeled(), *ori));
            assert_eq!((gw, gh), (*w, *h), "dims ori={ori}");
            assert_eq!(&px, labels, "pixels ori={ori}");
        }
    }

    #[test]
    fn missing_file_defaults_to_identity() {
        assert_eq!(read_orientation(Path::new("/nao/existe.jpg")), 1);
    }

    #[test]
    fn orientation_is_read_back_from_a_real_jpeg() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("ori.jpg");
        jpeg_with_orientation(&path, 6);
        assert_eq!(read_orientation(&path), 6);
    }

    #[test]
    fn rebased_metadata_normalizes_orientation_to_identity() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("ori6.jpg");
        jpeg_with_orientation(&path, 6);

        let meta = ExifMeta::read_from(&path).expect("leu exif").rebased(3, 2);
        let ori = meta
            .fields
            .iter()
            .find(|f| f.tag == exif::Tag::Orientation)
            .and_then(|f| f.value.get_uint(0))
            .unwrap_or(0);
        assert_eq!(ori, 1);
    }

    #[test]
    fn embed_writes_exif_into_a_fresh_jpeg() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = dir.path().join("fonte.jpg");
        jpeg_with_orientation(&source, 6);
        let dest = dir.path().join("saida.jpg");
        labeled().save(&dest).expect("salvar destino");

        let meta = ExifMeta::read_from(&source)
            .expect("leu exif")
            .rebased(3, 2);
        let block = meta.to_tiff_block().expect("bloco");
        assert!(embed(&dest, &dest, &block));
        assert_eq!(read_orientation(&dest), 1);
    }

    #[test]
    fn embed_never_corrupts_the_pixels() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = dir.path().join("fonte.jpg");
        jpeg_with_orientation(&source, 6);
        let dest = dir.path().join("saida.jpg");
        labeled().save(&dest).expect("salvar destino");

        let meta = ExifMeta::read_from(&source)
            .expect("leu exif")
            .rebased(3, 2);
        let block = meta.to_tiff_block().expect("bloco");
        embed(&dest, &dest, &block);

        let reread = image::open(&dest).expect("destino válido");
        assert_eq!((reread.width(), reread.height()), (3, 2));
    }

    #[test]
    fn embed_is_skipped_where_there_is_no_place_for_exif() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join("saida.png");
        labeled().save(&dest).expect("salvar");
        let before = std::fs::read(&dest).expect("ler");

        assert!(!embed(&dest, &dest, &[0xaa]));
        assert_eq!(std::fs::read(&dest).expect("ler"), before);
    }

    #[test]
    fn embed_refuses_a_file_it_does_not_recognize() {
        // Lixo com extensão de JPEG: recusar é a única resposta segura, senão
        // o arquivo viraria bytes incompreensíveis com EXIF no meio.
        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join("lixo.jpg");
        std::fs::write(&dest, b"isto nao e um jpeg").expect("gravar");
        let before = std::fs::read(&dest).expect("ler");

        assert!(!embed(&dest, &dest, &[0xaa, 0xbb]));
        assert_eq!(std::fs::read(&dest).expect("ler"), before);
    }
}
