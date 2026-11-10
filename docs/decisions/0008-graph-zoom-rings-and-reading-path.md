# 0008. Feixes e Raízes: zoom, anel para nota, luz entre leituras e ligações no foco

**Status:** implementada em 2026-11-10, a pedido da pessoa, depois de ver o Universo. Complementa a 0006 e a 0007.

## Contexto

O Universo trouxe quatro coisas que Feixes e Raízes não tinham: dá para chegar perto, a forma diz se o corpo é nota ou memória, a leitura da IA vira um percurso entre itens, e o foco mostra todas as ligações. Com o cofre pessoal, Raízes ainda escondia as 1.257 ligações: a árvore só mostra tema e uso.

## Decisão

- **Zoom e arrasto.** Rolar ou pinçar aproxima até 8 vezes, com o ponto sob o mouse parado. Arrastar move o desenho, só quando ele está maior que o quadro. Dois cliques voltam ao desenho inteiro. Trocar de forma ou de filtro também volta. Isso revoga o "sem arrastar, sem zoom" da 0006.
- **Espessura cresce menos que a posição.** No zoom, linhas e pontos crescem pela raiz da aproximação: chegar perto separa os itens em vez de engordar tudo.
- **Nota é anel, memória é disco**, nas duas formas. Abaixo de 9 pixels o anel vira disco, porque o furo não se lê.
- **Luz entre leituras.** Duas leituras em até 25 s formam um percurso: a luz sai do item anterior por um arco, leva 0,8 s e só então o caminho do item acende.
- **Ligações no foco, em Raízes.** Mouse sobre um item, ou item selecionado, desenha até 32 arcos até os vizinhos.
- **Arcos soltos.** `Scene::arc` gera a geometria em `crates/graph`, fora da malha da base. A tela sobe esses arcos num buffer próprio e reaproveita o pipeline de destaque, com a mesma revelação por `along`.

## Alternativas

- **Luz reta entre os itens.** Corta o desenho por cima de tudo. O arco cede para o centro, como as ligações de Feixes.
- **Desenhar as ligações na base de Raízes.** Vira novelo sobre a árvore e apaga o que ela mostra bem.

## O que não foi verificado

- Zoom e arrasto na janela real: os testes dirigem o app sem janela, com os mesmos eventos.
- Pinça no trackpad: o evento chega ao zoom, mas nunca foi exercitado numa janela.
