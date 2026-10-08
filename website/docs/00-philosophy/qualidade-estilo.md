# Qualidade e estilo de código

> **Status: aprovado**

## Código autoexplicativo

O código deve priorizar leitura e intenção. Abreviações locais ou inventadas de
nomes de variáveis, parâmetros, funções e tipos não são aceitas.

Evitar:

```rust
fn select_photo(state: &mut AppState, index: usize, photo: PhotoPath) {
    state.sel = Some(index);
    state.current = Some(photo);
}
```

Preferir:

```rust
fn select_photo(
    application_state: &mut AppState,
    photo_index: usize,
    photo: PhotoPath,
) {
    application_state.selected_index = Some(photo_index);
    application_state.current_photo = Some(photo);
}
```

Também preferimos `photo_index`, `display_dimensions`, `sort_criteria`,
`search_query` e `current_photo` a nomes como `i`, `idx`, `sel`, `cfg`, `st`
e `cur`.

## Newtypes

Newtypes são preferidos quando o tipo primitivo não carrega significado
suficiente:

```rust
pub struct PhotoPath(PathBuf);
pub struct PhotoId(usize);
pub struct ThumbRevision(u64);
```

Aliases podem ser usados quando melhorarem a leitura sem fingir segurança de
tipo que não existe.

## Funções

Uma função deve representar uma responsabilidade compreensível. Funções grandes
devem ser divididas quando possuírem fases distintas, múltiplos níveis de
decisão, muitos temporários ou efeitos diferentes.

A regra não é um limite artificial de linhas. Clean Code é ferramenta, não
religião.

## Documentação

Comentários não devem repetir o código. Devem explicar intenção, invariantes,
decisões e restrições.

APIs públicas devem possuir Rustdoc instrutivo, deixando claro o contrato,
invariantes, erros, efeitos colaterais e unidades quando relevantes.

## Parâmetros booleanos

Chamadas opacas como `rotate(state, services, true)` devem ser evitadas.
Preferir enums com nomes explícitos:

```rust
pub enum RotationDirection {
    Clockwise,
    CounterClockwise,
}
```

## Side effects

Criação de structs de domínio não deve executar I/O implicitamente.
Filesystem, relógio, threads, diálogos e recursos do sistema entram por
serviços explícitos — no PhotoShow, por `Services` e `services::background`.
