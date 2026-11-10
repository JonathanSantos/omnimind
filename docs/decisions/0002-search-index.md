# 0002. Índice próprio em RAM por processo

**Status:** adotada em 2026-10-31.

## Contexto

Busca até 100 ms com 5.000 notas, app e vários servidores MCP rodando juntos, sem daemon, sem embeddings. O EdgeMind usou tantivy em disco, e cada falha séria de índice veio de estado compartilhado: brain vazio durante a recriação, varredura velha, dois indexadores, `fsync` de 60 ms por commit que estourou a meta (p95 de 111 ms na primeira busca após escrita).

## Alternativas

| Opção | Prós | Contras |
|---|---|---|
| SQLite FTS5 | maduro, WAL multi-processo | É C: fere "todo o produto em Rust". Sem stemmer PT, exigiria colunas pré-processadas. |
| tantivy | Rust puro, rápido | Segmentos em disco e escritor único exigem o protocolo de trava, geração e recuperação que o EdgeMind precisou construir. |
| Índice próprio em RAM | Sem estado compartilhado, atualização imediata, sem `fsync` | Mais código nosso; ocupa RAM em cada processo |

## Decisão

Cada processo mantém um índice invertido em RAM. O arquivo em disco é só um instantâneo gravado por `rename` atômico, que qualquer processo pode descartar. Antes de consultar, o processo reconcilia com `stat`.

## Por quê

O índice deixa de ser estado compartilhado. Isso elimina por construção: leitor esperando escritor, índice meio construído e geração velha. Para 5.000 notas de 3 KB o volume cabe em RAM com folga na trava de 150 MB, e BM25 em postings na memória fica muito abaixo de 100 ms.

Medido: varredura `stat` de 5.000 arquivos em 25 a 40 ms sob load 25, com pico de 60 ms. A cada 250 ms no máximo, por consulta, cabe no orçamento.

## O que reabre esta decisão

Memória do processo MCP acima de 150 MB em brains reais, ou brains muito maiores que 5.000 notas.
