//! photoshow: visualizador de fotos rápido em Freya.
//!
//! Organização: a raiz do app vive em [`app`]; o pipeline de imagem em
//! [`image_store`] e [`thumbs`]; o domínio puro (sem GUI) em [`config`],
//! [`editor`], [`exif`], [`fs_browser`] e [`cli`]. O visual fica em
//! [`theme`], [`ui`] e [`icons`].
//!
//! Tudo o que não depende de janela é testável sem abrir a UI: ver
//! `tests/pipeline.rs`. O que depende de janela mas não de display é
//! `tests/menu_task.rs` (headless, `freya_testing`).
//!
//! ## Roteiro para uma porta de volta ao egui
//!
//! A divisão em módulos já é a fronteira de portabilidade:
//!
//! | Camada | Módulos | Qué knobs no egui |
//! |---|---|---|
//! | Domínio (não muda) | `config`, `editor`, `exif`, `fs_browser` | — |
//! | Decisão de entrada | `cli` (o que abrir a partir de caminhos externos) | — |
//! | Transições de estado | `app/state.rs` (só `AppState`, sem freya) | — |
//! | Serviços assíncronos | `image_store`, `thumbs`, `app/services.rs` | `ImageHandle` volta a ser `egui::TextureHandle` |
//! | Só Freya | `app/*.rs` (menos `state.rs`), `theme.rs`, `ui.rs`, `icons.rs` | reescrever a árvore de elementos |
//!
//! Regra prática: **nenhuma decisão de produto pode morar em `ui.rs` ou em um
//! componente**. Ela vai para `config.rs` (com teste) ou `state.rs` (função
//! pura). Assim a troca de toolkit não leva decisão junto.

pub mod app;
pub mod cli;
pub mod config;
pub mod editor;
pub mod exif;
pub mod fs_browser;
pub mod icons;
pub mod image_store;
pub mod prelude;
pub mod theme;
pub mod thumbs;
pub mod ui;
