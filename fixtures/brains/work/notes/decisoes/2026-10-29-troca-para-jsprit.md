---
title: Troca do motor para jsprit
tags: [decisao, projeto/rota-certa]
---
# Troca do motor para jsprit

Data: 2026-10-29. Status: aceita. Substitui [[Motor de roteirização: VROOM]].

No teste com os dados reais da semana, a pausa obrigatória de 1 hora e as penalidades de [[Janelas de entrega]] não couberam bem no VROOM: precisamos de custo por minuto de atraso e de regra de carga por compartimento refrigerado. O jsprit aceita restrições próprias em código.

Trocamos para jsprit, rodando em um serviço Java separado chamado pelo `otimizador`. O tempo sobe de 2 para cerca de 9 minutos para 1.200 entregas, ainda dentro da meta de 15. Quem aprovou: Helena e Davi, na [[Reunião com operações, 28/10]] e confirmado no dia seguinte.
