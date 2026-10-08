# Release — decisão arquitetural

> **Status: aprovado**

## Responsabilidade

O Release prova que o software instala, abre, edita e salva — em Linux e
Windows, do zero, sem terminal. Ele não inventa número bonito: mede, congela
budget e barra regressão no CI.

```text
release
├── ci.yml        (fmt + clippy -D warnings → test linux/windows → build)
├── release.yml   (tarball + deb + rpm + zip windows + sha256)
├── install.sh    (one-line linux, sem root)
├── tests/        (pipeline 30 + menu_task 5 headless + 160 unit)
├── batch.rs      (0.5: fila em thread, cancelamento, relatório por arquivo)
└── ROADMAP.md    (0.1→1.0 entregue; 2.0 RAW como conjectura com orçamentos)
```

O **Media** tem budgets de RAM. O **Editor** tem invariante preview = bake. A
**Persistência** tem atomicidade. O **Release** transforma tudo isso em gate
automático.

> Afirmação sem evidence é proposta. Release só sai com gate verde.

## Diagnóstico atual

Levantado sobre o código real (`.github/workflows/ci.yml`,
`release.yml`, `install.sh`, `tests/pipeline.rs`, `tests/menu_task.rs`,
`src/batch.rs`, `ROADMAP.md`, `Cargo.toml`).

### Reutilizar

**CI em push/PR com gate real.** `lint` (fmt --check + clippy
`--all-targets --all-features -D warnings`) barra antes dos testes; `test`
roda em Linux **e** Windows com `--locked` mais build release. Sem Mac —
sem promessa de suporte, sem matriz fantasma. REUSE.

**Release multi-formato com checksum.** Tarball + `.deb` + `.rpm` + zip
Windows, tudo com SHA-256 computado no workflow. `install.sh` one-line sem
root. Metadados `deb`/`rpm` no `Cargo.toml` (dependência `libgl1` para a
pilha Skia/texto no Linux). REUSE.

**Dois níveis de teste de integração.** `pipeline.rs` (30 testes: varredura
→ decode → edição → bake → gravação → releitura, sem janela) prova o caminho
até o disco; `menu_task.rs` (5 testes headless via `freya_testing`: janela
real sem display) prova o que só existe com janela (hit-testing, ciclo de
task `background` vs `spawn`). 160 unit tests cobrem cada módulo puro.
REUSE a pirâmide; ver lacunas abaixo.

**Batch reutilizando o save.** `batch.rs` (0.5) não é segunda implementação:
decodifica (`decode_photo`), transforma, grava atômico (`save_baked`), em
thread com `AtomicBool` cooperativo e relatório sucesso/falha por arquivo.
`BatchProgress` via canal, modal acessível no Arquivo. REUSE o padrão para
qualquer fila futura.

**Roadmap como contrato de escopo.** 0.1→0.5 entregues com checkboxes reais;
1.0 com critério de saída ("instalável em 1 comando, sem terminal"); 2.0 RAW
como conjectura com 4 orçamentos duros (pasta abre tão rápido quanto JPEG,
thumb usa preview embutido, decode total só na atual, binário padrão +0
bytes) e lista explícita de FORA. `photoraw` separado por decisão para o
photoshow não virar Lightroom. REUSE como modelo de escopo.

### Reutilizar com refatoração

**Sem teste de installer.** `install.sh` corrigido mas sem smoke automatizado
(tarball → HOME temporário → `--version` → `.desktop`/ícone → cleanup).
Preservar o script, adicionar o job — é o P0 que o plano original já cobrava.

**Sem baseline de performance arquivada.** Orçamentos existem como intenção
(50k arquivos, TIFF 200MB+, `DISPLAY_MAX_DIM`, tetos de prefetch), mas sem
`evidence/` com números (startup, RSS steady/peak, p50/p95 de navegação,
tempo de primeiro viewport de thumbs). Preservar os tetos, medir e congelar
antes da 1.0 — sem threshold inventado, só baseline → budget → gate.

**Cobertura por módulo desigual.** `dialogs.rs`, `toolbar.rs`, `gallery.rs`,
`clipboard.rs`, `window.rs`, `mod.rs` com 0 testes; `viewer.rs` com 27,
`crop.rs` com 14. Preservar a pirâmide, fechar os zeros com testes de
transição pura (quando a decomposição do 03-application extrair as funções,
elas nascem testadas).

**Sem matriz de hardware.** WGPU-vs-Glow virou irrelevante (Freya/Skia agora),
mas não há matriz documentada (iGPU antiga/moderna, X11/Wayland, Windows) nem
decisão de renderer registrada. Preservar o escopo (um renderer), documentar
a matriz mínima na 1.0.

### Refatorar fortemente ou substituir a estrutura

- `install.sh` ainda aponta `raillen/photoshow` em URLs enquanto o repo anda
  em transição — normalizar para a identidade canônica antes da 1.0.
- `CHANGELOG.md` existe (Keep a Changelog) mas sem gate que cobre entrada
  por release — adicionar check de changelog no workflow de tag.
- `project-profile.json`/`prumo.lock`/`PHOTOSHOW_ENGINEERING_PRUMO_PLAN.md`
  convivem sem dono claro: o plano monolito de 1898 linhas foi a fonte desta
  documentação e agora é arquivo histórico — marcar como tal e apontar para
  `website/` como autoridade viva.

## Decisões aprovadas

1. CI barra em fmt, clippy `-D warnings`, testes Linux + Windows e build
   release — tudo `--locked`.
2. Release entrega tarball + deb + rpm + zip com SHA-256; installer tem smoke
   automatizado antes da 1.0.
3. Pirâmide de testes: unit por módulo puro, `pipeline` até o disco sem
   janela, `menu_task` headless com janela real. Zero-teste em módulo novo é
   dívida registrada, não padrão.
4. Batch reutiliza decode + bake + save atômico; fila nova nunca reimplementa
   o caminho de save.
5. Performance: baseline medida → budget congelado → gate. Sem número
   inventado, sem regressão silenciosa.
6. 1.0 sai quando: installer testado, docs de usuário (atalhos, formatos,
   FAQ), integração nos fluxos críticos, auditoria de performance, escopo
   congelado.
7. 2.0 RAW é feature-flag (`--features raw`), preview embutido primeiro,
   decode total só na atual e cancelável, binário padrão +0 bytes. Tudo fora
   da lista FORA continua fora.
8. `website/` é a autoridade viva; o plano monolito é arquivo histórico.
9. Identidade de repo/URLs normalizada antes da 1.0; application ID desktop
   (`io.github.raillen.photoshow`) não muda sem decisão de migração.
10. Toda afirmação de "corrigido/performático/pronto" exige evidence de
    código + teste + CI — a regra do plano original, mantida.

## Consequências

- Job de smoke do installer no `release.yml` antes da 1.0.
- `evidence/` com baseline (startup, RSS, navegação p50/p95, thumbs,
  save) vira entrada do budget.
- Cobertura dos módulos zero-teste fecha junto com a decomposição do
  03-application (função extraída nasce testada).
- Plano monolito marcado como histórico com link para `website/`.

## Regra anti-bloat

Não criar farm de CI (mais SOs que o suporte prometido), benchmark como
gate por PR (suite pesada é nightly/release), fuzzing obrigatório, nem
release train automatizado. Dois SOs, lint → teste → build, release por tag
com checksum e smoke — o resto é custo sem usuário.
