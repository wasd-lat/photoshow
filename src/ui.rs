//! Design system do photoshow: métricas, primitivas e o dropdown ancorado.
//!
//! Direção visual: minimalista neutra (referência: apps atuais da OpenAI).
//! Neutros quase acinzentados, um único acento, bordas de 1px no lugar de
//! sombra, tipografia pequena, cantos de 6–10px. Nada de gradiente.
//!
//! Este módulo é **a única fonte de medidas de UI**. Ele é propositalmente
//! "burro" em Freya: só cores, tamanhos e helpers. Toda decisão reaproveitável
//! no egui mora fora daqui — em `src/config.rs`, `src/editor.rs`,
//! `src/fs_browser.rs` e `src/app/state.rs`, que não importam freya.
//!
//! ## Por que o dropdown é nosso
//!
//! O `Menu` do Freya marca `Layer::Overlay`, que é só **ordem de pintura**. No
//! torin o nó continua `Position::Stacked`, ou seja, entra no fluxo: ao abrir,
//! a barra desce ~200px e o clique passa a bater no item errado — foi isso que
//! quebrou "Abrir pasta…" e o star de favorito. Aqui o menu sai do fluxo com
//! `Position::Absolute` + `offset_*`, ancorado na área medida do gatilho.

use freya::components::{Button, Menu, SvgViewer, Tooltip, TooltipContainer};
use freya::elements::label::Label;

use crate::icons;
use crate::prelude::*;

/// Papel tipográfico: tamanho base de cada peso de texto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Seção ("FAVORITAS"), versalete miúdo.
    Section,
    /// Texto de apoio (caminho, contagem).
    Small,
    /// Corpo padrão.
    Body,
    /// Destaque (título de diálogo).
    Title,
}

impl Role {
    /// Tamanho em px antes de aplicar a escala.
    const fn base(self) -> f32 {
        match self {
            Self::Section => 10.5,
            Self::Small => 12.0,
            Self::Body => 13.0,
            Self::Title => 15.0,
        }
    }
}

/// Medidas de UI já resolvidas para a escala do usuário.
///
/// `Copy` de propósito: cada componente resolve uma vez no topo do `render` e
/// repassa para os helpers, sem custo por elemento.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    scale: f32,
}

impl Metrics {
    /// Cria as métricas para uma escala (já validada por
    /// [`crate::config::sanitize_ui_scale`]).
    #[must_use]
    pub fn new(scale: f32) -> Self {
        Self { scale }
    }

    /// Escala vigente.
    #[must_use]
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Tamanho de fonte de um papel, em meios de pixel (evita blur).
    #[must_use]
    pub fn font(&self, role: Role) -> f32 {
        (role.base() * self.scale * 2.0).round() / 2.0
    }

    /// Espaçamento: `n` passos de 4px, escalado.
    #[must_use]
    pub fn gap(&self, n: f32) -> f32 {
        n * 4.0 * self.scale
    }

    /// Padding: `n` passos de 4px, escalado.
    #[must_use]
    pub fn pad(&self, n: f32) -> f32 {
        self.gap(n)
    }

    /// Lado do ícone.
    #[must_use]
    pub fn icon(&self) -> f32 {
        (icons::ICON * self.scale).round()
    }

    /// Altura mínima confortável de um alvo de clique.
    ///
    /// Acessibilidade: alvo pequeno demais é difícil de acertar — e foi
    /// exatamente o star de favorito, que ficou inclicável.
    #[must_use]
    pub fn hit(&self) -> f32 {
        (24.0 * self.scale).round()
    }

    /// Raio de canto padrão de superfícies.
    #[must_use]
    pub fn radius(&self) -> f32 {
        (8.0 * self.scale).round()
    }

    /// Raio de canto de controles pequenos.
    #[must_use]
    pub fn radius_sm(&self) -> f32 {
        (6.0 * self.scale).round()
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new(1.0)
    }
}

/// Padding vertical + horizontal, na escala da UI.
#[must_use]
pub fn gaps(m: &Metrics, vertical: f32, horizontal: f32) -> Gaps {
    // O par é `(vertical, horizontal)`, como `Gaps::from((f32, f32))`.
    Gaps::from((m.gap(vertical), m.gap(horizontal)))
}

/// Idem, quando só se tem `Metrics` por valor (cópia barata: é `Copy`).
#[must_use]
pub fn gaps_of(m: Metrics, vertical: f32, horizontal: f32) -> Gaps {
    gaps(&m, vertical, horizontal)
}

/// Rótulo com o papel e a cor certos.
#[must_use]
pub fn text(m: &Metrics, role: Role, color: Color, content: impl Into<String>) -> Label {
    label()
        .font_size(m.font(role))
        .color(color)
        .text(content.into())
}

/// Rótulo de uma linha só, para listas e nomes de arquivo.
///
/// `max_lines(1)` impede um nome comprido de virar três linhas e atropelar a
/// linha seguinte. Em raros lugares broke a medição do pai, então é opt-in.
#[must_use]
pub fn text_one_line(m: &Metrics, role: Role, color: Color, content: impl Into<String>) -> Label {
    label()
        .font_size(m.font(role))
        .max_lines(1)
        .color(color)
        .text(content.into())
}

/// Rótulo secundário.
#[must_use]
pub fn faint(m: &Metrics, content: impl Into<String>) -> Label {
    text(
        m,
        Role::Small,
        Color::from_argb(150, 130, 130, 138),
        content,
    )
}

/// Cabeçalho de seção: versalete miúdo.
#[must_use]
pub fn section(m: &Metrics, content: &str) -> Label {
    text(
        m,
        Role::Section,
        Color::from_argb(140, 130, 130, 138),
        content.to_uppercase(),
    )
}

/// Botão só com ícone, com alvo de clique mínimo e tooltip.
///
/// O alvo mínimo vem de um `padding` calculável, não de `min_height`: o
/// `Button` do Freya não implementa `ContainerSizeExt`, e um alvo pequeno
/// demais é justamente o que tornou o star de favorito inclicável.
#[must_use]
pub fn icon_button(m: &Metrics, name: &str, tip: &'static str, button: Button) -> impl IntoElement {
    icon_button_colored(m, name, None, tip, button)
}

/// Igual a [`icon_button`], mas com cor fixa no ícone (`None` = cor do tema).
#[must_use]
pub fn icon_button_colored(
    m: &Metrics,
    name: &str,
    color: Option<Color>,
    tip: &'static str,
    button: Button,
) -> impl IntoElement {
    let side = m.icon();
    let pad = ((m.hit() - side) / 2.0).max(0.0);
    let icon = match color {
        Some(c) => icons::icon_tinted(name, side, c),
        None => svg(m, name),
    };
    TooltipContainer::new(Tooltip::new_text(tip))
        .child(button.padding(pad).corner_radius(m.radius_sm()).child(icon))
}

/// Ícone na escala da UI.
#[must_use]
pub fn svg(m: &Metrics, name: &str) -> SvgViewer {
    icons::icon(name)
        .width(Size::px(m.icon()))
        .height(Size::px(m.icon()))
}

/// Linha divisória de 1px.
#[must_use]
pub fn hairline(color: Color) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .height(Size::px(1.))
        .background(color)
}

/// Dropdown ancorado ao gatilho, **fora do fluxo**.
///
/// É um componente porque precisa de hooks (`use_state` para as áreas medidas)
/// e hooks só podem ser chamados no topo de um `render`.
#[derive(PartialEq, Clone)]
pub struct Dropdown {
    open: bool,
    trigger: Element,
    items: Element,
    on_close: EventHandler<()>,
}

impl Dropdown {
    /// Monta o dropdown.
    ///
    /// `items` são os **itens**, não um `Menu`: este componente monta o `Menu`
    /// em volta deles (é quem sabe fechar no clique fora e no Esc).
    #[must_use]
    pub fn new(
        open: bool,
        trigger: impl IntoElement,
        items: impl IntoElement,
        on_close: impl Into<EventHandler<()>>,
    ) -> Self {
        Self {
            open,
            trigger: trigger.into_element(),
            items: items.into_element(),
            on_close: on_close.into(),
        }
    }
}

impl Component for Dropdown {
    fn render(&self) -> impl IntoElement {
        // Hooks no topo, sempre: `open` muda de frame para frame.
        let host = use_state(ScreenRect::default);
        let trigger = use_state(ScreenRect::default);

        // Só desenha o menu depois de medir o gatilho; senão o primeiro frame
        // abriria em (0,0).
        let (dx, dy) = {
            let h = host.peek();
            let t = trigger.peek();
            if h.width() <= 0.0 || t.width() <= 0.0 {
                (0.0, 0.0)
            } else {
                (t.min_x() - h.min_x(), t.max_y() - h.min_y())
            }
        };
        let ready = self.open && dx != 0.0;
        let on_close_menu = self.on_close.clone();
        let items = self.items.clone();
        rect()
            .on_sized({
                let mut host = host;
                move |e: Event<SizedEventData>| host.set_if_modified(e.area)
            })
            .child(
                rect()
                    .on_sized({
                        let mut trigger = trigger;
                        move |e: Event<SizedEventData>| trigger.set_if_modified(e.area)
                    })
                    .child(self.trigger.clone()),
            )
            .maybe(ready, |el| {
                el.child(
                    rect()
                        // Fora do fluxo: o pai não muda de tamanho.
                        .position(Position::new_absolute().left(0.).top(0.))
                        .offset_x(dx)
                        .offset_y(dy)
                        .layer(Layer::Overlay)
                        // `Menu` por fora dos itens: é ele que trata
                        // clique-fora e Esc.
                        .child(Menu::new().on_close(on_close_menu).child(items)),
                )
            })
    }
}

/// Estado de um menu/dropdown: só o `toggle`, sem hook.
pub fn toggle_menu(open: &mut bool) {
    *open = !*open;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_grows_font_and_spacing() {
        let base = Metrics::new(1.0);
        let big = Metrics::new(1.4);
        assert!(big.font(Role::Body) > base.font(Role::Body));
        assert!(big.gap(2.) > base.gap(2.));
        assert!(big.hit() >= base.hit());
    }

    #[test]
    fn scale_shrinks_everything_proportionally() {
        let base = Metrics::new(1.0);
        let small = Metrics::new(0.75);
        for role in [Role::Section, Role::Small, Role::Body, Role::Title] {
            assert!(small.font(role) < base.font(role), "{role:?}");
        }
        assert!(small.gap(3.) < base.gap(3.));
    }

    #[test]
    fn hit_target_is_never_tiny() {
        // O star de favorito falhava por ser pequeno demais.
        assert!(Metrics::new(0.75).hit() >= 18.0);
        assert!(Metrics::new(1.0).hit() >= 24.0);
    }

    #[test]
    fn roles_have_a_strict_size_order() {
        let m = Metrics::new(1.0);
        assert!(m.font(Role::Section) < m.font(Role::Small));
        assert!(m.font(Role::Small) < m.font(Role::Body));
        assert!(m.font(Role::Body) < m.font(Role::Title));
    }

    #[test]
    fn toggle_menu_flips_the_flag() {
        let mut open = false;
        toggle_menu(&mut open);
        assert!(open);
        toggle_menu(&mut open);
        assert!(!open);
    }
}
