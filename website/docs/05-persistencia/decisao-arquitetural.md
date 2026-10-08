# Persistência — decisão arquitetural

> **Status: aprovado**

## Responsabilidade

A Persistência guarda três coisas, cada uma no seu lugar, cada uma atômica:
preferências do usuário, organização por pasta e pixels salvos com metadados.
Nada aqui conhece widgets, GPU ou estado reativo.

```text
persistência
├── AppConfig      (JSON em <config_dir>/photoshow/config.json)
├── load_from      (tolerante: serde(default) + clamp + backup de corrupto)
├── save_to        (atômico: tmp + fsync + rename)
├── FolderSidecar  (.photoshow.json na pasta: ratings/colors/tags)
├── save_baked     (tmp vizinho + fsync + rename + sync de diretório)
├── ExifMeta       (rebased: Orientation=1, dims novas, sem offsets velhos)
└── embed          (só JPEG/WebP, só no temporário, recusa sem corromper)
```

O **Media** decodifica e publica. O **Editor** assa (`bake`). A **Application**
decide quando salvar (`start_save` em background). A **UI** só mostra
"Salvo em …" ou "Falha ao salvar: …".

> Disco é tudo-ou-nada. Original intacto ou novo completo — nunca truncado.

## Diagnóstico atual

Levantado sobre o código real (`config.rs`, `fs_browser.rs` sidecar/scan,
`editor.rs` `save_baked`, `exif.rs` `ExifMeta`/`embed`).

### Reutilizar

**Save atômico de verdade.** `save_baked`: EXIF lido antes do rename,
encode no temporário vizinho (mesmo filesystem, senão rename vira cópia),
`fsync` do arquivo, EXIF embutido no temporário, `rename` como único passo
irreversível, `sync_parent` no Unix, limpeza do tmp em todo erro. Temporário
oculto (`.nome.photoshow-<nanos>.ext`) com a extensão do destino para o
encoder acertar — e invisível à varredura se o processo morrer. REUSE.

**Config tolerante com recovery.** `load_from`: `serde(default)` por campo
(config antiga carrega), clamp de todas as faixas (qualidade, escala,
docks, slideshow), `retain` de favoritas mortas, e corrupto vira
`config.corrupt-<epoch>.bak` + padrões — nunca pânico, nunca silêncio total.
`save_to` atômico (tmp + `sync_all` + rename + limpeza). Testes cobrem config
antiga, roundtrip e NaN. REUSE.

**Sidecar portátil.** `.photoshow.json` na pasta: ratings (0 remove, 1–5
clampa), colors (0 remove), tags normalizadas (trim + lowercase, sem
duplicata, lista vazia remove a chave). Save atômico com tmp + rename.
Nunca banco escondido, nunca toca no original, nunca exige migração. REUSE.

**EXIF com invariante documentada.** Pixels salvos já orientados +
`Orientation = 1` é a única leitura consistente; dimensões acompanham o
recorte; offsets/strips do arquivo antigo descartados; `Orientation = 1`
garantido mesmo quando ausente. `embed` por container (APP1/EXIF), recusa
lixo e PNG/BMP/GIF/TIFF sem alterar um byte. Falha de EXIF nunca falha o
save — regrava sem metadado. Tudo testado, inclusive os 8 orientations. REUSE.

### Reutilizar com refatoração

**Sem `version` na config.** `serde(default)` aguenta campo novo, mas não há
`version: u32` para migração dirigida nem aviso `LoadedWithWarning`
(config corrompida volta a padrões sem contar à UI). Preservar formato e
tolerância, adicionar `version` + `Loaded/RecoveredFromCorrupt` como tipo de
retorno — a UI já tem canal `Status` para o aviso.

**Sidecar sem `version` nem validação.** `load_for_dir` aceita qualquer JSON
que parseie; rating 99 de arquivo editado à mão entra sem clamp na leitura
(só `set_rating` clampa). Preservar portabilidade, adicionar `version` e
sanitização na carga (clamp ratings, filtrar tags vazias).

**`save_baked` com 4 parâmetros posicionais.** `(img, dest, jpeg_quality,
source)` + `start_save` com 7 (`#[allow(clippy::too_many_arguments)]`).
Preservar o fluxo, agrupar em `SaveRequest { source_path, dest_path, edit,
display_base_dims, jpeg_quality, reload }` — o worker monta uma vez, o
`bake` consome.

**Temporários com relógio como unicidade.** `as_nanos`/`as_secs` do sistema
podem colidir em clock repetido; funciona, mas documentar como dívida —
sufixo aleatório ou PID resolve sem mudar o protocolo.

### Refatorar fortemente ou substituir a estrutura

- `delete_to_trash` via crate `trash` mora em `fs_browser.rs` sem menção
  neste domínio — trazer para cá como operação destrutiva com a mesma regra
  (evidente e recuperável: lixeira, não `remove_file`).
- `rename_photo` idem: renomear é persistência (mesmo diretório, atômico por
  rename), não varredura — mover o dono para cá.
- `ExifDetails::read_from` (painel `Ctrl+I`) faz IO síncrono no `select_photo`
  — caminho da UI; mover para leitura lazy em background com cache por
  `(path, mtime)`.

## Decisões aprovadas

1. Três donos, três arquivos, três escritas atômicas: `config.json`,
   `.photoshow.json`, foto salva. Nenhum divide protocolo com outro.
2. Original nunca é truncado: temporário + fsync + rename é o único caminho
   de escrita em arquivo do usuário.
3. EXIF entra no temporário antes do rename — imagem e metadado chegam
   juntos, sem instante de foto-salva-sem-metadado.
4. Falha de EXIF degrada para save sem metadado; falha de encode/persistência
   falha o save com mensagem — nunca o contrário.
5. Config corrompida vira backup + padrões + aviso; nunca pânico, nunca
   silêncio.
6. Sidecar é JSON portátil na pasta; rating 0/color 0/lista vazia removem a
   chave em vez de guardar zero.
7. `Orientation` salva é sempre 1; dimensões salvas são sempre as do arquivo
   novo.
8. Operação destrutiva é evidente e recuperável (lixeira); `remove_file` em
   foto do usuário é proibido.
9. `version: u32` em config e sidecar antes de qualquer campo novo que mude
   semântica.
10. `SaveRequest` explícito substitui parâmetros posicionais no caminho de
    save.

## Consequências

- `load_from` devolve `Loaded(config)` / `RecoveredFromCorrupt` — a UI avisa
  no `Status`.
- `SaveRequest` único do `start_save` ao `bake` ao `save_baked`; batch
  (0.5) reutiliza o mesmo tipo em vez de segunda implementação.
- `rename_photo` e `delete_to_trash` mudam de dono (varredura → persistência)
  sem mudar comportamento.
- `ExifDetails` vira lazy com cache; `select_photo` para de fazer IO
  síncrono.

## Regra anti-bloat

Não criar banco de dados interno, catálogo indexado, journalling próprio,
versionamento de arquivos, nuvem/sync/login, nem migração automática de
formato. JSON + rename atômico + lixeira do SO resolvem o escopo. SQLite
continua proibido para organização (decisão da 0.2, mantida).
