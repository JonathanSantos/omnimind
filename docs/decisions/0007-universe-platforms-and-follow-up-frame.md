# 0007. Universo, Windows e Linux, e o quadro seguinte

**Status:** implementada em 2026-11-10, a pedido da pessoa. Complementa a 0006: Feixes e Raízes continuam como estão.

## Contexto

Três coisas vieram do uso real, com o cofre pessoal (63 notas, 114 memórias, 1.257 ligações):

1. **A tela ficava velha depois de um clique.** A interface é imediata: o clique é tratado enquanto o quadro é pintado. A barra lateral é pintada depois da página, então o quadro que tratava o clique ainda mostrava a página antiga, e ninguém pedia o quadro seguinte. Com toque de trackpad, sem o ponteiro andar, a tela só mudava no próximo evento.
2. **Feixes estourava para branco.** Tags de projeto ligam todo mundo com todo mundo: 30 memórias com a mesma tag são 435 ligações no mesmo feixe. Somando luz, o feixe passava da cor do tema e virava branco no escuro e preto no claro.
3. A pessoa pediu uma vista em três dimensões e pacotes para Windows e Linux.

## Decisão

- **Quadro seguinte.** Toda entrada (clique, tecla, rolagem, mensagem da thread de dados) deixa quadros devidos em `App::follow`. O app continua sem gastar quadro parado: a dívida zera e o laço volta a dormir. Superfície perdida, troca de escala e janela que volta a aparecer também pedem quadro.
- **Base do grafo cobre, destaque soma.** As linhas e os pontos da base usam mistura por cobertura: empilhadas, chegam à cor do tema e param ali. Leitura, foco e nascimento continuam somando luz (ou tirando, no claro), que é o que os faz acender.
- **Universo**, terceira forma do grafo:
  - Geometria e câmera em `crates/graph/src/cosmos.rs`, puras e testadas. Tela, GPU e animação em `crates/app/src/graphview/universe*.rs`.
  - Cada tema é um aglomerado. Temas com muitas ligações entre si ficam vizinhos, e dentro do tema quem é ligado fica perto (molas com repouso, até 1.500 itens; acima disso, só os cantos por subtema).
  - Memória é estrela, nota é planeta com anel. A forma diz o tipo: a identidade não depende só de cor.
  - As linhas da base são um esqueleto: até três ligações por corpo e três pontes por par de temas. O foco do mouse e a leitura mostram todas as ligações do corpo.
  - Leitura ao vivo: uma luz sai do último corpo lido, viaja por um arco até o próximo, e as ligações dele acendem.
  - Arrastar gira, rolar ou pinçar aproxima. Isso revoga, só para o Universo, o "sem arrastar, sem zoom" da 0006.
  - O céu gira sozinho apenas na abertura e enquanto a IA lê. Parado, não pede quadro.
- **Windows e Linux.** `platform.rs` virou fachada, com `platform/macos.rs` (Objective-C) e `platform/portable.rs` (`arboard` para a área de transferência, `rfd` para o seletor de pasta). O wgpu ganhou DirectX 12, Vulkan e OpenGL. Fora do macOS a tecla de comando é Ctrl, e a janela mantém a moldura do sistema.
- **Entrega.** `.github/workflows/release.yml` gera os três pacotes numa tag `v*`; `ci.yml` compila nos três sistemas; `pages.yml` publica `site/`.

## Alternativas

- **Diminuir a opacidade das linhas em Feixes.** Resolve o tronco e apaga os fios soltos. A cobertura mantém os dois.
- **Cortar ligações de tag no núcleo.** Mudaria o dado para consertar um desenho. O corte ficou na vista, e só no Universo.
- **Universo com física ao vivo.** Bonito, mas gasta quadro parado e muda de lugar a cada abertura. O assentamento roda uma vez, fora do quadro, e é determinístico.

## O que não foi verificado

- O app em Windows e em Linux nunca foi aberto numa máquina real.
- No primeiro CI (2026-11-10): no Ubuntu o app e a CLI compilaram e os testes do núcleo, da CLI, da geometria e do editor passaram; no Windows tudo compilou e os testes de geometria e editor passaram. Os testes do núcleo e da CLI ainda não rodam no Windows.
- Nesse mesmo CI os jobs de macOS nem começaram: o rótulo `macos-14` não existe mais. Os workflows passaram a usar `macos-latest`, ainda sem nova execução.
- O workflow de release nunca rodou: não há tag `v*`.
- Fora do macOS o app não lê a preferência de reduzir movimento, e o executável de Windows ainda não tem ícone.
