# Code Agents — estratégia de testes

> **Status: aprovado**

## Diagnóstico atual

Medido em 2026-10-08 (`cargo test --locked`, máquina do mantenedor):

| Alvo | Testes | Tempo | O que é |
|---|---|---:|---|
| lib unit | 162 | 3.9s | funções puras + transições + geometria |
| menu_task | 4 | 3.1s | janela headless real (`freya_testing`) |
| pipeline | 30 | 4.4s | varredura → decode → bake → disco, sem janela |
| doc-tests | 0 | 0s | nenhum doc-test no projeto |

Total ~11s. Compilação incremental sem mudança: 0.25s — o incremental
funciona, o custo está em recompilar o mundo quando `src/` muda (Freya +
Skia na árvore) e em fixtures redundantes.

Gargalos reais, não imaginados:

1. **Fixtures gigantes repetidas.** `pipeline.rs` escreve PNG 3000×2000,
   800×600 e 640×480 por teste via `from_fn` pixel a pixel; `image_store.rs`
   repete 3000×2000. Cada teste paga encode + decode + disco do zero.
2. **Três PNGs 4×2 escritos em disco** em `image_store.rs` para testar
   `display_base_dims` — lógica que nunca toca disco, mas paga `tempdir` +
   PNG 3 vezes.
3. **Cobertura zero em 6 módulos de UI** (`dialogs`, `toolbar`, `gallery`,
   `clipboard`, `window`, `mod`) contra 27 testes em `viewer.rs` — a
   pirâmide é torta: o que é puro não é testado onde nasceu, o que é UI é
   testado via geometria pura onde dá.
4. **Sem `cargo-nextest`.** O runner padrão roda alvos em sequência
   (lib → menu_task → pipeline); cada alvo relinka o binário Freya.
5. **`freya-testing` como dev-dependency normal** — todo `cargo test --lib`
   recompila o harness headless mesmo sem rodar `menu_task`.

## Decisões aprovadas

1. **Fixtures por tamanho, uma vez.** `decoded_sample(w, h)` em memória
   para lógica (já existe no pipeline); disco só onde o teste é sobre disco
   (save, scan, EXIF). PNG 3000×2000 nasce uma vez por arquivo de teste
   (`OnceLock`), não uma vez por `#[test]`.
2. **Lógica sem disco não toca disco.** `display_base_dims`,
   `preview_always_starts_from_base`, `changing_photo_resets_base_dims`
   usam `install_sized` em memória (já existe) — os 3 PNGs 4×2 somem.
3. **`cargo-nextest` no CI e no CONTRIBUTING.** Alvos em paralelo,
   binário por teste isolado, retry de flaky separado de falha real.
   Comando canônico vira `cargo nextest run --locked`.
4. **Harness headless atrás de feature.** `freya-testing` vira
   `dev-dependency` opcional (`headless`); `cargo test --lib` para de
   recompilar a janela. `menu_task` roda com `--features headless`.
5. **Regra de cobertura por camada** (não por porcentagem global):
   - domínio puro (`adjust`, `editor`, `exif`, `fs_browser`, `config`,
     `cli`): toda função pública com teste de unidade;
   - transições puras (`transitions.rs` quando existir): tabela de
     entrada → estado;
   - geometria (`crop.rs`, `compute_draw`, `compare_rect`): propriedade
     (simetria, idempotência, contenção), não só exemplo;
   - efeitos (`save_baked`, `scan_blocking`, `embed`): caminho até o disco
     com tempdir + falha injetada;
   - UI (`dialogs`, `toolbar`, `gallery`): zero-teste só para o que é
     montagem; lógica extraída nasce testada (regra do 03-application).
6. **Teste lento declara o custo.** `#[ignore]` + grupo `slow` para o que
   passa de 1s (TIFF 200MB+, pasta 50k sintética quando existir); CI rápido
   por PR, suite pesada nightly/release — como o 06-release já manda.
7. **Sem doc-test obrigatório.** 0 doc-tests hoje; exemplos na doc viram
   teste só quando a API é pública e estável — doc-test quebrado por refactor
   interno é custo sem usuário.

## Consequências

- `tests/pipeline.rs` perde ~40% do tempo (fixtures compartilhadas +
  tamanhos mínimos por caso).
- `src/image_store.rs` perde os 3 PNGs de 4×2 (memória direta).
- `Cargo.toml`: `freya-testing` opcional + `nextest` no CI.
- CONTRIBUTING documenta `cargo nextest run --locked` e o grupo `slow`.

## Regra anti-bloat

Sem mock framework, sem fixture factory genérica, sem snapshot testing de
UI, sem cobertura percentual como gate (100% mente — cobre montagem e perde
invariante). Pirâmide torta se conserta extraindo lógica, não testando
widget.
