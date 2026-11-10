# Pesquisa de mercado: memória para agentes, second brains e riscos

Data da pesquisa: 2026-10-31. Contexto: app local-first em Rust, com notas Markdown e memória automática para agentes via MCP, sem embeddings, sem nuvem e sem chave de API.

Convenção: cada achado traz URL e data. Quando a página não mostra data, aparece "acessado 2026-10-31". A marca **[fraco]** indica fonte secundária, agregadora ou de vendedor sem confirmação primária.

## 0. Metodologia e limites

- **Reddit não foi lido.** A ferramenta de busca recusou o domínio. Reclamações "reais" vêm de threads do Hacker News, issues do GitHub, fórum do Cursor e reviews citando usuários de G2 e Capterra.
- **A cota de busca web acabou** durante a pesquisa. Ficaram sem busca própria: hooks entre agentes, suporte a sampling por cliente, crítica a benchmarks, BM25 versus embeddings, Context7 e CoSnitch. Depois disso só li páginas de URL já conhecida.
- **Números de benchmark de vendedores são autodeclarados** (Mem0, Zep, Hindsight, Cognee). Hindsight afirma reprodução independente, mas a afirmação é dele.
- **A página primária da OpenAI sobre Dreaming retornou 403.** Os dados vêm de PCWorld e Thurrott.
- **Reviews de apps de notas são majoritariamente SEO ou afiliados.** Usei-os só para fatos de produto, e os trato como sinal fraco de sentimento.
- **Paper não é produção.** Quase todos os resultados de segurança são preprints, com taxas que dependem de modelo e configuração.

## 1. Memória para agentes

### 1.1 Frameworks

| Produto | Estado em 2026 | Por que importa | Fonte e data |
|---|---|---|---|
| **Mem0** | Algoritmo novo com extração ADD-only em uma chamada de LLM, sem UPDATE/DELETE. Busca funde semântica, BM25 e entidades. BM25 é só reforço: candidatos vêm da busca semântica. Grafo saiu do OSS e ficou só na plataforma. | O líder de mercado desistiu de reconciliar fatos na escrita. O motivo declarado é que update e delete perdiam contexto. | https://mem0.ai/blog/mem0-the-token-efficient-memory-algorithm (2026-04-16, atualizado 2026-09-28); https://docs.mem0.ai/migration/oss-v2-to-v3 (acessado 2026-10-31) |
| **Mem0, métricas** | Autodeclara LoCoMo 92,5 e LongMemEval 94,4, com cerca de 7 mil tokens por consulta. Exige LLM e, por padrão, embedder da OpenAI. Tem cerca de 68 mil estrelas. | Fraqueza declarada em 10M: raciocínio temporal, ordem de eventos e multi-sessão. | https://github.com/mem0ai/mem0 (acessado 2026-10-31) |
| **Graphiti / Zep** | Grafo bi-temporal: fatos superados são invalidados, não apagados. Backends Neo4j, FalkorDB e Neptune; Kuzu, o embarcado, foi depreciado. O servidor MCP é referência, sem autenticação. Exige LLM e embedder. Cerca de 32 mil estrelas. | Bi-temporal é o padrão mais rigoroso, mas pesado: nada embarcado restou. | https://github.com/getzep/graphiti (acessado 2026-10-31) |
| **Letta** | Context Repositories: memória em arquivos versionados por git. Subagentes usam worktrees e fazem merge. Reflexão em sleep-time e "defrag" com backup prévio, reorganizando em 15 a 25 arquivos. | Maior framework stateful migrou para arquivos e git. | https://www.letta.com/blog/context-repositories (2026-02-12) |
| **Letta, tese** | Publicou que memória não pode ser plugin e pertence ao harness. | Argumento direto contra um produto MCP externo. Ver seção 5.4. | https://www.letta.com/blog ("Why Memory Isn't a Plugin", abril de 2026) |
| **Letta, benchmark** | Agente com arquivos simples chegou a 74,0% no LoCoMo, contra 68,5% do melhor Mem0 da época. Letta também diz que esses benchmarks significam pouco. | Evidência a favor de "sem maquinário". É do próprio vendedor e anterior ao Mem0 novo. | https://www.letta.com/blog/benchmarking-ai-agent-memory (2025-08-12) |
| **Cognee** | 1.0 com quatro verbos: remember, recall, improve, forget. `improve` repondera a memória pelo uso real. Roda em Postgres único e ganhou SDK Rust para on-device. | Vocabulário da API convergindo; "melhora com o uso" virou verbo. | https://www.cognee.ai/newsroom/cognee-1-0-is-live (2026-07-29) |
| **Hindsight** | Retain, recall e reflect. Consolidação em segundo plano gera observações com evidência e contagem de provas, refinadas em vez de sobrescritas. "Mental models" são respostas prontas lidas sem LLM. Usa Postgres com pgvector, exige LLM, MIT. A página mostra cerca de 57 mil estrelas. | Modelo de observação com prova é o mais próximo de "cérebro que melhora com evidência". | https://github.com/vectorize-io/hindsight (acessado 2026-10-31); https://arxiv.org/abs/2512.12818 (dezembro de 2025) |
| **Basic Memory** | Markdown como verdade, índice SQLite local, MCP. Adicionou busca vetorial híbrida com FastEmbed. Nuvem opcional a partir de US$ 15/mês. AGPL, cerca de 4,4 mil estrelas. Ferramentas trazem anotações read-only e destrutiva. | Concorrente mais próximo. Note que ele mesmo adotou embeddings. | https://github.com/basicmachines-co/basic-memory (acessado 2026-10-31) |

### 1.2 Memória nativa dos agentes

**Claude Code** (https://code.claude.com/docs/en/memory, acessado 2026-10-31)

- CLAUDE.md é escrito por humanos. Auto memory é escrita por Claude, em `~/.claude/projects/<project>/memory/`, com índice `MEMORY.md` e um arquivo por memória.
- Limite: as primeiras 200 linhas ou 25 KB do índice carregam em cada sessão. Arquivos de tópico são lidos sob demanda.
- Tipos no frontmatter: `user`, `feedback`, `project`, `reference`. Um campo `modified` guarda a data da escrita.
- Regra de exclusão: não grava o que pode ser derivado do código ou do git.
- A documentação trata memória como contexto, não como configuração imposta. Para bloquear ação de fato, indica hook PreToolUse.

**Auto Dream do Claude Code**

- A busca textual por "dream" na documentação oficial de memória retornou zero ocorrências.
- O recurso existe no binário atrás de flag de servidor `tengu_onyx_plover`, com `minHours: 24` e `minSessions: 3` na conta do reporter, segundo análise de usuário do bundle na v2.1.228. O `autoDreamEnabled: true` é ignorado se a flag estiver fechada. https://github.com/anthropics/claude-code/issues/86209 (2026-08-12, fechada como not planned)
- Um usuário perdeu 23 arquivos em cerca de 24 horas, sem log nem confirmação, e só detectou por backup manual. https://github.com/anthropics/claude-code/issues/47959 (2026-04-14, fechada como not planned)
- Posts sobre `/dream` se contradizem. Não os trato como fato.

**Dreams, da Anthropic (Managed Agents)** (https://platform.claude.com/docs/en/managed-agents/dreams, research preview desde 2026-05-06, ver https://docs.anthropic.com/en/release-notes/overview)

- Recebe um memory store e de 1 a 100 sessões. Produz outro store com duplicatas fundidas, entradas vencidas ou contraditas substituídas e novos insights.
- **O store de entrada nunca é modificado.** O usuário revisa a saída e decide se usa ou descarta.
- Instruções guiam síntese, não editam linhas específicas. Leva de minutos a horas.

**OpenAI**

- **ChatGPT Dreaming V3:** memória em segundo plano com resumo legível e editável. Liberado a Plus e Pro nos EUA, com expansão anunciada. Substitui o saved memories, que a OpenAI considerou manual e sujeito a conflito. https://www.thurrott.com/a-i/337052/openai-is-improving-how-chatgpts-memory-works (2026-06-05); https://www.pcworld.com/article/3158111/chatgpt-new-dreaming-feature-makes-it-way-better-at-remembering-you.html (2026-06-05)
- Recall de 41,5% para 82,8% aparece só em fontes secundárias e com conflito. **[fraco]** https://tech-insider.org/chatgpt-dreaming-v3-memory-update-2026/ (acessado 2026-10-31)
- **Codex:** memórias locais em `~/.codex/memories/`, desligadas por padrão, geradas em segundo plano após a conversa ficar ociosa. Redige segredos. A documentação manda deixar regras obrigatórias em AGENTS.md. https://learn.chatgpt.com/docs/customization/memories?surface=app (acessado 2026-10-31)

**GitHub Copilot Memory**

- Cada memória é um fato com citações a trechos do código. Antes de usar, o agente verifica as citações. Se o código contradiz, grava versão corrigida; se confirma, renova. https://github.blog/ai-and-ml/github-copilot/building-an-agentic-memory-system-for-github-copilot/ (2026-01-15)
- Testes A/B do GitHub: merge de PR de 83% para 90%; feedback positivo em code review de 75% para 77%. Memórias falsas semeadas foram corrigidas pelos agentes.
- Expiração em 90 dias sem reforço. Voto positivo renova; **apenas recuperar não renova**. Memórias com segredos detectados são rejeitadas. Escopos: repositório e usuário. A documentação avisa que a verificação é julgamento do modelo, não garantia. https://docs.github.com/en/copilot/concepts/agents/copilot-memory (acessado 2026-10-31)

**Cursor**

- A interface de Memories saiu na 2.1.x. Funcionários indicaram exportar para regras `.mdc`. https://forum.cursor.com/t/memories-not-showing/143820 e https://forum.cursor.com/t/are-my-memories-gone/144057 (fim de 2025)
- A documentação atual de regras nem menciona memórias. https://cursor.com/docs/rules (acessado 2026-10-31)
- Automations teria um `MEMORIES.md` entre execuções. **[fraco]** https://blog.memoryplugin.com/cursor-memory/ (acessado 2026-10-31)

**Windsurf / Devin Desktop**

- Memórias automáticas ficam presas ao workspace; regras são a parte durável. https://docs.devin.ai/windsurf/plugins/cascade/memories.md (acessado 2026-10-31)
- O agente novo documenta regras em `.devin/rules/` e AGENTS.md, skills e hooks, sem citar memórias. https://docs.devin.ai/desktop/cascade/memories.md (acessado 2026-10-31)

**Google**

- **Gemini CLI Auto Memory (experimental):** minera sessões ociosas, gera `.patch` e `SKILL.md`, e deixa tudo numa caixa `/memory inbox`. **Nada se aplica sem aprovação**, e o agente não edita arquivos ativos. https://geminicli.com/docs/cli/auto-memory/ (atualizado 2026-05-13; a página avisa da troca por Antigravity CLI em 2026-06-18 em planos gratuitos)
- **App Gemini:** importa memórias e histórico de outros assistentes por prompt ou ZIP de até 5 GB. https://blog.google/innovation-and-ai/products/gemini-app/switch-to-gemini-app/ (acessado 2026-10-31)
- A Anthropic tem oferta parecida, e a thread teve 592 pontos no HN. https://claude.com/import-memory ; https://news.ycombinator.com/item?id=47204571 (2026-03-01)

### 1.3 Formatos abertos: o novo campo de batalha

- **OKF, do Google Cloud.** v0.1 em junho de 2026; v0.2 traz sinais de confiança porque agentes geram milhares de conceitos sem responsável. Campos: `sources[]`, `generated{by,at}`, `verified[{by,at}]`, `status` (`draft`, `stable`, `deprecated`), `stale_after`. Só `type` é obrigatório. https://cloud.google.com/blog/products/data-analytics/okf-v0-2-adds-trust-signals (2026-07-24)
- **Tiers do OKF** derivam de `verified`: unverified, machine-confirmed, human-reviewed. São consultivos. `superseded_by` **não existe** na v0.2. O registro `log.md` é uma lista datada, do mais novo para o mais antigo. https://github.com/GoogleCloudPlatform/open-knowledge-format (repositório canônico, acessado 2026-10-31)
- **Memoryfields**, de Cal Paterson: zip de Markdown com índice SQLite descartável. Defende busca semântica sobre caminhada em grafo. https://calpaterson.com/memoryfields.html (agosto de 2026); HN com 191 pontos: https://news.ycombinator.com/item?id=49508317 (2026-08-31)
- **OKF Agent Memory:** Go, BM25, MCP, git, com `code_refs` ligando conceito a arquivo. https://github.com/okf-memory/okf-agent-memory ; HN: https://news.ycombinator.com/item?id=49581240 (2026-09-05)
- **LLM Wiki, de Karpathy:** fontes imutáveis, wiki de propriedade da LLM e arquivo de schema. Operações ingest, query e lint, com `index.md` e `log.md`. O índice basta até cerca de 100 fontes, sem RAG por embeddings. https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f (2026-04-04)

### 1.4 Para onde converge

| Pergunta | Veredito | Evidência |
|---|---|---|
| **Bi-temporal?** | A *ideia* venceu: não apagar, marcar. O *grafo* não. Graphiti invalida, Mem0 acumula com ADD-only, Hindsight refina crenças, ChatGPT atualiza entradas com o tempo. Há um porém sério: marcar sem impor na leitura não funciona (seção 5.3). | Graphiti, Mem0, Hindsight, arXiv 2609.08258 |
| **Arquivos?** | Sim, entre agentes de código: Claude Code, Codex, Letta, Gemini CLI, Basic Memory, OKF, Karpathy. Mem0 removeu grafo do OSS e Graphiti perdeu o embarcado. Exceção: serviços hospedados seguem com bancos. | Seção 1.1 a 1.3 |
| **Consolidação / dream?** | Sim, é padrão: Anthropic Dreams, Codex, Letta, ChatGPT, Gemini CLI, Hindsight, Cognee. A separação em duas fases, captura rápida e consolidação lenta, é consenso; o paper Auto-Dreamer consolidou com banco cerca de 12 vezes menor. https://arxiv.org/abs/2605.20616 (2026-05-20) | Ver também arXiv 2608.01679: consolidar é onde mora o risco. |
| **Decay?** | Parcial e por reforço, não por relógio. Copilot exige voto para renovar. No HN, críticos dizem que tempo real pune quem sai de férias e que erro frequente não some por decay. https://news.ycombinator.com/item?id=47914367 (2026-04-26) | Copilot, HN |
| **Hooks?** | Sim, ao menos no Claude Code: SessionStart, UserPromptSubmit, PreCompact, Stop, SessionEnd. Devin Desktop também documenta hooks. Portabilidade entre agentes não foi verificada. | https://code.claude.com/docs/en/hooks (acessado 2026-10-31) |
| **Revisão humana?** | Divergência real. Gemini CLI e Dreams revisam antes; Claude Code e Copilot agem e deixam auditar depois. | Seção 1.2 |

## 2. Second brains e notas com IA

### 2.1 Por produto

| Produto | Elogios | Reclamações | Fonte e data |
|---|---|---|---|
| **Obsidian** | Arquivos Markdown já são amigáveis a agentes. CLI oficial saiu na 1.12.4. Virou o "IDE" do padrão de Karpathy. | Plugins de RAG frágeis. Não achei posição oficial sobre IA. | https://obsidian.md/changelog/2026-02-27-desktop-v1.12.4/ (2026-02-27); https://news.ycombinator.com/item?id=46826277 (2026-01-30) |
| **Reflect** | **reflect-open**: reescrita aberta em **Rust e Tauri**, Markdown como verdade, MIT, chave própria, CLI, `private: true`. Em beta, cerca de 1,6 mil estrelas; a página não cita MCP. | Reviews sem queixas substantivas em 2026; o app antigo depende de infraestrutura da empresa. | https://github.com/team-reflect/reflect-open (acessado 2026-10-31); **[fraco]** https://makerstack.co/reviews/reflect-review-2/ |
| **Mem** | Organização automática, Mem Chat com fontes. | Agrupa temas sem relação sem override fácil. Apps atrás do web, sync lento, suporte fraco, preço instável. | **[fraco]** https://nubiapage.com/mem-ai-review-2026-pricing-app-alternatives-free-plan/ (2026-09-17) |
| **Tana** | Supertags, transcrição de reunião, grafo unificado. | Curva de aprendizado, preço, mobile atrás, exportação que vira projeto. | **[fraco]** https://www.saner.ai/blogs/tana-reviews (2026-03-16) |
| **Capacities** | Objetos tipados. Assistente com ~27 ferramentas: leitura livre, **mudanças com prévia e aprovação**, desfazer disponível. | Sem modelos locais ou customizados. Conector MCP é beta. | https://docs.capacities.io/reference/ai-assistant (acessado 2026-10-31); **[fraco]** https://vantaige.io/ai-tool/capacities |
| **Notion AI** | Velocidade em tarefas repetitivas, Q&A interno; 4,6 no G2 para o produto geral. | Créditos: Custom Agents pagos desde 2026-05-04, a US$ 10 por mil créditos. Usuários reclamam de custo, de "IA sobre o ombro" e de banners. Exfiltração via imagem Markdown reportada e fechada como Not Applicable. | https://www.eesel.ai/blog/notion-ai-review (2026-05-07); https://news.ycombinator.com/item?id=46531565 (2026-01-07) |
| **Logseq** | Outliner, tags tipadas, DB rápido, licença aberta, CLI para agentes. | Beta 2.0 trocou Markdown por SQLite como verdade. A maior objeção do HN: perder arquivos, grep e git. Anos de bugs e estagnação, aviso de perda de dados. | https://kompozy.io/news/logseq-2-0-db-version-beta (2026-07-13); https://news.ycombinator.com/item?id=48896229 (2026-07-13) |
| **Anytype** | Privacidade, E2E, local-first, autohospedagem. | Curva do modelo de objetos, mobile, ecossistema pequeno. Review não cita IA. | **[fraco]** https://makerstack.co/reviews/anytype-review/ (2026-03-19) |
| **Heptabase** | Whiteboard, cartões, PDF, flashcards. | Sem plano gratuito, mobile básico, sem plugins, exportação limitada. | **[fraco]** https://makerstack.co/reviews/heptabase-review/ (2026-03-19) |

### 2.2 Novos entrantes "Markdown + agente"

- **OpenKnowledge** (GPL-3, ~4,8 mil estrelas). Perguntas no HN: migração, app só para macOS, preservação de frontmatter pelo editor, tamanho das skills no contexto, e por que não bastaria Obsidian. https://github.com/inkeep/open-knowledge ; https://news.ycombinator.com/item?id=48675435 (2026-06-25)
- **WUPHF** (wiki Markdown e git, licença Sustainable Use), elogiado por BM25 antes de vetores. https://github.com/nex-crm/wuphf ; https://news.ycombinator.com/item?id=47899844 (2026-04-25)
- **Rowboat**, **Atomic**, **Recall** e **Total Recall** mostram mercado lotado de ferramentas pequenas. https://news.ycombinator.com/item?id=48819808 (2026-07-07); https://news.ycombinator.com/item?id=47889110 (2026-04-24); https://news.ycombinator.com/item?id=48622590 (2026-06-21); https://github.com/davegoldblatt/total-recall (HN: 2026-02-05)

### 2.3 Sentimento real (HN)

Paráfrases, sem pesar como estatística:

- **Escrever é pensar.** Tomar nota é ler criticamente e encaixar no modelo mental; automatizar isso tira o valor. https://news.ycombinator.com/item?id=47899844 (2026-04-25)
- **Separar texto da IA.** Usuários temem não distinguir o que escreveram do que foi gerado. Elogiam IA como camada de consulta e pedem diretórios separados com rótulo. https://news.ycombinator.com/item?id=49450898 (agosto de 2026)
- **Colapso por reescrita.** Compor resumos sobre resumos degrada o conhecimento. https://news.ycombinator.com/item?id=47640875 (2026-04-04)
- **Uma linha envenenada estraga o resto.** Há quem prefira não usar memória nenhuma. Outro lembra que falta saber o que recuperar e quando. https://news.ycombinator.com/item?id=49508317 (2026-08-31)
- **Memória distrai.** Agente questiona o usuário com base em conversa de ontem. https://news.ycombinator.com/item?id=47914367 (2026-04-26)
- **Aprovação humana.** Um comentarista diz que toda memória automática falha com o tempo; outro quer poda gerada e confirmada. Há dúvida sobre sessões paralelas gravando o mesmo log. https://news.ycombinator.com/item?id=46907183 (2026-02-05)
- **Acumular era o problema.** Parte da thread diz que o gargalo é coletar demais, não organizar. https://news.ycombinator.com/item?id=46826277 (2026-01-30)

## 3. Contexto para código

- **AGENTS.md venceu como formato comum.** Foi doado à Agentic AI Foundation junto com MCP. https://openai.com/blog/agentic-ai-foundation (2025-12-09). Contagens variam de 40 mil a 60 mil projetos. **[fraco]**
- **Claude Code cedeu.** Lê AGENTS.md nativamente desde a v2.1.277, mas só se não houver CLAUDE.md, salvo configuração. https://code.claude.com/docs/en/memory ; HN com 741 pontos: https://news.ycombinator.com/item?id=49760187 (2026-09-18)
- **Cursor:** Project Rules `.mdc`, User Rules, Team Rules e AGENTS.md. https://cursor.com/docs/rules (acessado 2026-10-31)
- **Devin/Windsurf:** gatilhos `always_on`, `glob`, `model_decision`, `manual`; importa regras legadas. https://docs.devin.ai/desktop/cascade/memories.md (acessado 2026-10-31)
- **Agent Skills:** spec aberta desde 2025-12-18, com relatos de 16 a 40 plataformas e lacuna de portabilidade. **[fraco]** https://atlan.com/know/ai-agent/ai-agent-skills/what-are-agent-skills/ (julho de 2026)
- **Contexto passivo vence skill.** Evals da Vercel: skills 53%, com instrução explícita 79%, índice em AGENTS.md 100%. Em 56% dos casos a skill não foi invocada. https://vercel.com/blog/agents-md-outperforms-skills-in-our-agent-evals (2026-01-27)
- **Mais contexto não é melhor.** Arquivos de contexto geralmente não aumentam o sucesso e custam mais de 20% de inferência. Instruções são seguidas, mas visões gerais do repositório não ajudam; valem para práticas não padrão. https://arxiv.org/abs/2602.11988 (v1 2026-02-12, v3 2026-09-29). HN: quem adiciona só após falha observada relata melhor resultado. https://news.ycombinator.com/item?id=47034087 (2026-02-16)
- **Consenso dos vendedores:** memória auxilia, regras obrigam. Dizem isso GitHub, OpenAI, Anthropic e Devin.
- **Context7:** só achei relato não verificado de CVE-2026-75130, injeção via instruções customizadas até a 2.1.2. **[fraco, não verificado]** https://feed.craftedsignal.io/vendors/context7/feed.json (acessado 2026-10-31)

**MCP revisão 2026-07-28** (https://modelcontextprotocol.io/specification/2026-07-28/changelog)

- **Sampling foi depreciado**, com janela mínima de 12 meses. A migração sugerida é integrar direto com APIs de LLM, o que este produto não fará.
- Protocolo stateless, sem `initialize`; `server/discover` é obrigatório; requisições iniciadas pelo servidor viram MRTR; Roots e Logging também depreciados.
- `tools/list` deve ter ordem determinística, para ajudar cache de prompt.

**Limites práticos do Claude Code**

- `mcp_tool` hooks são pulados no SessionStart inicial, porque o MCP ainda não subiu. https://code.claude.com/docs/en/hooks (acessado 2026-10-31)
- `additionalContext` tem teto de 10 mil caracteres, sem configuração para ampliar.
- Texto imperativo pode acionar defesas de injeção; a documentação pede fatos.
- SessionEnd compartilha orçamento de 1,5 segundo.
- Saída de MCP avisa em 10 mil tokens, limite padrão de 25 mil, e texto acima de 50 mil caracteres vai para arquivo. https://code.claude.com/docs/en/mcp (acessado 2026-10-31)

## 4. Riscos de memória automática

### 4.1 Ataques

| Ataque | O que mostrou | Fonte e data |
|---|---|---|
| **SpAIware** | Injeção indireta grava instruções persistentes na memória do ChatGPT e exfiltra conversas futuras. | https://dl.acm.org/doi/10.1016/j.future.2025.107994 (divulgado em 2024; artigo publicado 2025/2026) |
| **ZombieAgent** | Persistência via memória mais Connectors no Deep Research. | https://www.darkreading.com/endpoint-security/chatgpt-memory-feature-prompt-injection **[fraco]** (janeiro de 2026) |
| **AI Recommendation Poisoning** | **No mundo real:** 50+ prompts de 31 empresas em 14 setores, via botões "Summarize with AI". Empresas legítimas, com ferramentas prontas. | https://www.microsoft.com/en-us/security/blog/2026/02/10/ai-recommendation-poisoning/ (2026-02-10) |
| **MemoryTrap** (Cisco) | Pacote npm sobrescreve MEMORY.md, hooks globais e shell. Anthropic removeu memórias do system prompt na v2.1.50 e disse que o principal usuário é confiável. | https://blogs.cisco.com/ai/identifying-and-remediating-a-persistent-memory-compromise-in-claude-code (2026-04-01) |
| **Trojan Hippo** | Payload dormente vindo de uma chamada de ferramenta; 85–100% de sucesso sem defesa; ativa após 100 sessões benignas. | https://arxiv.org/abs/2605.01970 (v1 2026-05-03, v4 2026-09-28) |
| **MemPoison** | Aceito na CCS 2026; até 0,95 de sucesso, sobrevive a extração e reescrita. Otimiza embeddings. | https://arxiv.org/abs/2605.29960 (2026-05-28) |
| **MemoryGraft** | Implanta "experiências de sucesso" imitadas pelo agente. Conteúdo benigno como README é vetor. A recuperação unia léxica e embedding. | https://arxiv.org/abs/2512.16962 (2025-12-18) |
| **MINJA, AgentPoison** | Injeção só por consultas; gatilhos otimizados. Citados como base da área, não reli os originais. | citados em https://arxiv.org/abs/2605.14421 e https://arxiv.org/abs/2512.16962 |
| **FARMA** | Forja o *raciocínio* lembrado e se autorreforça; até 100%, derrota filtros de palavras e A-MemGuard. | https://arxiv.org/abs/2607.05029 (2026-07-06) |
| **PMPA** | Em Claude Code: 66,9% de injeção e 81,7% de sucesso entre sessões. Defesa no prompt ajuda pouco após o envenenamento. | https://arxiv.org/abs/2609.13889 (2026-09-12) |
| **Bad Memory** | Em Claude Code e Codex, sobrescrever a própria memória via conteúdo externo foi difícil, mas payload já plantado funciona. Conflita em parte com PMPA; varia por configuração. | https://arxiv.org/abs/2607.14611 (2026-07-16) |
| **Rules File Backdoor** | Unicode invisível em regras leva o agente a inserir código malicioso; invisível em PR. Cursor e GitHub atribuíram responsabilidade ao usuário; GitHub depois avisou sobre Unicode oculto. | https://www.pillar.security/blog/new-vulnerability-in-github-copilot-and-cursor-how-hackers-can-weaponize-code-agents (2025-03-18) |
| **Scary Agent Skills** | Tags Unicode escondem backdoor em SKILL.md. Gemini, Claude e Grok interpretam. Claude Code passou a recusar em teste de 2026-02-10. | https://embracethered.com/blog/posts/2026/scary-agent-skills/ (2026-02-11) |
| **CoSnitch, Deadbugz, Miasma** | Memória do Copilot envenenada via resumo de página (CVE-2026-24301). MCP malicioso muda metadados após três chamadas. Worm usa configuração de ferramentas de IA. | https://genai.owasp.org/2026/10/08/genai-and-agentic-ai-exploit-roundup-q3-2026/ (2026-10-08) |

**Duas conclusões incômodas**

- Agentes que gravam e recuperam memória mais agressivamente são mais exploráveis, e defesas comuns de injeção cobrem mal esse caso. https://arxiv.org/abs/2606.04329 (2026-06-03). Isso bate de frente com "a IA grava sozinha".
- Abrir mão de embeddings não protege. MemoryGraft funcionou com recuperação que inclui a parte léxica, e a porta de entrada é a escrita, não a busca. Essa é uma inferência minha: o paper não testou BM25 isolado.

### 4.2 Defesas avaliadas (Trojan Hippo)

Números aproximados para Gemini 3.1 Pro, N=100, lidos de tabela achatada. https://arxiv.org/html/2605.01970v2

| Defesa | Sucesso do ataque | Custo |
|---|---|---|
| Gravar só turnos do usuário | 5–10% (5% em RAG) | Perde memória de saída do assistente e ferramentas |
| Não gravar após ler conteúdo não confiável na sessão | ~0–5% | Fluxos que dependem de conteúdo externo zeram |
| Limitar tamanho a 80 caracteres | até 85% | A mais fraca |
| Política IFC: sessão contaminada não envia | 0% | Quebra tarefas que leem e depois enviam |

### 4.3 Defesas em uso real

- **Citações e verificação no uso:** Copilot.
- **Tirar memória do system prompt:** Claude Code v2.1.50.
- **Saída de consolidação sem tocar na entrada:** Anthropic Dreams.
- **Caixa de aprovação e proibição de editar arquivos ativos:** Gemini CLI.
- **Rejeição ou redação de segredos:** Copilot e Codex.
- **Aviso de Unicode oculto:** github.com (2025-05-01).
- **Separação de conteúdo externo e controles visíveis de memória:** Microsoft.
- **Sandbox:** recomendação recorrente de Rehberger e Cisco.

### 4.4 Defesas ainda de pesquisa

- **MemLineage:** tratar como proveniência, não como filtro de conteúdo. Log assinado, grafo de derivação, taint e bloqueio de ação sensível; zerou o ataque em seu harness. https://arxiv.org/abs/2605.14421 (2026-05-14)
- **Rótulos de autoridade:** derrubaram ações não autorizadas de 16,9% para 0,0% sem perda de utilidade. https://arxiv.org/abs/2608.01679 (2026-08-03)
- **SENTINEL:** análise estrutural de entradas de raciocínio, sem falsos positivos em 326 traços. https://arxiv.org/abs/2607.05029 (2026-07-06)
- **CaMeL:** separa fluxo de controle e dados; dobra chamadas de modelo. Não achei adoção em produção. https://agentpatterns.ai/security/camel-control-data-flow-injection/ (acessado 2026-10-31) **[fraco]**

### 4.5 OWASP

- **Agentic Top 10 (2026):** ASI06 é Memory & Context Poisoning. Publicado em 2025-12-09. https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/ . Lista completa em https://www.promptfoo.dev/docs/red-team/owasp-agentic-ai
- **Texto oficial de mitigação do ASI06 não foi lido.** Resumos secundários citam segmentação, proveniência, expiração, escrita validada e rollback. **[fraco]** https://goteleport.com/blog/owasp-top-10-agentic-applications/
- **Coautor do ASI06:** memória é superfície de ataque, junto com hooks e configuração local. https://genai.owasp.org/2026/05/13/memory-is-a-feature-it-is-also-an-attack-surface/ (2026-05-13)
- **Cornucopia AAI3:** tratar histórico como não confiável, assinar ou hashear na escrita, distinguir histórico do usuário e verificado. https://cornucopia.owasp.org/edition/companion/AAI3/1.0/en (acessado 2026-10-31)
- **Agent Memory Guard:** projeto incubador, em Python, Apache-2.0. https://owasp.org/www-project-agent-memory-guard/ (acessado 2026-10-31)
- **LLM01:2025:** menor privilégio, aprovação humana, segregar e rotular conteúdo externo. https://genai.owasp.org/llmrisk/llm01-prompt-injection/ (2025)

## 5. O que ninguém resolveu bem

### 5.1 Esquecer de verdade

Benchmarks medem recall; falhas de produção seriam de esquecimento. O ForgetEval mostra três regimes:

- Primitivas determinísticas falham em canonicalização: 5% em identificadores ofuscados e **0% entre idiomas**.
- LLM na escrita corrige canonicalização (100%), mas zera em exclusão por intenção.
- LLM na mutação atinge 78–85% nisso e 91,7–93,2% no total, a cerca de 2,3 segundos por caso.

https://arxiv.org/abs/2606.15903 (2026-06-14)

### 5.2 Autoridade na consolidação

Colapso de autoridade em 48 de 49 configurações: o fato sobrevive, a restrição de quem pode usar some. Ações não autorizadas em 50,3%. https://arxiv.org/abs/2608.01679 (2026-08-03)

### 5.3 Revogação

Nenhum de cinco sistemas impõe revogação por padrão. O fato revogado aparece, supera o substituto e leva à ação insegura. https://arxiv.org/abs/2609.08258 (2026-09-08)

### 5.4 Outros problemas abertos

- **Consolidação sem auditoria.** Issue #47959 e reclamações de Cursor: memória que some sem aviso.
- **Decidir o que vale guardar.** Regras como a do Claude Code e os cinco portões do Total Recall são heurísticas; nada mede utilidade futura.
- **Provar que a memória ajuda.** Só o Copilot tem A/B público. Benchmarks são autodeclarados; Letta os relativiza e um comentário no HN chama o LoCoMo de fácil de burlar.
- **Degradação por resumos em cadeia.** Temor recorrente, sem medição boa.
- **Memória entre agentes.** Cada um grava no seu lugar. OKF e memoryfields tentam, nada ganhou.
- **Conflito entre sessões paralelas.** Letta usa worktrees; arquivos simples não têm resposta.
- **Memória fora do harness.** Letta argumenta que precisa estar dentro do loop. Ferramenta MCP depende do agente decidir chamar, e 56% de não invocação em skills na Vercel é um sinal de alerta.
- **Utilidade versus segurança.** Toda defesa forte do Trojan Hippo custa fluxo útil.

## 6. Contradições entre fontes

- **Expiração do Copilot:** 28 dias em post de terceiros, 90 dias na documentação atual. Valho-me da documentação.
- **Data do post do Copilot:** uma URL traz 2026-01-09; a página informa 2026-01-15.
- **Lançamento do Cognee 1.0:** junho, com memo datado de 2026-07-29.
- **Mem0 versus Zep:** ambos se declaram melhores; tratei como marketing.
- **"Graph memory" vive?** Removido do OSS do Mem0, mas Graphiti cresce. Grafo ficou como produto de serviço, não como armazenamento local.

## 7. Achado → o que muda no produto

| # | Achado | Decisão de produto | Fonte e data |
|---|---|---|---|
| 1 | MCP 2026-07-28 deprecia Sampling e sugere API direta | Servidor nunca chama LLM. Extração e consolidação são tarefas que o agente cliente executa via ferramentas. Servidor só guarda, indexa, valida e aplica. | https://modelcontextprotocol.io/specification/2026-07-28/changelog (revisão 2026-07-28) |
| 2 | Claude Code pula `mcp_tool` em SessionStart | Entregar binário CLI junto do servidor MCP, usado em hook `command`. O mesmo binário Rust atende CLI e MCP. | https://code.claude.com/docs/en/hooks (acessado 2026-10-31) |
| 3 | `additionalContext` limitado a 10 mil caracteres; MEMORY.md 200 linhas ou 25 KB; aviso MCP em 10 mil tokens | Orçamento rígido: briefing de sessão abaixo de 8 mil caracteres, e resultados MCP paginados abaixo de 10 mil tokens. | https://code.claude.com/docs/en/hooks ; https://code.claude.com/docs/en/memory ; https://code.claude.com/docs/en/mcp (acessados 2026-10-31) |
| 4 | Dreams nunca altera a entrada; #47959 perdeu 23 arquivos | Consolidação gera proposta como diff; nunca sobrescreve no lugar. Backup obrigatório antes de reorganizar. | https://platform.claude.com/docs/en/managed-agents/dreams (preview desde 2026-05-06); https://github.com/anthropics/claude-code/issues/47959 (2026-04-14) |
| 5 | Logseq trocou Markdown por SQLite e sofreu rejeição; Basic Memory e memoryfields usam índice descartável | Markdown é a verdade. SQLite é cache reconstruível. Teste de release: apagar o banco e reconstruir sem perda. | https://news.ycombinator.com/item?id=48896229 (2026-07-13); https://github.com/basicmachines-co/basic-memory (acessado 2026-10-31); https://calpaterson.com/memoryfields.html (agosto de 2026) |
| 6 | OKF v0.2 definiu proveniência, confiança e ciclo de vida | Frontmatter compatível com OKF: `sources`, `generated`, `verified`, `status`, `stale_after`. Tier derivado e exibido. `supersedes` como extensão, já que `superseded_by` não existe. | https://cloud.google.com/blog/products/data-analytics/okf-v0-2-adds-trust-signals (2026-07-24); https://github.com/GoogleCloudPlatform/open-knowledge-format (acessado 2026-10-31) |
| 7 | Revogação não é imposta na leitura em nenhum sistema testado | `deprecated` e substituídos ficam fora da busca padrão, em código e não por prompt. Histórico só com flag explícita. Teste com fato revogado e substituto. | https://arxiv.org/abs/2609.08258 (2026-09-08) |
| 8 | Colapso de autoridade em 48 de 49 configurações | Campos `scope` e `authority` (usuário, agente, externo) obrigatórios. Consolidação herda o nível mais baixo das fontes. Memória nunca concede permissão. | https://arxiv.org/abs/2608.01679 (2026-08-03) |
| 9 | Trojan Hippo e PMPA: payload entra por ferramenta e dorme | Toda memória ganha `origin`. Conteúdo externo entra em quarentena (`status: draft`), fora do briefing automático, até confirmação. Limite de tamanho não conta como defesa. | https://arxiv.org/abs/2605.01970 (v4 2026-09-28); https://arxiv.org/abs/2609.13889 (2026-09-12) |
| 10 | MPBench: escrita agressiva amplia ataque; MemLineage: proveniência supera filtro | Dois tiers: captura livre em log diário e promoção com portão. Taint se propaga na derivação: filho de externo continua externo. | https://arxiv.org/abs/2606.04329 (2026-06-03); https://arxiv.org/abs/2605.14421 (2026-05-14) |
| 11 | Tags Unicode, zero-width e variation selectors chegam ao modelo | Sanitizador em Rust na escrita e no indexador, inclusive para arquivos editados fora do app. Remover tags, sinalizar o resto, sem quebrar emoji. Comando `lint`. | https://embracethered.com/blog/posts/2026/scary-agent-skills/ (2026-02-11); https://www.pillar.security/blog/new-vulnerability-in-github-copilot-and-cursor-how-hackers-can-weaponize-code-agents (2025-03-18) |
| 12 | Copilot e Codex filtram segredos | Scanner determinístico de segredos no servidor, rejeitando a escrita. Não confiar na instrução ao modelo. | https://docs.github.com/en/copilot/concepts/agents/copilot-memory ; https://learn.chatgpt.com/docs/customization/memories?surface=app (acessados 2026-10-31) |
| 13 | Copilot renova por voto e não por recuperação; HN critica decay por relógio | Decay por reforço e por sessões de uso, nunca por data pura. Recuperar não renova; ser usado com sucesso, sim. Decay só ranqueia; arquivar é outra decisão. | https://docs.github.com/en/copilot/concepts/agents/copilot-memory (acessado 2026-10-31); https://news.ycombinator.com/item?id=47914367 (2026-04-26) |
| 14 | Citações do Copilot deram +7 pontos em merge | Campo `citations` com arquivo, linha e hash. Ferramenta de verificação devolve se a fonte mudou. Citação quebrada rebaixa a memória e cria tarefa. | https://github.blog/ai-and-ml/github-copilot/building-an-agentic-memory-system-for-github-copilot/ (2026-01-15) |
| 15 | Gemini CLI, Capacities e Total Recall têm aprovação; HN desconfia de automação total | Inbox de revisão no app com diff, aprovar e negar. Por tipo, o usuário escolhe automático ou revisado. Padrão automático só para `origin: usuário`. | https://geminicli.com/docs/cli/auto-memory/ (2026-05-13); https://docs.capacities.io/reference/ai-assistant (acessado 2026-10-31); https://news.ycombinator.com/item?id=46907183 (2026-02-05) |
| 16 | ForgetEval: sem LLM, 0% entre idiomas; LLM na escrita resolve | Sem embeddings, sim, mas o agente grava `aliases` e `keywords` em PT e EN no frontmatter. FTS5 indexa isso. A busca sinônima é pré-computada pelo cliente. | https://arxiv.org/abs/2606.15903 (2026-06-14). Aliases gravados pelo agente são inferência. |
| 17 | Letta 74% com arquivos; Karpathy até ~100 fontes; Mem0 usa BM25 e entidades; Basic Memory e reflect-open adotaram semântica | FTS5 com BM25, entidades, wikilinks e `index.md` por pasta. Medir antes de prometer. Risco declarado: recall semântico será a principal comparação com concorrentes. | https://www.letta.com/blog/benchmarking-ai-agent-memory (2025-08-12); https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f (2026-04-04); https://docs.mem0.ai/migration/oss-v2-to-v3 ; https://github.com/team-reflect/reflect-open (acessados 2026-10-31) |
| 18 | ForgetEval: LLM na mutação resolve exclusão por intenção | `forget` em duas etapas: servidor lista alvos, agente confirma IDs, servidor aplica. Soft delete com `log.md` append-only. | https://arxiv.org/abs/2606.15903 (2026-06-14) |
| 19 | Ataques em repositórios, hooks e worm usando configuração de IA (Cisco, Miasma) | Não executar nada vindo da memória. Nunca tocar settings ou hooks fora do instalador. Sem rede por padrão. Arquivos com permissão restrita. | https://blogs.cisco.com/ai/identifying-and-remediating-a-persistent-memory-compromise-in-claude-code (2026-04-01); https://genai.owasp.org/2026/10/08/genai-and-agentic-ai-exploit-roundup-q3-2026/ (2026-10-08) |
| 20 | Notion: exfiltração por imagem Markdown | Preview do app não carrega imagens remotas por padrão; links externos pedem confirmação. | https://news.ycombinator.com/item?id=46531565 (2026-01-07); relato do thread, artigo original não lido |
| 21 | Vercel 100% contra 53%; arXiv 2602.11988: contexto extra custa 20% sem ganho | Briefing curto e passivo por hook, com regras não óbvias e ponteiros, não resumo do repositório. Skill/ferramenta para o resto. Medir invocação. | https://vercel.com/blog/agents-md-outperforms-skills-in-our-agent-evals (2026-01-27); https://arxiv.org/abs/2602.11988 (v3 2026-09-29) |
| 22 | Todos os vendedores: memórias não são regras | Comando `export` promove memórias aprovadas a bloco gerenciado em AGENTS.md ou CLAUDE.md, com diff. Instalador adapta por cliente. | https://docs.github.com/en/copilot/concepts/agents/copilot-memory ; https://learn.chatgpt.com/docs/customization/memories?surface=app ; https://code.claude.com/docs/en/memory (acessados 2026-10-31) |
| 23 | HN: medo de misturar texto humano e de IA, e de modelo colapsando | Pastas e metadados distintos: notas humanas nunca são reescritas pela IA; IA propõe. Consolidação preserva ligação com fontes brutas imutáveis. | https://news.ycombinator.com/item?id=49450898 (agosto de 2026); https://news.ycombinator.com/item?id=47640875 (2026-04-04). Regra de separação é inferência. |
| 24 | Claude Code: texto imperativo aciona defesas | Memória guardada como fato, com linguagem descritiva. Lint alerta sobre imperativo e "ignore", sem ser barreira de segurança. | https://code.claude.com/docs/en/hooks (acessado 2026-10-31) |
| 25 | Gemini e Claude importam memória; OpenKnowledge cobrado por migração | Importadores: pasta Obsidian, MEMORY.md do Claude Code, `~/.codex/memories` e exportação OKF. Sem frontmatter destruído em round-trip. | https://blog.google/innovation-and-ai/products/gemini-app/switch-to-gemini-app/ (acessado 2026-10-31); https://news.ycombinator.com/item?id=47204571 (2026-03-01); https://news.ycombinator.com/item?id=48675435 (2026-06-25) |
| 26 | LoCoMo/LongMemEval criticados; ForgetEval e Trojan Hippo Bench são abertos | Suíte própria: esquecimento, revogação, autoridade e poisoning no CI. Não competir em LongMemEval como mensagem principal. | https://www.letta.com/blog/benchmarking-ai-agent-memory (2025-08-12); https://news.ycombinator.com/item?id=47914367 (2026-04-26); https://arxiv.org/abs/2606.15903 (2026-06-14); https://arxiv.org/abs/2605.01970 (v4 2026-09-28) |
| 27 | MCP sem estado e `server/discover` obrigatório; tools ordenadas | Suportar stdio com negociação antiga e nova. `tools/list` determinístico. Poucas ferramentas, com anotações read-only e destrutiva. | https://modelcontextprotocol.io/specification/2026-07-28/changelog (revisão 2026-07-28); https://github.com/basicmachines-co/basic-memory (acessado 2026-10-31) |
| 28 | Concorrentes usam Rust, Tauri e Markdown (reflect-open) e OKF | Diferencial não é "Markdown local". É memória auditável e segura: proveniência, revogação imposta, reversão. Mensagem do produto parte daí. | https://github.com/team-reflect/reflect-open ; https://github.com/okf-memory/okf-agent-memory (acessados 2026-10-31). Posicionamento é inferência. |
