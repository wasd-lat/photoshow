# Triagem do código atual

> **Status: diagnóstico aprovado para orientar migração.**

Levantamento feito sobre o código real em `src/`: `editor.rs` (pilha
não-destrutiva + `save_baked` atômico), `adjust.rs` (matemática pura de cor),
`image_store.rs` (decode + prefetch + `LoadState`), `thumbs.rs` (worker de
miniaturas), `fs_browser.rs` (`PhotoPath`, sort, sidecar), `config.rs`
(preferências), `app/state.rs` (canais Radio + transições) e
`app/services.rs` (pump + `background`).

## Reutilizar

### Pilha de edição não-destrutiva

`editor.rs` possui `EditorState` pequeno (`rot` + `crop` + `adjust`),
`EditorStack` com undo/redo barato, `bake()` que escala o crop do display
para a full-res, e `save_baked` atômico (temporário vizinho + `fsync` +
`rename` + EXIF preservado no temporário). A implementação deve ser
preservada, com revisão de nomenclatura (`rot`, `sel`, `cur`) quando
necessário.

### Matemática de cor pura

`adjust.rs` possui funções puras (`adjust_pixels`, `histogram`), ordem fixa
documentada (exposição em linear, resto em gamma), comparação por `EPSILON`
para não empilhar undo a cada pixel de mouse. Classificação: REUSE.

### `PhotoPath` como anti-primitivo

`fs_browser.rs` já usa `PhotoPath(PathBuf)` validado pela extensão em vez de
`String` solta, com `sort_by_cached_key` e `FolderSidecar` em JSON portátil
(nunca banco escondido). Classificação: REUSE.

### Regra `services::background`

`app/services.rs` documenta por que todo trabalho que sobrevive ao clique usa
`spawn_forever` e não `spawn` escopado. Classificação: REUSE.

## Reutilizar com refatoração

### `ImageStore`

O fluxo select → thread de decode → `mpsc` com geração → `poll` publica
`LoadState` é correto, e o prefetch leve (`decode_for_display` sem reter
full-res) mais o teto por bytes reais de RAM já aplicam o plano. Mas o módulo
mistura decode puro, contabilidade de cache e `ImageHandle` do Freya, e usa
nomes abreviados (`dec`, `pref`, `tx`, `rx`, `cur`). Preservar comportamento,
separar fronteira GUI e expandir nomes.

### `ThumbCache`

Worker único com fila + `ThumbMap` reativo é a arquitetura certa, mas
`THUMB_RADIUS` (±25) e `THUMB_CAP` (200) não derivam do viewport visível —
a galeria e o cache não compartilham a mesma fonte de verdade. Preservar o
worker, tornar o agendamento viewport-driven.

### `AppState` e canais Radio

A divisão em 6 canais (`Photos`, `Viewer`, `Edit`, `Status`, `Config`,
`Dialogs`) com `derive_channel` explícito é boa e deve ser preservada. Mas o
struct acumula ~40 campos (`sel`, `current`, `visible`, `photos`,
`crop_aspect`, `applied_filter`, flags de modal) e transições que tocam disco
(`start_scan`, `open_dir_path`) vivem ao lado de transições puras
(`toggle_compare`, `set_compare_split`). Preservar os canais, decompor o
struct e separar transições puras de efeitos.

### `AppConfig`

Preferências com `serde(default)` por campo e `save()` best-effort são
recuperáveis, mas não há versão/migração explícita nem escrita atômica
documentada. Preservar o formato, adicionar `version` e recovery.

## Refatorar fortemente ou substituir a estrutura

- `sel: Option<usize>` solto — introduzir `PhotoId`/`SelectedPhoto` newtype.
- `rotate(state, services, cw: bool)` — parâmetro booleano opaco; usar
  `RotationDirection`.
- ` CropRect { x, y, w, h }` — campos abreviados; expandir para nomes completos.
- `EditorState { rot, crop, adjust }` — campo `rot` abreviado.
- `image_store.rs` com `Rc<RefCell<Inner>>` + 20 campos — decompor em
  decode puro, cache e publicação.
- `state.rs` com transições de disco e puras no mesmo módulo — separar.
- `format_filter: usize` + `FORMAT_FILTERS: &[&str]` — índice mágico; usar enum.

O objetivo é preservar algoritmos e comportamento comprovado, não perpetuar
contêineres arquiteturalmente ruins.
