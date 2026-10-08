# Filosofia — mapa mental

> **Status: aprovado**

O mapa de navegação da documentação: de quem ela vem, para quem ela serve,
por que existe, como funciona e para que leva. Se você é novo aqui (humano
ou agente), comece por este documento.

## Manifesto

```text
Não reescrever por reescrever.
Recuperar uma arquitetura pequena, compreensível e sustentável —
preservando o máximo de código existente que seja coerente, legível,
testável e tecnicamente saudável.

Cada domínio só vira decisão quando discutido e aprovado.
Hipótese permanece proposta. Aprovado não se reescreve.
Afirmação sem evidence é proposta.
```

## De quem

- **Mantenedor**: Raillen — decisão final de produto, escopo e release.
- **Autoridade técnica viva**: `website/` — este caderno. O que está aqui
  com `Status: aprovado` vale; o resto é hipótese.
- **Arquivo histórico**: `PHOTOSHOW_ENGINEERING_PRUMO_PLAN.md` (monolito de
  1898 linhas) — foi a fonte desta documentação, hoje é referência, não
  autoridade.
- **Fornecedores de playbook**: Prumo (`poppy-team/prumo`) — agentes,
  skills, recipes. O Prumo governa contratos e workflow; o PhotoShow é dono
  das decisões de produto.
- **Referência de stack do site**: Petunia3D (`website/` zero-build) — mesma
  filosofia, mesmo formato, outro projeto.

## Para quem

| Leitor | Entra por | Sai com |
|---|---|---|
| Usuário final | `docs/uso/`, README, ajuda F1 | como usar cada ferramenta |
| Contribuidor novo | este mapa → 00-philosophy → triagem | onde mexer sem quebrar |
| Code agent (LLM) | 07-agentes/diretivas-llm | prompt + caminho + skills |
| Revisor de PR | diretivas § "cinco sim" + domínio | critério de merge |
| Mantenedor | 06-release + propostas | o que falta decidir |
| Pessoa neurodivergente | `nd-explain`, `cognitive-clarity` | texto legível sob carga |

## Porquês (por que documentar assim)

1. **Porque código mente menos que memória, e doc com status mente menos
   que código.** `Status: aprovado/proposta/diagnóstico` diz o grau de
   verdade de cada página.
2. **Porque agente sem autoridade adivinha.** Diretiva + domínio + skill =
   implementação assertiva; sem isso, LLM propõe dependência e reescreve
   aprovado.
3. **Porque escopo vaza sem lista FORA.** Cada domínio tem anti-bloat
   explícito — o que não fazer é tão decisão quanto o que fazer.
4. **Porque acessibilidade tardia é retrofit caro.** A11y e neurodivergência
   entram na arquitetura (canal, token, foco, texto), não no "depois".
5. **Porque performance sem baseline é opinião.** Medir → congelar → gate;
   número inventado é proposta disfarçada.
6. **Porque o projeto sobrevive ao mantenedor.** Decisão escrita com
   rationale sobrevive a troca de humano e de modelo.

## Comos (como a máquina funciona)

```text
diagnóstico (triagem Reuse/Refactor/Move/Rewrite)
   ↓
proposta (arquivo *-proposta, Status: proposta, molde → direção → anti-bloat)
   ↓
discussão (humano + agente, com skills e evidence)
   ↓
decisão aprovada (decisao-arquitetural.md, Status: aprovado, N decisões fechadas)
   ↓
implementação (prompt canônico, teste primeiro, doc atualizada)
   ↓
gate (nextest + fmt + clippy + a11y se UI + review cinco-sim)
   ↓
release (changelog + checksum + smoke + evidence arquivada)
```

Regras de tráfego:

- Proposta nunca edita aprovado — cria arquivo próprio.
- Aprovado nunca é reescrito junto de feature — só por decisão nova.
- Todo documento tem anti-bloat no fim — sem exceção.
- Toda afirmação técnica tem evidence (teste, CI, número) — sem "é rápido".
- Todo texto de UI será token (i18n) — literal novo é dívida.

## Para quê (para que leva)

```text
00-philosophy  → pensar igual (regras, estilo, triagem)
01-media       → pixels rápidos e baratos (decode, prefetch, thumbs)
02-editor      → edição confiável (preview = bake, undo honesto)
03-application → coordenação sem acoplamento (canais, transições)
04-interface   → gesto vira intenção (UI burra, tokenizada, legível)
05-persistencia → disco tudo-ou-nada (atômico, lixeira, EXIF)
06-release     → prova automática (CI, checksum, baseline, escopo)
07-agentes     → humanos e LLMs operando igual (skills, diretivas, gates)
```

Destino final: **1.0** — organizador + editor leve completo, instalável em
1 comando, sem terminal — e um caderno que permite chegar lá sem reescrever
o projeto a cada milestone.

## Mapa dos mapas

| Pergunta | Resposta está em |
|---|---|
| O que posso mexer? | triagem (Reuse/Refactor/Move/Rewrite) por domínio |
| O que vai entrar? | arquivos `*-proposta` (02, 03, 04) |
| Como implemento? | 07-agentes/diretivas-llm (prompt + tabela de caminhos) |
| Com que qualidade? | 07-agentes/estrategia-testes + skills obrigatórias |
| Quem revisa? | 07-agentes/agentes-skills + "cinco sim" |
| O que nunca entra? | anti-bloat no fim de cada documento |
| Por que assim? | este mapa (manifesto + porquês) |
