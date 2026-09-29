# Changelog

Todos os lançamentos notáveis deste projeto. Formato baseado em
[Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/).

## [Unreleased]

### Alterado
- **Migração de egui/eframe para Freya 0.5 (Skia)**, mantendo as mesmas
  funcionalidades. Ver [README → Arquitetura](./README.md#arquitetura).
  - `app.rs` (2.116 linhas, monólito) foi dividido em componentes:
    raiz, estado, serviços assíncronos, atalhos, visualizador, navegador,
    galeria, barra de status e modais.
  - Estado global passa a ser gerenciado por Freya Radio com canais
    (`Photos`, `Viewer`, `Edit`, `Status`, `Config`, `Dialogs`), para que
    cada painel re-renderize só quando o que ele mostra muda.
  - `image_store` e `thumbs` deixaram de conhecer a GUI: emitem
    `ImageHandle` do Freya em vez de texturas egui.
  - `egui_dock` → `ResizableContainer` (mesmos painéis redimensionáveis).
  - `egui-elegance` → temas Freya próprios (mesmos quatro nomes:
    slate, charcoal, frost, paper).
  - `egui-phosphor` → SVGs Lucide embutidos.
  - `egui-dropdown` → componente `Select`.
  - `ctx.request_repaint()` → tasks (`spawn` + `thread`) escrevendo em
    `State` reativo, sem polling de canal na thread de UI.
- Documentação de uso e perfil do projeto atualizados para o novo stack.

### Corrigido
- Layout da barra superior: um espaçador com `Size::fill()` no meio da linha
  comia todo o espaço restante e espremia o filtro de formato até o texto
  quebrar em 8 linhas (barra de 164px). A barra agora usa dois grupos com
  `Alignment::SpaceBetween`.
- Áreas com `Size::fill()` no eixo principal empurravam o rodapé para fora da
  janela (dock) e sumiam com a lista de fotos (navegador). Ambos passaram a
  dividir a altura com `Content::flex()` + `Size::flex()`.
- Zoom e pan do visualizador não alteravam a imagem: o elemento não era
  dimensionado pelo retângulo calculado, então o Skia sempre desenhava o fit.
- A lista de fotos mantinha o realce na seleção anterior: o `PartialEq` do
  `VirtualScrollView` não enxerga estado capturado na closure, então a seleção
  agora faz parte do `builder_data`.
- Miniaturas nunca apareciam: o `State` era escrito de dentro do próprio render
  da galeria, o que não agenda o próximo frame. O drain virou um pulso do pump e
  foi para o render da raiz.
- Varredura e salvamento concluídos em background não redesenhavam a UI (a raiz
  não assinava esses canais).
- `ImageStore::poll` reconstruía o `ImageHandle` a cada tique do pump, subindo a
  imagem para a GPU 60× por segundo e forçando repaint contínuo.
- Hooks do Freya chamados dentro de `.maybe()`, de handlers de evento e de
  funções auxiliares: abortavam o render no frame seguinte.
- Miniaturas 4:3 ficavam com faixa morta no rodapé da célula (agora centralizadas).

## [0.1.0-rc1] - 2026-09-19

Primeiro release candidate do visualizador.

### Adicionado
- Abertura de pastas (varredura assíncrona, sem travar a UI) e arquivos soltos
- Navegação por árvore de pastas com expansão lazy e pastas favoritas fixadas
- Viewer com zoom ancorado no cursor, pan, fullscreen e modo maximizado
- Galeria de miniaturas em grade fluida com tamanho configurável (48–192px)
- Edição não-destrutiva: rotate 90°, crop por arrasto (mover + gizmos,
  proporção opcional, Enter aplica), undo/redo, reset
- Salvar (com confirmação) e Salvar como… em thread, qualidade JPEG configurável
- Correção automática de orientação EXIF
- Renomear arquivos (F2) com validação de extensão
- Menu Arquivo único, menu de contexto (copiar caminho/imagem, abrir,
  mostrar na pasta), temas claro/escuro (egui-elegance), ícones Phosphor
- Painéis redimensionáveis via dock, layout restaurável
- Configuração persistente (favoritas, última pasta, preferências)
- Lista de fotos virtualizada e prefetch de vizinhos com teto de memória

### Notas
- Binário release Linux x86_64 (~22MB, `target/release/photoshow`)
- Requer Rust 1.95+ para compilar
