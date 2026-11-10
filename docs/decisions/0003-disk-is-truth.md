# 0003. Markdown é a verdade; o diário é a história

**Status:** adotada em 2026-10-31.

## Contexto

`docs/research/technical.md` recomendou journal append-only como verdade das memórias, por causa dos problemas de sincronização bidirecional do Beads. `docs/research/market.md` mostrou a rejeição ao Logseq quando trocou Markdown por banco.

## Decisão

Memórias e notas são arquivos `.md` e valem por si. O diário guarda eventos com o texto antes e depois, para auditar, desfazer e reproduzir a história. Divergi da recomendação do journal como verdade.

## Por quê

- A pessoa precisa poder ler, editar e versionar a memória com qualquer ferramenta. Com journal como verdade, cada edição à mão precisaria ser reimportada, e os dois lados poderiam discordar.
- Direção única: disco → evento. Mudança externa vira evento `external`. Nunca se reconstrói arquivo a partir do diário sem pedido explícito, o que evita o problema do Beads.
- Apagar `.varzea/` perde história e índice, nunca conhecimento.

## Custo

Sem diário, some o desfazer. Por isso o diário fica fora do git por padrão, mas é incluído em exportação e backup.
