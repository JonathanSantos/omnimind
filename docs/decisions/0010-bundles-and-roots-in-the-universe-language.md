# 0010. Feixes e Raízes com a linguagem visual do Universo

**Status:** implementada em 2026-11-10, a pedido da pessoa, que achou o Universo mais bonito que as outras duas vistas. Complementa a 0006, a 0008 e a 0009.

## Contexto

O Universo agradou por quatro coisas que Feixes e Raízes não tinham: um fundo com poeira e névoa, corpos com halo, linhas finas em vez de massas, e o esqueleto das ligações na base com o resto guardado para o foco. Feixes desenhava as 946 ligações do cofre pessoal de uma vez, e os leques de cada projeto viravam blocos sólidos. Raízes tinha troncos grossos e ramos de textura com o mesmo peso dos que levam a itens.

## Decisão

- **Fundo comum.** As duas vistas ganham poeira (230 pontos fixos na cor neutra) e uma névoa suave por tema: ao longo do arco em Feixes, no meio dos itens em Raízes. É `Scene::backdrop`, que não é item e não se clica.
- **Item com halo.** Os pontos da base são desenhados duas vezes: maiores e suaves somando luz, depois o miolo nítido por cima. A nota fica com anel fino e miolo pequeno.
- **Linha com brilho.** A base é desenhada duas vezes: uma fita larga, sem borda, por baixo, e a linha nítida por cima. O brilho usa cobertura, como a base, e por isso para na cor do tema. Acima de 1.500 itens não há brilho.
- **Feixes desenha o esqueleto.** Cada item fica com as três ligações mais fortes e, no empate, as de quem está mais perto no círculo. As outras continuam na malha, depois da base, e acendem no foco e na leitura. A contagem na tela segue sendo o total.
- **Raízes mais fina.** A espessura dos galhos caiu cerca de um terço, os ramos que não levam a item nenhum ficam a 42% de opacidade, e o centro ganha uma luz neutra.
- **Paleta com oito entradas.** A oitava é a cor neutra da poeira.

## Alternativas

- **Brilho somando luz.** Num feixe com dezenas de ligações soma até o branco, o defeito que a 0007 tirou.
- **Pós-processamento com bloom de verdade.** Mais fiel, mas é outro passe de tela inteira e a 0006 tirou o bloom do produto. Redesenhar a mesma geometria larga custa menos e para onde a geometria para.
- **Manter todas as ligações em Feixes e só afinar.** Com cliques de tag, cada fio do leque carrega dezenas de ligações idênticas: afinar não desfaz o bloco.

## Medidas

`proof-graph`, sem janela, mesma máquina, antes e depois desta mudança:

| Feixes | antes, p50 | depois, p50 |
|---|---|---|
| 540 itens, 1.600 ligações | 2,95 ms | 3,55 ms |
| 10.000 itens, 30.000 ligações | 12,22 ms | 5,46 ms |

O ganho em volume grande vem do esqueleto: menos linhas na base.

## O que não foi verificado

- O visual novo na janela real: foi julgado por telas geradas sem janela, no escuro e no claro.
- A escolha de três ligações por item em Feixes é de gosto, não de medida.
