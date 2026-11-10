---
title: Reprocessar roteiros
aliases: [rerun plan, replanejar rotas, reprocess routes]
tags: [runbook, procedimento]
---
# Reprocessar roteiros

Quando usar: rota publicada com problema depois das 4h, como em [[INC-042 Rota por via interditada]].

1. Avise o Ricardo e pause o `despacho` com `rc-admin despacho pause`.
2. Invalide o cache: `rc-admin matriz flush --regiao guarulhos`.
3. Gere um novo plano só para veículos que ainda não saíram: `rc-admin plano replan --desde agora`.
4. Ricardo aprova o plano novo na tela de revisão.
5. Retome com `rc-admin despacho resume`.
6. Registre no canal de incidentes o número do plano antigo e do novo.

Não rode o passo 3 sem o passo 2: o plano sai igual ao anterior.
