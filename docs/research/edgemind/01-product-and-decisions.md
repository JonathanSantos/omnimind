# EdgeMind: produto, modelo de memória e decisões

Fonte: leitura somente de `~/Developer/projects/Second Brain` (README, `docs/product`, `docs/brainstorm`, `docs/brand`, `docs/specs/v2`, `docs/specs/initial`, `docs/specs/contracts`, `docs/specs/v1-backlog.md`, `docs/prompts/rival-prompt.md`). Nada foi rodado nem alterado lá. Tudo datado de 31/10/2026.

Ressalvas de leitura: (1) a "lista de rejeitados" não existe como arquivo; ela está espalhada em `decisions.md`, `naming.md`, `ideas.md` e nos "Fora da v2". (2) `brand/screens/` são mockups HTML/SVG, não PNG (faltava o `support.js`, não renderizei); li o HTML, o CSS e vi o logo PNG. (3) Os "8 problemas" vêm de `docs/prompts/rival-prompt.md`, não da doc do EdgeMind; o mapeamento abaixo é minha síntese.

## 1. Visão do produto

EdgeMind = app de notas local (Tauri, estilo Obsidian) + binário `edgemind` (CLI e servidor MCP por stdio). Um mesmo formato de pasta ("brain") liga os dois. Primeira usuária: quem construiu, dev com Claude Code pessoal e Copilot na empresa, mundos separados. Tagline: "Your notes, your AI's memory, joined by edges you can see."

Princípios (`product/overview.md`): local primeiro (Markdown na máquina); brains isolados; revisão humana (a IA propõe, a pessoa decide); autoria visível; tudo auditável; complexidade só com evidência.

Não é: nuvem, substituto de Claude Code/Copilot, compatível com plugins Obsidian, mobile ou sync (v1).

Estado: v1 pronta só no macOS; Windows/Linux apenas configurados, nunca compilados. Nota de simulação 7,5; meta 9 na 1.3.0. A v2 (improve, tags) está só especificada.

### Os 8 problemas e como o EdgeMind responde

| # | Problema | Resposta do EdgeMind | Fraqueza observada |
|---|---|---|---|
| 1 | IA esquece | Memórias condensadas + `search/read` por MCP; hook `SessionStart` injeta contexto essencial; `PreCompact` captura a sessão | Corte silencioso de 9.000 bytes no `SessionStart`: com 500 memórias só entram conceitos, sem decisões |
| 2 | IA não conhece a pessoa | Tipo `preference`, `schema/preferences.md`, procedures | Procedure com 1 caso é só episódio; detectar padrão entre projetos exige ingerir `.sessions/` (custo) |
| 3 | Memória não confiável | IA só escreve na `inbox/`; aprovação humana; procedência; flags `instruction_like`, `oversized`, `unknown_edge` | Aprovação em lote alcança cartões não vistos; U+202E e largura zero sem marca na UI |
| 4 | Conhecimento envelhece | `source.hash`: nota mudou, memória vira `stale`; `valid_until`; `supersedes` | Só 1 `sourceNote` por memória; contradição entre duas decisões ativas não é detectada pelo lint; pessoa não consegue ligar `supersedes` depois |
| 5 | Ninguém vê | Log JSONL append-only; grafo ao vivo (SSE) com aresta acesa; painel de eventos | IA usa `hops: 0` em 33 de 45 leituras, então só o nó acende; ao ler 10+/s só a mais recente por quadro acende |
| 6 | Contextos se misturam | Brain = pasta; MCP preso a 1 brain na partida; sem índice compartilhado; export manual para Copilot | Cliente só é filtrado por substring (`xclaudecodex` passa em `allowed_clients`); sinapses entre brains adiadas |
| 7 | Escrever tem que ser bom | Editor CodeMirror, wikilinks, backlinks, `Cmd+K`, notas diárias | Texto perdido em transições (troca de nota, Quit/`terminate:`, conflito); editor em WebView |
| 8 | Dá trabalho | Skills `edgemind-ingest`/`edgemind-lint`, hooks, inbox em lote | O trabalho continua: a pessoa revisa cada proposta; v2 diz explicitamente "a pessoa dispara cada rodada" |

### Fluxo ponta a ponta (v1)

1. `edgemind init` cria o brain; `claude mcp add edgemind -- edgemind mcp --brain <pasta>` conecta o Claude Code (Copilot: MCP se permitido, senão `edgemind export copilot --out ...`).
2. Pessoa escreve notas (`notes/`, só ela edita; a IA lê).
3. Ingest: skill lê `ingest:pending` (até 25 notas por chamada) e chama `propose`; sessões também viram propostas via `PreCompact`/`.sessions/`.
4. Proposta cai em `inbox/<id>.md` com `proposal{client, session, rationale, flags}`.
5. Pessoa aprova (move para `memory/`, grava `confirmed_by: human`), edita ou rejeita (vai para `.trash/inbox/`, motivo vira evento).
6. Nas sessões seguintes a IA usa `search` (BM25, resumo, aliases) e `read` (corpo + vizinhos a 1-2 saltos); cada chamada gera evento, o app acende a aresta.
7. Correções da pessoa (`correct`) entram em `schema/corrections.md` (20 mais novas) e nas `instructions` do `initialize` (5 mais novas, 800 chars).
8. `lint` acha links quebrados, duplicadas, órfãs, `stale`.

## 2. Modelo de memória

- **Tipos** (`mcp-tools.md`): `concept`, `episode`, `procedure`, `decision`, `preference` (a preference aprovada vira linha em `schema/preferences.md`, não arquivo em `memory/`).
- **Camadas** (inspirado no LLM Wiki de Karpathy): `notes/` (humano, imutável para a IA) / `memory/` (IA condensa, humano aprova) / `schema/` (regras, evals, correções).
- **Frontmatter**: `id` ULID estável, `title`, `summary` (curto; é o que a busca devolve), `aliases` (PT+EN), `edges` tipadas (`relates_to`, `depends_on`, `supersedes`, `contradicts`, `applies`, `derived_from`), `source{note,hash}`, `author`, `confirmed_by/at`, `valid_from/until`, `origin` (reservado para sinapses).
- **Estados calculados, nunca gravados**: `active`; `stale` (fonte sumiu ou hash mudou; continua na busca com aviso); `expired` (`valid_until` passou; some da busca); `superseded` (outra ativa aponta `supersedes`; some).
- **Ciclo de vida**: nota -> proposta (`pending_review` + flags) -> aprovada (`active`) -> `stale` (fonte mudou) -> confirmar (recalcula hash) ou substituir (`supersedes`) -> `expired`/`superseded`. Histórico nunca é apagado; encerra-se com `valid_until`.
- **Procedência**: `author` (cliente de IA; `human` é recusado como cliente), `confirmed_by`, `source.note+hash`, `proposal.client/session/rationale`. Na UI: selo `human`/`ai` + "Claude Code · session 41".
- **Índices são cache**: Markdown é a verdade; Tantivy (texto, BM25, bom em PT) + SQLite (grafo) recriáveis. O catálogo gerado `memory/index.md` lista memórias ativas.
- **Sinais de uso**: eventos `search, read, propose, approve, reject, correct, lint`, jsonl por mês; `use_count` existe mas zera em rename/rebuild.
- **Resultado vazio**: `search` devolve `incomplete` + `notice` (conteúdo é dado, não instrução; vazio não prova inexistência).

## 3. Decisões-chave, motivo e rejeições

| Decisão | Motivo | Para a direção automática |
|---|---|---|
| IA só escreve na inbox | Envenenamento de memória persistente (pesquisa: Akto); revisão humana | Reconsiderar: é o gargalo do problema 8. Trocar "aprovar antes" por "aplicar com auditoria + desfazer + quarentena para o arriscado" |
| Só 3 tools MCP (`search`, `read`, `propose`); ids reservados (`ingest:pending`, `tags:list`) em vez de 4a tool | Descrições de tools gastam contexto e são vetor de injeção | Vale manter pequeno. Mas precisa de verbo de escrita direto (`remember`/`correct`) |
| MCP preso a 1 brain; brain nunca é parâmetro | IA não "salta" entre pessoal e trabalho | Manter. É o melhor desenho de isolamento deles |
| Markdown é verdade, índice descartável | Recuperação e portabilidade | Manter, mas projetar para N processos desde o dia 1 (o app, o MCP e clientes escrevem juntos) |
| Sem RAG vetorial; BM25 + grafo + aliases | Simplicidade; "medir antes"; recall de paráfrase mesmo assim: 0,333 (próprio) e 0,083 (corpus QA) | Mesma trava do rival (sem embeddings): paráfrase é o buraco real. Aliases gerados na escrita + reformulação aprendida do log |
| Tantivy + SQLite (FTS5 só tem stemmer em inglês) | PT-BR | Reaproveitável |
| Tags no máx. 2 níveis, facetas (`projeto/`, `area/`, `fase/`, `fonte/`) | Evitar caminhos equivalentes | Ver seção 6 |
| Interface em inglês na v1, voz sem emoji/"AI-powered", IA sem nome próprio | Público global | A direção rival decide a sua própria voz |
| Copilot sem MCP via export; sem chave pessoal no brain de trabalho | Empresa pode bloquear MCP; não enviar dado corporativo a conta pessoal | Manter o princípio de separação |
| Complexidade só com evidência (eval em `schema/evals.md`, 20-30 perguntas reais) | Benchmarks de memória são frágeis (Letta 74% com arquivos vs Mem0 68,5%) | Copiar a prática; usar perguntas reservadas |

**Rejeitado ou adiado, com motivo**
- Subtag com dois pais: dois caminhos para a mesma ideia, renomear afeta várias árvores, cada sessão da IA escolheria diferente.
- Índice compartilhado entre brains; sinapses (v1): risco de vazamento; só reservam `origin`.
- Plugins Obsidian, canvas, sync, mobile: escopo.
- Embeddings locais, consolidação avançada, reforço por uso: "só com evidência". Não há evidência de que consolidação offline ajude agentes de código (research.md).
- Melhoria automática sem revisão; agendar rodadas; IA alterar notas (Fora da v2): confiança e injeção.
- Memória criada por qualquer pedido ou correção, sem revisão: nunca foi tentada; a doc a trata como risco, não como risco medido.
- Nomes: Cerne (PT), Hebb, Myelin, Loci (já ocupados). Aviso: logo de cérebro com circuitos é comum; falta busca de anterioridade (INPI).
- Aprendizado de segurança: texto escondido chegava à memória (bloco `---`, largura zero, tags Unicode) e só apareceu em QA; hoje há flag e regra única de aprovação (`desktop/src/features/inbox/review.ts`).

**O que a direção "automática" precisa reconsiderar:** a prova de que "revisão humana" funciona é fraca (propostas aprovadas sem edição: meta 60%, como na v1; a aprovação em lote chega a cartões não vistos). O rival terá de substituir o portão por: escrita direta + diff legível + motivo + desfazer em 1 clique + quarentena de texto suspeito + trilha de procedência. Os riscos listados em `improve.md` (inchaço, alias ruim desviar buscas, consultas sensíveis no relatório, prompt injection pelo log) são o seu checklist de segurança.

## 4. Marca e visual atual (onde é tradicional)

- **Marca:** cérebro visto de cima, circuitos que terminam em nós. Hemisfério em linha = o que você escreve; preenchido = IA; linha dourada = aresta acesa. Visto em `exports/png/edgemind-mark.png`: preto e dourado sobre branco, traço grosso.
- **Cor:** ouro `#F5B62E` só para atividade ao vivo e ação primária; `cresyl` (violeta `#CF96E5` escuro / `#7B2D93` claro) = conteúdo da IA; `ink` = conteúdo humano; azul `ok` e laranja `danger` (daltonismo). Escuro é o principal; fundo `#14101A`, superfície `#1C1723`.
- **Tipografia:** Bricolage Grotesque (display, só estados vazios/boas-vindas), Instrument Sans (UI), Spline Sans Mono (labels, tipos). Corpo de UI 13px, 4px de grid, raios 2-8px.
- **Movimento:** um só gesto: aresta acesa, desenha em 240ms, fica acesa durante a leitura, apaga em 600ms. Interações em até 120ms. Sem gradientes, brilhos ou sombras (só em overlay).
- **Grafo:** forma = tipo (círculo, círculo maior, ponto, quadrado, losango); cor = autor (nó cheio tinta = humano; contorno violeta = IA); aresta repouso `line-strong`, lida em ouro 2.5px, obsoleta tracejada. Mockup: SVG estático de ~10 nós, 740x660, rótulos de texto fixos, legenda embaixo.

**Onde é tradicional e fraco (para o rival não parecer com ele)**
- Layout clássico de 3 colunas (sidebar 232px, principal, painel lateral 288px), cartões em lista, botões Approve/Edit/Reject: parece Obsidian/Linear comum.
- Marca de "cérebro com circuitos" é clichê saturado de IA; o próprio `naming.md` admite. Paleta violeta + dourado é correta mas contida ("em tela parada quase não deve haver ouro").
- O grafo é a "tela que define o produto", mas é desenhado em SVG, limitado a 300 nós fora do WebGL (Sigma), layout de 2.000 nós leva 2,8-5,2 s no WKWebView, posições não persistem, rótulos se sobrepõem, só "um nó acende" quando `hops: 0`.
- Um único movimento de marca; sem física, partículas ou shaders; descrição de estados vazios é só texto ("Nothing to review...").
- Inbox é lista de cartões a revisar uma a uma: processo de triagem, não de descoberta. Nada é divertido ou compartilhável.
- Eles cuidam de daltonismo, foco e reduced-motion; o MVP rival deixou acessibilidade de fora, então essa não é uma vantagem de diferenciação.

## 5. Especificação V2: improve

Problema medido: na simulação 14 de 38 notas nunca foram abertas e a IA afirmou "não há memória" sobre algo que existia.

- **Fluxo**: `/edgemind-improve` -> `edgemind improve --json` (sem IA, <1 s com 100.000 eventos) -> `edgemind eval` (baseline) -> IA abre só os itens suspeitos -> propõe -> pessoa revisa -> eval de novo -> `improve --history` (`.index/improve-history.jsonl`).
- **Lacunas calculadas do log**: `uncovered`, `dead_end` (busca sem resultado e sem leitura depois), `reformulated`, `unused` (90 dias sem leitura), `corrected`, `stale_used`, `pair_gap` (lidos juntos em 3+ sessões sem aresta), `tag_debt`. Ruído controlado: exige sinal em 2 sessões ou 1 correção.
- **Mais valioso: `reformulated`.** Duas+ buscas na mesma sessão até um `read` provam que a frase falha deveria apontar para aquela memória, vira alias. Melhora paráfrase sem embeddings e sem tokens.
- **Propostas `patch`**: `addAliases/addTags/removeTags/addEdges/summary/validUntil`; título, corpo, fonte ficam de fora (mudança grande = memória nova + `supersedes`); diff campo a campo, aprovação parcial, `baseHash` para conflito.
- **Limites**: máx. 20 lacunas por rodada, `--budget` em dólares, crescimento <= 5% do brain (mín. 5), saldo final criadas/ajustadas/encerradas.
- **Medição**: seção `## Reservadas` em `schema/evals.md` que a rodada não vê (evita ajustar-se à prova); perguntas nascidas de lacunas nunca vão para as reservadas; métricas de longo prazo: `dead_end`/100 buscas, reformulações/sessão, correções/semana, buscas até a primeira leitura.
- **Modo de trabalho**: `procedure` exige `evidence` com 2+ projetos/sessões; correções de conduta viram `preference`.
- **Vale aproveitar**: relatório sem IA a partir do log; `reformulated`->alias; encolher como melhoria (`unused`, `stale_used`); eval com reservadas; teto de crescimento; saldo no fim da rodada.
- **Não aproveitar**: gatilho manual da rodada e portão humano total; o rival pode rodar incremental, contínuo e barato (o cálculo é de graça).

## 6. Especificação V2: tags

- Problema real, só 17 memórias: `rota-certa` vs `projeto/rota-certa`; tags que repetem o tipo (`decisao`); 5 de 8 tags no brain de trabalho usadas uma vez.
- Tags servem para agrupar/navegar, não para achar fato (isso é alias/busca).
- 2 níveis: pai = faceta (`projeto`, `area`, `fase`, `fonte`) + subtag; subtag sempre de um pai só; combinação de facetas no lugar de 3 níveis. Filtrar por pai inclui filhos.
- Vocabulário `schema/tags.md` editável, com sinônimos (`rota-certa -> projeto/rota-certa`); `read tags:list` (máx. 200), `search` com filtro `tags`, `propose` normaliza sinônimos e flaga `new_tag`.
- Lint: `tag_duplicate` (distância de edição <= 2), `tag_is_type`, `tag_singleton` (30 dias), `tag_unknown`, `memory_untagged`, `tag_too_deep`, `tag_parent_unknown`, `tag_shadowed`.
- IA recomenda: <= 3 tags por memória, <= 3 tags novas por rodada, prefere as existentes. Medição: duplicadas = 0, 1-3 tags em >= 90%.
- **Vale aproveitar**: tag como faceta, normalização por sinônimo no momento da escrita, vizinhos pela tag mais rara, cor/agrupamento no grafo por faceta.
- **Fraqueza**: vocabulário manual + aprovação de cada tag nova: burocracia. Um rival automático pode derivar facetas da procedência (projeto = pasta/repo da sessão) sem vocabulário escrito à mão.

## 7. Lacunas onde um rival mais automático pode vencer

1. **Captura sem atrito**: tudo depende de a pessoa disparar skill/revisar inbox; nenhuma ferramenta MCP mostra a inbox à IA (a IA não sabe o que já propôs). Escrita direta + correção virando memória na hora.
2. **Paráfrase**: recall@5 0,083-0,333. Aliases na escrita e reformulação aprendida do log (a ideia `reformulated` é deles, mas manual).
3. **Frescor ativo**: `stale` só aparece no lint; o backlog pede aviso. Revalidar sozinho contra a fonte e avisar no ponto de uso, não só no `SessionStart`.
4. **Vazio honesto**: `incomplete` existe só em `search`, não em listagens. Todo resultado deveria dizer "completo / parcial / vazio confirmado".
5. **Contradições**: duas decisões ativas sem `contradicts` passam no lint; `sourceNote` único. Detecção na escrita.
6. **Cobertura**: 14 de 38 notas ignoradas por ingest dependente de skill; o produto precisa saber o que já foi coberto (1.3.0 trata, mas ainda via skill).
7. **Vigilância**: grafo acende só o que a IA leu com `hops` > 0; nenhum lugar mostra "o que a IA aprendeu hoje" como história, nem "o que mudou" com diff/reversão.
8. **Interface**: WebView, SVG, grafo limitado, 1 animação. O espaço para GPU nativa, física, shaders e densidade é grande.
9. **Segurança em fluxo automático**: ninguém olha antes de gravar. O desenho de quarentena/diff/rollback/prova (a pessoa vê o que mudou, por que e volta atrás) é o diferencial a vencer, não só um requisito.
10. **Robustez de dados que deles falhou** (aprenda o catálogo): perda de texto em Quit/transições, índice stale, `memory/index.md` perdendo memória com 2 processos, rename sequestrando links, symlink escrevendo em outro brain, transcrição de sessão no git do brain, hook entregando memória velha como fato.
11. **Custo/privacidade**: o próprio EdgeMind admite `claude -p` com cobrança incerta (simulação: US$ 0,86 em 9 sessões); um rival "sem chave de API" precisa que o processamento seja da IA que a pessoa já paga, e a memória a incluir no brain do contexto certo.
