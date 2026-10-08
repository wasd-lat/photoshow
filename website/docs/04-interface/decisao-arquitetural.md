# Interface — decisão arquitetural

> **Status: aprovado**

## Responsabilidade

A Interface traduz gesto em chamada de transição e desenha estado. Ela nunca
decide regra de produto: toda decisão reaproveitável mora em `config.rs`,
`editor.rs`, `fs_browser.rs` ou `app/state.rs` — os módulos que não importam
freya e sobrevivem a uma troca de toolkit.

```text
interface (só Freya: app/*.rs menos state.rs, theme.rs, ui.rs, icons.rs)
├── Toolbar      (menu Arquivo, edição, filtro, fullscreen, config)
├── Browser      (favoritas, árvore lazy, lista virtualizada, busca)
├── Viewer       (zoom ancorado, pan, crop, comparador, menu de contexto)
├── Gallery      (grade fluida, células com thumb + sel + rating)
├── AdjustPanel  (histograma + 4 sliders, sem estado próprio)
├── Dialogs      (renomear, config 2 colunas, ajuda F1, EXIF, lote)
├── StatusBar    (posição, pasta, dims, zoom, mensagens)
├── Shortcuts    (atalhos globais, mesma tabela do egui)
├── ui.rs        (única fonte de medidas; dropdown ancorado próprio)
└── theme.rs     (slate, charcoal, frost, paper via palette())
```

`app/mod.rs` compõe: toolbar + dock redimensionável (navegador | viewer /
galeria) + statusbar, com `Panels` como `Component` para os hooks caírem no
escopo certo. Fullscreen/presentation trocam a árvore inteira pelo `Viewer`.

> UI expressa intenção; não governa domínio. Nenhuma decisão de produto mora
> em `ui.rs` ou num componente.

## Diagnóstico atual

Levantado sobre o código real (`app/mod.rs`, `toolbar.rs` ~1000 linhas,
`viewer.rs` ~1250, `browser.rs`, `gallery.rs`, `dialogs.rs` ~1320,
`adjust_panel.rs`, `shortcuts.rs`, `statusbar.rs`, `ui.rs`, `theme.rs`).

### Reutilizar

**`ui.rs` como fonte única de medidas.** `Metrics` (`Copy`, escala 0.75–1.40),
`Role` (Section/Small/Body/Title), `gaps`, `section`, `faint`, `icon_button` —
um lugar para tipografia, espaçamento, raio e alvo de clique. O dropdown
ancorado próprio (`Position::Absolute` + offset do gatilho) existe porque o
`Menu` do Freya entra no fluxo e deslocava a barra em ~200px. Direção visual
documentada: neutros, um acento, borda 1px, sem gradiente. REUSE.

**Gesto local no viewer.** `crop_rect`, `drag`, `area` como `use_state` local;
`DragKind::Pan/Crop/Compare` resolvido no gesto; `compute_draw` puro
(área + dims + zoom + offset → rect desenhado); comparador com dois nós
`Clip` em vez de shader; `compare_rect` como fração da foto desenhada.
Testes cobrem divisória segue-foto, segue-offset e alça só sobre a foto. REUSE.

**Lista virtualizada, galeria em janela.** O browser usa `VirtualScrollView`
para 50k fotos; a galeria desenha janela de ±300 ao redor da seleção com
`key(index)`, célula quadrada com `ImageCover::Center`, borda `ACCENT` no
selecionado, badge de cor + ★ sobrepostas, duplo-clique maximiza. REUSE o
padrão; ver refatoração sobre a janela.

**Shortcuts centralizados.** `shortcuts::global_shortcuts` num `on_global_
key_down` da raiz, mesma tabela do egui (`←/→`, `+/-/0`, `F2`, `Ctrl+Z/Y`,
`Enter`, `F9`, `F11`, `Esc`, `Ctrl+O`, `Ctrl+Shift+O`, `Ctrl+1/2/3`,
`Ctrl+B`, `Ctrl+=/-`), com `HelpDialog` (`F1`/`?`) auditável. REUSE.

**Diálogos como componentes de canal.** Renomear, Config (2 colunas, tags
coloridas em acordeões), Ajuda, EXIF (`Ctrl+I`), Lote — abrem/fecham por
`AppChannel::Dialogs`, sem estado global espalhado. REUSE.

### Reutilizar com refatoração

**`toolbar.rs` e `viewer.rs` grandes.** ~1000 e ~1250 linhas: menu Arquivo +
edição + filtro + fullscreen + config + save fluxo num; viewer com gesto +
draw + crop + comparador + menu de contexto + estados vazio/loading/falha.
Preservar comportamento, extrair `file_menu.rs`, `edit_bar.rs`,
`viewer_gestures.rs`, `compare_overlay.rs` — cada um com suas transições já
existentes, sem inventar camada.

**Galeria em janela de seleção, não de viewport.** `WINDOW = 300` ao redor de
`sel` desenha até ~601 células independente do viewport, enquanto o
`ThumbCache` só alimenta ±25 — as duas janelas não conversam.
(Já decidido no 01-media: scheduler viewport-driven. Aqui o outro lado:)
preservar a célula e a grade `wrap`, trocar `sel ± 300` por linhas visíveis
do `ScrollView`.

**Viewer escreve no canal durante o gesto via `update`.** Correto (handler
não pode usar hook), mas `cell_view` resolve índice por `position()` linear
na lista visível a cada clique — O(n) por clique, aceitável hoje, documentar
como dívida quando `PhotoId` chegar (decidido no 03-application).

**`ACCENT`, `DIM`, `CROP_STROKE` espalhados.** Constantes de cor no viewer e
na galeria (`Color::from_rgb(40,40,43)` inline na célula) em vez da paleta do
tema. Preservar valores, mover para `theme.rs`/`ui.rs` como tokens.

### Refatorar fortemente ou substituir a estrutura

- `dialogs.rs` com 1320 linhas e cinco modais — quebrar por modal
  (`rename_dialog.rs`, `settings_dialog.rs`, `help_dialog.rs`,
  `exif_dialog.rs`, `batch_dialog.rs`) sem mudar canais.
- `file_menu` com `dirty`/`saving`/`has_photo` como booleanos posicionais —
  agrupar em `FileMenuState` explícito.
- `tag_color(id: u8)` com `_ => cinza` silencioso — mapear via
  `config.color_tags` (fonte do usuário), não via match fixo.
- `show_filmstrip` vs `hide_gallery`: dois flags para a mesma visibilidade
  (`enabled` lê um, layout lê outro) — unificar num só.

## Decisões aprovadas

1. Nenhuma decisão de produto em componente ou `ui.rs`: vai para `config.rs`
   (com teste) ou `state.rs` (função pura).
2. Gesto crop/compare/pan é estado local do viewer; só o commit atravessa o
   canal (`apply_crop`, `set_compare_split`).
3. Comunicação toolbar → viewer por `CropCommand` (`Apply`/`RefitAspect`) via
   `Services::crop_cmd` — sem poluir o estado global.
4. `compute_draw` e `compare_rect` são funções puras testáveis; overlay com
   nós `Clip`, nunca shader para recorte.
5. Lista virtualizada para 50k; galeria migra para linhas visíveis do
   viewport (scheduler do 01-media alimenta a mesma janela).
6. Atalhos centralizados em `shortcuts.rs`; todo atalho aparece no
   `HelpDialog` — atalho sem linha na ajuda é bug.
7. Cores por token de tema; constante inline é dívida, não padrão.
8. Modais abrem/fecham por canal `Dialogs`; nenhum modal guarda verdade que
   o domínio não tenha.
9. `ui.rs` continua "burro em Freya": medidas e primitivas, zero regra de
   negócio.
10. Visibilidade de painel tem uma fonte por painel (`hide_gallery`); flag
    duplicado é bug a remover.

## Consequências

- `toolbar.rs`/`viewer.rs`/`dialogs.rs` quebrados por superfície, sem mudar
  canais nem transições.
- Galeria viewport-driven (outro lado da decisão do 01-media).
- Tokens de cor no tema; `tag_color` lê `color_tags` do usuário.
- `FileMenuState` explícito no lugar de booleanos posicionais.

## Regra anti-bloat

Não criar design-system externo, biblioteca de componentes, motor de temas
com herança, nem DSL de layout. `ui.rs` + `theme.rs` + componentes Freya
resolvem o escopo. Animação além de transição de drawer, gráfico além do
histograma de `rect`s, e overlay além de crop/compare são complexidade
futura, não presente.
