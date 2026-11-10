# EdgeMind: app desktop (pesquisa)

Fonte (somente leitura): `~/Developer/projects/Second Brain/desktop` e `docs/specs`. Stack deles: Tauri 2 + WebView (React 19, CodeMirror 6, Sigma.js/WebGL2, TanStack Query, Zustand) sobre `edgemind-core` em Rust. A UI inteira é DOM/CSS num WebView. Tudo abaixo vem do código e das specs, não de uso do app.

## 1. Telas, fluxos e atalhos

Rotas (hash, `src/app/router.ts`): welcome, notes (editor), graph, inbox, memories (lista e detalhe), search, connections, health (lint), settings. Barra lateral: troca de brain, 6 destinos, árvore de pastas virtualizada, rodapé (Health, Settings, tema).

| Tela | O que faz | Como a IA aparece |
|---|---|---|
| Welcome | Criar brain (nome, pasta `~/brains/<slug>`) ou abrir pasta existente; lista de recentes com "forget" | Nada. Texto: "a IA propõe memórias, você decide o que fica" |
| Notes | Abas, árvore, editor, painel lateral (backlinks, memórias da nota, outline, tags, recentes) | Só no painel: memórias cuja origem é a nota. Nenhum sinal de "IA lendo esta nota agora" |
| Graph | Grafo + painel "Reading now" + log de eventos (40) | Aresta dourada se desenha, nó lido fica dourado e maior, tag "Claude Code reading", chip QUERY |
| Inbox | Propostas da IA: aprovar, editar, rejeitar com motivo, lote, correções | Contador na barra lateral; cartão com origem, flags e texto bruto |
| Memories | Lista (virtualizada) e detalhe: estado, validade, origem, arestas, "uso", editar, confirmar, encerrar validade | Selo de autor (humano vs cliente), estado stale/expired/superseded |
| Search | Mesma paleta, como página | Resultados mostram autor |
| Connections | Cartão por cliente (Claude Code, Copilot), comando para copiar, exportar para Copilot | Status "connected" |
| Health | Lint do núcleo, filtro por tipo, correção de um passo (confirmar, estender) | Itens stale/expired |
| Settings | Só leitura: nome, pasta, id, idiomas, `allowed_clients`, contagens, estado do índice, "rebuild" | Nada |

Atalhos (`Cmd` no macOS, `Ctrl` no resto; `src/app/shortcuts.ts`): `K` paleta; `1..6` Notes, Graph, Inbox, Memories, Connections, Health; `,` Settings; `N` nova nota; `O` abrir por título; `W` fecha aba; `[`/`]` voltar/avançar; `/` ajuda; `Shift+N` nova janela; `Shift+L` tema; `Shift+D` nota do dia; `Shift+F` nova pasta; `Shift+[`/`]` aba anterior/próxima. Editor: `Cmd+B`, `Cmd+I`, `Cmd+Enter` (abre link sob o cursor ou marca tarefa), `Cmd+F`. Inbox: `J/K` move, `A` aprova, `E` edita, `R` rejeita, `X`/espaço seleciona, `Shift+A`/`Shift+R` agem na seleção, `Esc` limpa, `Cmd+Enter` aprova no editor. Grafo: `+ - 0`, setas, `Esc`; clique abre, `Shift`+clique ou clique direito foca a vizinhança (1 ou 2 saltos). Árvore: setas, `F2` renomear, `Delete`, `Shift+F10`, arrastar para mover.

Paleta (`Cmd+K`): grupos Notas (ou 5 recentes), Memórias, Comandos; busca a cada tecla (uma por vez, a mais nova substitui); Enter antes do resultado fica na fila; comandos lentos (rebuild) só por seta.

**Ponto cego central:** `useLive` (eventos da IA dos últimos 30 s) só é consumido por `GraphPage` e `LivePanel`. Fora do grafo, a presença da IA se resume a (a) bolinha na barra lateral, que significa "este cliente gerou algum evento nos últimos 7 dias" (`connections.rs`), e (b) contador da inbox. Quem trabalha no editor não vê a IA agir.

## 2. Editor

- **Motor:** CodeMirror 6 + `@lezer/markdown` GFM + extensão própria (`editor/syntax.ts`) que espelha o scanner do núcleo: frontmatter (limite 256 KB), `[[alvo|rótulo]]` (com `\|` e alvo vazio ignorado), `#tag` Unicode. `core-fixture.json` fixa o comportamento do núcleo e um teste compara os dois.
- **Preview ao vivo (estilo Obsidian):** decorações inline escondem as marcas (`**`, `#`, `[]()`, crases, escapes) em todas as linhas menos a ativa, onde ficam esmaecidas. Renderiza títulos H1 a H6 (setext incluso), ênfase, riscado, código inline e em bloco (linguagens via `language-data`), citações, listas com bullet `•`, checkbox clicável para tarefas, régua horizontal, links (clique com `Cmd` abre no navegador), imagens inline (só de `notes/`, ≤16 MB, via comando `note_image`), hashtags clicáveis.
- **Widgets de bloco:** frontmatter vira uma linha "N linhas" que abre ao pôr o cursor; tabelas viram widget renderizado e voltam a texto mono editável com o cursor dentro. Durante IME/composição, as decorações param de mudar.
- **Não suporta (nenhum sinal no código):** math/LaTeX, Mermaid, callouts, embeds `![[nota]]`, notas transcluídas, modo leitura separado, canvas, plugins. Wikilink para `#seção` tem parsing, mas só abre na seção via `Arrival`.
- **Wikilinks:** `[[` abre autocomplete (até 30) por título, nome do arquivo e alias, fuzzy; escolher alias insere `alvo|alias`; opção extra "criar nota <texto>" quando não resolve. Link sem alvo aparece tracejado (`cm-wikilink--dangling`); `Cmd+Enter` cria a nota no disco. Clique abre (na linha ativa, clique só posiciona o cursor). `#` completa tags com contagem. Renomear nota/pasta reescreve links via `rename_note(updateLinks)` do núcleo.
- **Autosave (`workspace.ts`, fora do React):** grava 400 ms após parar de digitar, no máximo a cada 2 s em digitação contínua; `write_note` com `baseHash` (concorrência otimista); retry 1/2/5/10/30 s; indicador saved/dirty/saving/conflict/error; copia de rascunho em `localStorage` a cada 120 ms (sobrevive a crash ou Quit); salva ao fechar a janela (grace de 3 s) e ao sair da rota. Sessão por nota guarda o `EditorState`, então desfazer sobrevive a trocar de aba.
- **Mudança externa:** o Rust não usa fs-notify. `watcher.rs` faz `stat` (mtime+tamanho) só das notas abertas a cada 200 ms e emite `note-changed`. Nota limpa: recarrega e preserva o cursor. Nota suja: abre **conflito** com três saídas (manter a minha, usar a do disco, manter as duas em `Cópia de ...`); "minha versão" é congelada ao entrar no conflito e nada a substitui; diff destaca linhas alteradas. Arquivo sumido: restaurar ou descartar. Janela de graça de 1,5 s para editores que salvam via renomear (Vim). Pasta do brain some: pseudoevento `BRAIN_LOST`, escrita suspensa até voltar.
- **Backlog admitido:** o diff do conflito marca bloco, não linha a linha; digitar nos primeiros 120 ms antes de um crash perde texto; editor externo lento (>1,5 s) vira "removida"; suspeita de bug com CRLF.

## 3. Grafo

- **Tecnologia:** Sigma 4 (exige WebGL2) + graphology; páginas carregam isso sob demanda. Formas por tipo (círculo: nota, conceito, episódio; quadrado: procedure; losango: decision), preenchimento sólido = humano, vazado com anel roxo (cresyl) = IA, aresta tracejada = stale. Rótulos automáticos por densidade; raio encolhe com a densidade.
- **Física:** não é simulação contínua. ForceAtlas2 (graphology) roda em Web Worker: 150 iterações de uma vez (30 se mudaram <10% dos nós), em blocos de 30 que atualizam a tela, Barnes-Hut acima de 300 nós, partida determinística por hash do id. Depois disso o grafo fica parado. Nó novo nasce ao lado de um vizinho. Posições só em memória: reabrir o app rearranja. Sem arrastar nós (`enableNodeDrag: false`), sem rotação.
- **Limites:** 2.000 nós por consulta (`graphLimit`; mostra "truncado"); alvo de 30 fps com 2.000 nós; layout leva 2,8 a 5,2 s para assentar 2.000 nós no WKWebView (6x mais lento que Node); sem WebGL2 cai num SVG com os 300 nós mais ligados (e só volta ao Sigma saindo da rota); WebContent chega a ~970 MB após 30 aberturas do grafo de 2.000 nós.
- **O que mostra:** notas e memórias (4 tipos), arestas de links e relações; filtros por tipo, humano/IA e "inativas"; foco por vizinhança com 1 ou 2 saltos (o resto esmaece a 14%); modo lista acessível; legenda; contagem de nós/arestas.
- **Como "acende":** evento `read` traz `edges: [[de, para], ...]` (a vizinhança do item lido; sem ordem útil, então a origem é heurística). Uma camada Canvas 2D **por cima** do canvas do Sigma desenha arestas douradas: 240 ms para se desenhar, ficam enquanto a leitura é a atual (teto de 30 s) e somem em 600 ms. Nó lido fica dourado e 1,5x maior; rótulos dos extremos aparecem (até 12 arestas). Evento `search` recente desenha um chip "QUERY" ligado ao nó de origem. `prefers-reduced-motion` troca animação por cor. Meta: evento até aresta em 500 ms, medida pelo teste `graph-live`.
- **Fraco:** acende só 1 leitura por vez (em >10 leituras/s só a mais recente por quadro); nó fora do corte de 2.000 não acende; sem histórico, replay ou timeline; sem layout salvo nem nó fixável; sem clusters, caminhos nem busca dentro do grafo; `rm` externo só reflete no próximo evento; truque do canvas sobreposto existe porque o Sigma desenha tudo num canvas só.

## 4. Tempo real: app, servidor MCP e clientes

Não há socket entre app e MCP. Tudo passa pelo **disco**:

1. O servidor `edgemind mcp --brain <caminho>` (stdio, um processo por cliente, brain fixo) usa `edgemind-core` e grava cada ação como linha JSON em `events/AAAA-MM.jsonl` (`read`, `search`, `propose`, `approve`, `reject`, `correct`, `ingest`, `lint`, `index`, `note`), com `client`, `session`, `ids`, `edges`, `query`.
2. No app, `watcher.rs` roda uma thread por janela que chama `brain.tail_events(cursor)` a cada **200 ms** (cursor começa no fim do log) e faz `emit_to` do canal `edgemind://event` só para aquela janela. O front (`useBrainEvents`) empurra o evento para `useLive` (evento humano é ignorado; janela de 30 s) e invalida queries do TanStack por tipo. Rajadas são agrupadas: autosaves esperam 1,5 s, aprovações 250 ms.
3. Escritas entre processos são serializadas por `.edgemind.lock` (espera 30 s por padrão). `.index/manifest.json` tem `generation` que avisa outros processos que o brain mudou; `indexer.lock` escolhe qual servidor indexa; `.journal/` retoma operações interrompidas.
4. Aprovar, rejeitar e editar no app chamam o núcleo como ator `human`; a IA propõe para `inbox/` (arquivos), nunca escreve memória direto.
5. **HTTP** (`edgemind serve`): API REST em `127.0.0.1` com token Bearer, checagem de `Host` e CORS só localhost; SSE em `/events/stream` (ping 15 s, `Last-Event-ID`, até 500 reenvios, entrega ≤250 ms mesmo se outro processo gravou). Serve o `HttpClient` (navegador e QA) e uma página embutida com grafo e log. O app desktop **não** usa HTTP: usa comandos Tauri (`TauriClient`), mesma interface `EdgeMindClient`.
6. Latência na prática: até 200 ms de polling + redesenho. Dois clientes ao mesmo tempo funcionam (sessões separadas, mesmo log).

## 5. Onboarding, conexões e brains

- **Onboarding:** Welcome com nome (padrão), pasta sugerida `~/brains/<slug>`, idiomas fixos `pt-BR` + `en` (sem seletor), botão "escolher pasta", "abrir existente". Sem tour, sem importação de vault Obsidian, sem nota de exemplo. Primeira nota: estado vazio sugerindo "nota de hoje". Abertura de brain sem índice mostra progresso estimado (arquivos via `opening_plan` + tempo; a barra nunca chega ao fim sozinha) porque `Brain::open` não reporta andamento (3 a 8 s; meta de 2 s só com índice).
- **Conexão com clientes:** **manual.** A tela mostra `claude mcp add edgemind -- edgemind mcp --brain <caminho>` (Claude Code) e o JSON de `mcp.json` (`servers`) para Copilot, com botões de copiar. Exige o binário `edgemind` no PATH; o app diz que nunca altera a configuração do cliente. Status = existe evento do cliente nos últimos 7 dias. Para Copilot há exportação (skill em `~/.copilot/skills/edgemind`, instruções ou `AGENTS.md`, limite padrão 8.000 caracteres) que só escreve na pasta confirmada. Sem teste de conexão ativo, sem botão "instalar", sem detecção do cliente instalado.
- **Vários brains:** registro `brains.toml` na pasta de config guarda só caminho, nome e último uso (escrita atômica com trava; entrada identificada pelo caminho canônico porque cópia da pasta repete o id). Trocar de brain no seletor da barra lateral troca tudo naquela janela; `Cmd`+clique abre em outra janela (se já aberto, foca). Segunda instância do app foca a janela existente. `available` é calculado, pasta ausente aparece desabilitada.

## 6. Isolamento entre brains

- Fisicamente: um brain é uma pasta com `brain.toml` (id ULID), `notes/`, `memory/`, `inbox/`, `events/`, `.index/`. Sem estado global compartilhado entre brains.
- No app: `AppState` mantém mapa `label da janela -> Arc<Brain>`; toda chamada resolve o brain pela janela que chamou (`live_brain`) e recusa se a pasta sumiu (evita o núcleo recriar pastas). Duas janelas no mesmo brain compartilham um único `Arc` (`find_open`, comparando raiz canônica **e** id lido de `brain.toml`).
- Nas queries: toda chave do TanStack começa com `info.path` (caminho, não id), e ao trocar de brain os dados são removidos, a store de eventos zerada e os workspaces suspensos/salvos antes. Rascunhos no `localStorage` têm prefixo `brain + path`. Watcher emite só para a própria janela (teste `windows`: dois brains, dois processos, sem vazar evento).
- No MCP: o brain é argumento fixo do processo; nenhuma ferramenta recebe caminho de brain. `allowed_clients` em `brain.toml` (vazio = qualquer cliente) devolve `ClientNotAllowed`. Útil para trabalho x pessoal: cada um com seu servidor e seus clientes permitidos.
- **Fraquezas assumidas no backlog:** `allowed_clients` casa `clientInfo` por substring (`xclaudecodex` passa como `claude-code`) e cliente mentiroso não tem barreira; nome com espaço não casa; `Settings` não edita `allowed_clients`; sem criptografia nem permissão por brain no registro; o cuidado entre brains é de caminho e processo, não de sandbox.

## 7. Funcional mas sem graça, ou onde UI nativa ousada venceria

- **A IA é invisível fora do grafo.** Uma UI nativa pode mostrar, no editor, o cursor/halo da IA na nota que ela está lendo, e na árvore uma pulsação nas pastas e notas tocadas. Barra de status global com "Claude lendo 3 notas" clicável.
- **Grafo parado e acotovelado a 2.000 nós.** GPU nativa (wgpu) pode manter física contínua e viva, arrastar nós, partículas viajando pelas arestas, trilhas do caminho que a IA fez, replay temporal do log (`events/*.jsonl` já tem tudo), 100 mil nós. Layout em 3 a 5 s no worker vira instantâneo, e posições persistem.
- **Latência de 200 ms por polling + dupla camada (WebGL + canvas 2D).** Usar fs-notify/notify e canal local (socket/IPC) para evento em <20 ms, e um único pipeline de render.
- **Editor "quase Obsidian":** preview por decoração em cima de texto bruto; tabelas e frontmatter alternam entre widget e texto; sem math, Mermaid, callouts, embeds. Um editor nativo próprio (rope + layout GPU) pode oferecer rich text real, animação de cursor e seleção multi-região sem custo de DOM.
- **Conflito em três botões, diff em bloco.** Merge de três vias inline, com a edição da IA aplicada como diff vivo sobre o texto (aceitar/rejeitar trecho).
- **Inbox em lista de cartões em `<pre>`** sem ordenação, filtro, agrupamento por sessão/origem, diff contra memória existente nem "por que a IA propôs isso". Boa segurança (flags, caracteres invisíveis, segundo passo), pouca descoberta.
- **Memórias como cartões de texto.** Falta linha do tempo de validade, cadeia superseded, uso (quantas vezes a IA a leu) em gráfico, comparação lado a lado.
- **Conexão manual por copiar e colar**, status por idade do último evento, sem "testar agora". Um instalador com verificação ativa (handshake) seria um salto.
- **Poeira de WebView:** chunk de ~700 KB, ~1 GB de memória em estresse, 3 a 8 s de abertura sem índice com barra estimada, fontes de marca embutidas. O app nativo tem a chance de abrir em <300 ms.
- **Settings somente leitura**, sem editar `allowed_clients`, nome ou idiomas; Welcome sem escolha de idioma; sem importação de vault; sem tour; sem tema além de claro/escuro (identidade: roxo escuro, ouro para "IA agindo", cresyl para IA).

## 8. Lista mínima que um usuário diário sentiria falta

1. Editor Markdown com preview ao vivo e autosave que nunca perde texto (rascunho + crash + Quit + brain sumindo).
2. Conflito com edição externa: recarga limpa preservando cursor, e três saídas quando sujo.
3. Wikilinks com autocomplete por título/alias, "criar ao digitar", link pendente destacado, backlinks, renomear atualizando links, tags com contagem.
4. Árvore de pastas com abas, arrastar para mover, criar/renomear/apagar, nota do dia, voltar/avançar.
5. Paleta `Cmd+K` única para notas, memórias e comandos, com busca a cada tecla e teclado completo.
6. Inbox de propostas com aprovar/editar/rejeitar (+ motivo), lote, atalhos J/K/A/E/R, marcação de risco (instrução disfarçada, bloco `---`, caracteres invisíveis) e correções registradas.
7. Detalhe de memória com origem, estado (ativa, stale, expirada, superada), validade, arestas, confirmar, encerrar e editar.
8. Grafo ao vivo que acende quando a IA lê (aresta, nó, quem, quando) e log de eventos; filtros por tipo/autor; foco por vizinhança; modo lista.
9. Conexões: comando pronto para Claude Code e Copilot, status e exportação.
10. Vários brains isolados (trocar, abrir em outra janela, esquecer, pasta ausente tratada), `allowed_clients` por brain.
11. Health/lint com correção de um passo e reconstruir índice.
12. Claro/escuro, ajuda de atalhos (`Cmd+/`), acessibilidade (foco, leitor de tela, reduced motion), estados vazios e erros explicados em todas as telas.
