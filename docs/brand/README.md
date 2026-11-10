# Omnimind: marca

**Omnimind** é o nome do produto. A forma `omnimind` vale para binário, pasta privada e identificadores.

Frase de abertura do app: **"Suas notas e o que a IA aprende com você, no mesmo lugar."**

A decisão completa está em `docs/decisions/0006-redesign-omnimind.md`. O protótipo aprovado fica no canvas do Claude Design: <https://claude.ai/artifact/UA2NnBkqLYyZhH2BC5LbTu>.

## Nome

O nome tem conflitos conhecidos:

- **OmniMind.ai:** agentes de IA no-code.
- **OmniMind:** ferramenta de pesquisa e mapa mental descrita como um second brain.

Nenhuma busca de marca foi feita. A pessoa manteve o nome por enquanto. A checagem está em `docs/next-steps.md`.

## Ícone

**Feixe:** um maço de cinco fios que se juntam no centro. Azulejo escuro, fios brancos.

| Tamanho | Desenho | Arquivo |
|---|---|---|
| Acima de 32 px | Cinco fios | `assets/icon/icon-1024.png` |
| 32 px ou menos | Três fios mais grossos | `assets/icon/icon-small.png` |

Os dois saem do bin `make-icon`. Dentro do app, a mesma marca é desenhada por `Ui::mark`, em cor de tinta.

## Tokens

Fonte única: `crates/app/src/theme.rs`.

Tinta neutra e um único acento verde. Há tema claro e escuro. O app segue o sistema até a pessoa escolher em Ajustes, e a escolha fica gravada.

| Papel | Claro | Escuro |
|---|---|---|
| Fundo | `#FFFFFF` | `#141416` |
| Barra lateral | `#F7F7F6` | `#19191C` |
| Painel | `#FFFFFF` | `#1D1D20` |
| Texto | `#18181A` | `#EDEDEE` |
| Texto secundário | `#55555C` | `#B0B0B7` |
| Texto apagado | `#6E6E76` | `#94949C` |
| Linha | `#E7E7E4` | `#2D2D32` |
| Sob o mouse | `#F0F0EE` | `#232327` |
| Selecionado | `#EBEBE9` | `#2A2A2F` |
| Fundo de código | `#F4F4F2` | `#1D1D20` |
| Acento | `#1F7A5C` | `#4CC29A` |
| Aviso | `#B4401F` | `#F2836C` |

**Regra de cor:** a interface é neutra. O acento marca o que está vivo ou pede atenção: IA ativa, memória em revisão, cursor. Aviso só marca problema, como conflito de nota. Sem cor decorativa.

### Cores de tema

Sete cores, as mesmas no grafo e nas métricas.

| Fatia | Claro | Escuro |
|---|---|---|
| 1 | `#C9452E` | `#F2836C` |
| 2 | `#B5801A` | `#E3B04B` |
| 3 | `#1F7A5C` | `#4CC29A` |
| 4 | `#1E7F96` | `#4FB3C8` |
| 5 | `#2B5FD9` | `#8AADF6` |
| 6 | `#C23B73` | `#EE8FB0` |
| 7 | `#7E6F55` | `#D8CBB3` |

As sete cores não passam em todas as checagens de daltonismo do validador de dataviz. Por isso a identidade nunca é só cor: o nome do tema sempre aparece.

Até sete temas, cada um tem cor e nome. Com oito ou mais, os seis maiores ficam com cor própria e o resto vai para "Outros", na sétima cor. Cor nunca se repete entre temas.

## Tipografia

Embutidas no binário, ambas OFL 1.1, licenças em `assets/fonts/`.

| Fonte | Uso |
|---|---|
| Familjen Grotesk | Interface, leitura e títulos |
| Sometype Mono | Dados, datas, caminhos |

O protótipo aprovado usou Hanken Grotesk e IBM Plex Mono. Ficamos com as fontes embutidas porque a troca exige baixar fontes, e a pessoa ainda não aprovou esse download. A pendência está em `docs/next-steps.md`.

## Símbolos

Cada tipo de memória tem um símbolo fino, desenhado por `Ui::glyph`.

| Símbolo | Significa |
|---|---|
| Ponto | Nota, nas listas |
| Anel no grafo, planeta com anel no Universo | Nota |
| Disco no grafo, estrela no Universo | Memória |
| Triângulo | Decisão |
| Losango | Procedimento |
| Anel | Conceito |
| Cruz | Episódio |
| Quadrado | Preferência |

## Forma

Cantos levemente arredondados: 8 px em botões e campos, 14 px em folhas sobre a tela. Linhas finas separam as áreas. Sombra só no que flutua: painel de detalhe, paleta e ficha do grafo.

## Movimento

Todo efeito explica um acontecimento e tem condição de parada.

| Efeito | O que aconteceu |
|---|---|
| O grafo se desenha, em cerca de 3,4 s, ou 4,2 s em volume grande | A pessoa chegou na tela do Grafo |
| O caminho até o item acende e o resto escurece | A IA leu aquilo, ao vivo |
| Uma luz viaja do último item lido até o próximo, nas três formas | A IA leu um depois do outro |
| Em Raízes, arcos saem do item até os vizinhos | Mouse sobre o item, ou item selecionado |
| No Universo, os corpos saem do centro e o céu gira um pouco | A pessoa chegou na vista, ou a IA está lendo |
| Um item nasce dentro do seu tema, ligado só ao que se relaciona com ele | Conhecimento novo |
| As ligações do item acendem | Mouse sobre o item, ou item selecionado |
| Aviso curto na base da tela | Uma sessão terminou |

Com a preferência de reduzir movimento ligada no sistema, o grafo aparece pronto, sem a abertura.

## Voz

Direta, em português do Brasil, com frases curtas. Diz o que aconteceu e o que fazer. Nunca diz "inteligente", "mágico" ou "poderoso".

Exemplos do app: "Seu cofre ainda está vazio." e "Esta nota mudou fora daqui enquanto você escrevia. Nada foi perdido."

## Vocabulário

A interface fala a língua da pessoa. O código e a CLI mantêm os nomes internos.

| Na tela | Significa | Interno |
|---|---|---|
| Cofre | A pasta que guarda notas e memórias | `brain`, flag `--brain` |
| Projeto | O que uma nota declara, com pasta e apelidos | `Project`, tag `project/<slug>` |
| Grupo | A categoria acima do projeto, que dá a cor no grafo | `Decl::group`, `Graph::regions` |
| Geral | Memória sem projeto: vale em todo projeto | sem tag `project/` |
| Nota | O que você escreve | `note` |
| Memória | O que a IA aprende | `memory` |
| Atividade | O que cada sessão mudou, com desfazer | `sessions` no modelo |
| Perfil | O que a IA sabe de você | `portrait`, `PortraitLine` |
| Ajustes | Conexões, cofre e aparência | `Page::Settings` |
| Em revisão | Memória que o portão reteve e a IA ainda não usa | `quarantined`, retida, `memory_held` |
| Arquivar | Tirar de uso, de forma reversível | `Forget`, `archived` |

`/sonhar` continua como está. O nome visível ainda não foi decidido.
