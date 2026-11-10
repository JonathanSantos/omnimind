# Servidor MCP, hooks e `connect`

Binário `omnimind`, package `omnimind-cli`. Laço stdio síncrono, JSON-RPC por linha, sem tokio nem rmcp. Stdout só leva protocolo. Sai no EOF do stdin.

```sh
omnimind mcp --brain PATH [--client NOME] [--session ID] [--cwd PATH]
```

Um processo abre um único brain fixo. Nenhuma tool recebe caminho de brain.

## Protocolo

Fala as duas gerações no mesmo processo. Verificado contra as páginas da revisão 2026-07-28 (`server/discover`, `basic/versioning`, `transports/stdio`, `server/tools`).

| Método | Comportamento |
|---|---|
| `initialize` | Legado. Devolve a versão pedida se conhecida, senão `2025-11-25`. Versões: `2025-11-25`, `2025-06-18`, `2025-03-26`, `2024-11-05` |
| `server/discover` | `supportedVersions`, `capabilities`, `instructions`, `ttlMs`, `cacheScope`, `_meta[io.modelcontextprotocol/serverInfo]` |
| `tools/list` | Quatro tools, ordem fixa, com `ttlMs` e `cacheScope: private` |
| `tools/call` | Serve sem `initialize`. Cliente vem de `--client`, `clientInfo` ou `_meta[io.modelcontextprotocol/clientInfo]` |
| `ping` | Mantido para clientes legados |
| `resources/list`, `prompts/list` | Vazios |

Todo resultado traz `resultType: "complete"` e `serverInfo`. Versão desconhecida em `_meta`: `-32022` com `data.supported` e `data.requested`.

| Entrada | Resposta |
|---|---|
| JSON inválido, UTF-8 inválido | `-32700`, `id: null`; segue vivo |
| Lote, não objeto, sem `jsonrpc` 2.0, sem `method`, `id` inválido, linha acima de 16 MiB | `-32600` |
| Método desconhecido | `-32601` |
| Tool desconhecida, `arguments` que não é objeto | `-32602` |
| Validação e erro de domínio | `isError: true`, com exemplo de entrada válida |
| Notificação, resposta solta | Nada |

Cancelamento: o laço é sequencial, então `notifications/cancelled` é aceita e ignorada; a chamada em curso termina.

**Cliente.** Depois de NFKC só passa `[a-z0-9._-]`, até 64 caracteres. `human`, `person`, `pessoa`, `user`, `system` e `omnimind` são recusados em qualquer forma, inclusive `ｈｕｍａｎ`. `allowed_clients` compara exato após NFKC. Lista vazia libera qualquer nome válido. Recusa em `initialize`: `-32001`; em `tools/call`: `isError`. O cliente se autodeclara: isso separa brains, não autentica.

## Tools

Esquema plano. Listas aceitam array, string com vírgulas, ou JSON de array em string. Números e booleanos aceitam string. `arguments` aceita string JSON. A saída vem igual em `content[0].text` e em `structuredContent`.

Toda resposta traz `notice` e `coverage`, salvo erro de protocolo. Orçamento padrão: 7.000 bytes, ou 2.000 tokens pela estimativa de 2 tokens a cada 7 bytes. Busca encolhe trechos de 240 até 40 caracteres, depois corta resultados e declara `truncated` e `next_offset`.

### `memory_search` (`readOnlyHint`, `anthropic/alwaysLoad`)

`query` (1 a 3), `limit` (5, máx. 20), `offset`, `type`, `tag`, `project`, `include_old`, `response_format` (`concise`, `detailed`).

`{ status, total, results: [{ id, kind, title, snippet, date, path?, project?, state?, authority?, origin?, warning?, flags? }], next_offset?, truncated?, corrected_query?, reason?, hint?, coverage, notice }`

`status`: `ok`, `weak`, `no_match`, `incomplete`. `kind`: `note` ou tipo da memória. Superseded, vencidas e arquivadas só com `include_old`. Quarentena nunca. Espera até 1,5 s pela indexação; depois responde `incomplete`.

### `memory_read` (`readOnlyHint`)

`id` (memória, `mem:<id>`, `[mem:<id>]`, `note:<caminho>` ou caminho, inclusive parcial), `offset`, `max_chars` (4.000), `include_old`, `distill`.

`{ status, id, kind, title, path, text, total_chars, next_offset?, hash, project?, state?, authority?, origin?, date?, superseded_by?, neighbors?, flags?, covered?, coverage, notice }`

`status`: `ok`, `not_found`, `hidden`. `hidden` não traz texto. Memória apagada responde `not_found` já na carência de 300 ms. `distill: true` conta cobertura quando a nota é lida até o fim, em páginas contíguas, no mesmo hash. Texto, `id` e `path` saem sem invisíveis, e segredos redigidos; `flags` diz o que havia: `invisible_text`, `invisible_name`, `instruction_like`, `secret_redacted`, `unverified_authority`. O `id` limpo de uma nota continua abrindo o arquivo.

### `memory_write` (`idempotentHint`)

`action` (`write`, `archive`, `restore`, `helped`, `hurt`), `key`, `id`, `base_hash`, `type`, `title`, `summary`, `body`, `aliases`, `tags`, `origin_kind`, `quote`, `note`, `supersedes`, `expires`, `reason`.

`{ outcome, id?, path?, into?, rule, reason?, hint?, hash?, event?, warnings?, neighbors?, tags?, tag_vocabulary?, current_text?, coverage, notice }`

`outcome`: `written`, `archived`, `restored`, `merged`, `superseded`, `quarantined`, `rejected`, `unchanged`, `recorded`. `archive` e `restore` exigem `base_hash`, e `id` aceita a forma citada `mem:<id>`. `rejected` não é `isError`: o modelo lê `hint` e corrige. `reason` usa os códigos de `core.md`, como `note_not_read`, `preference_needs_quote`, `must_supersede`, `base_hash_mismatch`, `secret`, `key_reused`, `session_growth_limit`, `memory_held`, `cannot_edit_person_memory`, `cannot_supersede_higher_authority`. `current_text` nunca traz memória retida nem segredo: nesses casos só `hash`. `warnings` diz quando a citação não sustenta o texto e a memória ficou como `agent`.

### `memory_agenda` (`readOnlyHint`)

`limit` (10), `offset`, `kinds`.

`{ items: [{ kind, id, title, detail, ids?, quote?, flags? }], remaining, total, done, next_offset, notes_covered, notes_total, coverage, notice }`

Texto que o portão barraria sai vazio, com o motivo em `flags`. Consulta hostil não vira `gap`.

`kind`: `signal`, `note_new`, `note_changed`, `stale`, `contradiction`, `near_duplicate`, `gap`, `idle`.

## Hooks

`omnimind hook session-start|prompt|post-tool|pre-compact|stop|session-end [--brain P] [--client C]`.

Lêem JSON do stdin, com limite de 8 MiB e espera de 250 ms, em `snake_case` ou `camelCase`. Só anexam ao diário e nunca abrem o índice. Saem com 0 em qualquer caso, inclusive pânico, brain ausente e JSON ruim.

| Hook | Efeito | Saída |
|---|---|---|
| `session-start` | Evento `session`, briefing leve | `additionalContext` |
| `prompt` | Classifica por regras PT/EN, sem colagens. Grava `signal` literal, sem segredo. Fecha `cited` pendentes como `hurt` ou `helped` | `additionalContext` factual, só se houver sinal |
| `post-tool` | Evento `tool`: nome e caminho | Nada |
| `pre-compact` | Evento `session`, `compact` | Nada; nunca código 2 |
| `stop` | `cited` para ULIDs em `last_assistant_message` | Nada |
| `session-end` | Evento `session`, `end` | Nada |

No Claude Code a saída é `hookSpecificOutput`; com `--client copilot`, `additionalContext` na raiz. O Copilot não injeta contexto em `userPromptSubmitted`, mas o sinal é gravado.

## `omnimind connect`

```sh
omnimind connect claude-code|copilot [--project PATH] [--brain PATH] [--print] [--global]
omnimind disconnect claude-code|copilot [--project PATH] [--print] [--global]
```

| Alvo | Arquivos |
|---|---|
| `claude-code` | `.mcp.json`, `.claude/settings.json`, `.claude/skills/{omnimind,sonhar}/SKILL.md` |
| `copilot` | `.vscode/mcp.json`, `.github/mcp.json`, `.github/hooks/omnimind.json`, `.github/skills/{omnimind,sonhar}/SKILL.md` |
| `--global` | `~/.claude.json`, `~/.claude/`, ou `~/.copilot/` |

Funde com o que existe e preserva o resto. Mostra diff. Copia o arquivo para `<nome>.omnimind-backup-<hora>` antes de trocar. Rodar de novo não muda nada. `--print` não escreve. JSON inválido fica intocado e o comando falha. Symlink no caminho é recusado sem `--global`, para `.claude` apontado para a home não contornar a regra. O comando grava o caminho absoluto do binário e do brain.

`/sonhar` chama `memory_agenda` em laço até `done` ou até o limite pedido.

## Linha de comando

Saída para pessoa em português; `--json` e identificadores continuam em inglês. `init` diz o próximo passo. `connect` e `disconnect` com JSON comentado explicam o motivo e deixam o arquivo como está. `disconnect` tira só a entrada `omnimind`, os hooks que chamam o Omnimind e as duas skills; arquivo e pasta que ficam vazios saem. Cada arquivo tem no máximo uma cópia de segurança, a de antes do Omnimind. Cada cofre lista seus projetos em `.omnimind/connections.json`, sem `--global`.

## Não verificado

Os hooks rodaram no Claude Code real em modo `claude -p`; veja `docs/qa/ai-sessions.md`. O modo interativo, o Copilot CLI e o VS Code não foram testados. O formato de hooks do Copilot e os arquivos `.github/` vêm da documentação lida em `docs/research/technical.md`.
