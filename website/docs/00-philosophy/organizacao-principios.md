# Princípios da organização

> **Status: aprovado**

## Objetivo

Reduzir complexidade acidental sem reduzir ambição funcional.

## Regras constitucionais

1. Reutilizar o máximo de código já implementado quando ele for coerente, saudável e adequado à nova arquitetura.
2. Refatorar código recuperável antes de considerar reescrita.
3. Reimplementar somente quando a estrutura existente custar mais para corrigir do que para substituir.
4. Uma responsabilidade deve possuir uma fonte de verdade.
5. Domínio não conhece GUI nem GPU.
6. GUI expressa intenção; não governa domínio.
7. Media runtime entrega pixels; não governa domínio.
8. Abstrações futuras não são implementadas antecipadamente.
9. Dependências precisam justificar sua existência.
10. Linux, Windows e hardware modesto são requisitos de primeira classe.
11. Newtypes devem ser usados quando eliminarem ambiguidade semântica ou impedirem uso incorreto de valores.
12. Termos técnicos consolidados como EXIF, JPEG, EXIF, RGBA, GPU e UUID podem permanecer abreviados quando constituírem linguagem do domínio.

## Política de migração

Cada componente é classificado como:

- **Reuse** — preservar praticamente como está.
- **Refactor** — preservar comportamento e melhorar estrutura/legibilidade.
- **Move** — preservar implementação, mas movê-la para a camada correta.
- **Rewrite** — último recurso.
