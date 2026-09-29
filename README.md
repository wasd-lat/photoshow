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
| `src/app/viewer.rs` | zoom, pan, crop e menu de contexto |
| `src/app/browser.rs` | favoritas, árvore de pastas e lista virtualizada |
| `src/app/gallery.rs` | grade de miniaturas |
| `src/app/dialogs.rs` | modais de renomear e configurações |
| `src/image_store.rs` | decode em background → `ImageHandle` (sem GUI) |
| `src/thumbs.rs` | fila de miniaturas |
| `src/editor.rs` | pilha de edição não-destrutiva (puro) |
| `src/fs_browser.rs` | varredura e renomear (puro) |
| `src/exif.rs` | orientação EXIF (puro) |
| `src/theme.rs` | temas Freya (slate, charcoal, frost, paper) |

O domínio (`editor`, `exif`, `fs_browser`, `config`) não conhece a GUI
e é testado sem janela.

## Desenvolvimento

```bash
cargo test && cargo fmt --all && cargo clippy --all-targets
```

Veja [CONTRIBUTING.md](./CONTRIBUTING.md) e o [ROADMAP.md](./ROADMAP.md)
(planejado até a 2.0, incluindo editor RAW enxuto via feature flag).

## Licença

MIT — veja [LICENSE-MIT](./LICENSE-MIT).

Apoie: [ko-fi.com/raillen](https://ko-fi.com/raillen)
