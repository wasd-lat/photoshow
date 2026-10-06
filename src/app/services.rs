//! Serviços assíncronos: scan de pasta, save e o *pump* que drena as
//! threads de decode/miniaturas para os `State` reativos.
//!
//! No egui isso era `ctx.request_repaint()` a cada frame. No Freya o
//! equivalente é uma task que acorda, drena os canais e escreve em `State`,
//! só quando algo muda, para não gastar render à toa.

use std::path::PathBuf;
use std::time::Duration;

use crate::prelude::*;

use crate::editor::{EditorState, bake, save_baked};
use crate::fs_browser::{self, ScanResult};
use crate::image_store::{ImageStore, LoadState};
use crate::thumbs::{ThumbCache, ThumbMap};

/// Tique do pump com trabalho pendente (~60 Hz): mantém a UI responsiva.
const PUMP_TICK_FAST: Duration = Duration::from_millis(16);
/// Tique do pump ocioso (~8 Hz): quase não acorda a CPU, e 125ms de atraso
/// num resultado que ninguém está esperando é imperceptível.
const PUMP_TICK_IDLE: Duration = Duration::from_millis(125);

/// Sobe uma task de **longa duração** (diálogo nativo, scan, gravação).
///
/// Usa `spawn_forever` e não `spawn` de propósito: `spawn` amarra a task ao
/// escopo do componente cujo handler a criou, e o Freya cancela as tasks do
/// escopo quando ele desmonta. Como os itens de menu fecham o menu no mesmo
/// clique (`open.set(false)`), uma task escopada morria junto com a lista e o
/// diálogo nativo nunca aparecia — nenhum botão de "abrir" fazia nada.
///
/// Regra do app: **todo trabalho que sobrevive ao clique usa [`background`]**.
/// `spawn` fica só para trabalho que pode ser descartado com o componente.
pub fn background(future: impl Future<Output = ()> + 'static) -> TaskHandle {
    spawn_forever(future)
}

/// Comando que a toolbar envia para o visualizador.
///
/// O rect de crop é estado local do viewer (só o gesto o produz), mas o botão
/// "Aplicar" e o dropdown de proporção vivem na toolbar. Esta caixa resolve
/// a comunicação entre os dois sem poluir o estado global.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CropCommand {
    /// "Aplicar" foi clicado (ou `Enter`): converte o rect e commita.
    Apply,
    /// A proporção mudou: reencaixa o rect existente.
    RefitAspect,
}

/// Resultado de um scan assíncrono.
#[derive(Clone)]
pub struct ScanOutcome {
    /// Geração do pedido (descarta obsoletos).
    pub seq: u64,
    /// Resultado bruto.
    pub result: ScanResult,
}

/// Resultado de um salvamento assíncrono.
#[derive(Clone, PartialEq, Eq)]
pub struct SaveOutcome {
    /// Mensagem para a barra de status.
    pub note: String,
    /// Arquivo sobrescrito: recarregar do disco e limpar o editor.
    pub reload: Option<PathBuf>,
}

/// Estado assíncrono compartilhado por toda a UI (injetado via contexto).
///
/// `Clone` compartilha o mesmo estado; não é `PartialEq` de propósito: os
/// campos reativos é que controlam o re-render, não o valor do serviço.
#[derive(Clone)]
pub struct Services {
    /// Cache de imagens + canal de decode principal.
    pub images: ImageStore,
    /// Estado reativo do carregamento (o viewer assina este).
    pub load: State<LoadState>,
    /// Fila de miniaturas.
    pub thumbs: ThumbCache,
    /// Mapa reativo de miniaturas prontas (a galeria assina este).
    pub thumb_map: State<ThumbMap>,
    /// Resultado do último scan concluído.
    pub scan: State<Option<ScanOutcome>>,
    /// Resultado do último save concluído.
    pub save: State<Option<SaveOutcome>>,
    /// Comando pendente da toolbar para o visualizador.
    pub crop_cmd: State<Option<CropCommand>>,
    /// Pulso "chegou miniatura" (ver [`Self::spawn_pump`]).
    pub thumb_wake: State<u64>,
}

impl Services {
    /// Cria os serviços e sobe a task do pump.
    #[must_use]
    pub fn new() -> Self {
        let this = Self {
            images: ImageStore::new(),
            load: State::create(LoadState::Empty),
            thumbs: ThumbCache::new(),
            thumb_map: State::create(ThumbMap::new()),
            scan: State::create(None),
            save: State::create(None),
            crop_cmd: State::create(None),
            thumb_wake: State::create(0),
        };
        this.spawn_pump();
        this
    }

    /// Task que drena a thread de decode para o `State` reativo.
    ///
    /// O tique é adaptativo: `FAST` enquanto há decode ou miniatura em voo
    /// (para a UI ficar responsiva durante a carga) e `SLOW` quando está tudo
    /// parado. O `SLOW` existe porque acordar a task a cada 16ms mesmo sem
    /// trabalho é o que segura o consumo em ~0,35% de CPU com a janela
    /// ociosa; a 8Hz isso cai para perto de zero sem causar atraso visível.
    fn spawn_pump(&self) {
        let this = self.clone();
        let mut wake = self.thumb_wake;
        let mut wake_gen = 0u64;
        background(async move {
            loop {
                let decoding = this.images.poll(this.load);
                // O worker de miniaturas roda numa thread solta e não pode
                // tocar no `State`. Enquanto ele tiver job em voo, dá um
                // pulso para o próximo frame vir buscar o resultado; sem
                // isso as miniaturas ficariam na fila até o usuário mexer
                // na janela.
                let thumbs_busy = this.thumbs.in_flight();
                if thumbs_busy {
                    wake_gen = wake_gen.wrapping_add(1);
                    wake.set(wake_gen);
                }
                let tick = if decoding || thumbs_busy {
                    PUMP_TICK_FAST
                } else {
                    PUMP_TICK_IDLE
                };
                timer(tick).await;
            }
        });
    }

    /// Um passo do pump: drena o canal de decode e publica o que mudou.
    ///
    /// Também é chamada pelo render da raiz, para que o primeiro frame já
    /// enxergue o que chegou da thread.
    pub fn tick(&self) -> bool {
        self.images.poll(self.load)
    }

    /// Enfileira/drena miniaturas da janela atual e publica em `thumb_map`.
    ///
    /// Chamado pelo render da raiz, que assina `thumb_map` (ver
    /// [`Self::subscribe_results`]): escrever no `State` de dentro do próprio
    /// render da galeria não agenda o próximo frame e a grade ficaria vazia.
    pub fn pump_thumbs(
        &self,
        visible: &[crate::fs_browser::PhotoPath],
        sel: Option<usize>,
    ) -> bool {
        self.thumbs.poll(visible, sel, self.thumb_map)
    }

    /// Dispara um scan de pasta em background; publica o resultado em `scan`.
    pub fn start_scan(&self, dir: PathBuf, opts: fs_browser::ScanOptions, seq: u64) {
        let this = self.clone();
        let mut this = this;
        background(async move {
            let result = thread(move || fs_browser::scan_blocking(dir, opts)).await;
            this.scan.set(Some(ScanOutcome { seq, result }));
        });
    }

    /// Consome o resultado de um scan (`None` se não havia nada novo).
    pub fn take_scan(&self) -> Option<ScanOutcome> {
        take(&self.scan)
    }

    /// Assina os canais de resultado para que o próximo frame aconteça.
    ///
    /// Quem drena `scan`/`save`/`thumb_map` é o render da raiz, mas a raiz
    /// não mostra nada desses canais: sem esta assinatura, um scan, um save
    /// ou uma miniatura que termina em background não redesenharia nada e a
    /// UI ficaria congelada.
    pub fn subscribe_results(&self) {
        let _ = self.scan.read();
        let _ = self.save.read();
        let _ = self.thumb_map.read();
        let _ = self.thumb_wake.read();
    }

    /// Assa a imagem em background e grava; publica em `save`.
    #[allow(clippy::too_many_arguments)]
    pub fn start_save(
        &self,
        full: image::DynamicImage,
        display_base: (u32, u32),
        edit: EditorState,
        dest: PathBuf,
        jpeg_quality: u8,
        reload: bool,
        source: Option<PathBuf>,
    ) {
        let this = self.clone();
        let mut this = this;
        background(async move {
            let outcome = thread(move || {
                let baked = bake(&full, display_base, &edit);
                match save_baked(&baked, &dest, jpeg_quality, source.as_deref()) {
                    Ok(()) => SaveOutcome {
                        note: format!("Salvo em {}", dest.display()),
                        reload: reload.then_some(dest),
                    },
                    Err(e) => SaveOutcome {
                        note: format!("Falha ao salvar: {e}"),
                        reload: None,
                    },
                }
            })
            .await;
            this.save.set(Some(outcome));
        });
    }

    /// Consome o resultado de um save (`None` se não havia nada novo).
    pub fn take_save(&self) -> Option<SaveOutcome> {
        take(&self.save)
    }

    /// Pede um comando de crop ao viewer.
    pub fn request_crop(&self, cmd: CropCommand) {
        let mut slot = self.crop_cmd;
        slot.set(Some(cmd));
    }

    /// Consome o comando de crop pendente (`None` se não havia).
    pub fn take_crop_cmd(&self) -> Option<CropCommand> {
        take(&self.crop_cmd)
    }

    /// Limpa caches ao trocar de pasta/arquivos (o `State` de thumbs também).
    pub fn reset(&self) {
        let mut map = self.thumb_map;
        self.images.clear_prefetch();
        self.thumbs.clear();
        map.set(ThumbMap::new());
    }
}

impl Default for Services {
    fn default() -> Self {
        Self::new()
    }
}

/// Esvazia um `State<Option<T>>` só quando há algo a retirar.
///
/// `write()` notifica os assinantes, e quem chama isto é o `render`: notificar
/// a cada frame com o canal já vazio viraria laço de repaint sem fim.
fn take<T: 'static>(state: &State<Option<T>>) -> Option<T> {
    if state.peek().is_none() {
        return None;
    }
    let mut state = *state;
    state.write().take()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_tick_is_much_slower_than_the_busy_one() {
        // A economia de CPU depende inteiramente desta razão: o pump acorda
        // `1/tick` vezes por segundo mesmo sem trabalho nenhum.
        let busy_hz = 1000.0 / PUMP_TICK_FAST.as_millis() as f64;
        let idle_hz = 1000.0 / PUMP_TICK_IDLE.as_millis() as f64;
        assert!(
            idle_hz < busy_hz / 4.0,
            "idle {idle_hz}Hz vs busy {busy_hz}Hz"
        );
    }

    #[test]
    fn idle_tick_is_still_fast_enough_to_feel_instant() {
        // 125ms é o teto: acima disso um clique já parece "demorou".
        assert!(PUMP_TICK_IDLE.as_millis() <= 150);
    }
}
