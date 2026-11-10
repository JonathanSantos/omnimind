---
title: Cache da matriz de distâncias
aliases: [distance matrix cache, cache de tempos de viagem]
tags: [decisao, projeto/rota-certa]
---
# Cache da matriz de distâncias

Data: 2026-10-28, revista em 2026-10-30.

Decisão inicial: cache da matriz no Redis com TTL de 6 horas, porque calcular a matriz completa leva 4 minutos.

Revisão depois do [[INC-042 Rota por via interditada]]: TTL reduzido para 1 hora, e o cache é invalidado na hora quando chega um boletim de interdição da central de trânsito. O custo extra é de cerca de 2 minutos por rodada, aceito pelo time.

Revisão de 2026-10-31: o boletim de interdição ainda não chega automaticamente, então o TTL baixou para 15 minutos até o feed entrar no ar. A invalidação por boletim fica suspensa.
