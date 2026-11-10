# 0004. Mapa em blocos, com estilos trocáveis

Substituída por [0006](0006-redesign-omnimind.md).

**Status:** adotada em 2026-10-31, por escolha direta da pessoa.

## Caminho

1. Bolinhas e linhas: descartado no primeiro teste.
2. Cianótipo: terreno com curvas de nível. Tecnicamente forte, mas reprovado na avaliação: "meio monocromático".
3. Relevo 3D em cores de atlas: a pessoa propôs terreno em que mais memórias significam mais altura, com rios. A hipótese foi validada e feita como PoC.
4. Blocos: a pessoa pediu algo como Minecraft, "com quadrados, minimalista e bonito ao mesmo tempo". Virou o padrão.

## Decisão

Blocos é o padrão. Cianótipo e Relevo 3D ficam como estilos registrados, e o mapa não conhece nenhum deles. A pessoa pediu explicitamente para continuar podendo trocar: ver `docs/specs/mvp/map-styles.md`.

## Medição

Janela real, 2880×1800, M2, com 134 nós e load de 8 a 13:

| Estilo | Intervalo p50 | p99 | Acima de 20 ms em 300 |
|---|---|---|---|
| Blocos | 16,67 ms | 17,32 ms | 0 |
| Relevo 3D | 16,67 ms | 17,12 ms | 1 |
| Cianótipo | 16,67 ms | 17,05 ms | 1 |

O custo não decide. A decisão é de gosto, e é da pessoa.

## Efeito colateral bom

A chapa de blocos mostra faixas de solo na lateral. É a imagem dos Estratos, a tela que substitui a inbox: o mapa e o histórico viram a mesma ideia.
