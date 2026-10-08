# Application — decisão arquitetural

> **Status: aprovado**

## Responsabilidade

A Application coordena o funcionamento do visualizador sem conhecer widgets,
GPU, janela ou APIs de plataforma. Ela recebe intenções (selecionar, filtrar,
editar, salvar), aplica transições de estado e delega pixels ao Media e
persistência aos serviços.

```text
application
├── AppState            (dados puros, sem GPU/thread)
├── AppChannel          (Photos/Viewer/Edit/Status/Config/Dialogs)
├── derive_channel      (quem re-renderiza quando)
├── transições puras    (select, step, filter, sort, crop_mode, compare)
├── transições com IO   (start_scan, open_dir_*, apply_scan_result, rename)
├── cli::Target         (resolve puro + open_target efeito, argv = diálogos)
├── Services            (scan/save/pump/thumbs via background, nunca spawn)
└── update/update_on    (escrita por canal, sem hook em handler/task)
```

O **Media** entrega pixels. O **Editor** decide o que a edição significa. A
**Persistência** guarda config, sidecar e arquivos. A **UI** traduz gesto em
chamada de transição e assina canais.

> Application determina como o visualizador funciona. Media entrega pixels.
> Editor transforma. UI apresenta.

## Diagnóstico atual

Levantado sobre o código real (`app/state.rs` ~1160 linhas, `app/services.rs`,
`app/mod.rs`, `cli.rs`).

### Reutilizar

**Canais Radio com derivação explícita.** Seis canais (`Photos`, `Viewer`,
`Edit`, `Status`, `Config`, `Dialogs`) e `derive_channel` documentando quem
acorda quem (Edit → Viewer + Status, Viewer → Edit + Status, Dialogs → Photos
+ Viewer). Cada painel re-renderiza só o que consome. Classificação: REUSE.

**`cli::resolve` puro + `open_target` efeito.** `Target::Folder | Selection |
Files` com regras ordenadas e testadas (pasta sozinha, foto existente abre a
pasta dela, foto inexistente vira solta por extensão, mistura nunca é pasta).
A mesma decisão serve `argv`, `%F` do `.desktop` e diálogos — antes o `argv`
era ignorado e a toolbar tinha cópia da lógica. Classificação: REUSE.

**Regra `background`, nunca `spawn`.** `services::background` (`spawn_forever`)
documenta por que task escopada morre com o menu que a criou. Pump adaptativo
(16ms ocupado / 125ms ocioso, testado por razão ≥ 4× e teto 150ms),
`subscribe_results` para acordar a raiz, `take()` que só notifica quando há o
que retirar (sem laço de repaint). Classificação: REUSE.

**Scan com geração e preservação.** `scan_seq` descarta resultado obsoleto,
`preserve_on_scan` mantém a foto no rescan, `apply_scan_result` ordena,
recarrega sidecar e reseleciona. `recompute_visible` preserva a seleção se a
foto continua visível, senão seleciona a primeira. Classificação: REUSE.

### Reutilizar com refatoração

**`AppState` com ~40 campos.** Fotos (`photos`, `visible`, `sel`, `current`),
árvore (`tree`, `current_dir`, `scanning`, `scan_seq`), edição (`editor`),
viewer (`zoom`, `offset`, `crop_mode`, `crop_aspect`), modos (`fullscreen`,
`maximized`, `compare`, `slideshow_active`, `presentation_mode`), cinco
filtros, ordenação, busca, sidecar, EXIF, quatro flags de modal, status/saving,
`applied_filter`, `rename_buf`. Preservar os canais, decompor em
`PhotoCollection` (lista + visível + seleção), `ViewerSession` (zoom/pan/crop/
modos), `FilterSet` (cinco filtros + busca + sort) — sem mudar o que cada
canal observa.

**Transições puras e com IO no mesmo módulo.** `toggle_compare`,
`set_compare_split`, `step`, `crop_ratio` (puras) ao lado de `start_scan`,
`open_dir_path`, `apply_scan_result`, `apply_rename` (precisam de `Services`
ou disco). Preservar comportamento, separar em `transitions.rs` (puras,
testáveis sem mock) e `effects.rs` (recebem `Services`).

**Nomes abreviados.** `sel`, `cur`, `st`, `cfg`, `st.editor.rotate_cw(base)`.
Preservar transições, expandir na decomposição (`selected_index`,
`current_photo`, `application_state`).

**Índices mágicos.** `format_filter: usize` + `crop_aspect: usize` indexando
`FORMAT_FILTERS` / `ASPECT_OPTIONS`. Preservar opções, virar enums
(`FormatFilter`, `CropAspect`) com `None = livre` explícito.

### Refatorar fortemente ou substituir a estrutura

- `state::update` vs `update_on`: dois caminhos de escrita porque tasks
  `ROOT` não enxergam o contexto — correto, mas indocumentado fora de um
  comentário; virar contrato do domínio com teste de canal.
- `open_dir_and_select` e `open_subdir` triplicam construção de `DirNode`
  raiz + sidecar + scan; extrair `open_directory(state, services, dir,
  preserve)`.
- `delete_current_photo` mistura lixeira, `retain`, recompute, reseleção e
  status em 30 linhas; quebrar em `remove_from_collection` + `reselect_after_
  removal`.
- `recompute_visible` aloca `to_lowercase` por foto por filtro; pré-computar
  chave minúscula ou `sort_by_cached_key` também no filtro de busca.

## Decisões aprovadas

1. Seis canais permanecem; `derive_channel` é o contrato de re-render, não
   detalhe de implementação.
2. Transições puras nunca recebem `Services`; efeitos nunca moram em função
   pura. A assinatura da função diz de que lado ela está.
3. `Target::Folder | Selection | Files` é a única porta de entrada externa;
   `argv`, `.desktop` e diálogos convergem para `open_paths`.
4. Foto solta abre a pasta dela com seleção (`Selection`), não lista de um
   item — navegar as vizinhas é o comportamento esperado.
5. Scan obsoleto é descartado por geração; rescan preserva a seleção quando
   possível; filtro preserva a foto atual quando ela continua visível.
6. `select_photo` reinicia tudo (editor limpo, crop fechado, zoom 1, offset
   0, EXIF relido) — trocar de foto nunca herda estado da anterior.
7. Pump adaptativo com razão ocioso/ocupado testada; `take()` nunca notifica
   em vazio.
8. Todo trabalho que sobrevive ao clique usa `background`; `spawn` fica só
   para o descartável com o componente.
9. `AppState` decomposto em coleção / sessão de viewer / filtros — canais
   observam as mesmas fatias de antes.
10. Índices mágicos viram enums antes de qualquer filtro ou aspecto novo.

## Consequências

- `state.rs` dividido em `state.rs` (tipos + canais) + `transitions.rs`
  (puras) + `effects.rs` (com `Services`/disco).
- `select_photo`, `step`, `recompute_visible` ganham nomes completos sem
  mudar semântica.
- `open_directory` único elimina a triplicação de raiz + sidecar + scan.
- Contrato `update`/`update_on` documentado como regra do domínio.

## Regra anti-bloat

Não criar command bus genérico, Redux/MVU, event bus, DI container, ECS,
nem workspace multi-crate. Seis canais Radio + funções de transição + um
`Services` resolvem o escopo atual. Comandos tipados (`AppCommand`) só entram
se um painel começar a escrever em três canais à mão — hoje nenhum faz isso.
