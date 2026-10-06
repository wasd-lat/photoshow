//! Edição não-destrutiva: estado normalizado + histórico para undo.
//!
//! Modelo: `rot` (quartos de volta CW acumulados, 0..=3) + um único `crop`
//! definido no espaço de pixels da imagem *já rotacionada* (display).
//! Rotacionar com crop ativo transforma o rect junto — o bake aplica
//! rotate e depois crop na full-res com o mesmo fator de escala.
//!
//! ## Gravação é transacional
//!
//! [`save_baked`] grava num temporário ao lado do destino, faz `fsync` e só
//! então troca o arquivo por `rename`. Um "Salvar" sobre o original passa a
//! ser tudo-ou-nada: falha de disco, queda de energia ou `kill -9` no meio da
//! gravação deixam o original intacto. Gravar direto no destino (o jeito
//! anterior) deixava um JPEG truncado no lugar da foto.

/// Recorte em pixels da imagem rotacionada (display ou full escalado).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CropRect {
    /// Canto superior-esquerdo + tamanho.
    pub x: u32,
    /// Canto superior-esquerdo + tamanho.
    pub y: u32,
    /// Largura (>0).
    pub w: u32,
    /// Altura (>0).
    pub h: u32,
}

/// Estado normalizado do editor.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EditorState {
    /// Quartos de volta CW (0..=3).
    pub rot: u8,
    /// Recorte opcional no espaço rotacionado.
    pub crop: Option<CropRect>,
    /// Ajustes globais de cor (exposição, contraste, saturação, temperatura).
    pub adjust: crate::adjust::Adjust,
}

impl EditorState {
    /// Estado limpo (sem edições).
    #[must_use]
    pub fn clean() -> Self {
        Self::default()
    }

    /// Sem edições?
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.rot == 0 && self.crop.is_none() && self.adjust.is_clean()
    }

    /// Dimensões após aplicar `rot` a uma base `(w, h)`.
    #[must_use]
    pub fn dims(&self, base: (u32, u32)) -> (u32, u32) {
        if self.rot % 2 == 1 {
            (base.1, base.0)
        } else {
            base
        }
    }
}

/// Pilha de edição com undo/redo (snapshots baratos: 3 campos).
#[derive(Debug, Clone)]
pub struct EditorStack {
    history: Vec<EditorState>,
    future: Vec<EditorState>,
}

/// `Default` manual: o derivado criaria `history` vazio, e a pilha sempre
/// precisa de pelo menos o estado limpo como base do undo.
impl Default for EditorStack {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorStack {
    /// Cria com estado limpo.
    #[must_use]
    pub fn new() -> Self {
        Self {
            history: vec![EditorState::clean()],
            future: Vec::new(),
        }
    }

    /// Estado atual.
    ///
    /// Usa `last()` com fallback em vez de `expect`: o `Default` derivado
    /// criava `history` **vazio**, e qualquer `EditorStack::default()`
    /// entrava em pânico aqui. `Default` manual (abaixo) já garante o
    /// estado limpo, mas o fallback mantém a função total.
    #[must_use]
    pub fn state(&self) -> EditorState {
        self.history.last().copied().unwrap_or_default()
    }

    /// Dois estados são o mesmo, tolerando ruído de `f32` nos ajustes.
    fn same(a: &EditorState, b: &EditorState) -> bool {
        a.rot == b.rot && a.crop == b.crop && a.adjust.eq_approx(&b.adjust)
    }

    /// Há edições pendentes?
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        !self.state().is_clean()
    }

    /// Há algo para refazer?
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Há algo para desfazer?
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.history.len() > 1
    }

    fn commit(&mut self, state: EditorState) {
        // Normaliza rotações completas para não acumular histórico inútil.
        let mut state = state;
        state.rot %= 4;
        if Self::same(&state, &self.state()) {
            return;
        }
        self.history.push(state);
        self.future.clear();
    }

    fn commit_adjust(&mut self, adjust: crate::adjust::Adjust) {
        let cur = self.state();
        let next = EditorState {
            adjust: adjust.clamped(),
            ..cur
        };
        if Self::same(&next, &cur) {
            return;
        }
        let mergeable = self.history.len() > 1
            && self.history[self.history.len() - 2].rot == cur.rot
            && self.history[self.history.len() - 2].crop == cur.crop;
        if mergeable {
            *self.history.last_mut().expect("len > 1") = next;
        } else {
            self.history.push(next);
        }
        self.future.clear();
    }

    pub fn tweak(&mut self, f: impl FnOnce(&mut crate::adjust::Adjust)) {
        let mut adjust = self.state().adjust;
        f(&mut adjust);
        self.commit_adjust(adjust);
    }

    pub fn reset_adjust(&mut self) {
        self.commit_adjust(crate::adjust::Adjust::default());
    }

    /// Gira 90° CW; transforma o crop existente junto.
    /// `base` = dimensões da imagem de display pré-rotação.
    pub fn rotate_cw(&mut self, base: (u32, u32)) {
        let cur = self.state();
        let dims = cur.dims(base);
        let crop = cur.crop.map(|r| rot_rect_cw(dims.0, dims.1, r));
        self.commit(EditorState {
            rot: cur.rot + 1,
            crop,
            ..cur
        });
    }

    /// Gira 90° CCW; transforma o crop existente junto.
    /// `base` = dimensões da imagem de display pré-rotação.
    pub fn rotate_ccw(&mut self, base: (u32, u32)) {
        let cur = self.state();
        let dims = cur.dims(base);
        let crop = cur.crop.map(|r| rot_rect_ccw(dims.0, dims.1, r));
        self.commit(EditorState {
            rot: cur.rot + 3,
            crop,
            ..cur
        });
    }

    /// Define/substitui o recorte (espaço rotacionado de display).
    pub fn set_crop(&mut self, crop: Option<CropRect>) {
        let cur = self.state();
        self.commit(EditorState { crop, ..cur });
    }

    /// Desfaz a última operação; `false` se já está limpo.
    pub fn undo(&mut self) -> bool {
        if self.history.len() > 1 {
            let cur = self.history.pop().expect("len > 1");
            self.future.push(cur);
            true
        } else {
            false
        }
    }

    /// Refaz a última operação desfeita; `false` se não há nada.
    pub fn redo(&mut self) -> bool {
        if let Some(st) = self.future.pop() {
            self.history.push(st);
            true
        } else {
            false
        }
    }

    /// Descarta tudo (Reset).
    pub fn clear(&mut self) {
        self.history.truncate(1);
        self.future.clear();
    }
}

/// Rotaciona um rect 90° CW dentro de uma imagem de altura `h`.
fn rot_rect_cw(_w: u32, h: u32, r: CropRect) -> CropRect {
    CropRect {
        x: h.saturating_sub(r.y + r.h),
        y: r.x,
        w: r.h,
        h: r.w,
    }
}

/// Rotaciona um rect 90° CCW dentro de uma imagem de largura `w`.
fn rot_rect_ccw(w: u32, _h: u32, r: CropRect) -> CropRect {
    CropRect {
        x: r.y,
        y: w.saturating_sub(r.x + r.w),
        w: r.h,
        h: r.w,
    }
}

/// Aplica estado a uma imagem (preview em display ou bake parcial).
#[must_use]
pub fn apply_to_image(img: &image::DynamicImage, st: &EditorState) -> image::DynamicImage {
    let mut out = match st.rot % 4 {
        1 => img.rotate90(),
        2 => img.rotate180(),
        3 => img.rotate270(),
        _ => img.clone(),
    };
    if let Some(c) = st.crop {
        let (w, h) = (out.width(), out.height());
        let x = c.x.min(w.saturating_sub(1));
        let y = c.y.min(h.saturating_sub(1));
        let cw = c.w.min(w.saturating_sub(x)).max(1);
        let ch = c.h.min(h.saturating_sub(y)).max(1);
        out = image::DynamicImage::ImageRgba8(
            image::imageops::crop_imm(&out, x, y, cw, ch).to_image(),
        );
    }
    if !st.adjust.is_clean() {
        let (w, h) = (out.width(), out.height());
        let mut raw = out.to_rgba8().into_raw();
        crate::adjust::adjust_pixels(&mut raw, &st.adjust);
        out = image::DynamicImage::ImageRgba8(
            image::RgbaImage::from_raw(w, h, raw).unwrap_or_else(|| out.to_rgba8()),
        );
    }
    out
}

/// Bake final: aplica `rot` na full-res e o crop escalado do display.
/// `display_base` = dimensões da imagem de display pré-rotação.
#[must_use]
pub fn bake(
    full: &image::DynamicImage,
    display_base: (u32, u32),
    st: &EditorState,
) -> image::DynamicImage {
    let rotated = apply_to_image(
        full,
        &EditorState {
            crop: None,
            // A cor entra **depois** do crop: os dois axes são independentes
            // (geometria x cor), e ajustar depois economiza o passe de cor
            // sobre os pixels que o crop jogou fora.
            adjust: Default::default(),
            ..*st
        },
    );
    let cropped = match st.crop {
        None => rotated,
        Some(c) => {
            // Display e full preservam aspecto: um fator serve para os dois eixos.
            let disp_dims = st.dims(display_base);
            let rw = rotated.width() as f64 / disp_dims.0.max(1) as f64;
            let x = (c.x as f64 * rw).round() as u32;
            let y = (c.y as f64 * rw).round() as u32;
            let w = (c.w as f64 * rw).round() as u32;
            let h = (c.h as f64 * rw).round() as u32;
            let (fw, fh) = (rotated.width(), rotated.height());
            let x = x.min(fw.saturating_sub(1));
            let y = y.min(fh.saturating_sub(1));
            let w = w.min(fw.saturating_sub(x)).max(1);
            let h = h.min(fh.saturating_sub(y)).max(1);
            image::DynamicImage::ImageRgba8(
                image::imageops::crop_imm(&rotated, x, y, w, h).to_image(),
            )
        }
    };
    if st.adjust.is_clean() {
        return cropped;
    }
    let (cw, ch) = (cropped.width(), cropped.height());
    let mut raw = cropped.to_rgba8().into_raw();
    crate::adjust::adjust_pixels(&mut raw, &st.adjust);
    image::DynamicImage::ImageRgba8(
        image::RgbaImage::from_raw(cw, ch, raw).unwrap_or_else(|| cropped.to_rgba8()),
    )
}

/// Grava a imagem assada de forma **atômica**, respeitando a qualidade JPEG.
///
/// O arquivo só troca no lugar depois de estar completo no disco: o encoder
/// escreve num temporário vizinho, `fsync` garante que ele chegou ao
/// dispositivo, e o `rename` final é uma operação atômica no POSIX. Qualquer
/// falha antes do rename deixa o destino como estava — que é o ponto todo
/// quando o destino é a foto do usuário.
///
/// `source` fornece os metadados EXIF a preservar (câmera, data, exposição):
/// sem ele o save apaga tudo isso. `None` salva sem metadado. Em "Sobrescrever
/// original" `source` **é** o `dest`, então o EXIF é lido *antes* do rename —
/// depois já não sobraria nada para ler.
///
/// O EXIF entra no temporário, não no destino: assim imagem e metadado
/// chegam ao disco na mesma operação atômica, e não existe um instante em que
/// a foto está salva sem os metadados dela.
pub fn save_baked(
    img: &image::DynamicImage,
    dest: &std::path::Path,
    jpeg_quality: u8,
    source: Option<&std::path::Path>,
) -> Result<(), String> {
    // Antes de qualquer escrita: no caminho de sobrescrita o `source` é o
    // próprio `dest`, que o rename abaixo sobrescreve.
    let block = source
        .and_then(crate::exif::ExifMeta::read_from)
        .map(|m| m.rebased(img.width(), img.height()))
        .and_then(|m| m.to_tiff_block());

    // Um único temporário do início ao fim: gerar o nome duas vezes daria
    // dois caminhos diferentes e o rename não acharia o arquivo.
    let tmp = temp_path(dest);
    encode_to_temp(img, dest, &tmp, jpeg_quality)?;

    if let Some(block) = block {
        // No temporário, antes do rename: assim o EXIF entra na mesma
        // operação atômica da imagem. Falhar aqui não é fatal — o
        // temporário é descartado e regravado sem metadado, porque perder a
        // câmera é ruim, mas falhar o save seria pior e mentiria ao usuário.
        if !crate::exif::embed(&tmp, dest, &block) {
            encode_to_temp(img, dest, &tmp, jpeg_quality)?;
        }
    }

    // Só o rename é irreversível, então é o último passo.
    commit_temp(dest, &tmp)
}

/// Codifica a imagem no temporário `tmp`.
fn encode_to_temp(
    img: &image::DynamicImage,
    dest: &std::path::Path,
    tmp: &std::path::Path,
    jpeg_quality: u8,
) -> Result<(), String> {
    let is_jpeg = dest
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("jpg") || e.eq_ignore_ascii_case("jpeg"))
        .unwrap_or(false);
    let write = if is_jpeg {
        use std::io::Write as _;
        let file = std::fs::File::create(tmp).map_err(|e| e.to_string())?;
        let mut buf = std::io::BufWriter::new(file);
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut buf,
            jpeg_quality.clamp(1, 100),
        );
        enc.encode_image(img)
            .map_err(|e| e.to_string())
            .and_then(|()| buf.flush().map_err(|e| e.to_string()))
    } else {
        // O `save` do crate decide o formato pela extensão, então o
        // temporário precisa manter a do destino.
        img.save(tmp).map_err(|e| e.to_string())
    };
    if let Err(e) = write {
        // O temporário não pode ficar para trás: ele apareceria na própria
        // lista de fotos do app.
        let _ = std::fs::remove_file(tmp);
        return Err(e);
    }
    fsync_file(tmp)
}

/// Garante que o conteúdo do temporário chegou ao disco, não ao buffer do SO.
fn fsync_file(path: &std::path::Path) -> Result<(), String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}

/// Troca o temporário pelo destino (o passo irreversível).
fn commit_temp(dest: &std::path::Path, tmp: &std::path::Path) -> Result<(), String> {
    match std::fs::rename(tmp, dest) {
        Ok(()) => sync_parent(dest),
        Err(e) => {
            let _ = std::fs::remove_file(tmp);
            Err(e.to_string())
        }
    }
}

/// Sincroniza o diretório: sem isso o rename pode se perder num crash.
///
/// ponytail: alguns sistemas de arquivos recusam `fsync` de diretório. Aí a
/// garantia é só do conteúdo do arquivo. Subir para `syncfs` quando for
/// necessário é problema futuro.
#[cfg(unix)]
fn sync_parent(dest: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = dest.parent()
        && let Ok(dir) = std::fs::File::open(parent)
    {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// No Windows o diretório não pode ser aberto para `fsync`; o `rename` já é
/// atômico pela própria API do SO.
#[cfg(not(unix))]
fn sync_parent(_dest: &std::path::Path) -> Result<(), String> {
    Ok(())
}

/// Caminho temporário vizinho do destino.
///
/// Precisa ser **no mesmo diretório** para o `rename` ser atômico: um
/// temporário em `/tmp` e um destino em `~/Imagens` estariam em sistemas de
/// arquivos diferentes, e o rename viraria cópia+remove — um "quase atômico"
/// que não protege nada.
///
/// O nome começa com ponto e termina em `.tmp`: se o processo morrer antes da
/// limpeza, o arquivo não aparece na varredura de fotos. A extensão original
/// é mantida no meio porque `DynamicImage::save` escolhe o encoder por ela —
/// sem isso um PNG cairia num temporário `.tmp` sem encoder conhecido.
fn temp_path(dest: &std::path::Path) -> std::path::PathBuf {
    let stem = dest
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| String::from("photoshow"));
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let ext = dest.extension().and_then(|e| e.to_str());
    let name = match ext {
        Some(ext) => format!(".{stem}.photoshow-{unique}.{ext}"),
        None => format!(".{stem}.photoshow-{unique}"),
    };
    dest.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_moves_crop_with_it() {
        let mut ed = EditorStack::new();
        ed.set_crop(Some(CropRect {
            x: 10,
            y: 20,
            w: 30,
            h: 40,
        }));
        ed.rotate_cw((100, 80));
        // dims atuais (100,80) -> CW: x' = 80-20-40 = 20, y' = 10
        assert_eq!(
            ed.state().crop,
            Some(CropRect {
                x: 20,
                y: 10,
                w: 40,
                h: 30
            })
        );
        assert_eq!(ed.state().rot, 1);
        ed.rotate_ccw((100, 80));
        assert_eq!(ed.state().rot, 0);
        assert_eq!(
            ed.state().crop,
            Some(CropRect {
                x: 10,
                y: 20,
                w: 30,
                h: 40
            })
        );
    }

    #[test]
    fn full_turn_is_noop() {
        let mut ed = EditorStack::new();
        for _ in 0..4 {
            ed.rotate_cw((50, 50));
        }
        assert!(ed.state().is_clean());
        assert!(!ed.is_dirty());
    }

    #[test]
    fn undo_and_clear() {
        let mut ed = EditorStack::new();
        ed.rotate_cw((50, 50));
        assert!(ed.is_dirty());
        assert!(ed.can_undo());
        assert!(ed.undo());
        assert!(!ed.is_dirty());
        assert!(!ed.undo());
        ed.rotate_cw((50, 50));
        ed.clear();
        assert!(!ed.is_dirty());
    }

    #[test]
    fn redo_reapplies_undone_and_new_op_clears_it() {
        let mut ed = EditorStack::new();
        ed.rotate_cw((50, 50));
        assert!(ed.undo());
        assert!(ed.can_redo());
        assert!(ed.redo());
        assert_eq!(ed.state().rot, 1);
        assert!(!ed.redo());
        // Nova operação limpa o redo.
        assert!(ed.undo());
        ed.rotate_ccw((50, 50));
        assert!(!ed.can_redo());
        assert_eq!(ed.state().rot, 3);
    }

    #[test]
    fn bake_scales_crop_to_full_res() {
        use image::{DynamicImage, RgbImage};
        let full = DynamicImage::ImageRgb8(RgbImage::new(200, 160));
        let st = EditorState {
            rot: 1, // CW: full 200x160 -> 160x200
            crop: Some(CropRect {
                x: 10,
                y: 20,
                w: 40,
                h: 30,
            }),
            ..Default::default()
        };
        // display base 100x80 -> rotacionado 80x100; fator = 160/80 = 2
        let out = bake(&full, (100, 80), &st);
        assert_eq!((out.width(), out.height()), (80, 60));
    }

    #[test]
    fn apply_crop_only() {
        use image::{DynamicImage, RgbImage};
        let img = DynamicImage::ImageRgb8(RgbImage::new(100, 80));
        let st = EditorState {
            rot: 0,
            crop: Some(CropRect {
                x: 5,
                y: 5,
                w: 20,
                h: 10,
            }),
            ..Default::default()
        };
        let out = apply_to_image(&img, &st);
        assert_eq!((out.width(), out.height()), (20, 10));
    }

    #[test]
    fn baked_image_saves_in_all_mvp_formats() {
        use image::{DynamicImage, RgbImage};
        let dir = tempfile::tempdir().expect("tempdir");
        let full = DynamicImage::ImageRgb8(RgbImage::from_fn(64, 48, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        }));
        let st = EditorState {
            rot: 1,
            crop: Some(CropRect {
                x: 4,
                y: 4,
                w: 20,
                h: 12,
            }),
            ..Default::default()
        };
        let out = bake(&full, (64, 48), &st);
        for ext in ["jpg", "png", "bmp", "gif", "tiff", "webp"] {
            let path = dir.path().join(format!("out.{ext}"));
            out.save(&path).unwrap_or_else(|_| panic!("save {ext}"));
            assert!(path.exists());
        }
    }

    #[test]
    fn stack_clone_preserves_history() {
        let mut ed = EditorStack::new();
        ed.rotate_cw((10, 10));
        ed.set_crop(Some(CropRect {
            x: 1,
            y: 1,
            w: 2,
            h: 2,
        }));
        let copy = ed.clone();
        assert_eq!(copy.state(), ed.state());
        assert!(copy.can_undo());
        assert!(!copy.can_redo());
    }
}
