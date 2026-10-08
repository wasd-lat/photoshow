# PhotoShow — Organização arquitetural

Este site é o caderno vivo da organização completa do PhotoShow.

A meta não é reescrever por reescrever. A meta é recuperar uma arquitetura
pequena, compreensível e sustentável, preservando o máximo de código existente
que seja coerente, legível, testável e tecnicamente saudável.

## Prioridades

- Linux e Windows primeiro.
- Hardware modesto como requisito real.
- Menos dependências e menos caminhos simultâneos.
- Uma única fonte de verdade por responsabilidade.
- Separação clara entre Domínio, Media Runtime, Application, Persistência e UI.
- Acessibilidade desde a arquitetura.
- Código autoexplicativo, documentação instrutiva e nomes completos.
- Newtypes quando tornarem contratos mais explícitos.
- Reescrita somente quando corrigir a estrutura existente custar mais que substituir.

## Ordem inicial

1. Filosofia e triagem do código atual.
2. Media runtime (decode, display, thumbs, prefetch).
3. Editor e ajustes de cor.
4. Application e estado (seleção, filtros, sessão).
5. Interface (toolbar, browser, viewer, galeria, diálogos).
6. Persistência (config, sidecar, save atômico).
7. Release, testes e consolidação final.
