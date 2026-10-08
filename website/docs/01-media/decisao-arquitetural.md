# Media — decisão arquitetural

> **Status: aprovado**

## Responsabilidade

O Media é o runtime de pixels. Ele decodifica, orienta, reduz, publica e
pré-carrega — sem conhecer edição, seleção, filtros ou qualquer widget. Ele
recebe caminhos e devolve handles GPU mais histogramas.

```text
media
├── decode            (decode_photo, decode_for_display, decode_thumb)
├── orientation       (exif::read_orientation + apply_orientation)
├── display           (DISPLAY_MAX_DIM = 2048, thumbnail THUMB_MAX = 160)
├── ImageStore        (foto atual: select → thread → poll → LoadState)
├── ThumbCache        (vizinhança: worker único → ThumbMap)
├── prefetch          (PrefetchedPhoto leve, teto por bytes reais de RAM)
└── publish           (LoadState: Empty/Loading/Loaded/Failed, dirty-gated)
```

A camada **Editor** (`editor.rs`, `adjust.rs`) decide o que a edição significa.
A camada **Application** (`app/state.rs`, `app/services.rs`) decide quando
selecionar, pré-carregar e salvar. A **UI** só assina `LoadState` e
`ThumbMap`.

> Media entrega pixels. Editor decide o que a edição significa. Application
> decide quando. UI só mostra.

## Diagnóstico atual

Levantado sobre o código real (`image_store.rs`, `thumbs.rs`, `exif.rs`,
`app/services.rs`).

### Reutilizar

**Decode puro e testado.** `decode_photo` (full + display) e
`decode_for_display` (só display + `full_size`) são funções puras: abrem,
decodificam, aplicam orientação, reduzem para `DISPLAY_MAX_DIM`. Sem GUI, sem
thread, sem estado — testáveis com PNG sintético.

**Prefetch leve com teto real.** `PrefetchedPhoto` guarda só display +
`full_size`, nunca full-res na RAM das vizinhas. Evicção por contagem
(`PREFETCH_CAP = 4`) **e** por bytes reais (`ram_bytes = w × h × 4`,
`PREFETCH_BYTES_CAP = 256 MiB`). O guarda de entrada por tamanho de arquivo
continua como filtro barato antes do decode.

**Cancelamento por geração.** `select` incrementa `current_id` e publica em
`current_gen` atômico; a thread confere antes **e** depois do decode, e o
`poll` descarta `msg.id != current_id`. Navegação rápida nunca mostra foto
velha nem converte pixels obsoletos para a GPU.

**EXIF em duas metades explícitas.** `read_orientation`/`apply_orientation`
(corrigir na entrada, com fallback 1) separados de `ExifMeta::rebased` +
`embed` (preservar câmera/data/GPS na saída, com `Orientation` normalizada
para 1 e dimensões do arquivo novo). `embed` só age em JPEG/WebP, só no
temporário ainda não publicado, e recusa lixo sem corromper — tudo testado
(inclusive os 8 orientations com grade rotulada).

**Thumbs com worker único.** Um thread, uma fila, `JOBS_PER_FRAME = 12` por
poll para não sufocar a UI, candidatos baratos primeiro (ordem por tamanho de
arquivo), `THUMB_CAP = 200` com `retain` na janela, `failed` separado de
`queued` para não retentar lixo. `window_range` pura e testada nas bordas.

**Publicação dirty-gated.** `publish` devolve `None` quando nada mudou —
reconstruir `ImageHandle` custa uma cópia para a GPU e não pode acontecer a
cada tique do pump. `poll` só escreve no `State` quando o valor difere
(`*load.peek() != next`).

### Reutilizar com refatoração

**`Inner` com 20 campos.** `image_store.rs` mistura decode, cache de prefetch,
geometria base (`base_px` vs `display_px`), comparador (`compare_on` +
`compare_geom`), erros e flags de publicação num único `Rc<RefCell<Inner>>`.
Preservar comportamento e testes, decompor em três peças: decode puro (já
existe), cache de vizinhança, publicação da foto atual.

**Fronteira GUI no meio do Media.** `image_handle`, `LoadState` com
`ImageHandle` dentro, `poll(load: State<LoadState>)` e `apply_preview(...,
load: State<LoadState>)` amarram o runtime ao Freya. O módulo diz "o domínio
não conhece a GUI", mas o `State` do Freya é parâmetro de quatro funções.
Preservar o fluxo, extrair um `DisplayFrame` neutro (bytes RGBA + dims) e
deixar a conversão para `ImageHandle` num adapter fino na fronteira UI.

**`full_image` clona e decodifica no getter.** O caminho feliz clona a
full-res retida; o caminho prefetch decodifica de forma síncrona dentro de um
getter chamado pela UI. Preservar o lazy, mas mover o decode sob demanda para
o worker de save (`start_save` já recebe `full` por valor) em vez de esconder
I/O num acessor.

**Thumbs fora do viewport.** `THUMB_RADIUS = ±25` ao redor da **seleção**, não
do que está visível na galeria; `THUMB_CAP` despeja por janela de seleção. A
galeria e o cache não compartilham a mesma fonte de verdade. Preservar o
worker, tornar o agendamento viewport-driven (a janela visível vira
prioridade 0, a seleção prioridade 1).

**Guarda de prefetch por arquivo comprimido.** `ensure_prefetched` pula por
`metadata.len() <= max_file_bytes` — um JPEG pequeno pode virar centenas de
MB decodificados. O teto de saída já é por bytes reais; o filtro de entrada
continua aproximado. Preservar como filtro barato, documentar que a autoridade
é o teto de RAM, não o tamanho no disco.

### Refatorar fortemente ou substituir a estrutura

- `compare_base` mora no Media mas implementa regra do Editor (mesma
  geometria, cor neutra) — mover para o Editor como função pura, Media só
  chama (já decidido no 02-editor, registrar aqui o outro lado).
- `select` faz reset de 10 campos, consulta prefetch, publica geração e
  dispara thread — cinco responsabilidades num método; quebrar em
  `begin_selection` + `consume_prefetch` + `spawn_decode`.
- `PrefetchMsg { result: Option<...> }` com `None` silencioso — prefetch que
  falha some sem rastro; distinguir `Failed` de `Skipped` para o scheduler
  não retentar lixo nem pular foto legível.
- Nomes abreviados: `dec`, `pref`, `tx`, `rx`, `cur`, `pre_tx`, `pre_rx`,
  `lo/hi`, `sel` — expandir na decomposição.

## Decisões aprovadas

1. Decode é função pura (`Path → DecodedPhoto`); thread, geração e `State`
   vivem na borda, nunca dentro do decode.
2. A foto atual publica `LoadState` dirty-gated; nada reconstrói handle GPU
   sem mudança real.
3. Vizinhas viajam leves (`PrefetchedPhoto`: display + `full_size`); full-res
   de vizinha nunca mora na RAM.
4. Teto de prefetch por bytes reais decodificados (contagem + RAM); tamanho
   de arquivo é só filtro barato de entrada.
5. Seleção interativa sempre vence prefetch: geração cancela decode obsoleto
   antes e depois do trabalho, e `poll` descarta resultado velho.
6. Preview sempre deriva de `base_display`, nunca do preview anterior —
   aplicar duas vezes o mesmo estado dá o mesmo resultado.
7. `display_base_dims` guarda a geometria estável separada da imagem mutável;
   rotate/crop/bake nunca derivam contra dims transformadas.
8. Comparador publica `base_image` só quando ligado; toggle é só flag +
   dirty, sem upload especulativo para a GPU.
9. EXIF: corrigir na entrada (orientação física), preservar na saída
   (câmera/data/GPS com `Orientation = 1` e dims novas), só em JPEG/WebP,
   só no temporário.
10. Thumbs: worker único, baratos primeiro, `12` jobs por poll, `failed`
    separado de `queued`, invalidação explícita após overwrite.
11. Fronteira GUI: Media produz `DisplayFrame` neutro; adapter fino converte
    para `ImageHandle`. Nenhuma decisão de produto mora no adapter.
12. `full_image` síncrona sai do caminho da UI; save carrega full-res no
    worker, nunca num getter.

## Consequências

- `Inner` decomposto em decode / cache de vizinhança / publicação — sem mudar
  `LoadState` observado pela UI.
- Adapter `DisplayFrame → ImageHandle` na fronteira UI; `image_handle` e
  `image_handle_with_histogram` mudam de dono (Media → adapter), não de
  comportamento.
- `compare_base` muda de dono (Media → Editor) como função pura.
- Scheduler de thumbs passa a receber a janela visível da galeria; seleção
  vira prioridade 1, não centro da janela.
- `start_save` passa a ser o único caminho que materializa full-res sob
  demanda; `full_image` some ou vira detalhe do worker.

## Regra anti-bloat

Não criar pool de threads genérico, executor async, fila com prioridades
arbitrárias, cache em disco de thumbs, nem decoder próprio. Thread por
seleção + worker único de thumbs + `mpsc` + geração atômica resolvem o
escopo atual. Tokio continua fora até necessidade demonstrada.
