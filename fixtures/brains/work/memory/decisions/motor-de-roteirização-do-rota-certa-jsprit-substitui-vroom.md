---
id: 01M69V4ETCDMZE0ZAKSVDSFDZG
type: decision
title: 'Motor de roteirização do Rota Certa: jsprit (substitui VROOM)'
summary: 'Em 29/10 trocamos VROOM por jsprit, em serviço Java separado chamado pelo `otimizador`, porque o VROOM não modelou pausa de 1h, penalidade por atraso e carga refrigerada; tempo sobe de ~2 para ~9 min (meta: 15).'
aliases:
- troca VROOM jsprit
- motor de otimização
- routing engine
- solver de rotas
- VROOM replaced by jsprit
tags:
- projeto/rota-certa
edges: []
source:
  note: notes/decisoes/2026-10-29-troca-para-jsprit.md
  hash: e4d2d1a7
  session: 01M69V345Z9XT1ETEFPJADPQ25
author: claude-code
confirmed_by: human
confirmed_at: 2026-10-31
valid_from: 2026-10-31
valid_until: null
origin: null
created_at: 2026-10-31T20:41:33Z
updated_at: 2026-10-31T20:43:00Z
---

Decisão de 2026-10-29, aprovada por Helena e Davi (reunião com operações de 28/10, confirmada no dia seguinte).

Contexto: em 26/10 foi escolhido o VROOM (1.200 entregas em <2 min, integra com OSRM, API simples). Com dados reais, 40% das entregas de Guarulhos têm produto refrigerado (máx. 3h no veículo), e a pausa obrigatória de 1h e as penalidades de janela suave não couberam bem no VROOM.

Escolha: jsprit, que aceita restrições próprias em código (custo por minuto de atraso, regra de carga por compartimento refrigerado). Roda em serviço Java separado chamado pelo `otimizador`. Tempo: de ~2 para ~9 min para 1.200 entregas, dentro da meta de 15 min aceita por Helena.

A decisão do VROOM (26/10, responsável Davi) fica superada.

Código interno do piloto: ACME-WRK-9F3X.
