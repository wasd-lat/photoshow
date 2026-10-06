//! Geometria do crop: alças, proporção e conversão tela -> px.
//!
//! Extraído do antigo `app.rs` (egui) para um módulo puro e testável: a UI
//! só fornece o rect de tela e o retângulo do desenho, e recebe o `CropRect`
//! em pixels da imagem de preview.

use crate::editor::CropRect;
use crate::prelude::*;

/// Recorte mínimo em px (display) para aceitar o gesto.
pub const CROP_MIN_PX: f32 = 8.0;
/// Raio de clique das alças de crop (px de tela).
pub const HANDLE_GRAB: f32 = 11.0;
/// Lado do quadrado desenhado em cada alça (px de tela).
pub const HANDLE_BOX: f32 = 9.0;

/// Alças de redimensionamento do crop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    /// Canto superior-esquerdo.
    Nw,
    /// Borda superior.
    N,
    /// Canto superior-direito.
    Ne,
    /// Borda direita.
    E,
    /// Canto inferior-direito.
    Se,
    /// Borda inferior.
    S,
    /// Canto inferior-esquerdo.
    Sw,
    /// Borda esquerda.
    W,
}

impl Handle {
    /// Todas as alças, na ordem de desenho.
    #[must_use]
    pub fn all() -> [Handle; 8] {
        use Handle::{E, N, Ne, Nw, S, Se, Sw, W};
        [Nw, N, Ne, E, Se, S, Sw, W]
    }

    /// Posição da alça no rect.
    #[must_use]
    pub fn point(self, r: &ScreenRect) -> Point2D {
        use Handle::{E, N, Ne, Nw, S, Se, Sw, W};
        let c = r.center();
        match self {
            Nw => r.min(),
            N => Point2D::new(c.x, r.min_y()),
            Ne => Point2D::new(r.max_x(), r.min_y()),
            E => Point2D::new(r.max_x(), c.y),
            Se => r.max(),
            S => Point2D::new(c.x, r.max_y()),
            Sw => Point2D::new(r.min_x(), r.max_y()),
            W => Point2D::new(r.min_x(), c.y),
        }
    }

    /// Ponto âncora oposto (fixo durante o resize).
    #[must_use]
    pub fn anchor(self, r: &ScreenRect) -> Point2D {
        use Handle::{E, N, Ne, Nw, S, Se, Sw, W};
        let c = r.center();
        match self {
            Nw => r.max(),
            N => Point2D::new(c.x, r.max_y()),
            Ne => Point2D::new(r.min_x(), r.max_y()),
            E => Point2D::new(r.min_x(), c.y),
            Se => r.min(),
            S => Point2D::new(c.x, r.min_y()),
            Sw => Point2D::new(r.max_x(), r.min_y()),
            W => Point2D::new(r.max_x(), c.y),
        }
    }
}

/// Gesto em andamento dentro da área do crop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragKind2 {
    /// Nova seleção a partir da âncora.
    New {
        /// Ponto fixo do arrasto.
        anchor: Point2D,
    },
    /// Movendo a seleção existente.
    Move {
        /// Offset entre o clique e o canto do rect.
        offset: Vector2D,
    },
    /// Redimensionando por uma alça (âncora oposta fixa).
    Resize {
        /// Alça arrastada.
        handle: Handle,
        /// Canto oposto, que não se move.
        anchor: Point2D,
    },
}

/// Gesto em andamento no visualizador (pan, crop ou divisória do comparador).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragKind {
    /// Arrasto de pan; guarda a última posição do cursor.
    Pan(CursorPoint),
    /// Arrasto de crop.
    Crop(DragKind2),
    /// Arrasto da divisória do comparador.
    Compare,
}

/// Alça mais próxima do ponto (raio [`HANDLE_GRAB`]); devolve (alça, âncora).
#[must_use]
pub fn hit_handle(r: &ScreenRect, p: Point2D) -> Option<(Handle, Point2D)> {
    Handle::all()
        .into_iter()
        .find(|h| h.point(r).distance_to(p) <= HANDLE_GRAB)
        .map(|h| (h, h.anchor(r)))
}

/// Rect de (âncora, ponteiro) respeitando a proporção w/h (`None` = livre).
#[must_use]
pub fn enforce_aspect(anchor: Point2D, pointer: Point2D, ratio: Option<f32>) -> ScreenRect {
    let Some(ratio) = ratio else {
        return from_points(anchor, pointer);
    };
    let d = pointer - anchor;
    if d.x == 0.0 || d.y == 0.0 {
        return from_points(anchor, pointer);
    }
    let (w, h) = (d.x.abs(), d.y.abs());
    let (w, h) = if w / h > ratio {
        (h * ratio, h)
    } else {
        (w, w / ratio)
    };
    from_points(
        anchor,
        Point2D::new(anchor.x + d.x.signum() * w, anchor.y + d.y.signum() * h),
    )
}

/// Rect que cobre os dois pontos (em qualquer quadrante).
#[must_use]
pub fn from_points(a: Point2D, b: Point2D) -> ScreenRect {
    ScreenRect::new(
        Point2D::new(a.x.min(b.x), a.y.min(b.y)),
        Size2D::new((b.x - a.x).abs(), (b.y - a.y).abs()),
    )
}

/// Rect centrado em `center` com o tamanho dado.
#[must_use]
pub fn centered(center: Point2D, width: f32, height: f32) -> ScreenRect {
    ScreenRect::new(
        Point2D::new(center.x - width / 2.0, center.y - height / 2.0),
        Size2D::new(width, height),
    )
}

/// Reaplica a proporção ao rect existente, mantendo o centro e o desenho.
#[must_use]
pub fn refit_aspect(r: ScreenRect, draw: ScreenRect, ratio: Option<f32>) -> ScreenRect {
    let Some(ratio) = ratio else {
        return r;
    };
    let c = r.center();
    let (w, h) = (r.width(), r.height());
    let (w, h) = if w / h > ratio {
        (h * ratio, h)
    } else {
        (w, w / ratio)
    };
    let (w, h) = (w.max(4.0), h.max(4.0));
    centered(c, w, h).intersection(&draw).unwrap_or(r)
}

/// Converte o rect de crop (coords de tela) para px da imagem de preview.
///
/// `draw` é o retângulo desenhado da imagem; `dims` as dimensões (w, h) em px
/// do preview. Devolve `None` se a seleção for pequena demais.
#[must_use]
pub fn crop_to_preview_px(
    draw: ScreenRect,
    dims: (u32, u32),
    crop: ScreenRect,
) -> Option<CropRect> {
    let (pw, ph) = (dims.0 as f32, dims.1 as f32);
    let size = draw.size;
    if pw <= 0.0 || ph <= 0.0 || size.width <= 0.0 || size.height <= 0.0 {
        return None;
    }
    let center = draw.center();
    // Normaliza a posição na tela para a fração 0..1 do preview.
    let to_px = |p: Point2D| {
        let rel_x = (p.x - center.x) / size.width;
        let rel_y = (p.y - center.y) / size.height;
        Point2D::new(
            (rel_x * pw + pw / 2.0).clamp(0.0, pw),
            (rel_y * ph + ph / 2.0).clamp(0.0, ph),
        )
    };
    let a = to_px(crop.min());
    let b = to_px(crop.max());
    let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
    let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
    if x1 - x0 < CROP_MIN_PX || y1 - y0 < CROP_MIN_PX {
        return None;
    }
    Some(CropRect {
        x: x0 as u32,
        y: y0 as u32,
        w: (x1 - x0) as u32,
        h: (y1 - y0) as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: f32, y: f32, w: f32, h: f32) -> ScreenRect {
        ScreenRect::new(Point2D::new(x, y), Size2D::new(w, h))
    }

    #[test]
    fn aspect_enforcement_keeps_ratio_from_anchor() {
        let a = Point2D::new(0.0, 0.0);
        let rect = enforce_aspect(a, Point2D::new(100.0, 10.0), Some(1.0));
        assert_eq!((rect.width(), rect.height()), (10.0, 10.0));
        let rect = enforce_aspect(a, Point2D::new(10.0, 100.0), Some(2.0));
        assert_eq!((rect.width(), rect.height()), (10.0, 5.0));
        // Negativo preserva o quadrante.
        let rect = enforce_aspect(a, Point2D::new(-40.0, -10.0), Some(1.0));
        assert_eq!((rect.min_x(), rect.min_y()), (-10.0, -10.0));
        // Livre não altera.
        let rect = enforce_aspect(a, Point2D::new(30.0, 7.0), None);
        assert_eq!((rect.width(), rect.height()), (30.0, 7.0));
    }

    #[test]
    fn handles_hit_corners_and_report_opposite_anchor() {
        let rect = r(10.0, 10.0, 80.0, 60.0);
        let (h, anchor) = hit_handle(&rect, Point2D::new(10.0, 10.0)).expect("canto NW");
        assert_eq!(h, Handle::Nw);
        assert_eq!(anchor, Point2D::new(90.0, 70.0));
        assert!(hit_handle(&rect, Point2D::new(50.0, 40.0)).is_none());
    }

    #[test]
    fn every_handle_point_and_anchor_are_inside_the_rect() {
        let rect = r(0.0, 0.0, 100.0, 50.0);
        // `contains` exclui a borda máxima, então comparamos com `>=`/`<=`.
        let inside = |p: Point2D| {
            p.x >= rect.min_x() && p.x <= rect.max_x() && p.y >= rect.min_y() && p.y <= rect.max_y()
        };
        for h in Handle::all() {
            assert!(inside(h.point(&rect)), "ponto fora: {h:?}");
            assert!(inside(h.anchor(&rect)), "âncora fora: {h:?}");
        }
    }

    #[test]
    fn from_points_covers_any_quadrant() {
        let r1 = from_points(Point2D::new(100.0, 100.0), Point2D::new(40.0, 20.0));
        assert_eq!(
            (r1.min_x(), r1.min_y(), r1.width(), r1.height()),
            (40., 20., 60., 80.)
        );
    }

    #[test]
    fn centered_builds_the_expected_rect() {
        let r1 = centered(Point2D::new(50.0, 25.0), 20.0, 10.0);
        assert_eq!(
            (r1.min_x(), r1.min_y(), r1.width(), r1.height()),
            (40., 20., 20., 10.)
        );
    }

    #[test]
    fn crop_px_maps_full_view_to_full_image() {
        let draw = r(0.0, 0.0, 800.0, 600.0);
        let crop = r(80.0, 60.0, 320.0, 240.0);
        let out = crop_to_preview_px(draw, (800, 600), crop).expect("válido");
        assert_eq!((out.x, out.y, out.w, out.h), (80, 60, 320, 240));
    }

    #[test]
    fn crop_px_scales_with_the_draw_rect() {
        // Preview desenhado na metade: 320x240 de tela = 640x480 px.
        let draw = r(0.0, 0.0, 400.0, 300.0);
        let crop = r(40.0, 30.0, 160.0, 120.0);
        let out = crop_to_preview_px(draw, (800, 600), crop).expect("válido");
        assert_eq!((out.x, out.y, out.w, out.h), (80, 60, 320, 240));
    }

    #[test]
    fn crop_px_rejects_tiny_selection() {
        let draw = r(0.0, 0.0, 800.0, 600.0);
        assert!(crop_to_preview_px(draw, (800, 600), r(10.0, 10.0, 2.0, 2.0)).is_none());
    }

    #[test]
    fn crop_px_rejects_degenerate_inputs() {
        assert!(
            crop_to_preview_px(ScreenRect::default(), (800, 600), r(0., 0., 100., 100.)).is_none()
        );
        assert!(crop_to_preview_px(r(0., 0., 800., 600.), (0, 0), r(0., 0., 100., 100.)).is_none());
    }

    #[test]
    fn refit_keeps_ratio_and_center() {
        let draw = r(0.0, 0.0, 800.0, 600.0);
        let crop = r(200.0, 200.0, 200.0, 100.0);
        let out = refit_aspect(crop, draw, Some(1.0));
        let ratio = out.width() / out.height();
        assert!((ratio - 1.0).abs() < 0.001, "ratio = {ratio}");
        assert!((out.center().x - crop.center().x).abs() < 0.001);
        assert!((out.center().y - crop.center().y).abs() < 0.001);
    }

    #[test]
    fn refit_is_identity_without_ratio() {
        let draw = r(0.0, 0.0, 800.0, 600.0);
        let crop = r(200.0, 200.0, 200.0, 100.0);
        assert_eq!(refit_aspect(crop, draw, None), crop);
    }
}
