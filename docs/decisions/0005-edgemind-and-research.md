# 0005. O que veio do EdgeMind e o que a pesquisa mudou

Síntese para quem chega agora. O detalhe está em `docs/research/`.

## Aproveitado do EdgeMind

| Ideia | Como ficou |
|---|---|
| Markdown como verdade, índice descartável | Igual, mas o índice virou RAM por processo |
| Tipos de memória, estados derivados, procedência | Igual, com autoridade e citação verificável |
| Cobertura contada pelo produto | Agenda só esvazia com hash lido |
| Correção como item de primeira classe | Capturada por hook, literal, antes de qualquer IA |
| Busca que diz o que cobriu | `coverage` em toda resposta |
| Analisador PT, pesos por campo, aliases | Mantidos |
| Conflito de edição, saída única | Mantidos, com gerador aleatório sobre transições |
| Brain fixo por servidor, sem tool com caminho | Igual |
| Dado, não instrução | Igual |

## Feito de outro jeito

| EdgeMind | Várzea | Por quê |
|---|---|---|
| IA propõe, pessoa aprova | IA grava, tudo é visível e reversível | Aposta central: menos trabalho |
| Inbox | Estratos com diff, motivo e desfazer | Revisar por exceção |
| Três tools, IA só escreve na inbox | Quatro, com portão determinístico | Sem humano antes, o portão é código |
| Tauri, WebView | Rust puro, GPU própria | Trava do projeto |
| tantivy com protocolo de geração | RAM por processo | Elimina índice compartilhado |
| Hooks em `sh` | Subcomandos do binário | Frágeis e sem aviso de corte |
| Grafo Sigma, sem persistência | Mapa em blocos, geografia estável | Momento de impacto |
| IA visível só no grafo | Barra global, margem da nota, cheia de fim de sessão | "Ninguém vê o que acontece" |
| Só escrita da pessoa corrige | Retrato editável | Corrigir é editar uma frase |

## O que a pesquisa mudou

| Achado | Mudança |
|---|---|
| MCP deprecou Sampling | Servidor nunca chama LLM; julgamento vira agenda |
| Copilot só tem tools | Nada essencial em resources ou prompts |
| Revogação não é imposta em nenhum sistema testado | Substituída sai da busca por código |
| Agentes que gravam mais são mais exploráveis | Origem, autoridade, quarentena, taint |
| Exposição não deve reforçar | Só uso útil pesa; decaimento só ordena |
| Consolidação não pode destruir | Fusão só de idêntico, sempre reversível |
| Contexto sem filtro é neutro ou negativo | Briefing curto, corte declarado, abster-se vale |
| Detectar melhora exige muitas tarefas | Métrica: correção repetida por sessão |
| Modelos convergem até para o "anti-roxo" | Marca derivada de uma metáfora, não de proibições |
| Nomes brasileiros sobrevivem, técnicos não | Várzea |
| Fontes do sistema custam 170 ms | Embutidas no início; sistema chega por thread |
| Latência 1 recomendada | Medida e rejeitada: latência 2 não perde quadro |

Onde a pesquisa recomendou e a medição discordou, venceu a medição: latência de swapchain e journal como verdade, ver `0003`.
