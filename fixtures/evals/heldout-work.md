# Held-out: work

## Direta
- reprocessar roteiros rc-admin matriz flush -> Reprocessar roteiros publicados com problema
- canary 25% shadow plan -> Deploy do otimizador (janela e canary 25%)
- INC-042 Fernão Dias -> INC-042: rota por via interditada (29/10/2026)
- outbox RabbitMQ Kafka -> Mensageria: Outbox no Postgres + RabbitMQ (sem Kafka)
- jsprit VROOM -> Motor de roteirização do Rota Certa: jsprit (substitui VROOM)
- janela dura janela suave -> Janelas de entrega: dura vs. suave e pausa do motorista

## Morfologia
- por que substituímos o VROOM -> Motor de roteirização do Rota Certa: jsprit (substitui VROOM)
- janelas de entrega duras e suaves -> Janelas de entrega: dura vs. suave e pausa do motorista
- replanejando rotas já publicadas -> Reprocessar roteiros publicados com problema
- idempotência dos consumidores de eventos -> Mensageria: Outbox no Postgres + RabbitMQ (sem Kafka)
- entregas atrasadas por vias interditadas -> INC-042: rota por via interditada (29/10/2026)
- cacheamos a matriz com validade de quantos minutos hoje -> Cache da matriz de distâncias: Redis com TTL curto (15 min provisório)

## Paráfrase
- A rota saiu errada e já passou das quatro da manhã, o que eu faço passo a passo? -> Reprocessar roteiros publicados com problema
- Em que horários posso colocar uma versão nova do otimizador no ar e por que não em certos dias? -> Deploy do otimizador (janela e canary 25%)
- Que parte dos pedidos do dia anterior é usada para testar a versão nova antes de liberar de vez, e quando a diferença de quilometragem cancela tudo? -> Deploy do otimizador (janela e canary 25%)
- O que deu errado naquela manhã em que os caminhões pegaram a rodovia fechada? -> INC-042: rota por via interditada (29/10/2026)
- Como garantimos que um evento não se perde quando o serviço grava no banco e avisa os outros? -> Mensageria: Outbox no Postgres + RabbitMQ (sem Kafka)
- Por que desistimos da primeira ferramenta que escolhemos para montar as rotas dos veículos? -> Motor de roteirização do Rota Certa: jsprit (substitui VROOM)
- Quanto custa por minuto quando uma farmácia recebe a entrega atrasada e depois de quanto tempo conta como perdida? -> Janelas de entrega: dura vs. suave e pausa do motorista
- De quanto em quanto tempo os tempos de viagem guardados são renovados hoje? -> Cache da matriz de distâncias: Redis com TTL curto (15 min provisório)
- Por que baixamos de novo a validade dos tempos de viagem guardados no dia 31 e o que estamos esperando? -> Cache da matriz de distâncias: Redis com TTL curto (15 min provisório)
- Quem responde pela decisão de reduzir a validade do cache depois do incidente? -> Cache da matriz de distâncias: TTL 1h com invalidação por boletim
- Quem ficou de reduzir a validade do cache e quem ficou de assinar o feed de avisos de trânsito até o início de novembro? -> Cache da matriz de distâncias: TTL 1h com invalidação por boletim
- Depois do incidente da Fernão Dias, para quanto ficou a validade do cache e o que passou a limpá-lo na hora? -> Cache da matriz de distâncias: TTL 1h com invalidação por boletim

## Abstenção
- Quanto custa por mês o cluster de Redis em produção?
- Qual é o prazo de resposta exigido para incidentes de severidade 1?
- Como funciona o plano de capacidade do plantão?
- Qual foi o resultado do piloto em redução de quilômetros rodados?
- Como configuro o monitoramento e os alertas do RabbitMQ?
- Quantos paletes cabem em um caminhão toco?
