# Code Agents — agentes e skills

> **Status: aprovado**

Workforce do Prumo (`poppy-team/prumo`, `src/prumo/resources/workforce`):
39 agentes, 189 skills, 20 recipes. Este documento lista o que é obrigatório,
o que é complementar e o que é proibido para o PhotoShow — um app desktop
Rust/Freya de visualização e edição leve.

Nada aqui inventa agente ou skill: nomes são os do repositório Prumo. O que
este documento decide é **quais** valem para este projeto e **quando**.

## Agentes obrigatórios

| Agente | Quando acionar | Domínio |
|---|---|---|
| `implementer` | toda correção, melhoria e feature | todos |
| `reviewer` | todo PR antes de merge | todos |
| `tester` | toda mudança de comportamento | 07 (testes) |
| `debugger` | regressão, falha de CI, bug reportado | todos |
| `architect` | decomposição de módulo, nova fronteira | 01, 03 |
| `accessibility-reviewer` | toda mudança de UI | 04 |
| `performance-agent` | Media, prefetch, thumbs, decode, save | 01, 05 |
| `security-reviewer` | save, rename, delete, scan, installer | 05, 06 |
| `documentation-maintainer` | toda mudança que altera decisão | website/ |
| `explorer` | antes de qualquer refactor (mapear real) | todos |

## Agentes complementares

| Agente | Quando acionar |
|---|---|
| `quality-reviewer` | release candidate, fechamento de milestone |
| `release-verifier` | tag, checklist da 1.0 |
| `issue-triager` | inbox de issues, classificação P0/P1/P2 |
| `issue-author` | proposta nova (molde diagnóstico → direção) |
| `design-system-engineer` | tokens de tema, `ui.rs`, i18n visual |
| `ux-architect` | fluxo novo (triagem, filtros salvos, comparador) |
| `ui-component-engineer` | componente novo (célula, modal, slider) |
| `backend-engineer` | workers, pump, `Services`, batch |
| `devops-engineer` | CI, release.yml, install.sh |
| `technology-decision-agent` | trocar dependência, renderer, runner de teste |
| `isolation-auditor` | verificar fronteira (domínio sem GUI/GPU/IO) |
| `systems-architect` | mudança que atravessa 3+ domínios |

## Agentes proibidos (fora do escopo)

`advertising-designer`, `brand-designer`, `creative-director`,
`motion-designer`, `svg-artist`, `networking-engineer`,
`database-engineer`, `scientific-computing-agent`, `prototyper`,
`visual-identity-auditor` — sem banco, sem rede, sem nuvem, sem rebrand.
Se um dia entrar (ex.: site de download), abre-se proposta própria.

## Skills obrigatórias

| Skill | O que cobra | Onde dói se faltar |
|---|---|---|
| `lang-rust` | borrow checker, `Result`/`Option`, clippy `-D warnings` | todo `.rs` |
| `clean-code` | nomes completos, função única, sem abreviação inventada | 00-philosophy |
| `refactoring` | cheiro → comportamento → teste → transformação mínima | decomposições |
| `testing-quality` | pirâmide, determinismo, regressão, isolamento, flaky | 07, 06-release |
| `error-handling` | erro tipado, `Result` até a borda, sem `expect` em produção | 05 |
| `filesystem-security` | caminhos como dado, sem shell, lixeira, rename atômico | 05 |
| `concurrency-quality` | geração, cancelamento, bounded, sem thread por clique | 01 |
| `caching` | dirty-gate, teto real, invalidação explícita | 01 |
| `performance-native` | passe único, amostragem, sem clone de full-res | 01, 02 |
| `benchmarking` | baseline → budget → gate, sem número inventado | 06-release |
| `documentation` | Rustdoc instrutivo, invariante documentada | todo domínio |
| `documentation-for-llms` | docs legíveis por agente (contrato, não prosa) | website/ |
| `architecture-quality` | uma fonte de verdade, fronteira, anti-bloat | 00, 03 |
| `ci-cd` | lint → teste → build, matriz honesta | 06-release |
| `dependency-management` | dependência justifica existência, `--locked` | Cargo.toml |
| `secure-coding` | sem `unwrap` em caminho de usuário, input validado | todos |
| `supply-chain-security` | lockfile, checksum, auditoria de crate nova | 06-release |

## Skills de acessibilidade e neurodivergência (obrigatórias em UI)

| Skill | O que cobra | Exemplo no PhotoShow |
|---|---|---|
| `accessibility` | checklist WCAG 2.2 AA completo | baseline de todo modal |
| `cognitive-clarity` | comunicação inequívoca, uma decisão por vez | textos de status e erro |
| `nd-explain` | legível sob carga cognitiva (TDAH/dislexia) | ajuda F1, diálogos, docs |
| `screen-reader` | nome/função/estado anunciado | botões só-ícone da toolbar |
| `keyboard-accessibility` | tudo operável por teclado, sem armadilha | crop por teclado, galeria |
| `focus-management` | foco visível, ordem lógica, retorno ao fechar modal | dialogs, menu Arquivo |
| `contrast` | 4.5:1 texto, 3:1 componente, nos 4 temas | slate/charcoal/frost/paper |
| `motion-accessibility` | respeita `prefers-reduced-motion` | slideshow, transições |
| `zoom-reflow` | 200% sem perda, texto 75–140% sem vazar | `ui_scale`, `ROW_PAD` |
| `design-tokens` | cor/medida/texto por token, nunca inline | i18n TOML + `theme.rs` |
| `design-system` | primitivas em `ui.rs`, zero regra de negócio | `Metrics`, `Role` |
| `minimalist-ui` | um acento, borda 1px, sem gradiente | direção visual atual |
| `user-flows` | fluxo documentado antes do componente | triagem, filtros salvos |
| `interaction-design` | gesto local, commit via canal | viewer, crop, compare |
| `visual-regression` | screenshot por estado crítico | empty/loading/failed/saving |

## Skills complementares (por demanda)

`color-science` (ajustes, exposição linear), `state-management` (canais
Radio), `serialization` (config/sidecar TOML+JSON), `api-contract-testing`
(fronteira `DisplayFrame`), `github-*` (issue/PR/release),
`release-engineering` (empacotamento), `observability` (status, logs),
`threat-modeling` + `security-review` (auditoria de release),
`prompt-engineering` (diretivas deste domínio), `project-intelligence`
(navegação do repo), `design-research` (fluxo de triagem real).

## Recipes do Prumo aplicáveis

| Recipe | Uso no PhotoShow |
|---|---|
| `bug-fix` | correção com regressão (menu Arquivo, crop, EXIF) |
| `feature-standard` | feature nova (flip, presets, pick/reject) |
| `architecture-change` | decomposição (`Inner`, `AppState`, `toolbar`) |
| `ui-feature` | componente novo (célula, modal, comparador) |
| `ui-review` | auditoria de acessibilidade por superfície |
| `documentation-refactor` | este site (foi assim que nasceu) |
| `release` | tag + changelog + checksums + smoke |
| `security-review` | save/rename/delete/scan antes da 1.0 |
| `security-audit-release` | gate da 1.0 |
| `github-issue` | proposta nova no molde do caderno |

## Como acionar

Agente e skill do Prumo são playbooks: antes de implementar, o code agent
lê a skill correspondente (`~/.agents/skills/<nome>/SKILL.md`) e segue o
checklist. O `accessibility-reviewer` + skills de a11y rodam em toda mudança
de UI — sem exceção, sem "depois a gente vê".
