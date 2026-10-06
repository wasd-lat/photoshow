# photoshow — Roadmap até 2.0

> **Visão:** visualizador ultrarrápido com organização prática e edição leve.
> **Anti-visão:** nunca virar GIMP/Lightroom — cada milestone entrega fluxos
> completos, nunca meio-recurso, nunca bloat.

Estado atual: `0.1.0-rc1` (navegação, filmstrip/galeria, rotate/crop
não-destrutivo, EXIF, favoritas, temas, dock redimensionável).

---

## 0.1.0 — Estabilização da rc1 (definitiva)

Foco: transformar a rc1 em software confiável e distribuível. Nada de
recurso novo grande.

### Persistência e estado
- [x] Persistir layout do dock (proporções navegador/galeria) no config
- [ ] Galeria acompanha a seleção: rolar até o thumb ativo ao navegar
      por setas/clique na lista
- [x] Migração tolerante de config antiga (com recuperação automática de config corrompida e backup)

### Fidelidade de arquivo
- [x] Preservar metadata EXIF no save — o `bake()` reconstruía a imagem e
      apagava tudo. Agora câmera/data/exposição/GPS voltam para JPEG e WebP,
      com `Orientation` normalizada para 1 e dimensões do arquivo novo; TIFF,
      PNG, BMP e GIF seguem sem metadado (não têm onde guardá-lo).
- [x] Save atômico: temporário vizinho + `fsync` + `rename`, para que falha de
      gravação não trunque o original
- [x] `Delete` → lixeira do sistema operacional via `trash` crate (tecla `Delete` ou menu de contexto)

### Empacotamento e repo público
- [x] `LICENSE-MIT` mantida exclusivamente conforme decisão do projeto
- [x] `README.md` com screenshot, recursos, build e atalhos
- [x] `CHANGELOG.md` (formato Keep a Changelog)
- [x] `.desktop` + ícone para Linux
- [x] CI contínua (`ci.yml`) e release (`release.yml`) com `install.sh` corrigido

### Validação
- [x] Auditoria de atalhos: listar todos numa janela de ajuda (`F1` ou `?`)

**Critério de saída:** instalar do zero → abrir pasta de 50k arquivos →
navegar, editar, salvar — sem freeze, sem erro vermelho, sem perda de metadata.

---

## 0.2 — Organização — **concluída**

- [x] Nota/classificação por foto (1–5 estrelas), persistido em sidecar JSON
      `.photoshow.json` na pasta (nunca banco escondido, nunca toca no original)
- [x] Ordenação: nome, data, tamanho (crescente/decrescente com chaves em cache)
- [x] Busca por nome (filtro incremental em tempo real no painel de fotos)
- [x] Painel de metadados EXIF (`Ctrl+I` ou botão na barra: dimensões, câmera, lente, exposição, abertura, ISO, data)

## 0.3 — Apresentação — **concluída**

- [x] Slideshow com temporizador configurável (`Espaço` ou botão na barra)
- [x] Modo apresentação: interface oculta, foco total na foto, `Esc`/`F11` sai

## 0.4 — Edição básica real (ainda não-destrutiva) — **concluída**

Tudo global, tudo na `EditorState` existente (stack + undo/redo + preview):

- [x] Exposição (EV), contraste, saturação, temperatura/tint
      (`src/adjust.rs`, funções puras; exposição em linear, o resto em gamma)
- [x] Histograma RGB + aviso de clipping (estourados/sombras),
      calculado no mesmo passe do RGBA que já ia para a GPU
- [x] Comparador split arrastável (original × editado, `Ctrl+B`), com a
      divisória presa à largura da foto
- [x] `bake()` estendido: preview e save usam a mesma função de ajuste

> **Ainda não feito de propósito:** ajustes locais (máscaras, pinça de branco),
> curva por canal e tonemap. O conjunto de quatro é fechado por decisão — é a
> mesma linha do plano (seção 30) que proíbe máscaras e edição local.

## 0.5 — Lote — **concluída**

- [x] Fila de operações em lote: rotacionar, converter formato,
      redimensionar, renomear com padrão (`foto_###.jpg` ou `foto_{i}`)
- [x] Progresso com cancelamento cooperativo, em thread (nunca trava a UI)
- [x] Relatório final com contagem de sucessos e falhas por arquivo
- [x] Modal `BatchDialog` acessível no menu Arquivo ("Processar em lote…")

## 1.0 — Polimento e distribuição

- [ ] Flatpak e/ou AppImage
- [ ] Testes de integração nos fluxos críticos (abrir→editar→salvar)
- [ ] Docs de usuário (atalhos, formatos, FAQ de performance)
- [ ] Auditoria de performance final (pastas gigantes + TIFFs de 200MB+)
- [ ] Congelar escopo: tudo que não coube vira proposta para 2.x

**Critério de saída da 1.0:** organizador + editor leve completo para
JPEG/PNG/WebP/TIFF, instalável em 1 comando, sem dependência de terminal.

---

## 2.0 (conjectura) — Editor RAW enxuto, sem virar Lightroom

> Princípio: RAW é um **modo a mais do viewer**, não um produto novo.
> O binário padrão continua sem ele (feature flag), e o ajuste máximo
> é global — sem máscaras, sem IA, sem banco de lentes.

### Por que dá para ser leve

1. **Preview embutido primeiro:** todo RAW traz um JPEG embutido. A
   navegação/galeria/thumbs usam ele — custo zero, velocidade igual à de JPEG.
2. **Decode total só sob demanda:** só a foto selecionada (e só ao entrar
   em modo RAW) passa pelo pipeline completo, em thread, com cache de 1.
3. **Feature flag `raw`:** `rawloader` (Rust puro, sem LibRaw/C) entra só com
   `--features raw`. O build padrão continua magro.
4. **Sem DB de lentes/câmeras pesado:** sem `lensfun`, sem perfis DCP —
   matriz de cor vem do próprio arquivo (`rawloader` expõe) + fallback sRGB.

### Pipeline proposto (`src/raw/`, ~4 ajustes, todos puros e testáveis)

```
CFA ──► demosaic bilinear c/ equilíbrio de verde ──► linear RGB
  ──► balanço de branco (multiplicadores do metadata + picker cinza)
  ──► exposição (EV) + recuperação de highlights (clip guiado)
  ──► nível de preto ──► tone-map fílmico simples ──► sRGB ──► textura
```

- Demosaic próprio (~200 linhas, testável em CFA sintético) em vez de
  puxar crate pesado; qualidade "boa", não "estado da arte" — documentado.
- Cada etapa é função pura `&[f32] -> Vec<f32>`: teste unitário barato,
  preview e bake compartilham o código (igual ao `bake()` atual).
- Reuso total da infra existente: threads de decode, `EditorState`
  estendido (`ev`, `wb_temp`, `wb_tint`, `highlights`), histograma e
  clipping da 0.4, save em thread.

### Conjunto fechado de ajustes (não cresce)

Exposição, temperatura/tint, highlights, sombras, contraste, saturação,
curva de tons simples, crop/rotate (os atuais). Ponto.

### Formatos (limitados ao que `rawloader` cobre bem)

NEF, CR2, ARW, RAF, RW2, DNG. **Fora:** CR3 comprimido total (limitação
conhecida do `rawloader` — documentar, não prometer).

### Orçamento de performance (regras duras)

- Abrir pasta com RAWs: tão rápido quanto JPEG (só previews embutidos)
- Thumb RAW: usa preview embutido, nunca decode total
- Decode total: só foto atual, só em modo RAW, cancelável ao navegar
- Binário padrão sem `raw`: zero bytes a mais

### Explicitamente FORA (para não virar bloat)

Correção de lente por banco de dados, denoise com IA, ajustes locais/
máscaras, catálogo com banco de dados, importação com presets, edição
de vídeo, integração com nuvem, plugins.

---

## Futuro / Projeto separado: `photoraw` (RAW Studio & Grading)

> **Decisão arquitetural:** Para manter o `photoshow` leve, rápido e focado em visualização ágil, os recursos avançados de revelação RAW e gradação de cores foram destacados para um projeto dedicado futuro (`photoraw`). O `photoshow` não implementará essas ferramentas de câmara escura.

### Especificação reservada para o `photoraw`:
1. **Highlight Reconstruction (Anti-Magenta):** clipping no espaço linear neutralizando estouros de canais desiguais do sensor Bayer.
2. **Conta-gotas de Balanço de Branco:** cálculo analítico de ganhos $k_r, k_b$ a partir de amostragem pontual de patch neutro ($3\times 3$).
3. **Níveis de Preto/Branco e Equalização de Faixa Dinâmica:** remapeamento linear de pontos extremos.
4. **Highlights & Shadows Analíticos:** curvas ponderadas em luminância sem dependência de mapa espacial pesado.
5. **Vinheta Radial Analítica:** compensação de queda de luz periférica da lente.
6. **Suporte a 3D LUT (.cube):** interpolação trilinear em grade $33\times 33\times 33$ para perfis fílmicos sem dependência externa.
7. **Nitidez Básica (Unsharp Mask):** filtro passa-alta rápido compensando demosaic bilinear.

---

## Como acompanhar

- `0.1.0`: issues com checklist acima, marco `v0.1.0` no GitHub
- `0.2`–`1.0`: uma issue de design curta por milestone antes de codar
- `2.0`: tudo aqui é conjectura — revalidar `rawloader` e escopo antes
  de qualquer linha de código
