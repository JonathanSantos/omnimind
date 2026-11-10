# Cofre de design: mesmo pedido, com e sem memória

Teste de 2026-11-01. O Sonnet 5.5 recebeu o mesmo texto e o mesmo print da tela Métricas para recriá-la em HTML, com um bloco novo, "Sem precisar repetir". O grupo com cofre teve o MCP, o resumo inicial e uma frase pedindo para consultá-lo. Pedido em `qa/ai-sessions/design-brain.py`, notas em `vault/`.

| | Com cofre | Sem cofre |
|---|---|---|
| Print | `with-vault.png` | `without-vault.png` |
| Cores do sistema | 15 de 15 | 0 de 12 |
| Fontes | Familjen Grotesk, Sometype Mono | Schibsted Grotesk, JetBrains Mono |
| Cabe em 1440 por 900 | Sim | Não: corta a última linha |
| Dado inventado | Quarta frase repetida em 30 dias | Comparação com período anterior e datas de abril e maio |
| Bloco novo | Painel à direita do gráfico | Abre a tela, com marcas de contagem |
| Seletor de período | Funciona | Funciona |
| Custo | US$ 0,22 | US$ 0,34 |
| Tempo | 122 s | 224 s |
| Chamadas de ferramenta | 14, sendo 8 perdidas | Nenhuma |

Com cofre, o resultado é mais fiel ao produto. Sem cofre, a composição é mais marcante: responde primeiro à pergunta da tela e tem uma ideia própria, as marcas de contagem. O cofre garantiu consistência, não criatividade. É uma execução por grupo, então custo e tempo não provam nada: a diferença pode vir do raciocínio do modelo em cada rodada.

Os prints foram capturados com movimento reduzido, porque a primeira captura pegou a animação pela metade.

## Criar um cofre

Criar é simples: `init`, Markdown em `notes/` e `connect`. Só a CLI foi testada; a tela de boas-vindas do app, não. Atritos, do maior ao menor:

1. O cofre não se apresenta: o resumo inicial não traz nome, assunto nem lista de notas. Sem `remember`, fica vazio.
2. Nota por nome curto dá `not_found`: `tokens-omnimind` falhou e custou 8 de 14 chamadas.
3. `remember` grava "Prefiro..." como conceito sem `--type`, e o título vira a frase inteira.
4. Todo comando pede `--brain`, e `remember --help` falha fora do cofre.
5. Nota de referência fica pendente para sempre na agenda: 0 de 8 cobertas.
6. Não há modelo inicial por finalidade.
7. `connect` usa uma chave só em `.mcp.json`, então um projeto não fica com dois cofres ao mesmo tempo.

## Segunda rodada: sem print

Depois das correções de criação do cofre, o mesmo teste com Grafo e tela inicial. Ninguém recebeu print, e o grupo com cofre não teve a frase pedindo para consultar. O cofre ganhou as notas `sistema/grafo` e `sistema/tela-inicial`, e a regra de reservar uma ideia própria em cada tela nova. Arquivos em `no-print/`.

| | Grafo com cofre | Grafo sem cofre | Início com cofre | Início sem cofre |
|---|---|---|---|---|
| Cores do sistema | 17 de 17 | 0 de 22 | 14 de 14 | 0 de 11 |
| Fontes do sistema | Sim | Não | Sim | Não |
| Alterna Feixes e Raízes | Sim | Sim | | |
| Custo | US$ 0,48 | US$ 0,45 | US$ 0,12 | US$ 0,21 |
| Tempo | 316 s | 316 s | 78 s | 165 s |
| Notas lidas | 6 | | 5 | |
| Linhas (pedido: até 240) | 279 | 235 | 180 | 172 |

O cofre se apresentou: sem nenhuma instrução, o Sonnet abriu 11 notas pelo nome curto, sem chamada perdida. Na rodada anterior, 8 de 14 chamadas tinham se perdido.

Sem cofre, a tela inicial usa três sinais listados no cofre: rótulo acima do título, serifa itálica e numeração 01, 02, 03. Ainda assim tem a melhor ideia da rodada: anéis de crescimento em que cada anel é uma sessão. O Raízes sem cofre também é o desenho mais bonito. Com cofre, nenhum dos sinais aparece, mas o Raízes é mais irregular e a árvore da tela inicial é mais crua. O ícone do Grafo com cofre não é o Feixe: a nota da tela inicial descreve o ícone, a do Grafo não. Foi uma execução por tela e grupo.
