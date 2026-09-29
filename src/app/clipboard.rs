//! Clipboard e integração com o gerenciador de arquivos do SO.

use std::path::Path;

/// Copia texto para o clipboard do SO.
pub fn copy_text(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .map_err(|e| format!("clipboard: {e}"))?
        .set_text(text.to_owned())
        .map_err(|e| format!("clipboard: {e}"))
}

/// Copia a imagem (RGBA) para o clipboard do SO.
pub fn copy_image(img: &image::DynamicImage) -> Result<(), String> {
    let rgba = img.to_rgba8();
    let data = arboard::ImageData {
        width: rgba.width() as usize,
        height: rgba.height() as usize,
        bytes: rgba.into_raw().into(),
    };
    arboard::Clipboard::new()
        .map_err(|e| format!("clipboard: {e}"))?
        .set_image(data)
        .map_err(|e| format!("clipboard: {e}"))
}

/// Revela o arquivo no gerenciador do SO (mais nativo possível).
pub fn reveal_in_folder(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .status()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        let ok = std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .status()
            .map_err(|e| e.to_string())?;
        return ok
            .success()
            .then_some(())
            .ok_or_else(|| String::from("open -R falhou"));
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let parent = path
            .parent()
            .ok_or_else(|| String::from("pasta inválida"))?;
        let ok = std::process::Command::new("xdg-open")
            .arg(parent)
            .status()
            .map_err(|e| e.to_string())?;
        ok.success()
            .then_some(())
            .ok_or_else(|| String::from("xdg-open falhou"))
    }
}
