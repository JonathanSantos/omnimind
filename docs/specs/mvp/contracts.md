# Contratos: API de `omnimind_core` para o app

Crate `crates/core`, package `omnimind-core`, lib `omnimind_core`. Tudo abaixo é reexportado na raiz, salvo indicação. Regras em `core.md`; MCP em `mcp.md`.

```toml
omnimind-core = { path = "../core" }
```

## Regras de uso

- `Brain` é `Clone + Send + Sync`. Clones compartilham um único estado. Tipos públicos de retorno são `Clone + Send + 'static`.
- Tudo é síncrono e pensado para uma thread de fundo. Consultas seguram um `Mutex` interno; `reconcile_slice` solta entre fatias.
- Nada cria um brain sem `init`. Brain movido ou apagado devolve `Error::BrainNotFound` e nada é recriado.
- Texto vindo do disco pode ter invisíveis. A interface usa `MemoryDetail.revealed` ou `inspect::reveal` e nunca interpreta HTML.
- O app abre com `client` próprio e escreve com `Actor::person()`.
- `NoteMeta.path` traz o nome real do arquivo, que pode ter invisíveis e bidi: mostre com `inspect::reveal`. `NoteMeta.flags`: `invisible_name`, `instruction_like`.

## Abrir

```rust
Brain::init(path, name) -> Result<Brain>                    // AlreadyExists se houver brain.toml
Brain::open(path) -> Result<Brain>
Brain::open_with(path, BrainOptions { client, session, cwd, lazy }) -> Result<Brain>
brain.config() -> &BrainConfig; brain.root() -> &Path; brain.session() -> &str
brain.for_client(name) -> Brain                             // mesmo estado, outro cliente no diário
brain.record_session("start" | "end", cwd) -> Result<()>
```

Para abrir sem travar a interface: `lazy: true`, e numa thread `while brain.reconcile_slice(Duration::from_millis(30))? > 0 {}`.

## Notas

```rust
notes() -> Result<Vec<NoteMeta>>
read_note(path) -> Result<NoteText>                         // meta.hash é o base_hash
save_note(path, text, base_hash: Option<&str>) -> Result<SaveResult>
    // SaveResult::Saved { hash } | Conflict { current_hash, current_text }
    // Nota nova: base_hash None. Nota existente sem hash: Conflict. Quem guarda o texto digitado é o app.
rename_note(from, to) -> Result<RenameReport>
    // Só reescreve wikilinks que resolviam para a nota. Err(RenameCapturesLinks { target, notes })
    // quando o nome novo capturaria link que hoje aponta para outra nota. Nada muda nesse caso.
delete_note(path) -> Result<TrashEntry>                     // lixeira interna
trash() -> Result<Vec<TrashEntry>>
restore_note(trash_id) -> Result<String>                    // AlreadyExists se o caminho foi ocupado
```

Caminhos são relativos ao brain, com `/`. Nota fica fora de `memories/` e de pastas ocultas. Resolução de link: caminho, nome do arquivo, título, alias; empate vai para o caminho mais curto.

## Memórias

```rust
memories() -> Result<Vec<MemoryMeta>>                       // todas, com estado derivado
memory(id) -> Result<Option<MemoryDetail>>                  // inclui quarentena; `revealed` mostra invisíveis, `flags` diz o que foi achado
apply(Change, Actor) -> Result<Applied>
undo(event_id) -> Result<UndoReport>
    // Err(UndoRefused { reason, chain }): a memória mudou depois; `chain` são os eventos a desfazer antes, do mais novo ao mais velho.
undo_session(session) -> Result<UndoReport>                 // ordem reversa, tudo ou nada
reinforce(id, EventKind::{Helped, Hurt, Cited}) -> Result<()>
```

```rust
enum Change {
    Write(MemoryDraft),                                     // sem id cria; com id exige base_hash
    Archive { id, base_hash, reason },
    Restore { id, base_hash, reason },
    Promote { id, base_hash, reason },                      // só Actor::person
    CorrectLine { id, base_hash, text },                    // linha do perfil editada
    Merge { into, from, base_hash, reason },
    AddAlias { id, alias, reason },
}
Actor::person() | Actor::agent(client, session) | Actor::system() | Actor::external(client) | Actor::import()
    .with_cwd(cwd) .with_session(s)
Outcome::{ Written, Merged { into }, Superseded { ids }, Quarantined { reason }, Rejected { reason, hint }, Unchanged }
```

`Applied { outcome, id, rule, hash, event, warnings, neighbors, tags, tag_vocabulary, current_text }`. `Rejected` não é erro de Rust: é resposta do portão. `base_hash_mismatch` traz `current_text` e `hash`; memória retida ou com segredo traz só o `hash`. `event` é o id para `undo`.

`MemoryMeta.flags` e `Hit.flags` podem trazer `unverified_authority`: o arquivo declara `person` sem evento no diário, e `authority` já vem como `agent`.

**Motivos de quarentena** (`MemoryMeta.quarantine_reason`, um código, ou vários separados por `, ` quando vêm do portão). Código estável em inglês; a interface traduz:

| Código | Significa |
|---|---|
| `external_authority` | veio de ferramenta ou de fora |
| `secret` | parece credencial, em arquivo editado à mão |
| `invisible_text` | caractere invisível |
| `disguised_text` | letra disfarçada ou alfabetos misturados na mesma palavra |
| `header_block` | bloco `---` que imita cabeçalho |
| `instruction_like` | frase de instrução, shell, exfiltração, marcador ou base64 |

Quem libera é a pessoa, com `Change::Promote`. Vale para aquele texto: editar devolve à inspeção. Agente recebe `memory_held` ao tentar mexer.

**`Rejected.reason` novos ou mudados:** `memory_held`, `cannot_edit_person_memory` (era `cannot_edit_higher_authority`), `key_reused` agora cobre mesma chave com outro conteúdo.

Para `CorrectLine`, use `PortraitLine.id` e `PortraitLine.hash`. O resultado é uma memória `person` nova que substitui a anterior.

## Busca

```rust
search(&SearchQuery) -> Result<SearchResponse>
SearchQuery::person(q)                                      // quarentena visível e marcada
SearchQuery::new(q)                                         // visão do agente
SearchQuery { queries, limit, offset, types, tags, project, include_old, audience, record, wait_ms }
```

`wait_ms: None` espera a indexação terminar; o app deve usar `Some(n)` e tratar `SearchStatus::Incomplete`. `SearchResponse { status, hits, total, next_offset, coverage, reason, corrected, took_ms }`. `Hit.id` é ULID da memória ou `note:<caminho>`. `Hit.path` abre direto.

## Ao vivo

```rust
journal_tail(&Cursor) -> Result<JournalBatch>
// JournalBatch { events, cursor, bad_lines, partial_tails }
reconcile() -> Result<ReconcileReport>
reconcile_slice(budget: Duration) -> Result<usize>          // arquivos ainda pendentes
flush() / maybe_snapshot()                                  // ao sair, e no máximo a cada 30 s
```

O app guarda o `Cursor` (serializável) e chama `journal_tail` ao receber aviso de `notify` em `.omnimind/journal/`, ou por releitura periódica. Eventos vêm unidos por `ts`, `seq`, `id`; o cursor só anda sobre linhas completas.

`Event { id, ts, seq, session, client, ev, target, path, before, after, before_hash, after_hash, reason, rule, undoes, key, kind, text, queries, results, status, cwd, project }`. `EventKind`: `Wrote, Merged, Superseded, Archived, Restored, Undid, Quarantined, Promoted, Rejected, Read, Searched, Cited, Helped, Hurt, Signal, External, Covered, Session, Note, Tool, Unknown`. Tipo futuro chega como `Unknown`. `EventKind::is_mutation()` diz o que pode ser desfeito.

## Visões

```rust
sessions() -> Result<Vec<SessionInfo>>
graph() -> Result<GraphData>
health() -> Result<Health>
agenda(&AgendaRequest { limit, offset, kinds }) -> Result<Agenda>
portrait() -> Result<Portrait>
learning_curve() -> Result<LearningCurve>
briefing(cwd, client) -> Result<Briefing>
sleep(budget: Duration) -> Result<SleepReport>              // chamar ao abrir o app; report.session é desfazível
lint() -> Result<Vec<LintIssue>>
export(path) / import_markdown(path)
```

`GraphData { nodes: [GraphNode { id, kind, title, weight, type?, authority?, state?, born }], edges: [GraphEdge { from, to, kind, weight }] }`. `kind` da aresta: `wikilink`, `source`, `supersedes`, `tag`. Peso de nó vem de `helped` e `cited`, nunca de exposição; nota usa leituras. Aresta `tag` pesa `1/√n`, e tag em mais de 40 nós não gera aresta. Saída determinística. Memória em quarentena aparece com título `(quarantined)`. `born` é ISO 8601.

`AgendaItem { kind, id, title, detail, ids, quote, flags }`: item que o portão barraria sai com `title`, `detail` ou `quote` vazios e o motivo em `flags`, com os mesmos códigos.

`LearningCurve { points: [LearningPoint { session, started, corrections, repeat_corrections }], total_repeat, repeat_per_100_sessions }`.

## Erros

`Error::code()` é estável: `brain_not_found`, `already_exists`, `invalid_path`, `not_found`, `busy`, `io`, `config`, `client_not_allowed`, `invalid`, `rename_captures_links`, `undo_refused`, `export_refused`. Mensagem sem caminho absoluto. `Busy`: trava ocupada por mais de 20 s; repetir é seguro.
