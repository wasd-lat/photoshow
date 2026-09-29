//! Prelude do photoshow.
//!
//! O `freya::prelude` não reexporta alguns tipos que o app usa direto
//! (geometria do `torin`, `ImageHandle`, Radio). Centralizar aqui evita
//! `use` repetido e mantém os módulos de UI enxutos.
//!
//! `ScreenRect` é o atalho para o retângulo de layout do torin
//! (`euclid::Rect<f32, ()>`), usado em zoom/pan/crop.

pub use freya::elements::image::{AspectRatio, ImageCover, ImageHandle};
pub use freya::engine::prelude::AlphaType;
pub use freya::prelude::*;
pub use freya::radio::{Radio, RadioStation};
pub use torin::prelude::{Alignment, Area, CursorPoint, Direction, Point2D, Size2D, Vector2D};

/// Retângulo de layout (posição + tamanho em px de tela).
pub type ScreenRect = Area;
