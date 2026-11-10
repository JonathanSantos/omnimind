---
id: 01M69V4H7VB2DF4D4F6V8YP1RV
type: decision
title: 'Mensageria: Outbox no Postgres + RabbitMQ (sem Kafka)'
summary: Serviços gravam mudança e evento na mesma transação (tabela `outbox`) e um publicador envia ao RabbitMQ; entrega pelo menos uma vez, então consumidores devem ser idempotentes por `pedido_id`. Kafka descartado.
aliases:
- outbox pattern
- mensageria
- event bus
- fila de eventos
- RabbitMQ
- idempotência
tags:
- projeto/rota-certa
edges: []
source:
  note: notes/decisoes/2026-10-27-outbox.md
  hash: b6a3dfd5
  session: 01M69V345Z9XT1ETEFPJADPQ25
author: claude-code
confirmed_by: human
confirmed_at: 2026-10-31
valid_from: 2026-10-31
valid_until: null
origin: null
created_at: 2026-10-31T20:41:36Z
updated_at: 2026-10-31T20:43:00Z
---

Decisão de 2026-10-27, status aceita.

- Padrão outbox: mudança + evento na mesma transação do Postgres; um publicador lê `outbox` e publica no RabbitMQ.
- Kafka descartado: volume de poucos milhares de mensagens/dia e time sem experiência para operar cluster.
- Consequência: entrega at-least-once; todo consumidor precisa ser idempotente usando `pedido_id` como chave.
