# Changelog

Todos os lançamentos notáveis deste projeto. Formato baseado em
[Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/).

## [Unreleased]

### Adicionado
- **Processamento em Lote (0.5)**:
  - Fila de operações em background (`src/batch.rs`): conversão de formato, rotação, redimensionamento proporcional e renomeação sequencial (`foto_###.jpg` ou `foto_{i}`).
  - Cancelamento cooperativo sem travar a interface e relatório final com resumo de sucessos e falhas por arquivo.
  - Modal `BatchDialog` acessível via menu Arquivo ("Processar em lote…").
- **Exclusão Segura para Lixeira (0.1)**:
  - Remoção de foto via tecla `Delete` ou menu de contexto enviando o arquivo para a lixeira do sistema operacional (crate `trash`).
- **Automação e CI (0.1)**:
  - Pipeline de CI contínua (`.github/workflows/ci.yml`) com validação de formatação, clippy e compilação em Linux e Windows.
  - Correção do script de instalação one-line (`install.sh`) com detecção resiliente da pasta de staging do tarball e geração de checksums SHA-256 no release.
  - Atualização do manifesto `project-profile.json` para o schema Prumo v2.
- **Organização (0.2)**:
  - **Critérios de ordenação**: ordenação por **Nome**, **Data** de modificação e **Tamanho** do arquivo, com direção crescente/decrescente e chaves pré-calculadas em cache (`sort_by_cached_key`).
  - **Busca incremental**: campo de busca rápida em tempo real acima da lista de fotos.
  - **Painel de metadados EXIF (`Ctrl+I`)**: modal de informações exibindo câmera, modelo de lente, data e hora da captura, dimensões, velocidade do obturador, abertura, ISO e distância focal.
  - **Avaliação fotográfica (1–5 estrelas)**: notas atribuíveis por teclado (`1`..`5` para classificar, `0` para limpar) persistidas em arquivo sidecar `.photoshow.json` na própria pasta, sem banco de dados opaco e sem tocar no arquivo original.
- **Apresentação (0.3)**:
  - **Modo Slideshow**: reprodução automatizada e contínua das fotos da pasta com intervalo configurável (`Espaço` ou botão na barra de ferramentas).
  - **Modo Apresentação**: foco visual exclusivo na fotografia com interface oculta.
- **Estabilização e Arquitetura (0.1 P1)**:
  - **Prefetch leve sem full-resolution (PS-P1-03)**: pré-carregamento de vizinhos mantém apenas o buffer de exibição reduzido em RAM; a versão full-res é decodificada sob demanda apenas ao salvar.
  - **Orçamento de prefetch por RAM real (PS-P1-04)**: descarte LRU calcula os bytes reais da memória descompactada em vez de estimar pelo tamanho do arquivo em disco.
  - **Cancelamento de decodes obsoletos (PS-P1-05)**: geração atômica *latest-wins* descarta decodes de fotos intermediárias ignoradas em navegação rápida antes mesmo de processá-las.
  - **Cópia de imagem assíncrona (PS-P1-06)**: envio de pixels para a área de transferência do SO em worker desacoplado da thread de renderização.
  - **Recuperação de configuração corrompida**: salvamento atômico de preferências e fallback com backup automático (`.bak`) caso o JSON seja corrompido externamente.
  - **Janela de atalhos (`F1` / `?`)**: modal com inventário completo de todos os comandos do teclado.
  - **Persistência de dock**: proporções dos painéis lateral e inferior gravadas na configuração.
- **Edição de cor (0.4)**: painel de ajustes com **exposição** (EV), **contraste**,
  **saturação** e **temperatura**, mais **histograma RGB** com aviso de pixels
  estourados/travados.
  - A matemática é pura e vive em `src/adjust.rs`, sem dependência de GUI.
    Exposição opera em espaço **linear** (um `+1 EV` dobra a luz, como na
    física); temperatura, contraste e saturação em **gamma**, que é onde o olho
    percebe cor.
  - Preview e `bake` (arquivo salvo) chamam a **mesma** função: o que se vê na
    tela é o que vai para o disco. O crop é aplicado antes do ajuste, para não
    pagar o passe de cor sobre pixels descartados.
  - `Ctrl+3` (ou o botão na barra) recolhe o painel; persistido em
    `hide_adjust`.
  - Ajuste é **não destrutivo** como o resto: entra na pilha de undo/redo, e um
    arrasto de slider vira **um** passo de histórico (o `Ctrl+Z` volta o ajuste
    inteiro, não 1% por vez).
  - "Zerar ajustes" limpa só a cor e preserva o enquadramento (rotação/crop) —
    é uma intenção diferente de "Reset".

- **Comparador original × editado**: botão na barra ou `Ctrl+B` abre uma
  divisória arrastável sobre a foto.
  - A metade "antes" mantém a geometria (rotação/crop) e zera só a cor, então as
    duas metades ficam alinhadas e a comparação não mente.
  - A divisória segue a **largura desenhada da foto** (e não a da janela), então
    ela acompanha zoom e pan.
  - Só aparece quando há edição: sem ajuste, o "antes" e o "depois" são a mesma
    imagem.

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

### Adicionado
- **Escala tipográfica** (75%–140%) no modal de configurações, com atalhos
  `Ctrl+=` / `Ctrl+-`. Aparece no config em `ui_scale`; configs antigos sem a
  chave continuam carregando.
- **Recolher painéis**: botões na barra e atalhos `Ctrl+1` (navegador),
  `Ctrl+2` (galeria) e `Ctrl+0` / "Restaurar layout". Persistido em
  `hide_browser` / `hide_gallery`.
- **Botões de recolher** na barra, com ícone e tooltip próprios.

### Visual e leitura
- **Visual minimalista neutro** (referência: apps atuais da OpenAI): neutros
  de baixo contraste, um único acento, bordas de 1px no lugar de sombra,
  tipografia miúda. As quatro paletas existentes (slate/charcoal/frost/paper)
  foram retunadas sem mudar de nome, e o `text_placeholder` deixou de falhar
  em WCAG AA (ele é texto de verdade: caminho, "varrendo…", dica de erro).
- Novas paletas passam em contraste AA para placeholder e para texto do botão
  primário, e a borda tem contraste mínimo testado — é ela que dá a
  hierarquia quando não há sombra.
- **Dropdown fora do fluxo** (`ui::Dropdown`): o `Menu` do Freya marca
  `Layer::Overlay`, que é só ordem de pintura; no torin ele continua
  empilhado e empurrava a interface para baixo ao abrir.
- Um espaçador com `Size::fill()` no meio da barra comia todo o espaço
  restante e espremia o filtro de formato até o texto quebrar em 8 linhas
  (barra de 164px). A barra agora usa dois grupos com
  `Alignment::SpaceBetween`.
- Áreas com `Size::fill()` no eixo principal empurravam o rodapé para fora da
  janela (dock) e sumiam com a lista de fotos (navegador). Ambos passaram a
  dividir a altura com `Content::flex()` + `Size::flex()`.
- Linhas da lista de fotos com altura fixa faziam nomes longos invadir a
  linha de baixo; a altura agora deriva da tipografia.

### Corrigido
- **Salvar sobrescrevia a foto sem proteção**: a gravação ia direto no
  destino, então uma falha de disco ou um `kill -9` no meio do encode deixava
  um JPEG truncado no lugar do original. Agora o encoder escreve num
  temporário vizinho, `fsync` garante a chegada ao disco e o `rename` troca
  o arquivo de uma vez — o save é tudo-ou-nada. O EXIF entra no **mesmo**
  temporário, então não existe instante em que a foto está salva sem os
  metadados dela.
- **Salvar apagava o EXIF**: câmera, data, exposição e GPS sumiam no
  `bake()`. Agora são preservados para JPEG e WebP, com três correções que a
  reescrita exige:
  - `Orientation` vai a `1`, porque os pixels salvos já estão fisicamente
    orientados — copiar o valor antigo faria a foto aparecer girada de novo;
  - as dimensões passam a ser as do arquivo novo (depois de um crop, o valor
    antigo descreve uma imagem que não existe mais);
  - offsets de miniatura e de tiras são descartados, por apontarem para
    posições do arquivo antigo.
  - TIFF/PNG/BMP/GIF seguem sem metadado: não têm onde guardá-lo, e perdê-lo é
    melhor que gravar um arquivo corrompido.
- **Ajustes de cor não chegavam ao arquivo salvo** quando não havia crop: o
  `bake` zerava o ajuste antes de girar e retornava sem reaplicá-lo, então o
  preview mostrava a edição mas o `Salvar` gravava a imagem original. Coberto
  por teste de integração (`adjust_reaches_the_saved_file_not_only_the_preview`).
- **Menu Arquivo não clicável**: os itens iam como um `Menu` pronto e o
  `Dropdown` embrulhava em outro, então o menu interno engolia o externo. O
  `Dropdown` agora monta o `Menu` e recebe só os itens.
- **Zoom e pan não faziam nada**: a imagem não era posicionada pela área
  calculada, e o `Position::Absolute` somava a coordenada de tela uma segunda
  vez (435 + 900 = 1335), jogando a foto para fora da janela.
- **Pan não começava nunca**: `can_pan` ignorava o zoom, e como o `fit`
  garante que em zoom 1 tudo cabe, a resposta era sempre "não".
- **Pan soltava a foto**: arrastar até a borda levava a imagem para fora da
  janela. Agora o offset é limitado às bordas.
- **Crop nascia deslocado**: os handlers usam `element_location` (relativo),
  mas `draw`/`area` vêm do `on_sized` (de tela). O rect do crop é guardado em
  coordenadas locais e convertido para tela só na hora de desenhar.
- **Rotação a partir do 2º clique**: `display_base_dims()` lia `display_img`,
  que `apply_preview` sobrescreve com a imagem já transformada — a 2ª rotação
  derivava. Agora a base fica guardada à parte, e o preview sempre parte da
  imagem sem edição.
- **`EditorStack::default()` entrava em pânico**: o `Default` derivado criava
  `history` vazio e `state()` fazia `expect`.
- Alvo do star de favorito tinha 14px (inclicável); agora tem 24px.
- Miniaturas nunca apareciam: o `State` era escrito de dentro do próprio render
  da galeria, o que não agenda o próximo frame. O drain virou um pulso do pump.
- Varredura e salvamento em background não redesenhavam a UI (a raiz não
  assinava esses canais).
- `ImageStore::poll` reconstruía o `ImageHandle` a cada tique do pump, subindo a
  imagem para a GPU 60× por segundo e forçando repaint contínuo.
- Hooks do Freya em `.maybe()`, handlers e funções auxiliares abortavam o
  render no frame seguinte.

### Desempenho
- O pump acordava a cada 16ms mesmo sem trabalho, segurando ~0,35% de CPU com a
  janela ociosa. Agora o tique é adaptativo: 16ms com decode/miniatura em voo,
  125ms parado.
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
