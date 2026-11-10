# Travas, medidas

MacBook Air M2, 8 núcleos, 24 GB, tela 2560×1664. Cofre fictício de 5.000 notas e 800 memórias, gerado por `make-brain`. Janela real, com `-NSAppSleepDisabled YES`, depois da renomeação e do redesenho.

| Medida, com 5.000 notas | Limite | Medido | Como |
|---|---|---|---|
| App instalado | 100 MB | **12 MB** | `du -sh dist/Omnimind.app` |
| Abrir o app | 2 s | **0,59 s** até o primeiro quadro; instantâneo do cofre entre 516 e 890 ms, fora do quadro | `OMNIMIND_TRACE_START=1`, `--measure` |
| Memória do app em uso | 500 MB | **143 MB** | `ps -o rss`, no Grafo em repouso |
| Componente ligado à IA, em repouso | 150 MB | **102,6 MB** | `bench` |
| Busca | 100 ms | **p50 26,9 ms, p95 32,8 ms**, com varredura; p50 3,5 ms em rajada | `bench` |

Nenhum serviço fica ligado com o app fechado: o servidor MCP nasce e morre com a sessão.

## Outros números

| Medida | Resultado |
|---|---|
| Quadros em repouso | 0 em 4 s, janela real |
| Janela real, 300 quadros | Intervalo p50 16,67 ms, p99 17,05 ms, máximo 17,44 ms; 0 acima de 20 ms |
| Escrita de memória | p50 30,1 ms, p95 32,2 ms |
| Primeira busca após escrita | Mesmo processo p95 0,1 ms; outro processo p95 25,4 ms |
| Hook `session-start` | p95 32,2 ms, com 560 memórias |
| Hook `prompt` | p95 2,8 ms |

## Grafo

`cargo run --release -p app --bin proof-graph`. Sem janela, 1440×900 a 2x, com leituras chegando a cada 15 quadros. Os 30 primeiros quadros ficam de fora, porque criam pipelines e sobem a geometria. O tempo de quadro inclui a espera da GPU. O orçamento a 60 Hz é 16,67 ms.

| Itens / ligações | Feixes p50 | Feixes p99 | Raízes p50 | Raízes p99 |
|---|---|---|---|---|
| 540 / 1.600 | 2,53 ms | 5,22 ms | 2,08 ms | 4,96 ms |
| 2.000 / 6.000 | 4,37 ms | 7,28 ms | 2,26 ms | 5,24 ms |
| 4.000 / 12.000 | 9,14 ms | 12,69 ms | 3,00 ms | 6,64 ms |
| 10.000 / 30.000, a 1,5x | 15,51 ms | **21,10 ms** | 3,13 ms | 6,80 ms |

Feixes com 10.000 itens passa dos 16,7 ms em p99 (21,1 ms), mesmo com a resolução reduzida a 1,5x. É o único caso acima. Os outros cabem com folga.

## As quatro provas

| Prova | Estado depois do redesenho |
|---|---|
| Editor, preview ao vivo, 50 mil caracteres | **Passou:** 880 teclas, CPU p50 0,64 ms, p99 0,95 ms; com GPU p99 7,49 ms. Rolagem p50 1,08 ms, p99 7,09 ms |
| Acentos em Brazilian Pro, por eventos reais | **Passou** no `.app` final: `coração ação é ótimo`, com `--probe-ime` |
| Grafo com 2.000 nós | **Passou com `proof-graph`:** Feixes p99 7,28 ms, Raízes p99 5,24 ms. É medição sem janela. Rótulos agora são nomes de tema; nome de item só na ficha |
| Efeitos com a interface completa | **Não se aplica mais.** A prova antiga media bloom e a animação de fim de sessão, que saíram. O que sobra de efeito é o grafo, coberto pela prova acima com leituras chegando, e pela medição de 300 quadros na janela real |

## Testes

`cargo test --workspace`, depois do redesenho:

| Suíte | Resultado |
|---|---|
| Espaço de trabalho inteiro | 217 passam, 0 falham, 1 ignorado (`bench`, que roda à parte) |
| Segurança de texto | 40 sementes no padrão, 60 transições cada, mais troca de nota com texto pendente e em conflito |
| Quit real por `terminate:` | Passou no `.app` final, com `--probe-quit` |

`snapshot_open_is_much_faster_than_rebuild` compara dois tempos e falhou 1 vez em 6 rodadas seguidas da suíte, com a máquina carregada. O teste já existia e não mede nada que esta troca alterou.

## Revisão independente

Um revisor sem contexto da implementação encontrou 15 defeitos depois do redesenho, 3 graves: aprovar memória vendo texto velho, painel sem rolagem e nota substituída sem gravar. Todos foram corrigidos, com testes para os graves, a migração, o `connect`, as sessões e os itens sobrepostos. A revisão não foi repetida depois das correções.

## Não refeito depois da renomeação

Números anteriores, guardados como referência. Nenhum vale como medição atual.

| Medida | Último resultado |
|---|---|
| Segurança de texto com 150 sementes | Passou |
| QA independente `qa/repros/repro_top.py` | 13 achados, 0 reproduzem |
| Portão hostil, `t1d_bypass.py` | 5 de 108 variações ainda passam: é triagem, não garantia |
| Sessões reais de IA | Ver `ai-sessions.md` |

Recall@5 sem bater nas perguntas vistas, também da rodada anterior: direta 1,0; morfologia 1,0; paráfrase 0,875; abstenção 0,917. Paráfrase depende dos aliases que a IA grava.
