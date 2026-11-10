---
title: Deploy do otimizador
aliases: [deploy runbook, publicar otimizador]
tags: [runbook, procedimento]
---
# Deploy do otimizador

Deploys only between 10h and 16h, never on Friday, because the plan runs at 3h30 and nobody wants a surprise overnight.

1. Merge to `main` with green CI.
2. Tag the image `otimizador:YYYYMMDD-n`.
3. Canary: run the shadow plan with 25% of yesterday's orders and compare total km with production. Difference above 3% stops the deploy.
4. Promote with `rc-admin deploy otimizador --tag <tag>`.
5. Watch the 3h30 run the next morning. The on-call owns this.

Rollback: `rc-admin deploy otimizador --tag <previous>`.
