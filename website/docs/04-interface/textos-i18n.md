# Interface — textos tokenizados e tradução via TOML

> **Status: proposta**

Hoje cores e medidas têm fonte única (`theme.rs`, `ui.rs`), mas os textos não:
~280 literais em PT espalhados por toolbar (66), dialogs (113), viewer (42),
state/status (47+) e erros de domínio (`fs_browser.rs`, `cli.rs`). Traduzir
assim é caçar string — e cada idioma novo multiplica o caça.

A proposta: textos com a mesma disciplina dos tokens visuais. Uma fonte
única, chave estável, fallback garantido, tradução como dado (TOML), nunca
como código.

Nada aqui altera `decisao-arquitetural.md` — aquele documento permanece a
autoridade do que já foi aprovado. Esta proposta só vira arquitetura quando
discutida e aprovada.

## Diagnóstico atual

- Cores: `theme::palette(&nome)` — token por tema, resolvido fora do
  componente. Medidas: `ui::Metrics` + `Role` — uma fonte, `Copy`, sem regra
  de negócio.
- Textos: literais inline (`"Renomear"`, `"Ocultar/mostrar galeria (Ctrl+2)"`,
  `"Crop aplicado no preview — Salvar para gravar."`, `"Nome vazio."`).
  Tooltips repetem o atalho à mão — se o atalho muda, o texto mente.
- Erros de domínio (`"Nome vazio."`, `"Já existe um arquivo com esse nome."`,
  `"Falha ao renomear: {e}"`) nascem em `fs_browser.rs`/`cli.rs`, que hoje
  não importam freya — e não devem importar catálogo de UI.
- `format!` com interpolação (`"Varrendo {}…"`, `"{} movido para a lixeira."`,
  `"Tamanho do texto: {pct}%"`) exige tradução com placeholder nomeado, não
  posicional — ordem das palavras muda entre idiomas.

## Direção proposta

```text
interface
├── i18n.rs            (chave estável → texto; fallback pt-BR; sem freya)
├── locales/pt-BR.toml (fonte, commitada, completa — a UI nunca quebra)
└── locales/en.toml    (primeira tradução; cada idioma = um arquivo)
```

```toml
# locales/pt-BR.toml
[toolbar]
file_menu = "Arquivo"
hide_gallery = "Ocultar/mostrar galeria ({shortcut})"

[status]
scanning = "Varrendo {dir}…"
moved_to_trash = "{name} movido para a lixeira."

[error.rename]
empty = "Nome vazio."
exists = "Já existe um arquivo com esse nome."
```

```rust
// chave estável, texto nunca inline
ui::text(m, ui::Role::Body, ink, t("toolbar.file_menu"))
status!("status.scanning", dir = dir.display())
```

Regras:

1. **Chave estável, texto livre.** Renomear o PT não quebra código; a chave
   (`toolbar.file_menu`) é o contrato, o valor é dado.
2. **Fallback pt-BR sempre.** Chave ausente no idioma ativo = texto pt-BR +
   `debug_assert` + teste que lista chaves órfãs. A UI nunca mostra chave crua.
3. **Placeholder nomeado, nunca posicional.** `{dir}`, `{name}`, `{pct}` —
   tradutor reordena sem quebrar `format!`.
4. **Domínio devolve chave + dado, UI formata.** `rename_photo` devolve
   `RenameError::EmptyName`, não `String`; a borda (state/services) resolve
   para texto via `i18n.rs`. `fs_browser.rs` continua sem freya **e** sem
   catálogo de UI — erro tipado é dado, não texto.
5. **Atalho é dado, não texto.** Tooltip monta `"Ocultar/mostrar galeria
   ({shortcut})"` com o atalho vindo de `shortcuts.rs` — uma fonte para a
   tecla e para a ajuda `F1`. Atalho muda, tooltip acompanha.
6. **TOML, não fluent nem gettext.** Zero dependência nova se o parser for
   manual mínimo — ou `toml` se já estiver na árvore (via `serde`). Sem
   pluralização ICU na V1: dois formulários (`one`/`other`) resolvem
   "1 foto / 40 fotos"; o resto é dívida documentada, não bloqueio.
7. **Idioma no `config.rs`.** `language: "pt-BR"` com default, seletor no
   Config — mesmo padrão de `theme`. Troca aplica sem reiniciar (canal
   `Config` já re-renderiza tudo que consome texto).

## Migração pragmática

1. Criar `i18n.rs` + `pt-BR.toml` com as chaves extraídas (script conta
   literais por arquivo — a tabela acima é o ponto de partida).
2. Migrar tooltips e títulos primeiro (visíveis, sem interpolação).
3. Migrar status com placeholder nomeado.
4. Tipar erros de domínio (`RenameError`, `SaveError`) e resolver na borda.
5. Adicionar `en.toml` como prova de que o mecanismo funciona.
6. Teste `locales_complete`: toda chave usada existe em pt-BR; todo idioma
   cobre 100% ou declara fallback explícito.

Nenhum passo muda comportamento em pt-BR: tradução é troca de dado, e o
primeiro idioma é o próprio PT extraído.

## Consequências

- `state.status` continua `String` — o que muda é quem monta (chave +
   formato na borda, não literal no meio da transição).
- `HelpDialog` vira tabela gerada de `(atalho, chave)` — atalho sem linha
   continua bug, agora checável por teste.
- `color_tags`/`named_tags` são dados do usuário, não texto de UI — fora do
   catálogo, como hoje.

## Regra anti-bloat

Sem framework de i18n com ICU completo, sem detecção automática de idioma do
SO na V1 (seletor manual basta), sem tradução via rede/IA, sem pluralização
por regra de idioma além de `one`/`other`, sem RTL até haver usuário real.
Um TOML por idioma + fallback + teste de completude resolvem o escopo.
