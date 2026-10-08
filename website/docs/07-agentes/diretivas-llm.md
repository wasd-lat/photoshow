# Code Agents — diretivas para LLMs

> **Status: aprovado**

Prompt-guia para qualquer LLM operando neste repositório. Copie o bloco
abaixo para o contexto do agente antes de pedir implementação. Ele aponta
para os documentos que valem como autoridade — o agente lê o site, não
adivinha.

## Prompt canônico

```text
Você está implementando no PhotoShow (Rust + Freya, visualizador de fotos
rápido, sem banco, sem nuvem, sem IA).

AUTORIDADE (leia antes de codar, nesta ordem):
1. website/docs/07-agentes/agentes-skills.md — quais agentes/skills usar
2. website/docs/07-agentes/estrategia-testes.md — pirâmide e regra de cobertura
3. O domínio do que você vai mexer (01-media … 06-release) — decisão aprovada
4. Proposta correspondente (*-proposta) — só implemente o que está aprovado;
   proposta é hipótese, não ordem
5. website/docs/00-philosophy/qualidade-estilo.md — nomes, newtypes, side effects

REGRAS DURAS:
- Reuse > Refactor > Move > Rewrite. Reescrever é último recurso.
- Domínio não importa freya, GPU, thread, filesystem nem catálogo de UI.
- Transição pura nunca recebe Services; efeito nunca mora em função pura.
- Preview e bake chamam a mesma função. Full-res nunca em getter de UI.
- Original nunca truncado: tmp + fsync + rename. Lixeira, nunca remove_file.
- Um arrasto de slider = um undo. Teste prova, não slogan.
- Skill correspondente em ~/.agents/skills/<nome>/SKILL.md antes de implementar.
- accessibility-reviewer + skills de a11y em TODA mudança de UI.
- cargo nextest run --locked no fim. Se quebrou, você conserta.

ANTI-BLOAT (recuse educadamente e aponte para a proposta):
máscaras, curva por canal, IA, facial, nuvem, plugins, banco, vídeo,
multi-backend, framework de mock/snapshot, cobertura % como gate.

ENTREGA: código + teste que prova o comportamento + doc atualizada no domínio.
Sem doc atualizada, não está pronto.
```

## Caminhos por tipo de tarefa

| Tarefa | Ler primeiro | Agentes | Skills chave | Recipe |
|---|---|---|---|---|
| bug de UI | 04-interface + proposta i18n | debugger, accessibility-reviewer | `minimalist-ui`, `focus-management` | `bug-fix` |
| edição nova | 02-editor + novas-edicoes | implementer, tester | `color-science`, `testing-quality` | `feature-standard` |
| organização | 03-application + organizacao | implementer, ux-architect | `state-management`, `serialization` | `feature-standard` |
| decode/prefetch | 01-media | performance-agent, debugger | `concurrency-quality`, `caching` | `bug-fix` |
| save/EXIF | 05-persistencia | security-reviewer, tester | `filesystem-security`, `error-handling` | `bug-fix` |
| decomposição | 03-application + triagem | architect, explorer | `refactoring`, `architecture-quality` | `architecture-change` |
| componente novo | 04-interface | ui-component-engineer, accessibility-reviewer | `design-system`, `keyboard-accessibility` | `ui-feature` |
| release/tag | 06-release | release-verifier, quality-reviewer | `ci-cd`, `release-engineering` | `release` |
| doc nova | 00-philosophy + este domínio | documentation-maintainer, issue-author | `documentation`, `documentation-for-llms` | `documentation-refactor` |

## Ordem de implementação (rápida e assertiva)

```text
1. explorer mapeia o real (nunca proponha sobre código não lido)
2. skill correspondente lida (checklist na mão)
3. teste que prova o comportamento (vermelho primeiro)
4. implementação mínima (Reuse > Refactor > Move > Rewrite)
5. cargo nextest run --locked + fmt + clippy -D warnings
6. accessibility-reviewer se tocou UI
7. doc do domínio atualizada (decisão ou proposta, nunca as duas)
8. só então declarar concluído
```

Essa sequência é a regra do plano original (§28) com skills e agentes
acoplados — um code agent que pula o passo 1 ou 2 está adivinhando.

## Sinais de que o agente saiu do trilho

- Propôs dependência nova sem justificar (regra 9 da filosofia).
- Tocou em `decisao-arquitetural.md` aprovado para "melhorar" junto da
  feature — aprovado não se reescreve; proposta vai em arquivo `-proposta`.
- Testou widget em vez de extrair lógica (pirâmide torta).
- Implementou proposta com `Status: proposta` como se fosse aprovada.
- Adicionou máscara/IA/nuvem/banco "porque é fácil com crate X".
- Quebrou `cargo test` e declarou "pronto, só falta…" — não está pronto.

## Para humanos revisando agentes

1. O agente leu o domínio? (pergunte qual decisão cobre a mudança)
2. O teste prova comportamento ou só chama a função?
3. A doc foi atualizada no arquivo certo (decisão vs proposta)?
4. A11y rodou, se tocou UI?
5. `nextest` verde, fmt limpo, clippy sem warning?
```

Cinco "sim" = mergeável. Um "não" = devolve com o número da regra.
