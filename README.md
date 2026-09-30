# photoshow

Visualizador de fotos rápido em Rust ([Freya](https://github.com/marc2332/freya)):
abra pastas ou arquivos soltos, navegue, edite e salve — sem travar,
sem banco de dados, sem nuvem.

![photoshow em ação](assets/photoshow.png)

## Recursos

- **Abertura rápida** — pastas (varredura assíncrona com progresso) ou
  arquivos soltos; JPG, PNG, WebP, TIFF, BMP, GIF
- **Navegação** — árvore de pastas com expansão lazy, favoritas fixadas,
  lista virtualizada (50k fotos sem engasgo), galeria de miniaturas em
  grade fluida com tamanho ajustável
- **Viewer** — zoom ancorado no cursor, pan, fullscreen, modo maximizado,
  correção automática de orientação EXIF, prefetch de vizinhas
- **Edição não-destrutiva** — rotate 90°, crop por arrasto (mover +
  gizmos, proporção opcional, `Enter` aplica), undo/redo, preview
  instantâneo; Salvar sobrescreve (com confirmação) ou Salvar como…
- **Produtividade** — menu Arquivo único, botão direito (copiar
  caminho/imagem, abrir, mostrar na pasta), renomear (`F2`), atalhos
  de teclado, painéis redimensionáveis, temas claro/escuro,
  qualidade JPEG e prefetch configuráveis

## Atalhos

| Tecla | Ação |
|---|---|
| `Ctrl+O` | abrir arquivos |
| `Ctrl+Shift+O` | abrir pasta |
| `←` `→` | foto anterior / próxima |
| `+` `-` `0` | zoom + / − / ajustar |
| `F2` | renomear |
| `Ctrl+Z` / `Ctrl+Y` | desfazer / refazer edição |
| `Enter` | aplicar crop |
| `F9` | maximizar visualizador |
| `F11` | fullscreen (`Esc` sai) |

## Instalação

Via script (Linux x86_64, sem root — baixa a última release):

```bash
curl -fsSL https://raw.githubusercontent.com/raillen/photoshow/main/install.sh | bash
```

Por distribuição (na página de [releases](https://github.com/raillen/photoshow/releases)):

| Sistema | Arquivo |
|---|---|
| Debian/Ubuntu | `photoshow_*_amd64.deb` (`sudo dpkg -i`) |
| Fedora/openSUSE | `photoshow-*.x86_64.rpm` (`sudo rpm -i` / `dnf install`) |
| Windows | `photoshow-*-x86_64-pc-windows-msvc.zip` (só extrair o `.exe`) |
| Qualquer Linux | tarball + `install.sh` acima |

Do código-fonte (requer Rust 1.95+):

```bash
cargo install --path .
# ou rode direto:
cargo run --release
```

## Configuração

Tudo em `~/.config/photoshow/config.json`: favoritas, última pasta,
tema, qualidade JPEG, prefetch, comportamento da varredura. Editável
pelo menu Config dentro do app.

## Arquitetura

| Módulo | Responsabilidade |
|---|---|
| `src/app/mod.rs` | raiz da aplicação, dock e composição dos painéis |
| `src/app/state.rs` | estado global (Freya Radio) e transições |
| `src/app/services.rs` | scan, save e o pump que drena as threads de decode |
| `src/app/shortcuts.rs` | atalhos globais de teclado |
| `src/app/toolbar.rs` | menu Arquivo, ferramentas de edição e filtro de formato |
| `src/app/statusbar.rs` | posição, mensagens e dimensões |
| `src/app/viewer.rs` | zoom, pan, crop e menu de contexto |
| `src/app/browser.rs` | favoritas, árvore de pastas e lista virtualizada |
| `src/app/gallery.rs` | grade de miniaturas |
| `src/app/crop.rs` | geometria do recorte (puro) |
| `src/app/dialogs.rs` | modais de renomear e configurações |
| `src/app/clipboard.rs` | copiar caminho/imagem |
| `src/app/window.rs` | window state (maximizar, fullscreen) |
| `src/image_store.rs` | decode em background → `ImageHandle` (sem GUI) |
| `src/thumbs.rs` | fila de miniaturas |
| `src/editor.rs` | pilha de edição não-destrutiva (puro) |
| `src/fs_browser.rs` | varredura e renomear (puro) |
| `src/exif.rs` | orientação EXIF (puro) |
| `src/config.rs` | preferências persistidas (puro) |
| `src/cli.rs` | o que abrir a partir de caminhos externos (puro + efeito) |
| `src/theme.rs` | temas Freya (slate, charcoal, frost, paper) |
| `src/ui.rs` | design system: métricas, primitivas e o dropdown ancorado |

O domínio (`editor`, `exif`, `fs_browser`, `config`) não conhece a GUI
e é testado sem janela. `src/lib.rs` guarda os módulos e `src/main.rs`
só faz o `launch`, o que deixa `tests/pipeline.rs` testar o caminho
inteiro (varredura → decode → edição → bake → gravação → releitura)
sem abrir janela.

`src/ui.rs` é a fonte única de medidas de UI (escala tipográfica, espaçamento,
raio, alvo de clique) e de duas peças que não existem prontas no Freya: o
dropdown ancorado fora do fluxo e o texto de uma linha só. Nenhuma decisão de
produto mora lá — quando algo é regra de negócio, vai para `config.rs` (com
teste) ou `state.rs` (função pura), e a troca de toolkit não leva a decisão
junto.

### Task de background: `services::background`, nunca `spawn`

No Freya, `spawn` amarra a task ao **escopo do componente** cujo handler a
criou, e o runner cancela as tasks do escopo quando ele desmonta. Todo item
do menu Arquivo fecha o menu no mesmo clique, então uma task escopada morria
junto com a lista e o diálogo nativo nunca aparecia — nenhum botão de "abrir"
fazia nada.

Por isso todo trabalho que precisa sobreviver ao clique (diálogo nativo,
varredura, gravação) sobe por `services::background`, que é `spawn_forever`
(task da raiz). `tests/menu_task.rs` cobre isso com o `Dropdown` real e um
clique de verdade.

### Atalhos

| | |
|---|---|
| `Ctrl+1` / `Ctrl+2` / `Ctrl+0` | recolher navegador / galeria / restaurar |
| `Ctrl+=` / `Ctrl+-` | tamanho do texto (75%–140%) |
| `Ctrl+O` / `Ctrl+Shift+O` | abrir arquivos / abrir pasta |
| `←` `→` | foto anterior / próxima |
| `+` `-` `0` | zoom in / out / ajustar |
| arraste | pan (quando a imagem passa da viewport) |
| duplo clique | ajustar à viewport |
| `Ctrl+Z` / `Ctrl+Shift+Z` | desfazer / refazer |
| `F2` | renomear |
| `F9` | maximizar visualizador |
| `F11` | fullscreen |
| `Esc` | fecha modal, sai de fullscreen, cancela crop |

## Desenvolvimento

```bash
cargo test && cargo fmt --all && cargo clippy --all-targets
```

Veja [CONTRIBUTING.md](./CONTRIBUTING.md) e o [ROADMAP.md](./ROADMAP.md)
(planejado até a 2.0, incluindo editor RAW enxuto via feature flag).

## Licença

MIT — veja [LICENSE-MIT](./LICENSE-MIT).

Apoie: [ko-fi.com/raillen](https://ko-fi.com/raillen)
