# Interface

Barra lateral à esquerda, conteúdo à direita, painel de detalhe que abre por cima quando preciso. Sem cofre não há barra: só Boas-vindas. Com cofre, o app abre direto no Grafo.

| Tela | Atalho | Para quê |
|---|---|---|
| Notas | ⌘1 | Escrever. Lista, editor e coluna com as memórias daquela nota |
| Memórias | ⌘2 | Ver tudo o que a IA guarda, filtrar por tipo, revisar |
| Grafo | ⌘3 | Ver o cofre e a IA agindo |
| Atividade | ⌘4 | Ver e desfazer o que cada sessão mudou |
| Métricas | ⌘5 | Saber o que é lido, o que falta e o que revisar |
| Perfil | ⌘6 | Ler e corrigir o que a IA sabe de você |
| Ajustes | sem atalho | Conectar a IA, trocar de cofre, escolher o tema |
| Busca | ⌘K | Procurar em tudo e rodar comandos, de qualquer lugar |

Na barra lateral ficam também a marca, o campo Buscar, as contagens de notas e memórias e, no pé, o estado da IA: "Nenhuma IA conectada", "Claude Code conectado" ou a última ação ao vivo, como "Claude Code leu Frete".

Telas em `docs/brand/screens/`.

## Boas-vindas

Aparece só sem cofre. À esquerda: marca, frase, as três palavras do produto (Nota, Memória, Cofre) e as saídas **Começar**, **Abrir uma pasta** e **Ver um exemplo**. À direita: um grafo Raízes de 300 itens de mentira, sem dado nenhum. Ele sai de cena quando o cofre abre.

## Grafo

- Duas formas: **Feixes** e **Raízes**. Começa em Feixes.
- Filtro: **Tudo**, **Notas**, **Memórias**.
- Cada chegada na tela refaz a abertura. Trocar forma ou filtro também.
- Mouse sobre um item acende as ligações dele e mostra uma ficha: título, tipo e "lida N vezes em 30 dias".
- Clique em nota abre a nota. Clique em memória abre o painel de detalhe. Clique no vazio fecha o painel.
- Rodapé: "N itens · M ligações". Em Raízes, só os itens.
- Sem arrastar, sem zoom, sem física. A chave estável de cada item o mantém perto do mesmo lugar quando outros chegam.
- Nomes na tela são os dos temas. O nome de um item só aparece na ficha.
- Cofre vazio: "Seu cofre ainda está vazio.", com **Escrever uma nota** e **Conectar sua IA**. Sem desenho.

Três animações:

| Animação | Quando |
|---|---|
| Abertura | Ao chegar na tela |
| Leitura | Ao vivo, a cada evento `Live::Read` do diário |
| Criação | Item novo nasce dentro do seu tema, ligado só ao que se relaciona com ele. Acima de 48 itens de uma vez, o desenho só atualiza |

## Notas

Lista com título, pasta, data e quantas memórias vieram de cada nota. Editor com preview ao vivo, wikilinks com sugestão em `[[`, e estado "Salvo", "Salvando" ou "Conflito". Coluna à direita com **Memórias desta nota** e **Notas que apontam para esta**.

## Memórias

Filtros: Todas, Decisões, Preferências, Procedimentos, Conceitos, Episódios. Quando há memória em revisão, um aviso em acento ("3 em revisão") filtra só elas.

## Painel de detalhe

Abre ao lado direito, a partir de Memórias, Grafo, Notas ou Métricas. Mostra tipo, título, resumo, corpo, Origem, Fonte, Status, Criada, Motivo e a citação. Esc fecha.

| Botão | Quando | Faz |
|---|---|---|
| **Aprovar** | Só em memória em revisão, e só depois do corpo carregar | A IA passa a usar |
| **Abrir a nota** | Se a memória veio de uma nota | Abre a nota |
| **Arquivar** | Sempre | Tira de uso, de forma reversível |

## Atividade

Sessões agrupadas por dia, com cliente, hora, projeto e leituras. Cada mudança abre o motivo e o antes e depois. **Desfazer** tira uma mudança. **Desfazer sessão** tira a sessão inteira.

## Métricas

Tudo sai do diário, por `stats.rs`, na thread de dados. Períodos: 7, 30 e 90 dias, sempre comparados ao período anterior.

| Bloco | Mostra |
|---|---|
| Números | Leituras, Memórias novas, Correções suas, Buscas com resposta |
| Gráfico | Leituras da IA e suas, por dia; por semana em 90 dias |
| Mais lidas | Notas ou memórias, em abas |
| Temas mais lidos | Com a cor e o nome de cada tema |
| Para revisar | Em revisão, Desatualizadas, Sem leitura no período, Notas sem ligação, Para melhorar com a IA |
| Buscas sem resposta | Quantas vezes, com **Criar nota** |
| Quem usa | Sessões, leituras e escritas por cliente |
| Ritmo de escrita | Últimas 13 semanas, dias seguidos, dias ativos, dia mais forte |

Leitura da IA vem do evento `Read`. Leitura sua vem do evento `Tool` com `kind: note_opened`.

## Perfil

Frases agrupadas por seção, com a origem de cada uma. Clique edita, Enter grava, Esc cancela. Sair da tela grava. Vale como autoridade da pessoa.

## Ajustes

| Seção | Tem |
|---|---|
| Conexões | Claude Code e GitHub Copilot: **Conectar projeto…** e **Copiar comando** |
| Projetos | Um interruptor por projeto e cliente. O estado vem dos arquivos do projeto. Desligado, ou com a pasta ausente, mostra **Remover** |
| Cofre | Caminho, contagens, **Trocar** |
| Aparência | Tema, **Alternar** |

## Busca

⌘K abre a paleta. Sem texto, lista os comandos: ir para cada tela de ⌘1 a ⌘6, Nova nota, Nota do dia, Renomear nota, Apagar nota, Abrir outro cofre, Atalhos. Com texto, procura em notas e memórias enquanto se digita.

## Atalhos

| Tecla | Faz |
|---|---|
| ⌘K | Paleta |
| ⌘1 a ⌘6 | Telas |
| ⌘N | Nova nota |
| ⌘D | Nota do dia |
| ⌘S | Grava a nota agora |
| ⌘/ | Folha de atalhos |
| ⌘[ e ⌘] | Voltar e avançar entre notas |
| ⌘B e ⌘I | Negrito e itálico |
| `[[` | Criar link, com sugestões |
| ⌘ + clique | Seguir um link |
| Esc | Fechar o painel |

## Tema

Claro e escuro. Segue o sistema até a pessoa escolher em Ajustes. A escolha fica em `~/Library/Application Support/Omnimind/theme`. Trocar de tema não recria nada: os tokens são lidos a cada quadro.

## Regras

- Zero quadros em repouso: sem cursor piscando, sem animação de fundo.
- O grafo só desenha no Grafo e nas Boas-vindas, e só quando algo mudou. Digitar nunca paga pelo grafo.
- Toda animação tem condição de parada.
- Interface otimista: o quadro nunca espera disco. A thread de dados conversa por canais.
- Teclado primeiro: as telas de ⌘1 a ⌘6 e os comandos principais passam pela paleta. Ajustes ainda não passa.
- Identidade nunca é só cor: o nome do tema sempre aparece.
- A preferência de reduzir movimento do sistema tira a abertura do grafo.
