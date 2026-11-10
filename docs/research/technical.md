# Pesquisa técnica: MCP, hooks, busca lexical, avaliação, autoaperfeiçoamento e coerência

**Data da pesquisa:** 2026-10-31. Todas as URLs abaixo foram acessadas nessa data.
**Escopo do produto:** local-first, 100% Rust, notas Markdown + memórias automáticas via MCP para Claude Code e GitHub Copilot. Proibido: embeddings, banco vetorial, modelo local, processo pesado sempre ligado, gastar tokens por conta própria.

## Como ler este documento

- `[Mx]`, `[Hx]`, `[Bx]`, `[Ax]`, `[Sx]`, `[Cx]` apontam para a tabela de fontes no fim de cada tema, com URL e data.
- **Pub.** é a data de publicação quando a página a mostra; `n/d` quando não consegui confirmar.
- **Inferência** marca conclusão minha, não dita pela fonte.
- **Não verificado** marca o que veio de fonte secundária, de memória ou de busca sem leitura da fonte primária.
- A cota de buscas da sessão (200 por turno, compartilhada) acabou antes do fim. As lacunas estão listadas na seção "Lacunas".

---

## 1. MCP em 2026

### 1.1 Estado da especificação

- **A versão atual é `2026-07-28`.** O identificador muda só quando há quebra de compatibilidade [M1]. O RC saiu em maio e a versão final em 28/07/2026 [M2].
- **O protocolo virou stateless.** Saíram `initialize`/`notifications/initialized`, `Mcp-Session-Id` e `ping`. Cada requisição leva versão e capabilities em `_meta` [M3].
- **`server/discover` é obrigatório no servidor.** O cliente pode chamá-lo antes de tudo, ou usá-lo como sonda em stdio. Um cliente que fala as duas gerações sonda com ele e cai para `initialize` se a resposta não for reconhecida [M3][M4].
- **Multi Round-Trip Requests (MRTR) substitui requisições do servidor ao cliente.** O servidor devolve `resultType: "input_required"` com `inputRequests`. O cliente repete a chamada com `inputResponses`. Em stdio, o servidor **não pode** escrever requisições no stdout [M3][M4].
- **Todo resultado tem `resultType`** (`"complete"` ou `"input_required"`) [M3].
- **`ttlMs` e `cacheScope` são obrigatórios** em `tools/list`, `prompts/list`, `resources/list`, `resources/read` e `resources/templates/list` [M3]. O Claude Code 2.1.302 precisou tolerar servidores que os omitem, o que mostra o erro real [M8].
- **`tools/list` deve ter ordem determinística** e não pode variar por conexão. Isso melhora o cache de prompt [M3][M5].
- **Deprecados na 2026-07-28: Roots, Sampling, Logging.** Saem no mínimo na primeira revisão a partir de 2027-07-28. Migração: passar diretórios por parâmetro ou config, integrar direto com APIs de LLM, logar em `stderr` [M6].
- **Tasks saiu do core** e virou a extensão `io.modelcontextprotocol/tasks`, com polling por `tasks/get` [M3].
- **`subscriptions/listen` substitui `resources/subscribe`** e o GET de notificações [M3].

**Consequência direta para o produto.** A deprecação de Sampling coincide com a restrição "sem gastar tokens por conta própria". O servidor nunca chama LLM. Quem raciocina é o agente, dentro do turno que já está pagando.

### 1.2 Regras de tools que importam

Fonte: spec de tools [M5].

| Item | Regra |
|---|---|
| Nome | 1 a 128 caracteres; só `A-Za-z0-9_-.` |
| Sem parâmetros | `{"type":"object","additionalProperties":false}` |
| `structuredContent` | Qualquer JSON. Com `outputSchema`, o servidor **deve** obedecer ao schema. **Deve** (SHOULD) também devolver o JSON serializado em bloco de texto, por compatibilidade |
| Erro de protocolo | JSON-RPC, para tool inexistente ou requisição malformada. Pouco recuperável |
| Erro de execução | `isError: true` no resultado, com texto que permita ao modelo corrigir. Validação de entrada vai aqui |
| Estado | Sem sessão. Estado entre chamadas é handle explícito, com expiração na descrição |
| Anotações | Não confiáveis por padrão no cliente |
| stdio | Sair ao receber EOF no stdin. Nada além de JSON-RPC no stdout. Log em stderr [M4] |

Nota prática: o exemplo `list_users` da spec devolve, ao lado do `structuredContent`, um texto resumido e não o JSON cru [M5]. **Inferência:** renderizar o bloco de texto de forma compacta é aceitável e evita pagar duas vezes pelos tokens, se o cliente enviar ambos ao modelo. Não verifiquei o que cada cliente envia.

### 1.3 O que os clientes suportam de fato

| Capacidade | Claude Code | Copilot CLI | VS Code |
|---|---|---|---|
| Protocolo 2026-07-28 em stdio | **Sim, padrão desde 2.1.292** (2026-10-06). Opt-out `MCP_PROTOCOL_NEGOTIATION=legacy`. Servidores que ignoram a sonda são lembrados por 7 dias e conectados à moda antiga [M8] | Não verificado | Não verificado |
| Tools | Sim | Sim | Sim |
| Resources | Sim, `@server:protocol://path` [M7] | **Não.** Issues #1518 e #1803 seguem abertas; a última teve comentário em 05/10/2026 [M10][M11] | Sim [M13] |
| Prompts | Sim, `/servidor:prompt (MCP)` [M7] | **Não** [M10] | Sim, `/<servidor>.<prompt>` [M13][M14] |
| Elicitation | Sim, formulário e URL [M7] | Não verificado | Listado como suportado [M13] |
| Sampling | Não aparece na doc lida | Não verificado | Sim, com autorização [M13] |
| Roots | Sim, com `--add-dir` [M7] | Não verificado | Sim [M13] |
| Tasks | Beta público, v2.1.304+ [M7] | Não verificado | Não mencionado [M13] |
| Tool search | **Padrão ligado** [M7] | `deferTools: "auto"` (padrão) ou `"never"` [M12] | Não verificado |
| Server instructions | Sim, até 4.096 caracteres [M7] | Não verificado | Listado [M13] |

**Conclusão de arquitetura.** Tools é o único denominador comum confiável. Resources e prompts são extras só para Claude Code e VS Code. A issue #1803 relata ainda que servidores que devolvem URIs de recurso falham no Copilot CLI, porque ele tenta `web_fetch` [M11]. **Decisão derivada:** nunca devolver só `resource_link`; devolver trecho inline mais caminho no disco.

### 1.4 Formatos de configuração

| Cliente | Arquivo | Chave | Observações |
|---|---|---|---|
| Claude Code | `.mcp.json` (projeto), `~/.claude.json` (user/local) | `mcpServers` | stdio sem `type`. Expande `${VAR}` e `${VAR:-default}`. Projeto exige aprovação [M7] |
| Copilot CLI | `~/.copilot/mcp-config.json`, `.mcp.json`, `.github/mcp.json` | `mcpServers` | `type: "local"` ou `"http"`, campo `tools`. **Não lê `.vscode/mcp.json`** [M9] |
| VS Code | `.vscode/mcp.json` ou perfil do usuário | **`servers`** | Também `inputs` e `sandbox` (macOS e Linux) [M15] |

```json
{
  "mcpServers": {
    "memoria": {
      "command": "memoria",
      "args": ["mcp"],
      "env": { "MEMORIA_HOME": "${HOME}/.memoria" }
    }
  }
}
```

**Inferência:** `.mcp.json` sem `type` é lido por Claude Code. No Copilot CLI, a doc exemplifica `type: "local"`. O instalador deve gerar os três arquivos em vez de assumir um formato único, e testar em cada cliente.

### 1.5 Limites práticos do Claude Code para autores de servidor

Fonte: [M7], salvo indicação.

| Limite | Valor | Implicação |
|---|---|---|
| Server instructions | 4.096 caracteres | Escrever como "quando me procurar", com palavras-chave no início |
| Descrição de tool carregada no início | 4.096 caracteres | Idem |
| Descrição de tool diferida | 16.384 caracteres, mas a busca só lê a parte dentro do limite menor | Termos pesquisáveis no começo |
| Aviso de saída | 10.000 tokens | Meta de projeto: ficar bem abaixo |
| Limite padrão de saída | 25.000 tokens (`MAX_MCP_OUTPUT_TOKENS`) | Truncar com dica de refinamento |
| Texto acima de 50.000 caracteres | Vai para arquivo no disco, salvo `_meta["anthropic/maxResultSizeChars"]` | Evitar; o modelo recebe só uma referência |
| Conexão com `alwaysLoad` | Espera até 5 s no início | Servidor precisa subir rápido |

`_meta["anthropic/alwaysLoad"]: true` carrega uma tool antes de qualquer busca. A doc pede moderação, pois cada tool upfront tem custo em todo turno [M7]. A Anthropic recomenda manter de 3 a 5 tools mais usadas sempre carregadas [M19].

**Tool annotations.** `readOnlyHint` faz o Claude Code executar chamadas em paralelo com outras read-only. `idempotentHint` permite repetir chamada interrompida [M7]. No VS Code, tools com `readOnlyHint` não pedem confirmação [M13]. Sem anotação, a presunção é não read-only, potencialmente destrutiva, não idempotente e open-world [M16]. Anotações não impedem prompt injection nem vazamento [M16].

### 1.6 Descrições que agentes realmente chamam

- **Poucas tools, com fluxo completo.** `search_x` em vez de `list_x`; consolidar chamadas encadeadas numa só [M17].
- **`response_format` com `CONCISE` e `DETAILED`.** No exemplo do Slack, 72 contra 206 tokens [M17].
- **Evitar identificadores opacos.** Preferir nomes legíveis a `uuid` [M17].
- **Erros instrutivos**, com exemplo de entrada válida, e não tracebacks [M17].
- **Descrever como para um colega novo**, explicitando formato de consulta e termos de nicho [M17].
- **Nomes específicos melhoram a descoberta:** `search_customer_orders` melhor que `query_db_orders` [M19].
- **Exemplos de uso** elevaram a acurácia em parâmetros complexos de 72% para 90% em testes internos da Anthropic. A recomendação é de 1 a 5 exemplos, focados em casos ambíguos [M19].
- **Qualidade da descrição é mensurável.** Estudo com 10.831 servidores MCP mediu efeito significativo de "smells" na seleção de tools: funcionalidade +11,6% e acurácia +8,8% (p<0,001) [M20]. Outro estudo avaliou 856 tools de 103 servidores [M21].
- **Tool search:** 58 tools em 5 servidores custam cerca de 55K tokens; com busca, a sobrecarga inicial cai 85%, e a acurácia subiu de 79,5% para 88,1% no Opus 4.5 [M19].
- **Progressive disclosure e filtragem antes do contexto:** o caso de 150.000 para 2.000 tokens vem de executar código com as tools [M18]. Para um servidor de memória, a tradução é devolver pouco e permitir expandir.
- **Interface muda comportamento.** Em 11.700 trajetórias, interfaces estruturadas melhoraram a consistência em até 4,7× contra bash puro, e ferramentas leves de "registrar raciocínio" tiveram efeito limitado [M22].
- **O que entra no contexto importa mais que a estrutura interna.** "Smallest possible set of high-signal tokens" [M23].

### 1.7 Erros comuns a evitar

1. Omitir `resultType`, `ttlMs` ou `cacheScope` [M3][M8].
2. Escrever qualquer coisa no stdout além de JSON-RPC [M4].
3. Não encerrar no EOF do stdin [M4].
4. Devolver erro de validação como erro de protocolo, o que impede autocorreção [M5].
5. Depender de resources ou prompts para funcionar no Copilot CLI [M10][M11].
6. Depender de Sampling ou Roots em código novo [M6].
7. Descrição longa com a palavra-chave no fim, invisível à busca [M7].
8. Devolver mais de 50.000 caracteres e cair em arquivo [M7].
9. Criar tools que mudam conforme a conexão ou efeito colateral [M5].
10. Anotação incorreta: `readOnlyHint` em tool com efeito colateral [M16].

### 1.8 SDK em Rust

O SDK oficial `rmcp` declara implementar a 2026-07-28 e manter compatibilidade com 2025-11-25 e anteriores, com stdio pela feature `transport-io` [M24]. **Inferência:** cobre o caminho do Claude Code novo e dos clientes legados. Validar em teste de integração com os três clientes antes de confiar nisso.

### Fontes do tema 1

| ID | URL | Pub. |
|---|---|---|
| M1 | https://modelcontextprotocol.io/specification/versioning | n/d |
| M2 | https://blog.modelcontextprotocol.io/posts/2026-07-28/ | 2026-07-28 |
| M3 | https://modelcontextprotocol.io/specification/2026-07-28/changelog | 2026-07-28 |
| M4 | https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio | 2026-07-28 |
| M5 | https://modelcontextprotocol.io/specification/2026-07-28/server/tools | 2026-07-28 |
| M6 | https://modelcontextprotocol.io/specification/2026-07-28/deprecated | 2026-07-28 |
| M7 | https://code.claude.com/docs/en/mcp | n/d (doc viva) |
| M8 | https://code.claude.com/docs/en/changelog | até 2.1.316, 2026-10-31 |
| M9 | https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers | n/d |
| M10 | https://github.com/github/copilot-cli/issues/1518 | 2026-02-18 |
| M11 | https://github.com/github/copilot-cli/issues/1803 | 2026-03-03 |
| M12 | https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference | n/d |
| M13 | https://code.visualstudio.com/api/extension-guides/ai/mcp | n/d |
| M14 | https://code.visualstudio.com/docs/copilot/customization/mcp-servers | n/d |
| M15 | https://code.visualstudio.com/docs/copilot/reference/mcp-configuration | n/d |
| M16 | https://blog.modelcontextprotocol.io/posts/2026-03-16-tool-annotations/ | 2026-03-16 |
| M17 | https://www.anthropic.com/engineering/writing-tools-for-agents | n/d |
| M18 | https://www.anthropic.com/engineering/code-execution-with-mcp | n/d |
| M19 | https://www.anthropic.com/engineering/advanced-tool-use | n/d |
| M20 | https://arxiv.org/html/2602.18914v1 | 2026-02 |
| M21 | https://arxiv.org/html/2602.14878v1 | 2026-02 |
| M22 | https://arxiv.org/abs/2608.11386 | 2026-08 |
| M23 | https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents | n/d |
| M24 | https://github.com/modelcontextprotocol/rust-sdk | n/d |

---

## 2. Hooks, skills e instruções

### 2.1 Claude Code: mecânica geral

Fonte: [H1], salvo indicação.

**Entrada comum por stdin (JSON):** `session_id`, `prompt_id` (ausente até a primeira entrada), `transcript_path`, `cwd`, `scratchpad_dir` (v2.1.257+), `permission_mode`, `effort`, `hook_event_name`, e `agent_id`/`agent_type` em subagentes.

**Saída:** o stdout precisa ser só um objeto JSON. Campos universais: `continue`, `stopReason`, `systemMessage`, `terminalSequence`. `suppressOutput` é aceito, mas sem efeito.

**Códigos de saída:** `0` aplica o JSON. `2` bloqueia nos eventos que aceitam bloqueio, com o stderr como motivo. Outros são erro não bloqueante. Timeout não bloqueia, salvo `onFailure: "block"`.

**Tipos:** `command`, `http`, `mcp_tool`, `prompt`, `agent`. Para o produto, **só `command`**: `prompt` e `agent` gastam tokens.

**Limite de `additionalContext`:** 10.000 caracteres. Acima disso, o conteúdo vai para arquivo e o Claude recebe caminho e prévia de 2.000 caracteres, sem instrução de ler o arquivo.

**Aviso crítico para memória:** a doc pede afirmações factuais em `additionalContext`, porque frases que parecem comandos de sistema podem acionar as defesas contra prompt injection. Memória injetada deve ser escrita como fato com proveniência, nunca como ordem.

```json
{
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "Memória do projeto (2026-09-12, origem: usuário): o deploy usa o alvo staging-eu."
  }
}
```

### 2.2 Eventos relevantes

| Evento | Entrada específica | Saída útil | Particularidades |
|---|---|---|---|
| `SessionStart` | `source` (`startup`, `resume`, `clear`, `compact`, `fork`), `model` opcional, `session_title` | `additionalContext`, `watchPaths`, `sessionTitle`, `reloadSkills`, `initialUserMessage` | Só `command` e `mcp_tool`. stdout simples vira contexto. Roda de novo após `compact` |
| `UserPromptSubmit` | `prompt`; colagens entre `<pasted_content>` | `additionalContext`, `decision`, `sessionTitle` | Sem matcher. Timeout padrão de 30 s. Não reescreve o prompt. Dispara também em turnos iniciados pelo próprio Claude Code |
| `PreToolUse` | `tool_name`, `tool_input`, `tool_use_id`, `mcp_server` | `permissionDecision`, `updatedInput`, `additionalContext` | Precedência `deny` > `defer` > `ask` > `allow` |
| `PostToolUse` | `tool_response`, `duration_ms` | `additionalContext`, `updatedToolOutput`, `decision` | A tool já rodou |
| `PostToolBatch` | `tool_calls[]` com a resposta serializada | `additionalContext` | Uma vez por lote, sem matcher |
| `Stop` | `stop_hook_active`, `last_assistant_message`, `background_tasks`, `session_crons` | `decision`, `additionalContext` | Teto de 8 continuações consecutivas |
| `PreCompact` | `trigger`, `custom_instructions` | `decision: "block"` | `systemMessage` descartado |
| `PostCompact` | `trigger`, `compact_summary` | Nenhuma decisão | Resumo da compactação de graça |
| `SessionEnd` | `reason` | Nenhuma | **Orçamento compartilhado de 1,5 s**, até 60 s se configurado |
| `FileChanged` | `file_path`, `event` | `watchPaths` | Observador do Claude Code, ativo só durante a sessão |

**Transcrição.** `transcript_path` aponta para um JSONL em `~/.claude/projects/...`. Pode estar atrasado em relação à memória, e o arquivo não garante a mensagem final no `Stop`. A doc manda usar `last_assistant_message`. Em subagentes, usar `agent_transcript_path`. Não encontrei um schema público das linhas do JSONL. **Inferência:** tratar o formato como não contratual, com parser tolerante.

**Hooks assíncronos.** `async: true` existe só em `command` e não bloqueia. `additionalContext` chega no turno seguinte. Em `-p`, o hook é morto ao final.

### 2.3 Plugins

Fonte: [H2].

```text
plugin/
├── .claude-plugin/plugin.json
├── skills/<nome>/SKILL.md
├── hooks/hooks.json        (exige a chave "hooks" no topo)
├── .mcp.json
├── agents/
└── bin/                    (entra no PATH da tool Bash)
```

- `${CLAUDE_PLUGIN_ROOT}` muda a cada atualização. Não guardar estado ali.
- `${CLAUDE_PLUGIN_DATA}` persiste entre atualizações, mas **é apagado ao desinstalar**, salvo `--keep-data`. Nunca guardar notas do usuário ali.
- Um `CLAUDE.md` na raiz do plugin não é carregado. Instruções vão em skills.
- Nomes com `claude-`, `anthropic-` ou `cc-plugin-` no início são reservados.
- Hook em forma exec, com `args`, evita problemas de aspas.
- Plugins com `bin/` não são instalados em claude.ai e Cowork.

### 2.4 Skills e CLAUDE.md

- **SKILL.md:** frontmatter YAML com `description` e opcionais `when_to_use`, `disable-model-invocation`, `user-invocable`, `allowed-tools`, `context`, `hooks`, `paths`. `description` + `when_to_use` truncam em 1.536 caracteres. Corpo recomendado abaixo de 500 linhas. Após compactar, cada skill volta com seus primeiros 5.000 tokens [H3].
- **Custo:** cada skill habilitada acrescenta nome e descrição à listagem em todo turno [H3].
- **Portabilidade:** fora do Claude Code, só `name`, `description`, `license`, `compatibility`, `metadata` e `allowed-tools` são aceitos [H3]. O Copilot CLI lê `.github/skills/`, `.agents/skills/` e `.claude/skills/` [M12].
- **CLAUDE.md:** meta de menos de 200 linhas por arquivo. Imports com `@caminho`, até 4 níveis. O conteúdo é contexto, não regra imposta [H4].
- **Auto memory do Claude Code já existe:** primeiras 200 linhas ou 25 KB carregadas em toda sessão [H4]. O produto compete com isso e deve coexistir sem duplicar contexto.
- **AGENTS.md:** lido pelo Claude Code quando não há `CLAUDE.md`, a partir de v2.1.277 [H4].

### 2.5 Copilot: equivalentes

**Instruções** [M12][H6]: `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `.github/copilot-instructions.md`, `.github/instructions/**/*.instructions.md`, `.claude/rules/**/*.md`, `~/.copilot/copilot-instructions.md`. Todas se somam. Escopo por `paths` ou `applyTo`.

**Hooks** [H5]: `.github/hooks/*.json`, `~/.copilot/hooks/*.json` ou blocos `hooks` em `settings.json`. Entrada em camelCase.

```json
{
  "version": 1,
  "hooks": {
    "userPromptSubmitted": [
      { "type": "command", "bash": "memoria hook prompt", "timeoutSec": 5 }
    ]
  }
}
```

| Evento | Entrada | Saída | Injeta contexto? |
|---|---|---|---|
| `sessionStart` | `source`, `initialPrompt` | `additionalContext` | **Sim** |
| `userPromptSubmitted` | `prompt` | Ignorada em hooks de arquivo | **Não.** Mas o hook recebe o texto, então captura funciona |
| `preToolUse` | `toolName`, `toolArgs` | `permissionDecision`, `modifiedArgs` | Não |
| `postToolUse` | `toolName`, `toolArgs`, `toolResult` | `additionalContext`, `modifiedResult` | **Sim**, limite somado de 10 KB |
| `agentStop` | `transcriptPath`, `stopReason`, `stop_hook_active` | `decision` | Só forçando novo turno |
| `sessionEnd` | `reason` | Nenhuma | Não |

Outras diferenças: timeout padrão de 30 s, sempre fail-open. Exit 2 é só aviso, salvo em `preToolUse`. O `prompt` hook existe só em `sessionStart` [H5].

**VS Code** [H7]: lê `.github/hooks/*.json`, `~/.copilot/hooks/`, e também `.claude/settings.json`. Com nomes PascalCase (`PreToolUse`), a entrada vem em snake_case, mas a doc avisa que compartilhar o local do arquivo não garante compatibilidade de runtime. Hooks de várias fontes rodam juntos. **Risco:** o mesmo hook pode rodar duas vezes se instalado em dois formatos. O produto precisa de idempotência por `session_id` + hash do evento.

**Copilot Memory** concorre diretamente [S2][S3]:

- Fatos de repositório citam código e são validados contra a branch atual antes do uso.
- Expiração padrão de **90 dias**, reiniciada por upvote. **Só recuperar não reinicia.**
- Só o agente raiz grava (`store_memory`, `vote_memory`). Subagentes só leem.
- Segredos são rejeitados.

Custom agents ficam em `.github/agents/<nome>.agent.md`. O caminho em `~/.copilot/agents/` veio de guia de terceiro e não foi confirmado na doc oficial.

### 2.6 Capturar correções do usuário sem LLM

O padrão já foi provado: `claude-reflect` usa regex em `UserPromptSubmit`, grava em fila e sai sempre com 0 [H8]. Dois alertas da mesma fonte:

- Regex sozinho tem falsos positivos. O projeto usa LLM depois para filtrar, o que aqui é proibido.
- Uma variante com chamada de modelo por prompt mostra o que **não** fazer.

**Desenho proposto (inferência, baseado em [H1], [H5], [H8]):**

1. `Stop`/`agentStop` grava em sidecar a última resposta do agente e as tools usadas. Em Claude Code, usar `last_assistant_message`. Isso dispensa parsear o transcript.
2. `UserPromptSubmit`/`userPromptSubmitted` roda regex PT/EN e pontua, sem decidir. Remover `<pasted_content>` antes. Ignorar turnos iniciados pelo sistema.
3. Sinais fortes: início com "não,", "errado", "na verdade", "pare de", "nunca", "sempre", "use X em vez de Y", "no,", "actually", "don't", "instead", "stop".
4. Sinais independentes do texto: `PermissionDenied`, tool negada, reversão de edição feita pelo agente, repetição da mesma correção em sessões diferentes (fingerprint SimHash).
5. Saída: **candidato**, nunca memória ativa. Só promove com repetição, confirmação humana ou confirmação do agente na própria sessão.

A repetição é o melhor filtro sem LLM: uma correção vista em duas sessões distintas tem muito menos chance de ser ruído. Isso é hipótese a validar, não achado publicado.

### Fontes do tema 2

| ID | URL | Pub. |
|---|---|---|
| H1 | https://code.claude.com/docs/en/hooks | n/d (doc viva) |
| H2 | https://code.claude.com/docs/en/plugins-reference | n/d |
| H3 | https://code.claude.com/docs/en/skills | n/d |
| H4 | https://code.claude.com/docs/en/memory | n/d |
| H5 | https://docs.github.com/en/copilot/reference/hooks-configuration | n/d |
| H6 | https://github.blog/changelog/2026-06-23-copilot-coding-agent-now-supports-agents-md-custom-instructions/ | 2026-06-23 |
| H7 | https://code.visualstudio.com/docs/copilot/customization/hooks | n/d |
| H8 | https://raw.githubusercontent.com/BayramAnnakov/claude-reflect/main/README.md | n/d (lido via resultado de busca) |

---

## 3. Busca lexical sem embeddings

### 3.1 O que os dados dizem sobre BM25

| Medida | Resultado | Fonte |
|---|---|---|
| BM25 em LoCoMo (1.978 perguntas) | Hit@1 0,640; Recall@5 0,833; NDCG@5 0,746 | [B1] |
| BM25 em LongMemEval-S (subconjunto de 150 perguntas) | R@1 0,589; R@5 0,948; NDCG@5 0,916 | [B1] |
| Fusão densa sobre BM25 em LongMemEval-S | Ganho pequeno, não significativo: BM25 "satura" | [B1] |
| Fusão densa sobre BM25 em LoCoMo | Hit@1 0,752 com e5-large-v2, +11,2 pp | [B1] |
| Cross-encoder de busca web sobre top-10 | **Piorou** Hit@1 em 6,9 pp em LoCoMo | [B1] |
| MemoryBench | Nenhum de A-Mem, Mem0, MemoryOS supera de forma consistente as baselines RAG (BM25 e embeddings). Em LoCoMo eles superam BM25, mas isso não generaliza | [B2] |
| Letta, arquivo + `grep` + busca semântica | 74,0% em LoCoMo contra 68,5% declarados pela Mem0 | [B3] |

**Leituras obrigatórias junto com esses números:**

- O teste da Letta incluía `search_files` semântico, então **não** é um resultado de "só grep" [B3].
- A comparação 74% contra 68,5% está datada. A Mem0 declara 92,5% em LoCoMo depois de reescrever o pipeline, número próprio e não reproduzido aqui (secundário) [B4].
- Em LoCoMo, o maior ganho denso está em perguntas multi-hop e temporais; em adversariais, BM25 fica à frente [B1]. O ganho semântico é real, mas onde falta sinônimo e inferência.
- O número "~70% de R@5 para BM25 no paper do LongMemEval" é citação secundária, com escopo diferente. O paper original não confirma isso na página do resumo [B5]. Não confie nele.

**Conclusão honesta.** BM25 bem feito é um piso alto, não um teto. Sem embeddings, a perda esperada está em paráfrases, não em fatos literais. A compensação deve vir de aliases gravados na escrita e de reformulação pelo agente na leitura.

### 3.2 Agentic search contra RAG

- **Claude Code:** carrega `CLAUDE.md` e usa glob/grep sob demanda [M23]. O relato de Boris Cherny de que busca agêntica superou RAG com banco vetorial é citação secundária; não achei avaliação primária publicada [B6].
- **Cursor mediu o contrário em parte:** busca semântica aumentou a precisão em 12,5% offline (6,5% a 23,5%), mas o A/B mostrou só +0,3% de retenção de código no geral, +2,6% em bases com 1.000+ arquivos. Sem ela, os pedidos de acompanhamento insatisfeitos subiram 2,2%. A melhor combinação foi grep + semântica [B7].
- **Busca em linguagem natural** aumentou em mais de 11% o acesso a arquivos relevantes em um estudo de interfaces [M22]. Não sei se o mecanismo era semântico; o resumo não diz.

**Inferência.** Em bases de notas pequenas, o ganho semântico tende a ser menor que nos repositórios grandes do Cursor, mas é hipótese: o dado do Cursor é de código, não de notas. Medir no golden set próprio.

**Vantagem estrutural dos arquivos.** Memórias e notas em Markdown são pesquisáveis pelas tools nativas de qualquer agente, sem MCP. Isso dá um caminho de fallback gratuito e funciona no Copilot.

### 3.3 FTS5 contra tantivy

| Critério | SQLite FTS5 | tantivy 0.27 |
|---|---|---|
| Linguagem | **C** (via `rusqlite` bundled) | Rust puro |
| Stemmer português | Não tem. `porter` é inglês [B8] | Sim, 18 idiomas [B9] |
| Pesos por campo | `bm25(tbl, w1, w2, ...)`; `k1=1.2` e `b=0.75` fixos [B8] | `set_field_boost`, soma por campo [B10] |
| BM25F verdadeiro | Não confirmado. Não consegui ler `fts5_aux.c` | Não; boost não equivale a normalização por campo [B10] |
| Tipos | `unicode61 remove_diacritics 2` [B8] | Filtros de tokenização, reregistrados ao abrir o índice [B11] |
| Typos | Trigrama ou vocabulário via `fts5vocab` [B8] | `FuzzyTermQuery` com autômato de Levenshtein [B12] |
| Metadados, grafo, contadores | Mesma transação, mesmo arquivo | Exigem outro armazenamento |
| Multi-processo sem daemon | Vários processos, um escritor por vez, `busy_timeout` [C1] | `INDEX_WRITER_LOCK`: um único processo com `IndexWriter` [B13] |
| Leitura durante escrita | WAL permite [C1] | Permitida, com `reload` [B13] |
| Tamanho do índice | `detail=full` 743 MiB para 1.636 MiB de e-mail; `column` 340; `none` 134 [B8] | Não medido |

**Recomendação:** FTS5 como padrão, por causa de transação única, concorrência entre processos sem daemon e metadados no mesmo lugar. **Ressalva que precisa de decisão humana:** SQLite é C. Se "100% Rust" significa nenhum C no link, a alternativa é tantivy mais armazenamento Rust de metadados. Como o índice é derivado do journal (tema 6), a escolha é reversível atrás de uma trait. Medir antes de fechar.

### 3.4 Português

- **Stemming no FTS5:** criar colunas `*_stem` com texto já reduzido em Rust por `rust-stemmers` 1.2 (tem `Portuguese`) [B14] e aplicar a mesma função na consulta. Dispensa tokenizador custom em C.
- **Qualidade do Snowball PT:** relato antigo mostra que não une `avião` e `aviões` (`aviã`, `aviõ`) [B15]. É um caso de plural irregular comum em português.
- **RSLP:** supera o Porter português no paper original (Orengo e Huyck, 2001). Estudo posterior na coleção CLEF achou ganho significativo só com stemmer **mais leve** [B16][B17]. Não achei implementação Rust confiável, e a do NLTK se declara didática.
- **Proposta (inferência):** indexar dois campos, um literal sem diacríticos e um com stem leve. Aplicar antes do Snowball uma tabela curta de plurais irregulares (`-ões`, `-ães`, `-ãos`). Avaliar RSLP-S só se o golden set mostrar perda.
- **Inglês misturado:** notas técnicas em PT têm termos em inglês. Não stemizar tokens detectados como identificadores de código (`snake_case`, `camelCase`, caminhos, versões), e dividi-los em partes indexadas à parte.

### 3.5 Typos e n-gramas

Trigrama no nível de documento é caro: um relato aponta índice 3,4× maior e pior ranking, número secundário [B18]. Trigrama FTS5 não casa consultas com menos de 3 caracteres, e combinações `detail=` restringem consultas [B8].

**Melhor opção (inferência):** corrigir no **vocabulário**, não no documento. Ler termos e frequências de `fts5vocab` [B8], gerar candidatos por distância de edição, e só tentar isso quando a busca exata voltar vazia ou fraca. Mesma lógica vale para tantivy com `FuzzyTermQuery`.

### 3.6 Expansão, multi-query e HyDE sem LLM

- **Aliases na escrita.** LongMemEval já indicou expansão de chaves com fatos e expansão temporal de consultas como otimizações úteis, sem números no resumo [B5]. Aqui a expansão vem do próprio agente: `memory_write` aceita `aliases` e `keywords`, que entram em coluna de peso alto.
- **Multi-query.** `memory_search` aceita de 1 a 3 reformulações na mesma chamada, e o servidor funde por RRF. RRF é método padrão (Cormack et al., 2009); citado de memória, não lido nesta sessão.
- **HyDE sem LLM.** HyDE gera um documento hipotético e o codifica em vetor [B19]. Sem vetores, o análogo é o agente passar um rascunho curto de resposta, de onde o servidor extrai os termos mais raros por IDF. **É adaptação minha, sem validação publicada.** Medir antes de expor na descrição da tool.
- **Tokens.** Tudo isso sai do raciocínio que o agente já faz no turno. O servidor não gasta nada.

### 3.7 Grafo

PPR e spreading activation estão bem representados na literatura, mas **não achei sistema sem embeddings avaliado em LoCoMo**. SYNAPSE, AtomMem, HERO, Bi-Mem e HeLa-Mem usam vetores em alguma etapa [B20]. HippoRAG é a origem da ideia de PPR em recuperação [B20].

**Decisão de cautela:** começar com boost de um salto por wikilinks explícitos, depois de BM25. Só implementar PPR se o golden set mostrar ganho em consultas multi-hop. Co-acesso, como aresta, é fácil de capturar, mas também é um vetor de envenenamento (tema 5).

### Fontes do tema 3

| ID | URL | Pub. |
|---|---|---|
| B1 | https://arxiv.org/abs/2606.04194 e https://arxiv.org/html/2606.04194v1 | 2026-06-02 |
| B2 | https://arxiv.org/html/2510.17281v1 (ICML 2026, https://proceedings.mlr.press/v306/ai26e.html) | 2025-10-20 |
| B3 | https://www.letta.com/blog/benchmarking-ai-agent-memory | 2025-08-12 |
| B4 | https://pub.towardsai.net/mem0-vs-zep-vs-letta-a-folder-of-text-files-shouldnt-beat-the-61k-star-memory-layer-9d7e65c5799c | n/d (secundária) |
| B5 | https://arxiv.org/abs/2410.10813 | v2 2025-03 |
| B6 | https://smartscope.blog/en/ai-development/practices/rag-debate-agentic-search-code-exploration/ | n/d (secundária) |
| B7 | https://cursor.com/blog/semsearch | n/d |
| B8 | https://www.sqlite.org/fts5.html | n/d |
| B9 | https://docs.rs/tantivy/latest/tantivy/tokenizer/enum.Language.html | tantivy 0.27.0 |
| B10 | https://docs.rs/tantivy/latest/tantivy/query/struct.QueryParser.html | n/d |
| B11 | https://sayt2.readthedocs.io/en/latest/99-Maintainer-Guide/07-Dataset/index.html | n/d (secundária) |
| B12 | https://docs.rs/tantivy/latest/tantivy/query/struct.FuzzyTermQuery.html | tantivy 0.27.0 |
| B13 | https://docs.rs/tantivy/latest/tantivy/directory/static.INDEX_WRITER_LOCK.html | n/d |
| B14 | https://docs.rs/rust-stemmers/latest/rust_stemmers/enum.Algorithm.html | 1.2.0 |
| B15 | https://lists.tartarus.org/pipermail/snowball-discuss/2009-October/001120.html | 2009-10 |
| B16 | https://www.inf.ufrgs.br/~viviane/rslp | n/d |
| B17 | Orengo, Buriol, Coelho (CLEF), citado via resultado de busca ligado a B16 | n/d (não lido na íntegra) |
| B18 | https://dev.to/ahmet_gedik778845/migrating-video-title-search-from-sqlite-fts5-to-opensearch-fuzzy-queries-4bhj | n/d (secundária) |
| B19 | https://arxiv.org/abs/2212.10496 | 2022-12-20 |
| B20 | https://arxiv.org/html/2601.02744v1, https://arxiv.org/pdf/2606.19847 | 2026 |

---

## 4. Como medir se a memória ajuda

### 4.1 A evidência é desconfortável

| Estudo | Resultado |
|---|---|
| Gloaguen et al., ICLR 2026 [A1] | Arquivos de contexto não melhoram o sucesso; custo de inferência sobe mais de 20%. Visões gerais do repositório não ajudam. Recomendam usar para práticas não padronizadas |
| Ablação com Claude Code e Codex, 2026 [A2] | Claude: nenhum 53,3%, sempre ligado 55,6%, seletivo 55,6% (p=1,00). Codex: 58,8%, 56,9%, 52,9% (p=0,66). Detectar 10 pp exigiria de 120 a 200 tarefas |
| SWE-ContextBench [A3] | Experiência resumida e recuperada com precisão melhora resolução, tempo e tokens; contexto sem filtro ou mal escolhido traz ganho limitado ou **negativo** |
| MemoryBench, ICML 2026 [B2] | Sistemas de memória não superam RAG simples de forma consistente, nem aprendem bem com feedback procedural. Na tarefa SiLo, nenhum supera o modelo sem memória |
| Copilot Memory, A/B [S2] | Taxa de merge de 83% para 90%; feedback positivo em code review de 75% para 77%; p<0,00001 |
| Curated AGENTS.md (eficiência) | −28,6% de tempo e −16,6% de tokens de saída, citado por terceiros, não lido na fonte |

A ablação diz que as falhas vêm mais de habilidade de implementação que de falta de conhecimento do repositório [A2]. **Conclusão:** o produto não deve prometer "agentes acertam mais" como valor central sem medir. A promessa defensável é **menos correção repetida e menos tokens de redescoberta**.

### 4.2 Limites dos benchmarks públicos

- **LoCoMo:** métrica de substring favorece léxico por construção, segundo o ZenBrain, afirmação de um paper e não auditoria independente [A4]. A categoria de 42% é pontuada por substring, e há bug relatado no runner da Mem0 [A5]. Erros de rótulo são estimados em cerca de 6,4% [A6]. Também não tem perguntas de atualização de conhecimento, segundo a Zep, um fornecedor [A7].
- **LongMemEval:** cinco capacidades, incluindo atualização de conhecimento e abstenção, 500 perguntas [B5]. É o mais útil para nós, mas BM25 satura em recuperação na variante S [B1], então serve mais para regressão do que para diferenciação.
- **Janelas de contexto maiores** fazem benchmarks refletirem o raciocínio do modelo, não a memória. Isso vem de outro paper de 2026 visto em resultado de busca, sem identificação segura.
- **Nenhum deles** cobre português, código, ou correções repetidas.
- **Entity-Collision** (arXiv 2605.29630) propõe protocolo para atribuir ganho à recuperação. Não consegui ler o texto; só cito o título.

### 4.3 Métricas propostas

| Camada | Métrica | Como medir |
|---|---|---|
| Recuperação | Recall@k, MRR, nDCG | Golden set próprio, offline, sem LLM |
| Abstenção | Taxa de injeção vazia quando nada é relevante | Consultas sem resposta no golden set |
| Atualização | Fração em que a versão vigente vence a superada | Pares de memórias com `supersedes` |
| Custo | Tokens injetados por sessão; tokens por resposta de tool | Log local dos hooks e do servidor |
| **Correção repetida** | Mesma correção em sessões diferentes, por 100 sessões | Fingerprint das correções capturadas. **Métrica principal** |
| Redescoberta | Chamadas de Read/Grep antes do primeiro arquivo relevante | Via `PostToolUse` |
| Uso | Memória lida → citada na resposta → não corrigida em seguida | Ver 4.5 |
| Validade | Citações verificáveis no estado atual | Validação no uso |

Lista de medidas alinhada a [M17]: precisão, tempo, chamadas, tokens e erros de tool.

### 4.4 A/B realista

- Correção em nível de resultado é inviável com uso individual: 120 a 200 tarefas para 10 pp [A2].
- **Inferência:** alternar por sessão (switchback). Em uma fração das sessões, desligar só a injeção automática. Comparar métricas de processo por usuário.
- Sessões são heterogêneas. Coletar por tipo de tarefa, e não concluir nada com menos de dezenas de sessões por braço.
- Dados ficam locais. O usuário vê o próprio relatório, sem telemetria.
- A avaliação de tools recomenda conjuntos separados para evitar overfitting [M17].

### 4.5 Rastrear uso real sem LLM

Referências: Copilot verifica citações no uso, salva versão corrigida se contraditas, e renova o timestamp se válidas. Testes semearam memórias falsas, e os agentes as corrigiram [S2]. O voto reinicia a validade, **mas só recuperar não** [S3].

**Ciclo proposto (inferência):**

1. `memory_search` devolve memórias com ID curto e registra `exposed`.
2. A descrição da tool pede citar `[mem:ID]` quando usar.
3. `Stop` procura IDs em `last_assistant_message` e registra `cited`.
4. Se o prompt seguinte tiver sinal de correção, registra `cited_then_corrected`, com peso negativo.
5. Se não tiver, registra `cited_not_corrected`, reforço fraco.
6. Reforço forte exige evidência verificável: citação de arquivo ainda válida ou confirmação explícita.

**Cuidado:** feedback implícito é ruidoso [B2]. Exposição **nunca** reforça, ou o sistema cria um ciclo em que o que aparece se perpetua. Também não há garantia de que o agente cite o ID, então ausência de citação não é sinal negativo.

### Fontes do tema 4

| ID | URL | Pub. |
|---|---|---|
| A1 | https://arxiv.org/abs/2602.11988 | 2026-02 |
| A2 | https://arxiv.org/html/2607.27250v1 | 2026-07 |
| A3 | https://arxiv.org/abs/2602.08316 | 2026-02-09, v3 2026-05-06 |
| A4 | https://arxiv.org/pdf/2604.23878 | 2026-04 (trecho de busca) |
| A5 | https://arxiv.org/pdf/2604.10981 | 2026-04 (trecho de busca) |
| A6 | https://arxiv.org/pdf/2605.12213 | 2026-05 (trecho de busca; atribuição do paper não confirmada entre os resultados) |
| A7 | https://blog.getzep.com/lies-damn-lies-statistics-is-mem0-really-sota-in-agent-memory/ | n/d (fornecedor) |
| A8 | https://arxiv.org/abs/2608.20195 | 2026-08-20 |

---

## 5. Autoaperfeiçoamento sem LLM

### 5.1 Duplicatas

- SimHash gera uma impressão de 64 bits, com distância de Hamming tipicamente de 3 a 7 para detecção; MinHash estima Jaccard sobre shingles [S6].
- Em LSH, o limiar fica perto de `(1/b)^(1/r)` [S6].
- `gaoya` implementa ambos em Rust [S6]. Crates desse tipo têm manutenção variável.
- Texto curto dá poucos shingles, então Jaccard fica ruidoso. Isso é inferência da mesma busca; **memórias de uma frase caem nesse caso**.

**Proposta (inferência):**

1. Hash exato do texto normalizado.
2. Na escrita, buscar candidatos por BM25 com os termos mais raros da nova memória.
3. Jaccard de tokens e containment nos candidatos.
4. SimHash só como fingerprint de correções repetidas.

Em escala pessoal, o índice já resolve o problema de candidatos, e LSH é complexidade sem necessidade. Calibrar limiares no próprio corpus.

### 5.2 Decaimento

- FSRS-5: `R(t,S) = (1 + (19/81)·t/S)^(-0.5)`, em que `S` é o número de dias até R cair para 90% [S7]. Os parâmetros são ajustados em dados humanos de flashcards. **Inferência:** a forma pode ser usada, os pesos não se transferem.
- ACT-R: `B = ln(Σ t_j^(-d))`, `d ≈ 0,5`. De memória, não verificado nesta sessão.
- A doc da Anthropic sugere apagar arquivos de memória não acessados há muito tempo [S8]. O Copilot usa 90 dias, com voto automático; configurável entre 7 e 180 dias para fatos de repositório [S3].

**Proposta (inferência):**

- Decaimento **rebaixa no ranking**; não apaga.
- Por tipo: preferências e decisões não decaem por tempo, só por contradição. Fatos ligados a código são validados por citação. Observações de sessão decaem depressa.
- `S` só cresce com reforço forte (4.5). Exposição não conta.
- Um único parâmetro por tipo, calibrado com dados reais.

### 5.3 Consolidação

Sem LLM, consolidar é **fundir evidência**, não reescrever texto: manter a formulação de maior confiança, somar proveniências, registrar `merged_from`. Isso preserva reversibilidade.

**Promoção para arquivos de instrução:** só para práticas não padronizadas, curtas, confirmadas várias vezes, porque contexto sempre carregado aumenta custo em mais de 20% sem ganho [A1]. E contexto sem filtro prejudica [A3]. A promoção deve ser sugestão ao usuário, nunca automática.

### 5.4 Contradição heurística

Pelo que encontrei, ninguém descreve um sistema completo sem LLM; é uma composição de peças [S9].

- Estruturar quando der: entidade, atributo, valor, momento. Assim um conflito no mesmo atributo fica trivial [S9].
- Correções geram pares naturais: "use X em vez de Y" produz `(contexto, usa, X)` e invalida `(contexto, usa, Y)`.
- Mudança de polaridade com alta sobreposição de tokens: "usa/não usa", "sempre/nunca".
- Números, versões, caminhos ou URLs diferentes em texto quase igual.
- Recência é padrão, não regra. Uma fonte cuidadosa antiga pode valer mais que uma casual nova [S9].
- Não apagar. Cognee grava aresta `contradicts`; outros usam `supersedes` [S9]. Guardar tempo de validade e tempo de registro.
- Conflitos implícitos ficam de fora. Sem raciocínio, não dá para detectá-los [S9].
- Em conflito não resolvido, mostrar as duas versões e deixar o agente, ou o usuário, decidir.

### 5.5 Injeção e Unicode

**Ameaça real:**

- **MINJA** planta registros só por interação normal, sem acesso direto. Números de >95% de injeção vêm de resumos secundários [S10].
- **MemoryGraft** planta "experiências de sucesso" por README ou documento ingerido [S11]. É exatamente a superfície de memória automática.
- OWASP lista ASI06, Memory & Context Poisoning. Li só fontes secundárias [S12]. Orientação comum: validar escritas antes de salvar e tratar memória recuperada como não confiável ao montar o contexto.
- Defesa acadêmica com pontuação de confiança e decaimento depende de calibrar limiares [S13].
- "Lethal trifecta": dados privados, conteúdo não confiável e saída externa. Um servidor de memória é parte das duas primeiras [M16].

**Unicode:**

| Classe | Faixa | Ação |
|---|---|---|
| Tags | U+E0000–E007F | Quarentena |
| Bidi | U+202A–202E, U+2066–2069 | Quarentena fora de texto RTL legítimo |
| Zero-width | U+200B–200D, U+2060, U+FEFF | Remover na exibição; quarentena se em sequência longa. ZWJ é legítimo em emoji |
| Seletores de variação | U+FE00–FE0F, U+E0100–E01EF | Sinal de contrabando se em sequência |
| Homoglifos | UTS #39 | Marcar token com mistura de scripts |

As faixas são conhecimento padrão, não citadas em uma fonte lida aqui.

- **NFC/NFKC não pegam nada disso** (afirmação de uma ferramenta) [S14]. Inspecionar o texto **antes** de normalizar [S14].
- Cuidado com escape JSON: caracteres Tag viram pares substitutos UTF-16 [S14].
- UTS #39 v18: skeleton serve para testar confusão, e **não** deve ser usado como normalização [S15]. `unicode-security` 0.1.2 oferece `skeleton`, `MixedScript` e `RestrictionLevel` [S16].
- Para português, mistura real de scripts é rara. Um token com latim e cirílico é um sinal forte (inferência).
- Guardar o original no journal e exibir a versão sanitizada.

**Heurística de texto.** Padrões como "ignore as instruções", papéis falsos (`system:`), comandos de shell em memória, URLs novas, blocos base64, e menções a nomes de tools. Heurística tem falso positivo e negativo; é triagem, não garantia [S13].

### 5.6 Quarentena, proveniência e taint

Não achei fonte primária sobre taint em memória de agentes. A proposta abaixo é **desenho meu**, apoiado em [S12], [S13], [M16] e no padrão de citações do Copilot [S2].

| Classe de origem | Nível | Comportamento |
|---|---|---|
| Usuário escreveu diretamente | Alto | Pode ser injetada |
| Correção capturada | Médio | Candidata até confirmar |
| Afirmação do agente | Médio | Injetada só se tiver citação válida |
| Saída de tool ou arquivo do repositório | Baixo | Só via busca, nunca via hook |
| Web ou conteúdo externo | Mínimo | Quarentena |

Regras:

- Taint é propagado: fusão herda o **menor** nível.
- Hook nunca injeta memória de nível baixo.
- Promoção exige humano ou evidência independente, e nunca vem de conteúdo que pede a própria promoção.
- Toda saída marca origem e data, como dado.
- Co-acesso não cria arestas a partir de conteúdo tainted.
- Segredos são rejeitados na escrita, como no Copilot [S3].
- Caminhos são validados contra travessia, como pede a doc da memory tool [S8].

### 5.7 Versionamento

- O modelo de operações do Jujutsu grava operações em armazenamento endereçado por conteúdo, sem locks, e funde cabeças divergentes [C8].
- Journal append-only com compensação: `undo` é um novo evento, e nada é reescrito.
- Hash encadeado detecta adulteração (padrão conhecido, inferência para este uso).
- Git interno é opcional. Dá `diff` e `log` de graça, mas adiciona dependência. Segmentos JSONL já são versionáveis por quem usa git no vault.

### Fontes do tema 5

| ID | URL | Pub. |
|---|---|---|
| S2 | https://github.blog/ai-and-ml/github-copilot/building-an-agentic-memory-system-for-github-copilot/ | n/d |
| S3 | https://docs.github.com/en/copilot/concepts/agents/copilot-memory | n/d |
| S6 | https://openapps.pro/packages/gaoya, https://docs.rs/crate/sketch_oxide/latest/source/src/similarity/simhash.rs, https://mojoauth.com/security-guides/lsh-in-rust | n/d (secundárias) |
| S7 | https://theorempath.com/topics/fsrs-spaced-repetition-scheduling, https://borretti.me/article/implementing-fsrs-in-100-lines | n/d (secundárias) |
| S8 | https://platform.claude.com/docs/en/agents-and-tools/tool-use/memory-tool | n/d |
| S9 | https://www.memorylake.ai/en/blogs/memory-conflict-detection, https://www.hyperspell.com/blog/memory-conflict-resolution, https://docs.cognee.ai/examples/contradiction-handling | 2026-03-25 (Hyperspell); demais n/d; fornecedores |
| S10 | https://arxiv.org/abs/2503.03704 | v5 2026-02 |
| S11 | https://arxiv.org/pdf/2512.16962.pdf | 2025-12 |
| S12 | https://goteleport.com/blog/owasp-top-10-agentic-applications, https://workos.com/blog/ai-agent-memory-poisoning | n/d (secundárias) |
| S13 | https://arxiv.org/abs/2601.05504 | 2026-01 |
| S14 | https://pypi.org/project/glyphguard/, https://www.trackr.live/?p=1185 | n/d (secundárias) |
| S15 | https://www.unicode.org/reports/tr39/ | v18.0.0, 2026-08-27 |
| S16 | https://docs.rs/unicode-security/latest/unicode_security/ | 0.1.2 |

---

## 6. Coerência multi-processo em arquivos

### 6.1 O cenário

Cada sessão sobe seu próprio servidor stdio como subprocesso [M4]. Hooks são processos curtos. Com duas sessões e dois clientes, há vários escritores simultâneos e nenhum daemon.

### 6.2 SQLite WAL

Fonte: [C1], salvo indicação.

- Só na mesma máquina; não funciona em sistema de arquivos de rede.
- Um escritor por vez; leitores não bloqueiam.
- `SQLITE_BUSY` ainda ocorre em recuperação e no fechamento da última conexão. Usar `busy_timeout` e repetir.
- Leitor sempre ativo impede checkpoint, e o WAL cresce sem limite. **Servidor stdio não pode manter transação de leitura aberta.**
- Autocheckpoint perto de 1000 páginas. `journal_size_limit` controla o tamanho.
- `synchronous=NORMAL` em WAL pode perder a última transação em queda de energia, sem corromper.
- **Versão mínima:** bug de corrupção por corrida no reset do WAL corrigido em 3.51.3 (backports 3.44.6, 3.50.7).
- `PRAGMA data_version` muda quando **outra** conexão faz commit, inclusive de outro processo, e só compara na mesma conexão [C3]. É o generation counter pronto para invalidar cache.
- `PRAGMA fullfsync` usa `F_FULLFSYNC` no macOS; padrão desligado [C3].

Riscos [C2]:

- `close()` em qualquer fd do arquivo cancela os locks POSIX do processo. Nunca abrir o banco com `std::fs`.
- Renomear ou apagar um banco aberto pode parear journal errado. **Reconstrução do índice não pode trocar o arquivo sob conexões vivas.**
- Sincronização em nuvem: a doc não trata diretamente; a análise do risco é inferência a partir dela. **Índice fora de pastas iCloud, Dropbox ou similares.**

### 6.3 Escrita atômica

Receita: gravar temporário no **mesmo diretório**, `sync_all`, `rename`, `fsync` do diretório pai.

- Em Rust, `File::sync_all` e `sync_data` já usam `F_FULLFSYNC` em plataformas Apple, conforme o código da std [C4]. Não precisa de `fcntl` manual. Uma das buscas afirmou o contrário; o código-fonte prevalece.
- `F_FULLFSYNC` é caro. Reservar para eventos importantes.
- A durabilidade de `rename` no APFS via fsync de diretório **não está documentada de forma clara**: thread na Apple sem resposta conclusiva [C5]. Tratar como melhor esforço e recuperar via journal.
- `F_FULLFSYNC` pode falhar em alguns sistemas de arquivos. Código real faz fallback [C5].

### 6.4 Journal append-only como fonte de verdade

- Com `O_APPEND`, POSIX move o offset para o fim antes de cada escrita, sem intercalar deslocamento. Um registro por `write()` funciona em arquivo local [C6].
- **Não há teto de tamanho garantido** para atomicidade. A regra de `PIPE_BUF` vale para pipes [C6].
- Linha parcial após crash existe. Leitor precisa descartar a última linha incompleta e validar checksum.

**Lição do Beads.** O desenho original tinha JSONL em git como verdade, SQLite como cache e sincronização nos dois sentidos, e o ponto frágil era justamente a sincronização [C7]. Em 2026, o projeto migrou para Dolt e removeu cerca de 11.000 linhas, número de fonte secundária. Parte dos usuários solo sentiu falta do modelo simples. Há um port Rust que mantém SQLite + JSONL sem daemon [C7].

**Decisão:** fluxo em **uma só direção**. O journal gera o índice. O índice nunca escreve de volta. Apagar o índice é sempre seguro.

**Dois tipos de verdade:**

| Dado | Verdade | Derivado |
|---|---|---|
| Notas do usuário | Arquivos `.md`, editáveis em qualquer editor | Índice; hash observado vira evento |
| Memórias automáticas | Journal | Índice e `.md` renderizado |

Edição manual de uma memória renderizada é detectada por hash e vira evento `user_edit`, de nível alto.

**Segmentos por escritor (inspirado em [C8]).** Um arquivo por sessão, `journal/<ulid>.jsonl`, elimina disputa de append e facilita merge via git. Ordem global por ULID ou relógio híbrido. Compactação offline funde segmentos antigos. **Inferência:** mais simples que coordenar um arquivo único, ao custo de abrir vários arquivos.

### 6.5 Sem daemon: como manter o índice atualizado

1. Hooks **só anexam ao journal**. Sem SQLite em `SessionEnd` (orçamento de 1,5 s) [H1].
2. O índice guarda o offset aplicado por segmento.
3. Quem precisa ler recupera o atraso antes: verifica tamanho dos segmentos, aplica o que falta.
4. Vários processos podem tentar. O SQLite serializa, e aplicar é idempotente por ID de evento.
5. Em `SessionStart`, varredura de notas por `mtime` + tamanho, com hash quando o `mtime` for recente.
6. Validação cruzada periódica: reconstruir de zero e comparar contagens.

**`notify` é dispensável no núcleo.** Editores geram eventos diferentes, e NFS pode não emitir nada [C9]. Só faz sentido dentro de um processo vivo. Claude Code já oferece `FileChanged` + `watchPaths` durante a sessão [H1].

**Reconstrução segura:** gerar `index-<geração>.db` novo, trocar por um ponteiro atômico, e deixar o antigo morrer quando as conexões fecharem. Nunca renomear sobre banco aberto [C2].

**Locks.** Usar `flock` em arquivo de lock separado, nunca no banco. tantivy usa `flock` em Linux e macOS, com aviso sobre NFS [B13]. Se tantivy for escolhido, cada escritor de curta duração disputa `INDEX_WRITER_LOCK`. A saída é tentar sem bloquear, e quem não conseguir segue lendo o journal recente sem indexar. **Inferência, a medir.**

### Fontes do tema 6

| ID | URL | Pub. |
|---|---|---|
| C1 | https://www.sqlite.org/wal.html | n/d |
| C2 | https://www.sqlite.org/howtocorrupt.html | n/d |
| C3 | https://www.sqlite.org/pragma.html | n/d |
| C4 | https://doc.rust-lang.org/src/std/sys/fs/unix.rs.html | n/d (Rust estável) |
| C5 | https://developer.apple.com/forums/thread/849633, https://docs.rs/crate/libfreemkv/latest/source/src/io/writeback_file/macos.rs | n/d |
| C6 | https://www.open-std.org/JTC1/SC22/wg14/www/docs/n2830.pdf, https://nullprogram.com/blog/2016/08/03/ | 2016-08-03 (nullprogram) |
| C7 | https://www.dolthub.com/blog/2026-04-02-restoring-beads-classic/, https://dolthub.com/blog/2026-01-15-a-day-in-gas-town, https://pkg.go.dev/github.com/steveyegge/beads@v0.21.1, https://lwn.net/Articles/1070995/, https://wikidocs.net/382542 | 2026-01-15, 2026-04-02; demais n/d |
| C8 | https://git.joshthomas.dev/mirrors/jj/src/branch/main/docs/technical/concurrency.md | n/d (espelho) |
| C9 | https://docs.rs/notify/latest/notify/ | n/d |

---

## Lacunas e pontos não verificados

- **Copilot CLI:** suporte a sampling, elicitation, roots, tasks e à versão 2026-07-28 não confirmado. O campo exato de instruções do servidor também. VS Code: idem para tool search.
- **Transcript do Copilot:** o evento `agentStop` traz `transcriptPath` [H5], mas não verifiquei o formato do arquivo.
- **BM25F em FTS5:** não confirmei se a normalização de comprimento é por coluna; a página de código bloqueou o acesso automático.
- **Benchmarks de velocidade e tamanho** de FTS5 contra tantivy no nosso cenário: não encontrei. Medir.
- **Tempo de arranque** de processo Rust com SQLite ou tantivy para hooks: não medido.
- **Taxa real com que agentes chamam tools de memória** sem instrução forçada: a busca sobre isso não pôde ser feita (cota esgotada). Sobre um dado próximo, 70% das consultas de documentação são auto-iniciadas, e arquivos para agentes dominam 60,5% das interações [A8].
- **ACT-R, RRF, faixas Unicode:** citados de conhecimento próprio, sem leitura nesta sessão.
- **Entity-Collision (2605.29630):** texto ilegível, findings não usados.
- **Números de MINJA, trigrama 3,4×, "Claude remove tags", "~70% R@5":** secundários.

---

## Achado → Decisão

| # | Achado | Decisão | Fontes |
|---|---|---|---|
| 1 | Sampling deprecado; nenhuma chamada de LLM é necessária para servir memória | Servidor nunca chama LLM. Reformulação e julgamento ficam com o agente, no turno dele | M6, restrição |
| 2 | Copilot CLI só tem tools; #1518 e #1803 abertas; URIs de recurso falham | **Tudo essencial via tools.** Resources e prompts são extras. Resposta traz trecho inline e caminho, nunca só `resource_link` | M10, M11 |
| 3 | Claude Code negocia 2026-07-28 em stdio desde 2.1.292; clientes legados ainda existem | `rmcp` com as duas gerações, `server/discover`, `resultType`, `ttlMs`, `cacheScope`. Teste de integração com os três clientes | M3, M8, M24 |
| 4 | Tool search é padrão; a busca lê só os primeiros 4.096 caracteres; 3 a 5 tools devem ficar carregadas | No máximo 4 tools. `memory_search` com `anthropic/alwaysLoad`. Palavras-chave PT/EN no início. Instructions dizem quando buscar | M7, M19 |
| 5 | Limites de 10k tokens (aviso), 25k tokens e 50k caracteres; `CONCISE` usa ~1/3 dos tokens | `response_format` com padrão conciso. Meta de ≤2.000 tokens por resposta. Paginação e dica ao truncar | M7, M17 |
| 6 | `readOnlyHint` dá paralelismo e dispensa confirmação; anotação não é segurança | `readOnlyHint` nas leituras, `idempotentHint` nas escritas por chave. Segurança fica no servidor | M7, M13, M16 |
| 7 | `additionalContext` tem 10.000 caracteres; frases imperativas podem acionar defesa contra injeção | Injeção como fato com data e origem. Orçamento de ≤1.500 caracteres em `SessionStart` e ≤800 por prompt, a calibrar | H1 |
| 8 | Só `sessionStart` e `postToolUse` injetam no Copilot; `userPromptSubmitted` só captura | Injeção por arquivo tocado (`PostToolUse`/`postToolUse`) como mecanismo comum. Por prompt, só em Claude Code. Parsers separados para snake_case e camelCase | H1, H5 |
| 9 | Contexto sem filtro é neutro ou negativo; arquivos de contexto custam +20% | **Precisão acima de recall.** Abster-se é resposta válida. Só injetar acima de limiar calibrado; o resto fica na busca | A1, A2, A3 |
| 10 | BM25: R@5 0,948 em LongMemEval-S e 0,833 em LoCoMo; memória avançada não supera RAG simples; reranker web piorou | BM25 ponderado como base. Sem cross-encoder. Ganhos vêm de campos, aliases e reformulação | B1, B2 |
| 11 | FTS5 sem stemmer PT; `rust-stemmers` tem `Portuguese`; Snowball falha em `aviões` | Colunas `*_stem` geradas em Rust, tabela de plurais irregulares, campo literal paralelo. RSLP só se o golden set exigir | B8, B14, B15 |
| 12 | tantivy tem um único escritor; SQLite WAL serializa escritores entre processos; **SQLite é C** | FTS5 como padrão, atrás de trait. **Decisão humana pendente:** se "100% Rust" veta C no link, usar tantivy com recuperação do atraso pelo journal | B8, B13, C1 |
| 13 | Trigrama de documento é caro e piora ranking | Sem índice trigrama. Correção de typo no vocabulário (`fts5vocab`), só em fallback | B8, B18 |
| 14 | Sem evidência de PPR sem embeddings em LoCoMo | Começar com boost de wikilink a um salto. PPR só com ganho medido | B20 |
| 15 | Multi-query e aliases não gastam tokens do servidor | `memory_search(queries[1..3])` com RRF; `aliases` na escrita. Rascunho estilo HyDE fica experimental | B5, B19 |
| 16 | Regex em hook funciona, mas tem falsos positivos | Correção capturada vira candidata. Promove por repetição em sessões distintas ou confirmação. Sidecar do `Stop` evita parsear transcript | H1, H8 |
| 17 | Copilot valida citações no uso, com A/B de 83→90%; só recuperar não reinicia a validade | Citações verificáveis (arquivo, trecho, hash). Validar no uso. **Exposição nunca reforça** | S2, S3 |
| 18 | Detectar 10 pp exige 120 a 200 tarefas | Métrica principal é **correção repetida por 100 sessões**. Switchback por sessão com métricas de processo. Relatório local | A2 |
| 19 | LoCoMo tem viés de substring e erros de rótulo; nenhum benchmark cobre PT e código | Golden set próprio em PT-BR, com abstenção e atualização. Benchmarks públicos só como regressão | A4, A5, B5 |
| 20 | FSRS é ajustado para humanos | Usar só a forma da curva, por tipo. Decaimento rebaixa, não apaga. Preferências decaem só por contradição | S7, S8 |
| 21 | Sem proveniência, memória vira vetor persistente de ataque (MINJA, MemoryGraft) | Classes de origem, taint herdado pelo menor nível, quarentena. Saída de tool nunca é injetada por hook | S10, S11, S12 |
| 22 | NFKC não pega Unicode invisível; skeleton não é normalização | Inspeção no texto bruto, antes de normalizar. Guardar original, exibir sanitizado. `unicode-security` para mistura de scripts | S14, S15, S16 |
| 23 | Beads sofreu com sincronização bidirecional JSONL/SQLite | Journal append-only é a verdade das memórias, `.md` é a verdade das notas. Índice só derivado, sempre descartável | C7 |
| 24 | Um escritor por vez; leitor longo trava checkpoint; `SessionEnd` tem 1,5 s | Hooks só anexam ao journal. Segmento por sessão. Leitor aplica o atraso antes de consultar. Sem transação aberta ociosa. SQLite ≥3.51.3 | C1, C6, H1 |
| 25 | `notify` varia por editor; `data_version` detecta commits externos | Sem watcher no núcleo. Generation por tamanho de segmento e `data_version`. Reconciliação por `mtime` + hash no início da sessão | C3, C9 |
| 26 | `${CLAUDE_PLUGIN_DATA}` é apagado ao desinstalar; cloud sync corrompe bancos | Vault fora do plugin. Índice em cache local fora de pastas sincronizadas | H2, C2 |
| 27 | `sync_all` em Rust já usa `F_FULLFSYNC` no macOS; `rename` no APFS não tem garantia documentada | Usar `sync_all` sem `fcntl` manual, só em eventos críticos. Recuperar a partir do journal | C4, C5 |
| 28 | Claude Code já tem auto memory, e o Copilot tem Memory com 90 dias | Coexistir: não duplicar `MEMORY.md`. Diferenciais: local, multi-cliente, auditável, com undo e métrica de correção | H4, S3 |
</content>
