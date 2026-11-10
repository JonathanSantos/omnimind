# 0006. Omnimind: nome novo, visual neutro e grafo em Feixes e Raízes

**Status:** adotada em 2026-11-01. Substitui a 0004. Protótipo aprovado no canvas do Claude Design: <https://claude.ai/artifact/UA2NnBkqLYyZhH2BC5LbTu>.

## Contexto

O produto se chamava Várzea. A interface inteira era uma folha de mapa: terreno em blocos, água que subia ao fim da sessão, estratos para o histórico, azul de cianótipo em tema único. As telas tinham nomes da metáfora: Estratos, Retrato, Ligações.

O QA de design da época apontava problema de vocabulário ("brain", "retida") e coluna única entre telas. A base técnica era grande: física ForceAtlas2, hidrologia, terreno, registro de estilos de mapa, pós-processamento com bloom.

A pessoa pediu nome novo e redesenho, e aprovou um protótipo.

## Decisão

- **Nome:** Omnimind. O nome tem conflitos conhecidos e nenhuma busca de marca foi feita; ver `docs/brand/README.md`. A pessoa manteve o nome por enquanto.
- **Ícone:** Feixe, um maço de cinco fios, com três fios mais grossos a 32 px ou menos.
- **Visual:** tinta neutra, um acento verde (`#1F7A5C` no claro, `#4CC29A` no escuro), tema claro e escuro. Segue o sistema até a pessoa escolher em Ajustes.
- **Telas:** barra lateral com Notas, Memórias, Grafo, Atividade, Métricas, Perfil e Ajustes; paleta em ⌘K; painel de detalhe lateral. Boas-vindas só sem cofre. Com cofre, o app abre no Grafo.
- **Vocabulário:** Cofre, Nota, Memória, Atividade, Perfil, Ajustes. Código e flag de CLI mantêm `brain`. `/sonhar` fica como está, e o nome visível segue em aberto.
- **Grafo:** duas formas. Feixes (hierarchical edge bundling, Holten 2006) e Raízes (space colonization, Runions 2007). `crates/graph` é geometria pura. `GraphView` desenha numa textura fora da tela com wgpu, aditivo no escuro e subtração reversa no claro. A geometria é montada numa thread à parte.
- **Animações:** abertura; leitura ao vivo, pelos eventos `Live::Read` do diário; criação, em que o item novo nasce dentro do tema, ligado só ao que se relaciona com ele. Sem arrastar, sem zoom, sem física.
- **Métricas:** tela nova, com números reais do diário por `stats.rs`. Para contar leitura da pessoa, entrou o evento `Tool` com `kind: note_opened`.

## Alternativas

**Formas do grafo.** Oito formas foram estudadas com os mesmos dados: Feixes, Fios, Constelação, Raízes, Arcos, Metrô, Colmeia e Tecido. Feixes, Raízes e Tecido foram comparadas animadas, com pouco e muito dado. Ficaram Feixes, que mostra relações entre temas sem virar novelo, e Raízes, que mostra crescimento e uso. Tecido saiu: com muito dado vira textura e quase não se distingue item por item. Fios, Metrô e Constelação contam menos sobre as ligações reais ou exigem caminhos escolhidos à mão; Arcos não comporta rótulos; Colmeia pede explicação.

**Manter o mapa.** Era a decisão 0004, que tratava o estilo como questão de gosto, com custo de quadro parecido entre estilos. Saiu junto com a metáfora.

## Removido

Motor ForceAtlas2, hidrologia, layouts, registro `MapStyle`, terreno, voxel, cianótipo, cheia, régua do tempo, pós-processamento e bloom em `gfx`, bins `gallery` e `proof-map`, fonte Newsreader, `docs/specs/mvp/map-styles.md`.

Entrou o bin `proof-graph`.

## Renomeação em disco

| Antes | Depois |
|---|---|
| `.varzea/` | `.omnimind/`, com migração única em `Brain::open`, que também ajusta o `.gitignore` |
| `VARZEA_*` | `OMNIMIND_*` |
| binário `varzea` | `omnimind` |
| | Executável `OmnimindApp`, `dist/Omnimind.app`, bundle id `app.omnimind.desktop` |
| | Configuração em `~/Library/Application Support/Omnimind`, cofre padrão em `~/Omnimind` |

`omnimind connect` remove a entrada MCP antiga `varzea` e reconhece os hooks antigos.

## Medição

`proof-graph`, sem janela, 1440×900 a 2x, com leituras chegando a cada 15 quadros, sem os 30 primeiros quadros. O tempo inclui a espera da GPU.

| Itens / ligações | Feixes p50 | Feixes p99 | Raízes p50 | Raízes p99 |
|---|---|---|---|---|
| 540 / 1.600 | 2,57 ms | 5,36 ms | 2,07 ms | 5,01 ms |
| 2.000 / 6.000 | 4,50 ms | 7,73 ms | 2,40 ms | 5,77 ms |
| 4.000 / 12.000 | 9,38 ms | 12,95 ms | 2,97 ms | 6,68 ms |
| 10.000 / 30.000, a 1,5x | 15,93 ms | 23,53 ms | 3,51 ms | 6,87 ms |

Números do app completo em `docs/qa/limits.md`.

## Ressalvas

- **Fontes.** O protótipo aprovado usou Hanken Grotesk e IBM Plex Mono. O app ficou com Familjen Grotesk e Sometype Mono, já embutidas, porque a troca exige baixar fontes, e a pessoa ainda não aprovou esse download. Newsreader saiu, e a leitura passou para Familjen Grotesk.
- **Cores.** As sete cores de tema não passam em todas as checagens de daltonismo do validador de dataviz. A identidade nunca é só cor: o nome sempre aparece.
- **Avaliação de IA e portão hostil.** Os números em `docs/qa/` não foram refeitos depois da renomeação.
