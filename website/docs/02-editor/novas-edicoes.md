# Editor — novas edições

> **Status: proposta**

Novos ajustes e operações geométricas que estendem o `EditorState` sem mudar
a arquitetura: tudo continua global, puro, com preview = bake, um arrasto =
um undo, e conjunto fechado por decisão (este documento propõe a próxima
lista fechada).

Nada aqui altera `decisao-arquitetural.md` — aquele documento permanece a
autoridade do que já foi aprovado. Cada item abaixo só vira arquitetura
quando discutido e aprovado.

## Flip horizontal/vertical

Mesmo eixo de rotate/crop: geometria pura, sem pipeline novo.

```rust
pub struct EditorState {
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    // rot, crop, adjust: como hoje
}
```

`apply_to_image` e `bake` aplicam o espelho junto da rotação, antes do crop —
ordem fixa como a dos ajustes. Crop com flip ativo transforma o rect junto,
igual ao `rot_rect_cw/ccw`. Teste espelhado de `rotate_moves_crop_with_it`.

## Copiar/colar ajustes

O `EditorState` já é `Copy`:

```text
copiar  → guarda EditorState na área de transferência interna
colar   → commit(state_colado) na foto atual
```

Zero pipeline novo. Resolve "mesmo look em 20 fotos" sem lote. Colar entra
na pilha de undo como qualquer operação — `Ctrl+Z` desfaz o colar, não a
foto inteira.

## Ajustes em lote

O `process_single` do batch já decodifica e grava atômico; falta só aplicar
o `Adjust` copiado antes do `save_baked`:

```text
decode_photo → rotate/flip → adjust_pixels(adjust_colado) → crop? → save_baked
```

É o colar-ajustes em escala, sem segunda implementação de save (regra do
06-release). O `BatchConfig` ganha `adjust: Option<Adjust>` — `None` mantém o
comportamento atual.

## Presets de ajuste

`Adjust` nomeado persistido no `config.rs`, mesmo padrão de `named_tags` e
`color_tags`:

```rust
pub struct AdjustPreset {
    pub name: String,
    pub adjust: Adjust,
}
```

Salvar, listar, aplicar (`commit_adjust`), remover. Sem UI nova além de um
dropdown no painel — e o painel continua sem estado próprio (lê/escreve no
`EditorStack`).

## Vinheta radial

Função pura no buffer, um campo a mais:

```rust
pub struct Adjust {
    pub vignette: f32, // 0..=1, 0 = desligada
}
```

Corre no mesmo passe de `adjust_pixels` (escurece bordas por distância ao
centro), entra no `is_clean`/`eq_approx`/`clamped` como os outros quatro.
Barata porque não cria passe novo.

## Auto-níveis a partir do histograma

O histograma já existe de graça no `LoadState`:

```rust
pub fn auto_adjust(histogram: &Histogram) -> Adjust
```

Estica exposição/contraste pelos percentis (corta 0,5% em cada ponta).
Proposta com cautela: semântica de "auto" precisa do mesmo rigor dos outros
— teste de ida-e-volta como o de ±1 EV, e resultado sempre editável depois
(auto sugere, nunca trava: vira `commit_adjust` normal).

## Ordem proposta

1. Flip (geométrico, isolado, teste espelhado do rotate).
2. Copiar/colar (só estado, sem pipeline).
3. Lote com ajustes (compõe os dois anteriores no `process_single`).
4. Presets (persiste o que o colar já transporta).
5. Vinheta (mais um campo no passe existente).
6. Auto-níveis (exige definir "correto" antes de codar).

## Regra anti-bloat

Esta lista é a próxima fronteira fechada. Fora dela, continua proibido:
máscaras, ajustes locais, curva por canal, tonemap, denoise com IA, nitidez
adaptativa, correção de lente por banco de dados. Quando a 0.4 pedir Preview
Pipeline formal, será domínio próprio — não extensão deste documento.
