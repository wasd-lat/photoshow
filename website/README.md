# PhotoShow — Documentation Website

Site estático para a documentação técnica, decisões arquiteturais e
acompanhamento da organização do PhotoShow.

A pasta segue deliberadamente a stack, o design e a filosofia do website de
documentação do Petunia3D: **zero build**, dependências pequenas e versionadas
por CDN, leitura previsível, acessibilidade e baixo atrito para manutenção.

## Stack

HTML5, CSS nativo, Web Awesome 3.14.0, Phosphor Icons Web 2.1.2, Alpine.js
3.17.4, Marked 18.0.14, Fuse.js 7.5.0 e Markdown.

Não há Node.js, bundler, package manager, node_modules ou etapa de build.

## Filosofia

Este site é o **caderno vivo da organização**. Cada domínio só vira decisão
quando for discutido e aprovado. Hipóteses permanecem marcadas como propostas.

## Executar localmente

```bash
python3 -m http.server 8080 -d website
```
