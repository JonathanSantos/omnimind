# Evals

## Direta
- jsprit -> Motor de roteirização do Rota Certa: jsprit (substitui VROOM)
- rabbitmq -> Mensageria: Outbox no Postgres + RabbitMQ (sem Kafka)
- INC-042 -> INC-042: rota por via interditada (29/10/2026)
- janela dura -> Janelas de entrega: dura vs. suave e pausa do motorista
- canary -> Deploy do otimizador (janela e canary 25%)

## Morfologia
- roteiros reprocessados -> Reprocessar roteiros publicados com problema
- entregas atrasadas -> INC-042: rota por via interditada (29/10/2026)
- janelas suaves -> Janelas de entrega: dura vs. suave e pausa do motorista
- motores de roteirização -> Motor de roteirização do Rota Certa: jsprit (substitui VROOM)

## Paráfrase
- por que caminhões foram parar em estrada fechada -> INC-042: rota por via interditada (29/10/2026)
- quanto tempo guardamos a tabela de tempos de viagem -> Cache da matriz de distâncias: Redis com TTL curto (15 min provisório)
- posso publicar o serviço na tarde de sexta -> Deploy do otimizador (janela e canary 25%)
- como os serviços trocam recados entre si -> Mensageria: Outbox no Postgres + RabbitMQ (sem Kafka)
- descanso obrigatório do motorista -> Janelas de entrega: dura vs. suave e pausa do motorista
- refazer o planejamento depois que o caminhão saiu errado -> Reprocessar roteiros publicados com problema
- qual software calcula os trajetos -> Motor de roteirização do Rota Certa: jsprit (substitui VROOM)
