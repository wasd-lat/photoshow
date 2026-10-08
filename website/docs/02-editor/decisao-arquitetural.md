# Editor — decisão arquitetural

> **Status: aprovado**

## Responsabilidade

O Editor é o domínio puro da edição não-destrutiva. Ele não conhece GUI, GPU,
threads nem filesystem. Ele recebe pixels e estado, devolve pixels e estado.

```text
editor
├── EditorState          (rot + crop + adjust, Copy, barato para undo)
├── EditorStack          (history + future, merge de sliders)
├── CropRect             (recorte em px da imagem rotacionada)
├── apply_to_image       (preview em display)
└── bake                 (final em full-res, mesma matemática)

adjust
├── Adjust               (exposure, contrast, saturation, temperature)
├── adjust_pixels        (caminho único de preview e bake)
├── adjust_rgb           (temperatura → contraste → saturação, em gamma)
├── srgb_to_linear       (só a exposição usa, antes da curva)
└── Histogram            (64 faixas + blowout/shadow por pixel)

app/crop
├── Handle               (8 alças: Nw N Ne E Se S Sw W)
├── DragKind / DragKind2 (pan, crop New/Move/Resize, Compare)
├── resize_handle        (por alça, com aspect opcional)
├── clamp_to_bounds      (contido no desenho)
├── refit_aspect         (reaplica proporção preservando centro)
└── crop_to_preview_px   (tela → px de preview, None se < 8px)
```

A camada **Application** (`app/state.rs`) coordena: `rotate`, `apply_crop`,
`refresh_preview`, `undo`, `redo`, `reset_edits`, `toggle_compare`. A camada
**Media** (`image_store.rs`) publica o preview na GPU. A **UI** (`viewer.rs`,
`adjust_panel.rs`) só produz gestos e desenha.

> Editor decide o que a edição significa. Application decide quando aplicar. Media decide como chega à GPU. UI só mostra.

## Diagnóstico atual

Levantado sobre o código real (`editor.rs`, `adjust.rs`, `app/crop.rs`,
`app/adjust_panel.rs`, `app/state.rs`, `app/services.rs`, `app/viewer.rs`).

### Reutilizar

**Pilha não-destrutiva.** `EditorStack` com `history` + `future`, `commit`
que normaliza `rot %= 4` e descarta no-op via `same()` tolerante a ruído de
`f32`, `commit_adjust` que funde arrastos de slider num único passo de undo.
`is_dirty` deriva de `is_clean` — fonte única, sem `bool` competindo.

**Uma matemática só.** `apply_to_image` (preview) e `bake` (save) chamam o
mesmo `adjust_pixels`. Exposição em linear (`srgb_to_linear` oficial, não
`powf(2.2)`), resto em gamma, ordem fixa exposição → temperatura → contraste
→ saturação, alpha nunca tocado. Teste cobre ida-e-volta de ±1 EV e
`clean_adjust_is_the_identity`.

**Crop puro e testado.** `app/crop.rs` sem GUI: 8 alças com ponto e âncora
oposta, `resize_handle` por borda/canto com aspect opcional, `clamp_to_bounds`,
`refit_aspect` preservando centro, `crop_to_preview_px` com `CROP_MIN_PX = 8.0`.
Rotacionar com crop ativo transforma o rect junto (`rot_rect_cw/ccw`).

**Histograma sem passe extra.** `image_handle_with_histogram` tira handle GPU
e histograma do mesmo RGBA; amostragem em grade (~100k amostras) corta o
custo proporcional à área; clipping conta pixel, não canal; `peak()` normaliza
pela barra mais alta, não pelo total.

### Reutilizar com refatoração

**Nomes abreviados.** `CropRect { x, y, w, h }`, `EditorState { rot }`,
`dims()`, `bake(full, display_base, st)`, `rot_rect_cw(_w, h, r)`. Preservar
geometria e testes, expandir para `origin_x`, `rotation_quarter_turns`,
`rotated_dimensions`, `editor_state`.

**`bool cw` opaco.** `rotate(state, services, cw: bool)` e `rotate_cw` /
`rotate_ccw` duplicam o eixo de direção. Preservar comportamento, introduzir
`RotationDirection { Clockwise, CounterClockwise }` e um único `rotate`.

**`Handle` com nomes de bússola.** `Nw N Ne E Se S Sw W` são linguagem de
domínio consolidada (como UV/RGBA), mas `DragKind2` com `2` no nome é
inventado e `CursorPoint` vaza abstração de UI para o módulo puro. Preservar
as alças, renomear `DragKind2` para `CropDrag` e mover `DragKind` (que mistura
pan de viewer com crop) para a camada Application.

**Comparador em dois lugares.** A metade "antes" nasce em
`image_store::compare_base` (mesma geometria, cor neutra) e o recorte nasce em
`viewer::compare_rect` (fração da largura desenhada da foto, não da janela,
com `SPLIT_MIN/MAX` anti-zero). Funciona e é testado, mas a regra "geometria
igual, cor zerada" mora no Media e a regra "divisória segue a foto" mora na
UI. Preservar as duas, documentar o contrato no Editor: `compare_base(geom)`
é função pura do domínio, não detalhe do store.

### Refatorar fortemente ou substituir a estrutura

- `format_filter: usize` + `crop_aspect: usize` como índices mágicos em
  `ASPECT_OPTIONS` / `FORMAT_FILTERS` — virar enums.
- `adjust_panel.rs` lê `edit.read().editor.state().adjust` e escreve via
  `state::update` — correto, mas `ADJUST_SLIDERS` como `&[(&str, AdjustField,
  f32)]` mistura rótulo, campo e máximo num triplete posicional.
- Nenhum ajuste é local (máscaras, curva por canal, tonemap) — continuar
  assim por decisão, não por falta de lugar: o conjunto de quatro é fechado.

## Decisões aprovadas

1. `EditorState` permanece `rot + crop + adjust`, `Copy`, com `is_clean`
   como única fonte de "há edições".
2. Preview e bake chamam a mesma função de ajuste — "o que se vê é o que se
   salva" é invariante testada, não slogan.
3. Ajuste entra **depois** do crop: eixos independentes (geometria × cor), e
   ajustar depois economiza o passe de cor sobre pixels descartados.
4. Um arrasto de slider vira **um** passo de undo (fusão em `commit_adjust`
   com `eq_approx`), não um por pixel do mouse.
5. Crop vive no espaço de pixels da imagem **já rotacionada**; rotacionar
   transforma o rect junto; bake escala display → full por um fator.
6. Seleção de crop abaixo de 8px é `None` — gesto fantasma nunca vira estado.
7. Comparador mantém geometria e zera só a cor; divisória é fração da foto
   desenhada com respiro 2%–98%.
8. Histograma sai do mesmo buffer que vai para a GPU; sem segundo passe.
9. O painel de ajustes não guarda estado de edição: lê e escreve no
   `EditorStack`, que é a fonte da verdade.
10. Conjunto de ajustes é fechado em quatro (exposição, contraste, saturação,
    temperatura). Curva por canal, máscaras e tonemap pertencem a outro
    domínio futuro, não a este.

## Consequências

- `CropRect` ganha nomes completos sem mudar semântica nem quebrar `bake`.
- `rotate` ganha `RotationDirection`; `rotate_cw/ccw` viram detalhe interno.
- `DragKind2` vira `CropDrag`; `DragKind` (pan + crop + compare) sobe para
  Application, onde o viewer o consome.
- `compare_base` vira função pura documentada do Editor; Media só chama.
- `ASPECT_OPTIONS` e `FORMAT_FILTERS` viram enums antes de qualquer feature
  nova de crop ou filtro.

## Regra anti-bloat

Não criar pipeline de preview raster/GPU separado, shader de ajuste,
máscaras, edição local, curva por canal ou tonemap neste domínio. Quando a
0.4 pedir, será um Preview Pipeline formal em domínio próprio — não uma
extensão deste.
