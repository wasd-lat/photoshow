# PhotoShow — Plano de Correção, Adequação ao Prumo e Evolução Técnica

**Documento de engenharia / auditoria brownfield**  
**Data da auditoria:** 2026-09-23  
**Projeto auditado:** `wasd-lat/photoshow`  
**Baseline do PhotoShow:** `main` @ `90ab8c150d6609a67a1fcc393f13a4bf9441b742` (`0.1.0-rc1`)  
**Prumo canônico usado como baseline:** `poppy-team/prumo` `main` @ `c85d469724b7f305d7727506ad064cc76184d37a`  
**Linha corrente documentada do Prumo:** v0.6, apesar de parte da documentação ainda manter nomes históricos `v0.4`  
**Preview consultado, mas não tratado como autoridade canônica:** `feat/prumo-harness-complete` @ `a866d5a39661c070c30c9132a20d5bb54c270d8d`, 28 commits à frente de `main` no momento da auditoria.

---

## 1. Resumo executivo

O PhotoShow já possui uma base funcional coerente com sua proposta: visualizador nativo em Rust, interface em egui, navegação por diretórios, lista virtualizada, galeria de miniaturas, zoom/pan, crop, rotate, undo/redo, prefetch, carregamento assíncrono, temas, docking e empacotamento Linux/Windows.

A recomendação desta auditoria é **manter egui**. Não existe, no estado atual, justificativa técnica suficiente para migrar a interface para Slint ou Freya. O maior potencial de ganho está abaixo da UI: arquitetura, ciclo de vida das imagens, concorrência, cache, operações de arquivo, CI e conformidade ao Prumo.

Os principais problemas encontrados são:

1. **`src/app.rs` tornou-se um monólito de 2.117 linhas**, concentrando aplicação, UI, navegação, edição, IO, atalhos, dialogs, docking, scan e save.
2. **O domínio de imagens conhece diretamente o egui** (`TextureHandle`, `ColorImage`, `Vec2`), impedindo isolamento e testes adequados.
3. **O pipeline mantém full-resolution onde não precisa**, inclusive no prefetch, podendo consumir centenas de MB ou mais.
4. **O prefetch limita pelo tamanho comprimido do arquivo**, não pelo tamanho efetivamente decodificado em RAM.
5. **Navegação rápida cria threads de decode obsoletas que continuam trabalhando**, pois geração antiga é descartada somente depois do decode.
6. **Salvar e copiar imagem clonam a full-resolution**, produzindo picos desnecessários de memória.
7. **A galeria não é realmente virtualizada pelo viewport**: ela desenha uma janela fixa de até ~601 itens em torno da seleção, enquanto o cache só alimenta ±25.
8. **Sobrescrever uma imagem não é crash-safe/atomic**, portanto falha de disco/processo durante escrita pode danificar o original.
9. **Salvar atualmente descarta metadata EXIF**, algo já reconhecido no roadmap e que deve bloquear a 0.1 final caso `Salvar` sobrescreva o original.
10. **Não existe CI normal em push/PR**; só há workflow de release por tag.
11. **O instalador one-line possui incompatibilidade com a estrutura do tarball gerado** e precisa ser corrigido/testado.
12. **O `project-profile.json` está no formato antigo e não atende ao schema atual do Prumo**.
13. Há **drift documental**: o perfil ainda menciona `egui_taffy`, embora a dependência tenha sido removida; URLs continuam em `raillen/photoshow` enquanto o repositório atual é `wasd-lat/photoshow`.
14. As skills em `.ai/skills` apresentam **proveniência antiga do Prumo e checksums vazios**; devem ser tratadas como projeções geradas e reconciliadas, não como fonte canônica de engenharia do projeto.

### Decisão de direção

A ordem recomendada é:

```text
0.1 final
  ↓
confiabilidade + Prumo + CI + release
  ↓
memória/decode/worker model
  ↓
refatoração estrutural + virtualização real
  ↓
save seguro + metadata
  ↓
UX/acessibilidade
  ↓
0.2 organização
  ↓
0.3 apresentação
  ↓
0.4 edição fotográfica
  ↓
0.5 lote
  ↓
1.0
  ↓
2.0 RAW opcional
```

Não iniciar exposição, histograma, slideshow, batch ou RAW antes de estabilizar os fundamentos acima.

---

# 2. Escopo e método da auditoria

Esta análise foi feita sobre o código e documentação públicos do repositório, incluindo principalmente:

- `Cargo.toml`
- `README.md`
- `ROADMAP.md`
- `CONTRIBUTING.md`
- `project-profile.json`
- `.ai/skills/**`
- `.github/workflows/release.yml`
- `install.sh`
- `src/app.rs`
- `src/config.rs`
- `src/editor.rs`
- `src/exif.rs`
- `src/fs_browser.rs`
- `src/image_store.rs`
- `src/thumbs.rs`

Também foi comparada a estrutura com o Prumo canônico atual, principalmente:

- Adoption Engine / brownfield adoption;
- Documentation Control Plane;
- project profile v2;
- evidence/readiness;
- UI contracts e Interface Map;
- Context/authority model;
- W0–W22;
- quality gates;
- performance budgets;
- regras de consolidação presentes na linha mais nova do Harness.

## Limitação importante

Esta auditoria é primariamente **estática**. O código não foi clonado e compilado localmente durante esta análise. Há evidência de uma execução de release bem-sucedida no GitHub Actions em commit anterior, mas isso não substitui CI contínua nem comprova o HEAD atual em todas as plataformas.

Portanto, qualquer afirmação de “corrigido”, “performático” ou “pronto” deve futuramente ser ligada a evidência executável.

Essa postura é coerente com o Prumo atual:

```text
requisito
  → claim
  → evidence
  → verified
```

---

# 3. Estado estrutural atual

## 3.1 Código

| Arquivo | Linhas aproximadas | Testes unitários | Observação |
|---|---:|---:|---|
| `src/app.rs` | 2.117 | 3 | monólito de aplicação/UI |
| `src/config.rs` | 193 | 3 | pequeno e razoavelmente isolado |
| `src/editor.rs` | 398 | 7 | boa base de lógica pura |
| `src/exif.rs` | 91 | 2 | pequeno, mas metadata de saída ainda falta |
| `src/fs_browser.rs` | 316 | 9 | boa separação parcial; precisa otimizar índices/sort/errors |
| `src/image_store.rs` | 407 | 3 | maior foco de memória/acoplamento |
| `src/thumbs.rs` | 188 | 1 | cache simples; precisa ser viewport-driven |

Total observado: **28 testes `#[test]`**.

## 3.2 Pontos positivos que devem ser preservados

Não refatorar por refatorar. Há decisões atuais corretas:

- `EditorState` é pequeno, previsível e barato para undo/redo.
- rotate/crop são não-destrutivos até o save.
- scans e saves pesados não são executados diretamente na thread principal.
- geração/id já é usada para descartar resultados principais obsoletos.
- lista de fotos no painel lateral usa `ScrollArea::show_rows`, portanto já existe virtualização correta nessa superfície.
- carregamento de thumbnails é limitado por worker único.
- `DISPLAY_MAX_DIM` impede que o viewer sempre envie full-resolution à GPU.
- o escopo do produto continua disciplinado: viewer/organização/edição leve, não Lightroom/GIMP.
- release profile já usa otimizações adequadas (`opt-level=3`, thin LTO, strip, `codegen-units=1`).

O plano abaixo **preserva essas decisões** e corrige os gargalos ao redor delas.

---

# 4. Classificação das correções

## P0 — bloqueadores para a 0.1 final

| ID | Problema | Impacto |
|---|---|---|
| PS-P0-01 | Ausência de CI normal em push/PR | regressões podem chegar à tag de release |
| PS-P0-02 | Installer/tarball incompatíveis | instalação one-line pode falhar |
| PS-P0-03 | Save overwrite não atômico | risco de corromper original |
| PS-P0-04 | Save descarta EXIF/metadata | perda de fidelidade ao sobrescrever foto |
| PS-P0-05 | `project-profile.json` incompatível com Prumo atual | projeto não está adequadamente adotado pelo harness |
| PS-P0-06 | Falta baseline de performance/evidence | não há como provar regressões/melhorias |

## P1 — arquitetura e performance antes de features pesadas

| ID | Problema | Impacto |
|---|---|---|
| PS-P1-01 | `app.rs` com 2.117 linhas | baixo isolamento, difícil manutenção agentic |
| PS-P1-02 | `ImageStore`/`ThumbCache` acoplados a egui | domínio/testes dependem da GUI |
| PS-P1-03 | full-resolution persistente no prefetch | alto consumo de RAM |
| PS-P1-04 | limite baseado no tamanho comprimido do arquivo | orçamento de RAM inexato |
| PS-P1-05 | threads obsoletas não canceladas | desperdício de CPU/RAM em navegação rápida |
| PS-P1-06 | full image clonada no save/copy | pico de memória |
| PS-P1-07 | conversão RGBA parcialmente na UI thread | jank em imagens maiores |
| PS-P1-08 | preview de edição recria buffers | alocações/jank desnecessários |
| PS-P1-09 | filmstrip pseudo-virtualizado | custo crescente e comportamento inconsistente |
| PS-P1-10 | sort/display name alocam repetidamente | custo evitável em pastas de 50k |

## P2 — robustez, UX e manutenção

- erros de filesystem silenciosamente descartados no scan;
- config corrompida volta para default sem informar o usuário;
- dock layout ainda não persistido;
- atalhos espalhados pelo método principal;
- estados de UI não possuem contrato formal;
- acessibilidade ainda não possui evidence dedicada;
- URLs/metadados de repo estão em transição;
- skills Prumo geradas precisam de reconciliação;
- renderer/dependências de pacote devem ser medidos em vez de presumidos.

---

# 5. Auditoria profunda do pipeline de imagens

## 5.1 Problema: full-resolution mantida no estado permanente

Hoje `DecodedPhoto` contém:

```rust
pub struct DecodedPhoto {
    pub full: image::DynamicImage,
    pub display: image::DynamicImage,
    pub full_size: (u32, u32),
}
```

Para uma foto grande, portanto, o processo mantém pelo menos:

```text
full decoded CPU buffer
+
display decoded CPU buffer
+
egui/GPU texture
```

Para imagens menores que `DISPLAY_MAX_DIM`, há ainda:

```rust
let display = full.clone();
```

Ou seja, a mesma fotografia pode ser duplicada na RAM antes mesmo de considerar GPU e edição.

### Correção

O viewer não precisa manter full-resolution permanentemente.

Novo fluxo:

```text
seleção
  ↓
decode somente para display
  ↓
DisplayFrame / RgbaFrame
  ↓
texture da UI
```

Full-resolution deve ser decodificada **sob demanda**, em operações que realmente exigem full pixels:

- salvar/bake;
- copiar imagem full;
- exportar;
- análise futura que exija full-resolution.

Isso deliberadamente troca um pouco de CPU/IO ocasional por redução grande de RAM residente.

---

## 5.2 Problema: prefetch armazena full-resolution

O código atual usa:

```rust
const PREFETCH_RADIUS: isize = 2;
const PREFETCH_CAP: usize = 4;
const PREFETCH_BYTES_CAP: u64 = 512 * 1024 * 1024;
```

Porém cada entrada de prefetch contém `full + display`.

Isso contradiz o objetivo de um visualizador leve: o usuário normalmente precisa **ver rapidamente a próxima foto**, não editar/salvar full-resolution todas as vizinhas simultaneamente.

### Nova política

```text
foto atual
  ├── DisplayFrame
  └── TextureHandle

vizinho -1
  └── DisplayFrame pronto ou decode em andamento

vizinho +1
  └── DisplayFrame pronto ou decode em andamento

outros
  └── thumbnail somente
```

Começar com raio efetivo ±1 e medir antes de aumentar.

Não fixar “±1” como dogma; o Prumo deve guardar evidência de hit-rate, latência e RAM e só então permitir alterar o budget.

---

## 5.3 Problema: `prefetch_max_mb` mede o arquivo comprimido

Hoje um arquivo pode entrar no prefetch se:

```rust
metadata.len() <= max_file_bytes
```

Isso não representa RAM decodificada.

Exemplo conceitual:

```text
JPEG no disco: poucos MB
RGBA em memória: dezenas/centenas de MB
```

### Correção

Orçamento deve considerar:

- largura;
- altura;
- formato/pixel layout;
- bytes reais produzidos;
- memória atualmente ocupada pelo cache.

Idealmente o prefetch futuro nem retém a full-resolution. Para buffers já materializados, contabilizar bytes reais do buffer em vez de inferir `width * height * 4` universalmente.

---

# 6. Concorrência e cancelamento

## 6.1 Threads obsoletas

Ao selecionar uma foto, o código cria nova thread:

```rust
std::thread::spawn(move || {
    let result = decode_photo(&path);
    ...
});
```

O `current_id` evita que resultado velho seja aplicado, o que é correto para consistência visual.

Entretanto ele **não cancela o trabalho**.

Se o usuário segurar `→` por várias fotos:

```text
foto 1 → thread A
foto 2 → thread B
foto 3 → thread C
foto 4 → thread D
...
```

Resultados A/B/C podem ser descartados, mas CPU, IO e memória foram consumidos mesmo assim.

O mesmo raciocínio vale para scans de diretório: trocar de pasta substitui receiver/state, mas o scan anterior continua até terminar.

## 6.2 Arquitetura recomendada

Não introduzir Tokio só por isso.

Para o escopo atual, uma solução simples é suficiente:

```text
UI
 │
 ├── LoadRequest { generation, path, target_size }
 │
 ▼
bounded worker / small worker pool
 │
 ├── latest-wins queue
 ├── generation cancellation checks
 └── bounded concurrency
 │
 ▼
DisplayFrame
```

Características:

- worker persistente em vez de thread por clique;
- fila bounded;
- deduplicação por path;
- pedido mais novo da seleção tem prioridade;
- prefetch possui prioridade inferior;
- job verifica generation/cancellation em pontos seguros;
- resultados antigos são descartados antes de conversões extras sempre que possível.

### Regra

**Seleção interativa sempre vence prefetch.**

---

# 7. UI thread e preparação de pixels

`poll()` recebe `DynamicImage` e depois faz upload/conversão através de `to_color()`.

A conversão:

```rust
img.to_rgba8()
```

pode alocar e copiar pixels.

Essa preparação deve acontecer no worker.

## Novo tipo neutro

```rust
pub struct DisplayFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: std::sync::Arc<[u8]>,
}
```

ou equivalente.

A camada de mídia entrega `DisplayFrame`.

Somente o adapter egui converte isso para a textura necessária.

```text
media worker
  ↓
DisplayFrame
  ↓
ui::egui_image
  ↓
TextureHandle
```

Resultado:

- menos trabalho CPU na UI thread;
- media layer testável sem egui;
- substituição futura do frontend continua possível sem reescrever decode/cache.

---

# 8. Preview de rotate/crop

Hoje `rebuild_preview()`:

1. clona `display_img`;
2. aplica transformação;
3. cria uma nova imagem;
4. converte/upload de textura novamente.

Para rotate e crop, isso é desnecessariamente caro.

## Direção preferida

Manter a imagem base como textura e representar preview com transformação visual:

```text
base texture
+
EditorState
+
viewer transform/UV
```

Para crop:

- alterar UV/source rect;
- alterar viewport/draw rect;
- manter overlay/gizmos separados.

Para rotate:

- usar mesh/UV transform ou equivalente no painter.

Assim:

```text
rotate/crop preview = estado + geometria
```

não:

```text
rotate/crop preview = novo bitmap inteiro
```

Quando chegarem exposição/contraste/saturação, haverá necessidade de um pipeline de preview raster/GPU separado; **não antecipar essa complexidade antes da 0.4**.

---

# 9. Save e integridade de arquivo

## 9.1 Problema crítico: overwrite direto

O save atual cria/grava diretamente no destino.

Isso é inadequado quando a própria aplicação oferece “Salvar” sobre o original.

### Fluxo seguro

```text
original.jpg
   ↓
decode + bake
   ↓
.original.photoshow-temp-<id>.jpg
   ↓
encoder finish
   ↓
flush
   ↓
validar arquivo produzido
   ↓
metadata
   ↓
atomic replace / platform-safe replace
   ↓
original.jpg
```

Se qualquer etapa falhar antes do replace:

```text
original permanece intacto
```

Implementar abstração específica para as diferenças Windows/Linux/macOS em substituição de arquivo existente.

## 9.2 Evitar clone da full image

Hoje `full_image()` devolve clone.

O save deve, em vez disso:

```text
SaveRequest
 ├── source_path
 ├── destination_path
 ├── EditorState
 ├── display_base_dimensions
 └── encoder options
```

O próprio worker de save reabre e decodifica a fonte.

Isso remove a necessidade de manter/clonar full-resolution no AppState.

## 9.3 EXIF

Ao salvar pixels já auto-orientados:

- preservar metadata relevante;
- evitar copiar cegamente a antiga orientation tag;
- normalizar orientation para identidade quando os pixels já foram fisicamente orientados;
- testar câmera/data/exposição/orientation.

A política precisa ser explícita por formato.

---

# 10. Copy Image

Hoje “Copiar imagem” solicita clone da full-resolution e converte RGBA para clipboard.

Isso pode bloquear e duplicar muita memória.

Novo fluxo:

```text
CopyImageRequest(path, EditorState?)
  ↓
background worker
  ↓
decode/prepara pixels
  ↓
clipboard adapter
  ↓
status
```

Definir claramente se “Copiar imagem” copia:

- original orientado;
- preview editado;
- resultado final das edições.

Recomendação de UX: copiar o **resultado visual atual**, mas a especificação deve ser documentada antes da implementação.

---

# 11. Filesystem e indexação de 50k fotos

## 11.1 Ordenação atual

O sort chama repetidamente:

```rust
photo.display_name().to_lowercase()
```

em comparações O(n log n).

Isso cria strings temporárias muitas vezes.

### Correção mínima

Usar key pré-computada/cached:

```text
PhotoEntry
 ├── path
 ├── display_name
 ├── sort_name
 ├── extension
 └── metadata lazy
```

ou ao menos `sort_by_cached_key`.

## 11.2 `visible` duplica PathBuf

Atualmente:

```text
photos: Vec<PhotoPath>
visible: Vec<PhotoPath>
```

Filtrar 50k itens duplica estruturas/caminhos.

### Modelo melhor

```text
entries: Vec<PhotoEntry>
visible_indices: Vec<usize>
selected: Option<PhotoId/usize>
```

Assim filtros e ordenação trabalham sobre IDs/índices.

Também simplifica futuros:

- rating;
- sort por data/tamanho;
- EXIF lazy;
- seleção persistente;
- sidecars.

## 11.3 Erros de scan

Hoje `filter_map(Result::ok)` remove erros silenciosamente.

Substituir por resultado rico:

```rust
ScanReport {
    photos,
    files_seen,
    errors_seen,
    sample_errors,
}
```

A UI não precisa assustar o usuário, mas também não deve afirmar implicitamente que tudo foi lido quando permissões/IO falharam.

---

# 12. Galeria: virtualização real

A lista lateral já usa `show_rows` corretamente.

A galeria, porém, calcula:

```text
seleção - 300 ... seleção + 300
```

Isso limita o conjunto desenhado e cria até ~601 células no frame, independente do viewport.

Além disso, o `ThumbCache` só busca em torno de ±25 da seleção. Portanto viewport e cache não compartilham a mesma fonte de verdade.

## Arquitetura correta

Calcular:

```text
cols = largura / cell_width
rows = ceil(total_items / cols)
```

Virtualizar **linhas**:

```text
ScrollArea::show_rows(row_height, total_rows, visible_row_range)
```

Para cada linha visível:

```text
first_index = row * cols
last_index  = min(first + cols, total)
```

O `ThumbCache` deve receber a faixa realmente visível:

```text
visible viewport
  ↓
thumbnail scheduler
  ├── prioridade 0: viewport
  ├── prioridade 1: foto selecionada
  └── prioridade 2: pequena margem pré/pós viewport
```

Benefícios:

- 50k fotos continuam baratas;
- scroll livre por todo o conjunto;
- thumbnails carregam onde o usuário está olhando;
- implementar “rolar para a seleção atual” torna-se natural;
- menos widgets por frame.

---

# 13. Refatoração de `app.rs`

Não fazer rewrite.

Extrair em pequenos passos, preservando comportamento.

## Estrutura alvo

```text
src/
├── main.rs
├── app/
│   ├── mod.rs
│   ├── state.rs
│   ├── commands.rs
│   └── shortcuts.rs
├── media/
│   ├── mod.rs
│   ├── loader.rs
│   ├── thumbnails.rs
│   ├── save.rs
│   └── pixels.rs
├── ui/
│   ├── mod.rs
│   ├── toolbar.rs
│   ├── browser.rs
│   ├── viewer.rs
│   ├── filmstrip.rs
│   ├── dialogs.rs
│   ├── theme.rs
│   └── egui_image.rs
├── editor.rs
├── exif.rs
├── fs_browser.rs
└── config.rs
```

## Dependências desejadas

```text
editor ───────────────┐
fs_browser ───────────┤
media ────────────────┤
config ───────────────┤
                      ▼
                    app
                      ▼
                     ui
                      ▼
                 egui/eframe
```

### Regra importante

`editor`, `media` e `fs_browser` não devem importar egui.

Tipos egui devem ficar na fronteira UI.

## Evitar overengineering

Não introduzir:

- DI container;
- ECS;
- event bus genérico;
- dezenas de traits sem necessidade;
- workspace multi-crate prematuramente;
- Tokio apenas para poder dizer que é async.

O PhotoShow deve continuar pequeno.

---

# 14. Modelo de comandos/intents

Um meio simples de reduzir acoplamento é modelar ações do usuário:

```rust
pub enum AppCommand {
    OpenFolder(PathBuf),
    OpenFiles(Vec<PathBuf>),
    SelectPhoto(PhotoId),
    StepSelection(isize),
    RotateClockwise,
    RotateCounterClockwise,
    ApplyCrop(CropRect),
    Undo,
    Redo,
    Save,
    SaveAs(PathBuf),
    Rename(String),
}
```

A UI traduz interação em comando.

A aplicação executa o comando.

Isso não precisa virar Redux/MVU completo; serve apenas para impedir que cada painel altere vinte campos internos diretamente.

---

# 15. Configuração

## Problemas

- escrita direta;
- parse corrompido volta silenciosamente ao default;
- não há schema/version explícita da configuração;
- migração é baseada apenas em `serde(default)`.

## Evolução

Adicionar:

```rust
pub struct AppConfig {
    pub version: u32,
    ...
}
```

Carregamento:

```text
Loaded(config)
LoadedWithWarning(config, warning)
RecoveredFromCorrupt(default, backup_path)
```

Persistência:

```text
config.json.tmp
→ flush
→ rename/replace
→ config.json
```

Não é necessário transformar config em banco de dados.

---

# 16. Renderer: WGPU vs Glow

O projeto usa `eframe = "0.36"` com features default, portanto o caminho default atual inclui WGPU.

A documentação do eframe também oferece Glow/OpenGL e observa que Glow pode reduzir significativamente o tamanho do binário.

**Não trocar renderer por opinião.**

Criar duas variantes de benchmark:

```text
photoshow-wgpu
photoshow-glow
```

Medir em pelo menos:

- Intel integrada antiga;
- iGPU moderna;
- Windows;
- Linux X11;
- Linux Wayland.

Métricas:

- startup;
- RSS idle;
- tamanho do executável/pacote;
- VRAM quando disponível;
- frame time durante zoom/pan;
- upload da primeira foto;
- compatibilidade de driver.

Só depois decidir se:

- WGPU permanece default;
- Glow vira default;
- ambos viram builds/feature profiles.

Também revisar o `depends = "libgl1"` dos pacotes Linux para garantir que corresponde ao renderer realmente distribuído.

---

# 17. Release engineering

## 17.1 Bug do tarball/installer

O workflow cria um diretório de staging:

```text
photoshow-<tag>-x86_64-unknown-linux-gnu/
```

coloca arquivos dentro dele e então empacota **o diretório**.

O instalador extrai o tarball e procura diretamente:

```text
$TMP/photoshow
$TMP/photoshow.desktop
$TMP/photoshow.svg
```

A estrutura produzida é, entretanto:

```text
$TMP/photoshow-<tag>-x86_64-unknown-linux-gnu/photoshow
```

Essa incompatibilidade precisa ser corrigida antes da 0.1 final.

### Duas soluções válidas

A. gerar tarball com conteúdo no root;

ou

B. fazer o installer localizar/usar o diretório extraído.

Escolher uma e criar smoke test automatizado.

## 17.2 Teste obrigatório de instalação

Pipeline de release deve:

```text
build tarball
→ executar install.sh em HOME temporário/container
→ executar photoshow --version ou smoke equivalente
→ verificar .desktop/icon
→ uninstall/cleanup do ambiente temporário
```

## 17.3 Integridade do download

Adicionar arquivo de checksums da release, por exemplo SHA-256.

O instalador deve validar checksum antes da instalação quando o artefato correspondente existir.

## 17.4 URLs e identidade

Normalizar URLs de repositório/homepage/bugtracker para o repositório canônico atual.

**Não alterar automaticamente o application ID `io.github.raillen.photoshow`.** Identificadores desktop/Flatpak fazem parte da identidade instalada e sua migração deve ser uma decisão separada, com compatibilidade avaliada.

---

# 18. CI obrigatório

Hoje existe apenas workflow de release por tag.

Adicionar `.github/workflows/ci.yml` para `push` e `pull_request`.

## Gate mínimo Rust

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --locked
cargo build --release --locked
```

Adicionar scan de vulnerabilidades/dependências conforme ferramenta aprovada pelo projeto.

## Matriz

No mínimo:

```text
Linux
Windows
```

Mac pode ser introduzido quando houver distribuição/suporte assumido oficialmente.

## Prumo

Depois da adoção/reconciliação:

```text
Rust gates
  ↓
Prumo validate
  ↓
Prumo doctor
  ↓
documentation/UI/evidence gates configurados para o projeto
```

A versão exata dos comandos deve ser obtida da CLI Prumo instalada no CI, evitando copiar sintaxe histórica para sempre no workflow.

---

# 19. Adequação ao Prumo atual

## 19.1 Situação presente

O PhotoShow possui `project-profile.json`, mas ele está no formato antigo.

Faltam elementos requeridos pelo schema atual, incluindo:

- `version`;
- `project`;
- `project.name`;
- `project.type`;
- `ai.preferred_models` no formato de objetos `{ id, provider? }`.

Também existem chaves históricas como `risks`, enquanto o schema corrente usa `risk`.

Além disso, o profile menciona `egui_taffy`, embora o código/documentação indiquem que o taffy foi removido da galeria.

## 19.2 Regra de adoção

O Prumo atual possui Adoption Engine para brownfield e explicitamente trabalha sem forced layout.

Portanto:

**não copiar manualmente toda a estrutura do Prumo para dentro do PhotoShow.**

Fluxo correto:

```text
1. audit-only
2. classificar fatos vs inferências
3. revisar Confidence Ledger
4. gerar propostas
5. dry-run
6. aprovação humana
7. aplicar somente deltas confirmados
8. validate/doctor
```

Exemplo conceitual de sequência:

```bash
prumo help adopt
prumo adopt --audit-only --strict <photoshow>
prumo adopt --propose-migration --dry-run <photoshow>
# revisar
prumo adopt --apply <photoshow>
prumo validate <photoshow>
prumo doctor <photoshow>
```

A CLI instalada deve ser considerada autoridade sobre pequenas diferenças de sintaxe.

## 19.3 Target do profile v2

A estrutura deveria convergir para algo equivalente a:

```json
{
  "version": 2,
  "project": {
    "name": "photoshow",
    "type": ["desktop", "desktop-gui"]
  },
  "stack": {
    "languages": ["rust"],
    "runtime": ["eframe", "egui"],
    "tooling": ["cargo", "rustfmt", "clippy"]
  },
  "features": [
    "photo-viewer",
    "filesystem-navigation",
    "thumbnail-gallery",
    "zoom-pan",
    "non-destructive-editing",
    "metadata",
    "desktop-ui",
    "packaging"
  ],
  "quality": {
    "performance": "high",
    "accessibility": true,
    "security": "standard"
  },
  "risk": [
    "filesystem-write",
    "destructive-overwrite",
    "large-image-memory",
    "untrusted-media"
  ],
  "ai": {
    "orchestrator": "native",
    "autonomy": "controlled",
    "preferred_models": [
      { "id": "<modelo-confirmado-pelo-prumo>" }
    ]
  }
}
```

O trecho acima é **modelo estrutural**, não arquivo para copiar literalmente: o ID de modelo deve vir da configuração real/Prumo, não ser inventado.

---

# 20. Documentação Prumo para o PhotoShow

Criar ou reconciliar conhecimento canônico para pelo menos:

```text
product vision
project scope / anti-scope
system architecture
performance strategy
memory/cache strategy
image pipeline
filesystem/save safety
metadata policy
testing strategy
release/install strategy
security/trust assumptions
UI/UX architecture
accessibility
roadmap
```

Não é necessário obrigar filenames específicos se o binding do Prumo aponta para o documento correto.

## UI contract

Como `desktop-gui` é uma capacidade de UI reconhecida pelo Prumo, criar o inventário formal da interface:

```text
docs/ui-ux/interface-map.json
```

Mapear, no mínimo:

- top toolbar;
- File menu;
- edit toolbar;
- browser panel;
- favorites;
- folder tree;
- photo list;
- central viewer;
- filmstrip/gallery;
- status bar;
- settings dialog;
- rename dialog;
- crop overlay;
- crop handles;
- context menu;
- fullscreen state;
- maximized-viewer state;
- loading/error/empty/saving states.

Também registrar relações:

```text
selection → viewer
selection → statusbar
selection → filmstrip active item
filter → visible photo index
crop mode → viewer interaction mode
save state → toolbar enablement
settings → theme/cache behavior
```

Isso é particularmente valioso para code agents, porque impede que uma mudança visual “local” quebre uma relação funcional não documentada.

---

# 21. `.ai/skills` e generated surfaces

O repositório já possui várias skills locais, mas o manifesto raiz ativa somente:

- `lang-rust`;
- `rust-analyzer-context-indexing`.

Há skills de arquitetura, clean code, testing, refactoring etc. presentes no disco, mas isso não significa automaticamente que sejam todas ativas ou canônicas.

Algumas também carregam provenance histórica do Prumo 0.3/0.4 e checksum vazio.

## Correção

Durante adoção:

1. descobrir quais skills são realmente requeridas pelo profile;
2. reconciliar versões com o Prumo instalado;
3. regenerar/projetar superfícies quando aplicável;
4. não editar generated surfaces como fonte de verdade;
5. adicionar gate contra context rot;
6. registrar provenance/checksum quando o mecanismo corrente suportar.

O repositório PhotoShow deve continuar sendo dono das decisões específicas do PhotoShow; o Prumo governa contratos, evidence, workflow e projeções.

---

# 22. Performance: metodologia correta

O Prumo mais recente explicita uma regra importante: **não inventar thresholds com falsa precisão**.

Portanto, primeiro criar baseline reproduzível.

## 22.1 Métricas obrigatórias

### Startup

- process start → first frame;
- process start → UI interativa;
- startup abrindo última pasta;
- RSS após estabilização.

### Scan

- 1k arquivos;
- 10k arquivos;
- 50k arquivos;
- árvore com muitos não-imagens;
- com/sem `.gitignore`;
- erros de permissão.

### Viewer

- seleção → primeiro pixel em cache miss;
- seleção → primeiro pixel em cache hit;
- p50/p95/p99 de navegação;
- zoom/pan frame time;
- upload texture time.

### Memória

Cenários:

```text
sem foto
JPEG 12 MP
JPEG 24 MP
JPEG 50+ MP
TIFF grande
20 navegações rápidas
crop/rotate
save
copy-image
```

Coletar:

- RSS steady;
- peak RSS;
- cache bytes;
- número de decodes simultâneos;
- bytes de texturas quando mensurável.

### Thumbnails

- tempo para preencher primeiro viewport;
- scroll rápido por 50k;
- jobs descartados;
- cache hit ratio;
- frame time durante scroll.

### Save

- latência;
- peak RSS;
- integridade do original em falha simulada;
- metadata preservada.

### Distribuição

- binary size WGPU;
- binary size Glow;
- package size;
- cold start por renderer.

## 22.2 Budgets

Somente depois de medir baseline, congelar budgets no repositório/Prumo.

Exemplo:

```text
baseline → decisão de budget → CI regression threshold
```

Não o contrário.

---

# 23. Testes que faltam

## Unit

Preservar os 28 atuais e ampliar os módulos extraídos.

Adicionar especificamente:

- mapping de viewport → indices da galeria;
- cache eviction por bytes reais;
- latest-wins generation;
- cancellation;
- PhotoEntry/sort keys;
- config migrations;
- atomic replace policy;
- EXIF orientation normalization.

## Integration

Criar fluxos reais com tempdir:

```text
open → select → rotate → save-as → reopen
open → crop → overwrite → reopen
rename → rescan → selection survives
corrupt config → recovery
scan with permission errors
```

## Failure injection

Testar:

- encoder falha;
- espaço/IO insuficiente via abstraction/fake;
- temp write falha;
- replace falha;
- metadata copy falha;
- arquivo desaparece durante save;
- source muda durante processamento.

## Performance/regression

Benchmarks separados de unit tests.

Evitar tornar CI principal extremamente lenta: usar conjunto pequeno por PR e suite pesada nightly/release se necessário.

## Packaging smoke

- tarball installer;
- DEB;
- RPM;
- Windows zip.

---

# 24. Segurança e trust boundary

PhotoShow processa arquivos potencialmente não confiáveis.

Apesar de a decodificação estar em crates Rust, manter política explícita:

- nunca executar conteúdo da imagem;
- validar dimensões antes de alocações muito grandes quando possível;
- definir limite/pressure policy para imagens absurdamente grandes;
- não seguir symlinks durante recursive scan sem decisão explícita;
- tratar caminhos como dados, não comandos;
- evitar montar shell command strings;
- update/dependency audits;
- installer com checksum;
- operações destrutivas sempre evidentes e recuperáveis quando possível.

O código atual usa argumentos separados em `Command`, o que é melhor que concatenar shell strings. Preservar isso.

---

# 25. Acessibilidade e UX

Não basta o backend da GUI suportar mecanismos de acessibilidade; o PhotoShow deve fornecer semântica.

Prioridades:

- accessible labels para botões somente-ícone;
- ordem de foco previsível;
- operação completa por teclado;
- estado selecionado comunicado semanticamente;
- loading/saving/error não dependentes apenas de cor;
- crop handles com affordance e feedback;
- status de fullscreen/maximize claro;
- contraste validado em todos os temas;
- atalhos documentados e visíveis.

## State matrix mínima

Para cada superfície importante documentar:

```text
empty
loading
ready
focused
hovered
selected
disabled
error
saving
editing
crop-active
```

Nem todo componente precisa de todos os estados; usar `N/A` explícito com justificativa, conforme filosofia do Prumo, em vez de omissão silenciosa.

---

# 26. Roadmap revisado

## 0.1.0 final — Hardening real

**Nenhuma feature grande nova.**

Entregas:

- Prumo adoption/profile atual;
- CI normal;
- installer corrigido;
- smoke de packages;
- save atômico;
- metadata preservada;
- pipeline de RAM corrigido;
- bounded workers/latest-wins;
- galeria virtualizada;
- dock persistence;
- config migration/recovery;
- docs e UI contracts;
- performance baseline;
- accessibility baseline;
- regressions suite.

### Gate de saída

A 0.1 deixa de ser “RC que funciona na máquina do desenvolvedor” e passa a ser um viewer que pode ser distribuído com confiança.

---

## 0.2.0 — Organização, primeiro read-only

Ordem recomendada:

1. sort por nome/data/tamanho;
2. busca incremental;
3. painel EXIF/metadata;
4. estrelas/favorito por foto.

### Por que essa ordem

Os três primeiros itens são essencialmente leitura/índice e exercitam a nova `PhotoEntry` sem criar persistência nova.

Rating introduz sidecar, portanto deve vir depois que atomic storage/config patterns já estiverem sólidos.

## Sidecar

Usar sidecar explícito, portátil e simples.

Não introduzir SQLite escondido para essa feature.

Escrita de sidecar também deve ser atômica.

---

## 0.3.0 — Apresentação

- slideshow;
- timer;
- fade/cut simples;
- presentation mode;
- UI mínima;
- preload do próximo frame usando o mesmo scheduler.

Não criar motor genérico de transições.

---

## 0.4.0 — Edição básica

Antes dessa versão deve existir um **Preview Pipeline** formal.

Features:

- exposição;
- contraste;
- saturação;
- temperatura/tint;
- highlights/shadows;
- histograma RGB;
- clipping overlay;
- compare original/editado.

### Arquitetura

```text
EditorState
  ↓
PreviewPipeline
  ↓
preview frame
  ↓
viewer texture
```

Separar:

- estado de edição;
- execução do preview;
- execução final/bake.

Ambos devem usar a mesma semântica matemática, ainda que preview opere em resolução reduzida.

---

## 0.5.0 — Lote

Somente depois de existir `SaveService/FileOperation` robusto.

A fila de lote deve reutilizar:

- decoder;
- editor operations;
- atomic writer;
- metadata policy;
- worker scheduler;
- cancellation;
- progress reporting.

Assim batch não vira uma segunda implementação de save.

---

## 1.0

Requisitos adicionais:

- packaging estável;
- docs completas;
- performance budgets congelados com evidence;
- regressions conhecidas zeradas;
- integration suite dos fluxos críticos;
- hardware compatibility matrix documentada;
- decisão renderer fundamentada por benchmark;
- 50k dataset validation;
- TIFF grande validation.

---

## 2.0 — RAW opcional

A ideia atual do roadmap continua válida, mas deve ser condicionada a estas pré-condições:

```text
memory pipeline estável
worker scheduler estável
embedded-preview abstraction
metadata abstraction
preview pipeline estável
save/export abstraction estável
```

RAW não deve ser usado para justificar complexidade prematura na 0.x.

---

# 27. Waves de implementação

A execução deve seguir a filosofia no-big-bang do Prumo.

## Wave PS-W0 — Baseline, authority e Prumo

### Alterações

- rodar adoption audit;
- reconciliar profile v2;
- classificar docs canônicas/projeções;
- reconciliar `.ai/skills`;
- registrar architecture/performance/testing contracts;
- criar baseline de performance;
- adicionar CI principal.

### Não alterar

- comportamento visual do viewer;
- formato de config além do estritamente necessário;
- rendering pipeline.

### Exit gate

- `cargo fmt` green;
- `clippy -D warnings` green;
- tests green;
- Prumo validate/doctor green nos contratos adotados;
- baseline de performance arquivada como evidence.

---

## Wave PS-W1 — Release e integridade de dados

### Alterações

- corrigir tarball/install;
- package smoke tests;
- normalize repo URLs;
- atomic writer;
- config atomic save;
- EXIF/metadata preservation;
- overwrite recovery tests;
- checksums de release.

### Exit gate

Instalação e save/overwrite exercitados automaticamente sem perda de arquivo/metadata nos cenários suportados.

---

## Wave PS-W2 — Image Runtime

### Alterações

- `DisplayFrame` neutro;
- remover egui de decode core;
- remover full-resolution permanente;
- lazy full decode;
- bounded loader worker;
- latest-wins;
- priority selection > prefetch;
- actual-byte cache accounting;
- RGBA preparation off UI thread;
- copy-image worker.

### Exit gate

- navegação rápida não cria crescimento descontrolado de threads/RAM;
- stale results nunca são apresentados;
- peak RSS comparado ao baseline e documentado;
- media core testável sem egui.

---

## Wave PS-W3 — UI decomposition e galeria

### Alterações

- decompor `app.rs`;
- `ui/browser`, `viewer`, `filmstrip`, `toolbar`, `dialogs`;
- centralizar shortcuts;
- `PhotoEntry + visible_indices`;
- viewport virtualization;
- viewport-driven thumbnail scheduling;
- active item auto-scroll;
- persist dock layout.

### Exit gate

- nenhuma regressão funcional;
- `media` não depende de egui;
- app orchestration fica claramente separada de rendering/widgets;
- 50k browse/scroll scenario medido.

---

## Wave PS-W4 — UX, accessibility e state contracts

### Alterações

- interface map;
- state matrices;
- semantic labels;
- keyboard/focus review;
- visual state consistency;
- loading/error/save feedback;
- contrast validation;
- accessibility evidence.

### Exit gate

Prumo UI verification sem finding crítico e checklist manual/automatizado anexado como evidence.

---

## Wave PS-W5 — Fechamento da 0.1

### Alterações

- hardware matrix;
- WGPU vs Glow evidence;
- release candidate regression;
- docs finais;
- changelog;
- package installation on clean environments;
- 50k and large-TIFF stress.

### Exit gate

Release 0.1.0 somente após hard gates green.

---

# 28. Ordem interna de cada work package

Para qualquer alteração não trivial:

```text
1. resolver contrato/decision aplicável
2. inspecionar realidade do código
3. declarar delta
4. escrever/atualizar teste que comprova o comportamento
5. implementar
6. executar quality gates
7. medir impacto de performance quando aplicável
8. coletar evidence
9. atualizar documentação afetada
10. somente então declarar concluído
```

Essa sequência deve virar regra para code agents que trabalhem no PhotoShow.

---

# 29. Definition of Done da 0.1 final

A release só deve ser considerada concluída quando:

- [ ] profile Prumo atual está válido;
- [ ] adoption drift foi revisado;
- [ ] canonical/projected docs estão diferenciadas;
- [ ] CI roda em PR/push;
- [ ] fmt/clippy/tests/build passam;
- [ ] installer tarball é testado automaticamente;
- [ ] DEB/RPM/Windows artifacts possuem smoke validation adequada;
- [ ] save overwrite é crash-safe no limite documentado de cada plataforma;
- [ ] EXIF relevante é preservado;
- [ ] orientation não duplica rotação após save;
- [ ] config possui recovery/migration;
- [ ] image runtime não mantém full-res das vizinhas;
- [ ] navegação rápida usa concorrência bounded/latest-wins;
- [ ] galeria é virtualizada pelo viewport;
- [ ] thumbnail scheduler segue viewport;
- [ ] `app.rs` deixa de concentrar todas as responsabilidades;
- [ ] core de mídia não depende de egui;
- [ ] performance baseline existe;
- [ ] regressões têm budgets/evidence onde já for possível justificá-los;
- [ ] interface map/state contracts existem;
- [ ] acessibilidade básica foi auditada;
- [ ] README/roadmap/changelog refletem a implementação real;
- [ ] release metadata aponta para a identidade canônica desejada do projeto.

---

# 30. O que não fazer agora

Para proteger a identidade leve do PhotoShow:

- não migrar egui → Slint/Freya;
- não adicionar banco de dados interno para catálogo;
- não adotar Tokio sem necessidade demonstrada;
- não implementar RAW antes do pipeline base;
- não criar GPU compute para ajustes antes da 0.4;
- não criar sistema de plugins;
- não criar nuvem/sync/login;
- não criar IA;
- não adicionar máscaras/edição local;
- não criar motor genérico de efeitos/transições;
- não transformar arquitetura em dezenas de crates;
- não copiar a estrutura interna do Prumo para o aplicativo;
- não permitir que generated agent surfaces substituam documentação canônica do produto.

---

# 31. Direção final recomendada

A arquitetura desejada do PhotoShow é deliberadamente simples:

```text
┌──────────────────────────────┐
│            UI egui           │
│ toolbar/browser/viewer/etc.  │
└──────────────┬───────────────┘
               │ AppCommand
               ▼
┌──────────────────────────────┐
│       Application State      │
│ seleção / fluxo / estado UI  │
└───────┬─────────────┬────────┘
        │             │
        ▼             ▼
┌──────────────┐  ┌───────────────┐
│ Media Runtime │  │ Editor Domain │
│ loader/thumbs │  │ pure state    │
│ cache/save    │  │ undo/redo     │
└──────┬────────┘  └───────────────┘
       │
       ▼
┌──────────────────────────────┐
│ filesystem / codecs / EXIF   │
└──────────────────────────────┘
```

O egui fica exatamente onde ele é vantajoso: interação e renderização da interface.

O core fica independente o bastante para:

- testar;
- medir;
- otimizar;
- trocar renderer/frontend futuramente se um dia houver evidência para isso.

E o Prumo fica onde deve ficar:

```text
contracts
+ authority
+ planning
+ quality gates
+ evidence
+ UI map
+ agent instructions
+ adoption/drift control
```

sem virar uma dependência runtime do executável PhotoShow.

---

# 32. Prioridade imediata — checklist operacional

A próxima sequência de trabalho recomendada é:

1. **PS-W0 — Prumo + CI + baseline**.
2. **PS-W1 — installer + save atômico + EXIF**.
3. **PS-W2 — novo Image Runtime de baixo consumo**.
4. **PS-W3 — decompor app.rs + virtualização real**.
5. **PS-W4 — UI contracts + accessibility**.
6. **PS-W5 — fechar 0.1.0**.
7. Somente então iniciar **0.2 Organization**.

Isso reduz risco técnico sem interromper o produto com um rewrite e cria uma base muito melhor para code agents trabalharem com precisão.

---

# 33. Referências consultadas

## PhotoShow

- Repository: `https://github.com/wasd-lat/photoshow`
- `Cargo.toml`
- `README.md`
- `ROADMAP.md`
- `CONTRIBUTING.md`
- `install.sh`
- `.github/workflows/release.yml`
- `project-profile.json`
- `.ai/skills/**`
- `src/app.rs`
- `src/config.rs`
- `src/editor.rs`
- `src/exif.rs`
- `src/fs_browser.rs`
- `src/image_store.rs`
- `src/thumbs.rs`

## Prumo

- Repository: `https://github.com/poppy-team/prumo`
- canonical `main` audited at `c85d469724b7f305d7727506ad064cc76184d37a`
- `docs/runtime/adoption-engine.md`
- `docs/governance/m7-exit-gate.md`
- `docs/product/scope-v0.4.md`
- `docs/architecture/documentation-control-plane.md`
- `docs/development/waves.md`
- `schemas/project-profile.schema.json`
- UI interface/state/component schemas
- forward-looking review of `feat/prumo-harness-complete` for consolidated architecture, execution-contract and performance-budget direction.

## eframe

- eframe 0.36 documentation / feature set and renderer options were consulted only to define the WGPU × Glow benchmark recommendation. No renderer migration is mandated by this document.

---

## Status deste documento

**Proposta de engenharia baseada em auditoria.**  
Nenhum item deve ser marcado como implementado apenas porque está descrito aqui. Cada conclusão de implementação deve possuir evidence produzida pelo código, testes, CI e gates correspondentes.
