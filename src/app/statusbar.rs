//! Barra de status: posição, mensagem, dimensões e zoom.

use crate::prelude::*;

use crate::image_store::LoadState;

use super::services::Services;
use super::state::{self, AppChannel, channel};

#[derive(PartialEq, Clone)]
pub struct StatusBar;

impl StatusBar {}

impl Component for StatusBar {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Services>();
        let (pos, status, saving) = {
            let status_radio = channel(AppChannel::Status);
            let st = status_radio.read();
            let photos = channel(AppChannel::Photos);
            (
                state::position_label(&photos.read()),
                st.status.clone(),
                st.saving,
            )
        };
        let zoom = channel(AppChannel::Viewer).read().zoom;
        let detail = detail_label(&services.load.read().clone(), zoom);
        let saving_note = if saving { " · salvando…" } else { "" };

        rect()
            .width(Size::fill())
            .horizontal()
            .cross_align(Alignment::Center)
            .padding(6.)
            .spacing(10.)
            .background((0, 0, 0, 30))
            .child(secondary(&pos))
            .child(secondary(&status))
            .child(secondary(&format!("{detail}{saving_note}")))
    }
}

/// Detalhe técnico: dimensões, zoom, erro de carga ou nada.
#[must_use]
pub fn detail_label(load: &LoadState, zoom: f32) -> String {
    match load {
        LoadState::Loaded { full_px, .. } => {
            format!(
                "{}×{} px · {}%",
                full_px.0,
                full_px.1,
                (zoom * 100.0) as u32
            )
        }
        LoadState::Loading => String::from("carregando…"),
        LoadState::Failed(e) => e.clone(),
        LoadState::Empty => String::new(),
    }
}

/// Trecho em cor secundária.
fn secondary(text: &str) -> impl IntoElement {
    label()
        .font_size(12.0)
        .color((110, 110, 118))
        .text(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_load_has_no_detail() {
        assert_eq!(detail_label(&LoadState::Empty, 1.0), "");
    }

    #[test]
    fn loading_shows_placeholder() {
        assert_eq!(detail_label(&LoadState::Loading, 1.0), "carregando…");
    }

    #[test]
    fn failed_load_surfaces_message() {
        let load = LoadState::Failed(String::from("boom"));
        assert_eq!(detail_label(&load, 1.0), "boom");
    }
}
