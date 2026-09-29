//! photoshow: visualizador de fotos rápido em Freya.
//!
//! Organização: a raiz do app vive em [`app`]; o pipeline de imagem em
//! [`image_store`] e [`thumbs`]; o domínio puro (sem GUI) em [`config`],
//! [`editor`], [`exif`] e [`fs_browser`]. O visual e os temas ficam em
//! [`theme`] e [`icons`].
//!
//! Tudo o que não depende de janela é testável sem abrir a UI: ver
//! `tests/pipeline.rs`.

pub mod app;
pub mod config;
pub mod editor;
pub mod exif;
pub mod fs_browser;
pub mod icons;
pub mod image_store;
pub mod prelude;
pub mod theme;
pub mod thumbs;
