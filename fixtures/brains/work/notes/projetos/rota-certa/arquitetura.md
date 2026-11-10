---
title: Arquitetura do Rota Certa
aliases: [architecture, desenho da solução]
tags: [projeto/rota-certa, arquitetura]
---
# Arquitetura do Rota Certa

Três serviços:

- `pedidos-ingest`: recebe pedidos do ERP e normaliza endereços com geocodificação.
- `otimizador`: monta a matriz de distâncias com OSRM sobre o mapa local e chama o motor de roteirização (ver [[Motor de roteirização: VROOM]] e [[Troca do motor para jsprit]]).
- `despacho`: publica as rotas para o app do motorista.

Banco: PostgreSQL com PostGIS. Comunicação entre serviços: outbox no Postgres e RabbitMQ, ver [[Outbox com RabbitMQ]].

O plano roda às 3h30 e precisa estar pronto às 4h, quando o CD começa a carregar.
