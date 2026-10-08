# Application — organização e classificação

> **Status: proposta**

Novas capacidades de triagem que estendem o `FolderSidecar`, o `FilterSet` e
o `BatchConfig` sem mudar a arquitetura: tudo continua portátil (JSON na
pasta), atômico e sem banco escondido.

Nada aqui altera `decisao-arquitetural.md` — aquele documento permanece a
autoridade do que já foi aprovado. Cada item abaixo só vira arquitetura
quando discutido e aprovado.

## Pick/reject (flag ternário)

Mesmo padrão de `ratings`: um campo a mais no sidecar, zero no JSON remove a
chave.

```rust
pub enum PickFlag { None, Pick, Reject }
// get_flag / set_flag / filtro flag_filter, como rating_filter
```

"Mostrar só picks", "esconder rejects" — um filtro a mais em
`recompute_visible`, sem migração, sem mudar o modelo `sel: Option<usize>`.

## Ordenar por rating

Uma variante a mais no `SortCriteria` existente:

```rust
pub enum SortCriteria { Name, Date, Size, Rating }
```

Lê o sidecar na chave de ordenação (`sort_by_cached_key` como as outras).
Uma linha de sort + um item no dropdown — sem índice novo, sem cache novo.

## Filtros salvos / coleções

O `FilterSet` (formato + busca + estrelas + cor + tag + sort) já existe no
estado; persistir combos nomeados no config é o mesmo padrão dos presets de
ajuste:

```rust
pub struct SavedFilter {
    pub name: String,
    pub format_filter: FormatFilter,
    pub search_query: String,
    pub min_rating: u8,
    pub color_id: u8,
    pub tag: Option<String>,
    pub sort: (SortCriteria, bool),
}
```

"Verão 2025 ★4+" vira um clique. Aplicar = preencher os campos + um
`recompute_visible`. Sem query language, sem DSL — struct nomeada, como
`AdjustPreset`.

## Marcar o conjunto visível

Sem multi-seleção (trocar `sel` por conjunto contaminaria seleção, preview,
batch e galeria — decisão própria, fora deste documento):

```text
filtrar 40 fotos → "marcar visíveis com ★4" → loop de set_rating + um save
```

Reutiliza `recompute_visible` + `set_rating`. Resolve o fluxo de triagem
(marcar a leva de uma vez) sem mudar o modelo de seleção.

## Duplicatas por hash exato

Passe em background (padrão `Services`), hash exato por arquivo, agrupa no
browser:

```text
scan → hash dos candidatos (mesmo tamanho primeiro, barato antes) → grupo "7 duplicatas"
```

Sem dependência nova (hash std), sem hash perceptual (esse exigiria ML e
vira bloat com falsos positivos). Exato = mesma foto, decisão segura:
mostrar lado a lado e deixar o usuário apagar. Médio, mas encaixa no pump
existente.

## Rename com `{date}` do EXIF

O `BatchConfig.name_pattern` já substitui `{i}`/`###`; o `ExifDetails` já
extrai data. Só mais um token:

```text
"viagem_{date}_{i}" → "viagem_20250814_001.jpg"
```

Sem data (PNG sem EXIF) = string vazia, documentado — nunca falha o rename
por falta de metadado.

## Zoom 100% (pixels reais)

O `compute_draw` já resolve área + dims + zoom; falta só a transição pura
que fixa o multiplicador no inverso do fit:

```rust
pub fn zoom_to_actual_pixels(viewer: &mut ViewerSession, draw: ScreenRect, dims: (u32, u32))
```

Um atalho (`Ctrl+0`? não — ocupado pelo layout; propor `Ctrl+Shift+0` ou
duplo-propósito no `0`), sem pipeline novo. Conferir nitidez real antes de
salvar é o caso de uso.

## Ordem proposta

1. Pick/reject (mesmo molde do rating, isolado).
2. Ordenar por rating (uma variante, um dropdown).
3. Marcar visíveis (compõe filtro + rating existentes).
4. Filtros salvos (persiste o que o filtro já expressa).
5. Zoom 100% (uma transição pura).
6. Rename `{date}` (um token no padrão existente).
7. Duplicatas (único item médio — exige passe background novo).

## Regra anti-bloat

Fora desta lista, continua proibido: multi-seleção com `Ctrl+clique` (exige
decisão própria sobre o modelo de seleção), reconhecimento facial, tagging
com IA, geolocalização com mapa, escrever rating no EXIF do original (viola
"sidecar nunca toca no original"), catálogo com banco, nuvem/sync.
