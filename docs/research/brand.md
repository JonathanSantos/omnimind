# Pesquisa de marca: identidade, tipografia e nome

Data da pesquisa: 2026-10-31. Quando a fonte não tem data, escrevi "s/d". Todos os links foram acessados nesta data.

## 0. Como ler este documento

**Força da evidência.** [P] é fonte primária (o próprio autor, documento, dado ou repositório). [S] é fonte secundária (imprensa, blog). [O] é opinião de blog ou agência, muitas vezes vendendo a solução. [I] é inferência minha.

**Limites reais desta pesquisa:**

1. A cota de busca web da sessão acabou durante a verificação de tipografia. A checagem dos nomes **não foi feita por busca na web**, como você pediu. Foi feita por consulta exata: crates.io, npm, PyPI, Homebrew Cask, RDAP de domínios, título da página no domínio exato e Wikipédia. Isso pega conflito em registro e em domínio, mas não pega produto cujo site tem outro domínio. Falta uma rodada de busca web por nome (dá para retomar em nova mensagem).
2. INPI e USPTO não foram consultados. Os "sobreviventes" são candidatos, não nomes liberados.
3. Não baixei nenhum binário de fonte. Tamanho vem da API do GitHub e de requisição HEAD. Cobertura vem de metadados, listas de idiomas ou mapa de caracteres, não de inspeção do arquivo.
4. Páginas que não abriram: blog e método do Linear (carregam só com JS), artigo da Bloomberg sobre esconder complexidade (403), texto do UX Collective "The way we design AI looks like nostalgia already" (403), post do Indie Hackers sobre "AI purple problem" (403), tabela de tintas da Stencil (404).
5. Não existe, até onde achei, estudo que meça que porcentagem de sites de IA usa roxo ou modo escuro. A saturação é consenso de crítica, não medição.

**Contexto do concorrente**, de `docs/research/edgemind/01-produto-e-decisoes.md`: EdgeMind usa cérebro com circuitos, ouro `#F5B62E` + violeta `#CF96E5` sobre `#14101A`, Bricolage Grotesque, Instrument Sans e Spline Sans Mono, três colunas, um único gesto de movimento (aresta acesa). Isso **tira da mesa** essas três fontes, âmbar/ouro e violeta, mesmo quando seriam boas escolhas. Nomes que ele já rejeitou por conflito: Cerne, Hebb, Myelin, Loci.

---

## 1. O que está saturado no visual de IA (2025-2026)

### 1.1 Quadro geral

| Padrão | Estado | Evidência | Força |
|---|---|---|---|
| Gradiente roxo/azul | Saturado | Anthropic cita "purple gradients on white backgrounds" como padrão de LLM ([Anthropic, 2025-11-12](https://claude.com/blog/improving-frontend-design-through-skills)). Adam Wathan pediu desculpas em tom de piada pelo `bg-indigo-500` do Tailwind UI ([relato, 2025-10-26](https://prg.sh/ramblings/Why-Your-AI-Keeps-Building-the-Same-Purple-Gradient-Website); data 2025-08-07 em [AIbase](https://news.aibase.com/zh/news/20365)). As visualizações citadas divergem (68 mil ou 1 milhão); não use número. | [P] causa declarada, [S] resto |
| Sparkles | Saturado e ambíguo | NN/g: das 107 associações com o ícone isolado, nenhuma foi "inteligência artificial"; 16,82% favoritar/salvar e 16,82% efeitos visuais ([Kaplan, 2024-09-20](https://nngroup.com/articles/ai-sparkles-icon-problem/)) | [P], o melhor dado desta seção |
| Glassmorphism | Saturado e virou piada de legibilidade | Na lista de sinais de [impeccable.style/slop](https://impeccable.style/slop) (s/d). Liquid Glass, da Apple, recuou em beta por queixas de legibilidade ([Wikipedia](https://en.wikipedia.org/wiki/Liquid_Glass), [Gizmodo](https://gizmodo.com/ios-26-might-be-apples-riskiest-redesign-yet-2000613526), 2025). Não achei o texto da NN/g, só citação indireta. | [S] |
| Cards iguais | Saturado | "Identical card grids" em impeccable; Anthropic fala em "predictable layouts" | [O], convergente |
| Bento | Disputado | Parte das listas anti-slop **recomenda** bento e varia alturas ([ux-skill wiki](https://github.com/Laith0003/ux-skill/wiki/Anti-AI-slop-ban-list.md), s/d). O problema é a célula idêntica, não a forma. | [O] |
| Inter + modo escuro | Saturado como default, não como uso na web | Anthropic lista Inter, Roboto, Arial. Web Almanac 2025 mostra Inter em só 1,4-1,5% das páginas, contra cerca de 10% de Roboto ([HTTP Archive, 2025](https://almanac.httparchive.org/en/2025/fonts)). A saturação está na saída gerada e no nicho de SaaS, não na web. | [P] |
| Estética Linear | Saturada | Daryl Ginn: quatro sites não passaram no "squint test" ([Rectangle, 2023-01-10](https://rectangle.substack.com/p/the-linear-effect)). Três traços: fundo escuro, gradiente borrado, brilho animado. | [S] |
| Cérebros, nós neurais, azul | Saturado desde antes | [Better Images of AI](https://blog.betterimagesofai.org/wp-content/uploads/2023/02/Better-Images-of-AI-Guide-Feb-23.pdf) (fev/2023) lista azul, código caindo, cérebro humano e robôs brancos como o que evitar. O próprio EdgeMind admite que cérebro com circuitos é clichê. | [P] |
| Orb | Onipresente, pouca crítica escrita | Há componentes prontos ([21st.dev](https://21st.dev/@aqualang89/components/speaking-orb.md)) e coleções ([Dribbble](https://dribbble.com/AmazingUI/collections/7632650-Spheres-AI-Assistant-AI-Agent-by-AmazingUI)). Não achei crítica direta boa. | Só prova de ubiquidade |

### 1.2 Por que converge

A Anthropic chama isso de convergência distributiva: "Safe design choices... dominate web training data" ([2025-11-12](https://claude.com/blog/improving-frontend-design-through-skills)). O mesmo texto registra que o modelo, proibido de usar Inter, passa a convergir em **Space Grotesk**. Trocar o default por outro default não resolve.

### 1.3 O achado mais importante: a reação anti-roxo já virou clichê

A lista do impeccable.style inclui, como sinais de 2026, "italic serif display headlines", paletas creme/bege por padrão, Geist como fonte default, pontos de status pulsando, cursor piscando decorativo, rótulos numerados sem sequência, e até "Not a feature. A platform." como vício de texto. A fonte é opinião (62 regras detectoras, sem dados de frequência), mas a tendência é clara. Nenhuma fonte que achei cita Fraunces ou Instrument Serif pelo nome, então trate isso como risco, não como fato.

**Consequência [I]:** "creme + serifa itálica + preto" é o próximo roxo. Fugir só do roxo leva a outro ponto médio.

---

## 2. Marcas com personalidade: o que é copiável

Foquei no princípio. O visual delas é o sintoma.

| Marca | O que fez | Princípio copiável | Fonte |
|---|---|---|---|
| Teenage Engineering | Limites de hardware como motor; paleta curta; mono em tudo | Escolha a restrição antes da forma. Limite publicamente | [Crosley, s/d](https://blakecrosley.com/guides/design/teenage-engineering) [S], [Bizcommunity](https://bizcommunity.ug/Article/196/15/241169.html) [S] |
| Linear | Craft acima de velocidade; virou sinônimo de um estilo que nem é mais dela | Copiar o resultado sem as razões produz casca. Ginn notou que "todo mundo parece igual" | [Sequoia, s/d](https://sequoiacap.com/article/linear-spotlight/) [S], [Ginn, 2023](https://rectangle.substack.com/p/the-linear-effect) [S] |
| Arc / The Browser Company | Gradientes, onboarding memorável, vídeos com a própria equipe. Mas, na carta do CEO, Arc era "too different, with too many new things to learn, for too little reward"; venda por US$ 610 mi à Atlassian | Personalidade não paga curva de aprendizado. Público técnico tolera mais; ainda assim, primeiro minuto precisa ser simples | [Carta, 2025-05-26](https://browsercompany.substack.com/p/letter-to-arc-members-2025) [P], [TechNews](https://technews.tw/?p=1436592) [S] |
| Zed | "so responsive it almost disappeared"; editor tratado como jogo | Uma tese técnica pode ser a marca. O argumento é a personalidade | [Zed, 2023-03-07](https://zed.dev/blog/videogame) [P] |
| Raycast | "fast, simple, and delightful"; ícone é uma tecla, que diz "teclado" sem texto | O logo é o argumento do produto em um objeto. Atalho visível ensina enquanto se usa | [Raycast, 2022-07-19](https://www.raycast.com/blog/a-fresh-look-and-feel) [P] |
| Nothing | Matriz de pontos em tipo e hardware; Glyph Matrix com 489 LEDs | Tipografia e produto são a mesma ideia. Atenção à dívida: Design Milk aponta influência de TE e Designers Republic | [Madegood, s/d](https://madegooddesigns.com/nothing-brand-guidelines/) [O], [Android Authority](https://androidauthority.com/nothing-phone-3-glyph-matrix-teaser-3569071) [S], [Design Milk](https://design-milk.com/nothing-says-something-about-the-monochromatic-landscape-of-smartphones/) [S] |
| Panic / Playdate | Tela 1-bit sem backlight, manivela, TE no hardware | Um gesto físico estranho e próprio vale mais que um logo. Ninguém esquece a manivela | [play.date](https://play.date/) [P], [Domus, 2022-06-24](https://domusweb.it/en/design/gallery/2022/06/24/playdate-a-game-boy-designed-for-the-netflix-era.html) [S] |
| Things | Apple Design Award em 2009 e 2017; anos de trabalho no Things 3 | Lentidão deliberada é posicionamento. Pouca coisa, tudo polida. Não achei fonte primária sobre o processo | [Wikipedia](https://en.wikipedia.org/wiki/Things_(software)) [S] |
| Family | Revelação gradual; movimento como explicação; **curva delight-impacto**: quanto mais raro o recurso, maior pode ser o encanto | Regra aplicável direto: fluxo diário discreto, momentos raros memoráveis | [Family Values, 2024-07-08](https://benji.org/family-values) [P] |
| Rive | O produto é um formato: máquina de estados que o designer muda sem quebrar o código | Personalidade por decisão de arquitetura. Vale para nosso formato de dados | [Rive](https://rive.app/blog/a-new-graphics-format-for-the-interactive-era) [P] |
| Are.na | Sem anúncios, sem recomendação, "accountable only to our members", "connected knowledge collectors" | O modelo de negócio é a voz da marca. Dizer o que não faz vira identidade | [Are.na about](https://www.are.na/about) [P], [home](https://www.are.na/) [P] |
| Readymag | Tipografia como centro; editor livre, sem grade rígida | Faça o produto ser demonstração da própria tese | [Wikipedia](https://en.wikipedia.org/wiki/Readymag) [S] |
| Figma | Quatro palavras-guia (2019); em 2024 abandona o cursor: "we don't want to be stuck in the cursor zone" | Saiba quando largar o próprio ícone. Personalidade definida por poucas palavras decide brigas internas | [Figma, 2019-10-15](https://www.figma.com/blog/bringing-new-life-to-figmas-brand/) [P], [Figma, 2024-09-16](https://www.figma.com/blog/figma-on-figma-evolving-our-visual-language/) [P], [It's Nice That, 2024-09-16](https://www.itsnicethat.com/features/figma-brand-refresh-graphic-design-spotlight-160924) [S] |
| Vercel | Geist nasceu com Basement Studio; suíço, minimal, rápido | Fonte própria é identidade. **Ressalva:** liberada, virou commodity e já consta como default em listas anti-slop | [Basement](https://basement.studio/post/the-birth-of-geist-a-typeface-crafted-for-the-web) [P], [impeccable](https://impeccable.style/slop) [O] |
| Ghostty | Recusa "an unnecessary choice between speed, features, and platform-native GUIs"; sem fins lucrativos via Hack Club (dez/2025) | Declare a falsa escolha que você recusa. Governança pública gera confiança | [Hashimoto, 2024-10-22](https://mitchellh.com/writing/ghostty-is-coming) [P], [OMG Ubuntu, 2025-12](https://www.omgubuntu.co.uk/2025/12/ghostty-terminal-non-profit-fiscal-sponsorship) [S] |
| Sublime Text | Toolkit de UI próprio; Goto Anything; velocidade antes de tudo | Um comando-assinatura pode definir a categoria (⌘P hoje é padrão) | [wbond.net](https://wbond.net/thoughts/building_a_high_performance_text_editor) [P] |

### Sete princípios que sobram

1. **Uma tese técnica vira frase.** Zed e Ghostty mostram que dá para ter personalidade falando de arquitetura.
2. **Um gesto proprietário.** Manivela, tecla, Goto Anything. Precisamos de um; EdgeMind tem só "aresta acesa".
3. **A restrição é declarada e visível.** TE, Playdate, Nothing.
4. **O modelo é a voz.** Are.na e Ghostty: local-first não é recurso, é a posição.
5. **Regra da curva delight-impacto.** Família inteira de encantos nos momentos raros, nunca no fluxo diário.
6. **Personalidade não compensa fricção.** Lição do Arc.
7. **Direção de marca precisa de razão interna.** Quem copia sem a razão fica igual ao Linear falsificado.

---

## 3. Direções visuais incomuns

Para cada referência: o que se aproveita em software. O valor está na regra, não na citação.

### Instrumentos e sinal

| Referência | O que é | Regra aproveitável | Fonte |
|---|---|---|---|
| Unknown Pleasures / CP 1919 | Harold Craft, tese de 1970, Cornell, dados de Arecibo: cerca de 80 pulsos empilhados. Saville inverteu as cores e deixou o resto preto | **Empilhar repetições revela regularidade e desvio.** Craft usou uma rotina de oclusão para esconder linhas atrás de picos: legibilidade por ocultação | [Scientific American, 2015-02-18](https://www.scientificamerican.com/blog/sa-visual/pop-culture-pulsar-origin-story-of-joy-division-s-unknown-pleasures-album-cover-video/) [P] |
| Osciloscópio / fósforo | Traço com persistência; pode ser simulado fisicamente | A memória visual decai como fósforo. Em Rust/wgpu já existem [`phosphor-crt`](https://docs.rs/phosphor-crt) e [`typedrat/phosphor`](https://ithub.global.ssl.fastly.net/typedrat/phosphor) (estágio inicial, MPL 2.0) | [P] |
| Bloomberg Terminal | Preto e âmbar como marca reconhecível de longe (Bloomberg, 2017, trecho de busca); usuários têm orgulho da interface difícil | Densidade como status. Mas âmbar colide com o ouro do EdgeMind | [UX Mag](https://uxmag.com/articles/the-impossible-bloomberg-makeover) [S], [Core77](https://www.core77.com/posts/24893/Moneymaking-Multi-Monitor-Mayhem-and-Why-Some-Prefer-Interface-Design-That-Sucks) [S] |
| Connections Museum, Seattle | Comutadores eletromecânicos funcionando: step-by-step, Panel, Crossbar. Cada dígito move um mecanismo | **A rota é visível e audível.** Consulta da IA como comutação mecânica, não como brilho | [Telecom History Group](https://www.telcomhistory.org/ConnectionsSeattle.html) [P] |

### Cartografia, atlas, astronomia

| Referência | O que é | Regra aproveitável | Fonte |
|---|---|---|---|
| Naturgemälde, Humboldt, 1807 | Corte do Chimborazo, plantas por altitude; o nome da planta é escrito na altitude em que ela cresce | Posição tem significado. O rótulo é o próprio dado | [GHIS](https://germanhistory-intersections.org/en/knowledge-and-education/ghis:image-5) [P], [Linda Hall](https://www.lindahall.org/about/news/scientist-of-the-day/alexander-von-humboldt) [S] |
| Berghaus, 1847 | Inventor do atlas temático, segundo a [Linda Hall](https://www.lindahall.org/about/news/scientist-of-the-day/alexander-von-humboldt) | Várias camadas temáticas sobre a mesma base estável | [S] |
| Imhof | Relevo sombreado suíço com regras explícitas | Terreno contínuo em vez de nós e arestas. Fragment shader faz isso bem | [arXiv 2010.01256](https://arxiv.org/pdf/2010.01256) [S], [Arquivo de relevo](https://shadedreliefarchive.colororacle.org/NortheastSwitzerland.html) [P] |
| Trouvelot, 1882 | Cerca de 7 mil ilustrações em Harvard; portfólio de 15 estampas | Rigor e beleza não competem. Domínio público | [Public Domain Review](https://publicdomainreview.org/collections/the-trouvelot-astronomical-drawings-1882/) [S] |
| Projeto RADAM, Brasil | Radar de visada lateral porque nuvens impediam foto da Amazônia; 132 cartas-imagem 1:250.000 | **Ver através do que encobre.** Atenção: projeto da ditadura ("Integrar para não entregar"), não é referência neutra | [Pesquisa FAPESP](https://revistapesquisa.fapesp.br/projeto-radambrasil-completa-50-anos/) [S], [Rasiliana](https://tidsskrift.dk/bras/article/view/128402) [S] |
| NASA Graphics Standards, 1975 | Danne & Blackburn; manual reeditado em 2015 | Manual rigoroso é peça de marca. Publicar o nosso | [Design Milk](https://design-milk.com/nasas-1975-graphics-standards-manual-reissued/) [S] |
| Müller-Brockmann | Grade como "will to systematize" (1981). O termo certo é esse, não "willingness" | Grade rígida permite um gesto selvagem | [ensaio](https://openlab.citytech.cuny.edu/langecomd3504fa2019/files/2018/10/MullerBrockmann_Grid_Des-Phil.pdf) [P] |
| Observable | Plot: marcas e escalas. Notebook como documento editável definido por código | Vocabulário pequeno e composável. Não achei fonte sobre a identidade visual | [Bostock](https://observablehq.com/blog/future-of-data-work-q-a-with-mike-bostock) [P] |

### Biologia e matéria

| Referência | O que é | Regra aproveitável | Fonte |
|---|---|---|---|
| Anna Atkins, 1843 | Cianótipos de algas de herbário; 17 cópias conhecidas | **Fundo azul total, sem câmera, o objeto imprime a própria sombra.** Fuga do cinza escuro e do creme ao mesmo tempo | [NHM](https://www.nhm.ac.uk/discover/anna-atkins-cyanotypes-the-first-book-of-photographs) [P], [Met](https://metmuseum.org/art/collection/search/286656) [P], [Wikipedia](https://en.wikipedia.org/wiki/Anna_Atkins) [S] |
| Flora Brasiliensis, 1840-1906 | 15 volumes em 40 partes, 10.367 páginas, cerca de 23 mil espécies; digitalizada. Não são 130 volumes | Acervo brasileiro de ilustrações científicas. Conferir licença do digital antes de usar | [CRIA, 2006](https://lists.ku.edu/pipermail/taxacom/2006-September/108021.html) [P], [FAPESP](https://revistapesquisa.fapesp.br/rigor-e-sensibilidade/) [S] |
| Micélio / Physarum | Agentes depositam rastro, outros seguem. Jones 2010; Jenson 2019; milhões de agentes em GPU | **Caminho usado engrossa, caminho largado evapora.** É aprendizado por uso visível. EdgeMind rejeitou "reforço por uso" por falta de evidência; é aqui que nos separamos | [Bleuje](https://github.com/Bleuje/interactive-physarum) [P], [Ghassaei](https://apps.amandaghassaei.com/gpu-io/examples/physarum/) [P] |
| Ribbon (Jane Richardson, 1981) | Li "ondas Ribbon" como diagramas de fita de proteína, à mão, em tinta e pastel. Se você quis dizer outra coisa, me corrija | Uma pessoa criou a linguagem que os softwares depois imitaram. Convenção visual é escolha autoral, e ela própria diz que é subjetiva | [Quanta, 2024-08-23](https://quantamagazine.org/how-colorful-ribbon-diagrams-became-the-face-of-proteins-20240823) [S], [Richardson, 2000](https://web.stanford.edu/class/sbio228/public/readings/Introduction_Lecture1/Richardson_NAT_00_Ribbon_drawing_history.pdf) [P] |

### Impressão e tinta

| Referência | O que é | Regra aproveitável | Fonte |
|---|---|---|---|
| Risografia | Cores chapadas, fora de registro. Só achei kits e simuladores; **nenhuma marca de software com a linguagem em toda a interface** | Espaço vazio, mas risco de virar efeito de fundo. Tem que significar algo | [INKDRUM](https://abduzeedo.com/inkdrum-browser-risograph-print-simulator-inkdrum-team) [S] |
| Xilogravura de cordel | Dissertação da ESDI trata J. Borges como referência para design brasileiro; livro de 2026 analisa 500+ capas como sistema gráfico | Preto sobre papel colorido, sem degradê. Encomendar a um gravador vivo, com crédito; não imitar | [ESDI](https://www.esdi.uerj.br/en/projects/@relId/11353/@orderBy/name/@orderDir/desc) [P], [Diario de Pernambuco, 2026](https://www.diariodepernambuco.com.br/viver/2026/10/11726110-designer-lanca-livro-que-ressalta-sofisticacao-da-xilogravura-em-mais-de-500-capas-de-cordel.html) [S] |
| Manguebeat | "Caranguejos com Cérebro" (Recife, 1992): lama fértil + antena parabólica + computador. Influência citada no Porto Digital | **O único cérebro admissível nesta categoria.** Tecnologia de ponta enfiada na lama local, em vez do futuro limpo | [Fast Company Brasil](https://fastcompanybrasil.com/?p=36055) [S], [Sesc SP, 2016](https://portal.sescsp.org.br/online/artigo/compartilhar/10153_A+MUSICA+NASCIDA+DO+MANGUE) [S] |
| Plotter | "you can't cover anything up": só linhas | Sem preenchimento nem sombra. Identidade reproduzível em caneta real | [Hobbs, 2020](https://tylerxhobbs.com/essays/2018/executing-generative-art-with-a-plotter) [P] |
| Sumi-ê | Simulação em GPU existe desde 2005 (MoXi); Living Ink roda 13 simulações a 60 fps | Viável, mas "zen japonês" sem raiz nossa soa apropriação. Descartaria | [SIGGRAPH](https://history.siggraph.org/learning/moxi-real-time-ink-dispersion-in-absorbent-paper-by-chu-and-tai-2) [P], [Kataoka](https://www.drue.net/living-ink) [P] |

### O problema específico do grafo

"Local graph is actually useful. Full graph is mostly a poster" ([Code Culture, 2026-05-20](https://codeculture.store/blogs/developer-culture/obsidian-graph-view-useful)) resume o debate no ecossistema Obsidian. As críticas recorrentes: vira novelo em vaults grandes, posições mudam a cada abertura, pouco acionável ([Zettelkasten Forum](https://forum.zettelkasten.de/discussion/comment/8744), [Obsidian Forum](https://forum.obsidian.md/t/whats-the-point-of-the-graph-view-how-are-you-using-it/71316/19), [Konik em defesa](https://www.eleanorkonik.com/its-not-just-a-pretty-gimmick-in-defense-of-obsidians-graph-view)). O número "200 notas" circula só em blog de baixa qualidade; não use.

**Regra para as três direções [I]:** posição estável e com significado, antes de qualquer efeito. Força dirigida pura é exatamente o que todo mundo tem, inclusive o EdgeMind.

---

## 4. Tipografia

### 4.1 Regras de licença para embutir no binário

- **Pode embutir e vender.** A OFL permite empacotar com software, inclusive pago. Só não pode vender a fonte sozinha ([FAQ OFL](https://openfontlicense.org/ofl-faq/)).
- **Precisa creditar.** Mesmo embutida, inclua copyright, aviso e texto da licença; a FAQ 1.20 sugere a caixa "Sobre". 
- **Subset é modificação.** FAQ 2.6: cortar glifos conta como versão modificada. Se a fonte tem Reserved Font Name (RFN), a versão cortada precisa de outro nome. **Com RFN:** IBM Plex ("Plex") e Monaspace (mais Argon, Neon, Xenon, Radon, Krypton). Sem RFN no cabeçalho: as demais da tabela abaixo, exceto Redaction, que não abri.
- **pt-BR.** O subset `latin` do Google Fonts inclui Latin-1 Suplementar, que contém ã õ ç á é í ó ú â ê ô à ü. `latin-ext` é dispensável para nós.
- **WOFF2.** A pilha cosmic-text/swash/fontdb de `interface-e-otimizacao.md` trabalha com TTF/OTF. Conte o tamanho TTF. Confirme antes de assumir suporte a WOFF2.

### 4.2 Excluídas

| Fonte | Motivo |
|---|---|
| iA Writer (Mono/Duo/Quattro) | **Armadilha.** Tem OFL, mas o [README](https://github.com/iaolo/iA-Fonts) pede: "Do not create text editors that bundle our fonts"; RFN "iA Writer". Mesmo que a licença permita, somos exatamente quem eles pedem para não fazer isso |
| Bricolage Grotesque, Instrument Sans, Spline Sans Mono | Usadas pelo EdgeMind |
| Space Grotesk | Citada pela Anthropic como convergência |
| Inter, Geist, Roboto | Defaults. Geist segue ativa (v1.9.0 em 2026-10-30), mas não diferencia |
| Berkeley Mono | Paga. Se quiser o look: [IoskeleyMono](https://gittrend.io/repo/ahatem/IoskeleyMono), OFL, configuração de Iosevka que imita Berkeley; não testei |

### 4.3 Recomendadas

Todas OFL-1.1, com Redaction a confirmar (ver nota). Tamanho TTF/OTF é o que entra no binário sem subset. "Subset" é WOFF2 `latin` só com eixo de peso, via CDN do Fontsource, como referência de quanto dá para enxugar. Pesos vêm da [API do Fontsource](https://api.fontsource.org/v1/fonts/fraunces).

| Fonte | Uso | Personalidade | pt-BR | Pesos | TTF/OTF | Subset | Download direto |
|---|---|---|---|---|---|---|---|
| **Newsreader** | Leitura longa, títulos | Serifa de jornal com eixo óptico; Production Type | Sim (`latin`) | 200-800 | 452 KB + 496 KB itálico | 58 KB | [romana](https://github.com/google/fonts/raw/main/ofl/newsreader/Newsreader%5Bopsz,wght%5D.ttf), [itálica](https://github.com/google/fonts/raw/main/ofl/newsreader/Newsreader-Italic%5Bopsz,wght%5D.ttf) |
| **Literata** | Leitura longa | Serifa de livro feita para tela; Google Play Livros. Pesada | Sim (`latin`) | 200-900 | 955 KB + 903 KB | 52 KB | [romana](https://github.com/google/fonts/raw/main/ofl/literata/Literata%5Bopsz,wght%5D.ttf), [itálica](https://github.com/google/fonts/raw/main/ofl/literata/Literata-Italic%5Bopsz,wght%5D.ttf) |
| **Fraunces** | Display | "Wonky", eixos SOFT e WONK. Risco de virar serifa padrão | Sim (`latin`) | 100-900 | 360 KB + 415 KB | 37 KB | [romana](https://github.com/google/fonts/raw/main/ofl/fraunces/Fraunces%5BSOFT,WONK,opsz,wght%5D.ttf) |
| **Young Serif** | Display | Serifa pesada antiga, só um peso | Sim (`latin`) | 400 | 107 KB | n/d | [TTF](https://github.com/google/fonts/raw/main/ofl/youngserif/YoungSerif-Regular.ttf) |
| **Gabarito** | Display, UI | **Brasileira.** Naipe Foundry (Álvaro Franca, Felipe Casaprima, Rio) e Leandro Assis; feita para vestibular; tem símbolos matemáticos e científicos. Sem itálico | Sim (`latin`), origem BR | 400-900 | 158 KB | 34 KB | [TTF](https://github.com/google/fonts/raw/main/ofl/gabarito/Gabarito%5Bwght%5D.ttf), [repo](https://github.com/naipefoundry/gabarito) |
| **Familjen Grotesk** | UI | Grotesca sueca com desenho próprio e leve | Sim (`latin`) | 400-700 | 90 KB + 94 KB | 19 KB | [TTF](https://github.com/google/fonts/raw/main/ofl/familjengrotesk/FamiljenGrotesk%5Bwght%5D.ttf) |
| **Hanken Grotesk** | UI | Neutra e sóbria. Se precisar de calma | Sim (`latin`) | 100-900 | 133 KB + 139 KB | 35 KB | [TTF](https://github.com/google/fonts/raw/main/ofl/hankengrotesk/HankenGrotesk%5Bwght%5D.ttf) |
| **Atkinson Hyperlegible Next** | Leitura, UI | Braille Institute. Letras feitas para não se confundirem | Sim (`latin`) | 200-800 | 115 KB + 124 KB | 34 KB | [TTF](https://github.com/google/fonts/raw/main/ofl/atkinsonhyperlegiblenext/AtkinsonHyperlegibleNext%5Bwght%5D.ttf) |
| **Atkinson Hyperlegible Mono** | Código | A mesma ideia em mono. Muito leve | Sim (`latin`) | 200-800 | 54 KB + 56 KB | 18 KB | [TTF](https://github.com/google/fonts/raw/main/ofl/atkinsonhyperlegiblemono/AtkinsonHyperlegibleMono%5Bwght%5D.ttf) |
| **Commit Mono** | Editor, UI | "anonymous and neutral"; site gera versão com alternativas | Sim (lista do site inclui Portuguese) | 200-700 | cerca de 160 KB por estilo; zip 724 KB | n/d | [zip v1.143](https://github.com/eigilnikolajsen/commit-mono/releases/download/v1.143/CommitMono-1.143.zip) |
| **Departure Mono** | Display, medidas | Pixel, lo-fi técnico, ficção científica. Helena Zhang | **Sim, conferido:** 26 acentuados no mapa de 1.042 glifos | 1 estilo | 84 KB | n/d | [zip v1.500](https://github.com/rektdeckard/departure-mono/releases/download/v1.500/DepartureMono-1.500.zip) |
| **Martian Mono** | Rótulos, display | Mono larga com eixo de largura; Evil Martians | Sim (`latin`) | 100-800 | 148 KB | 24 KB | [TTF](https://github.com/google/fonts/raw/main/ofl/martianmono/MartianMono%5Bwdth,wght%5D.ttf) |
| **Sometype Mono** | Metadados | Mono fina e discreta | Sim (`latin`) | 400-700 | 64 KB + 66 KB | 17 KB | [TTF](https://github.com/google/fonts/raw/main/ofl/sometypemono/SometypeMono%5Bwght%5D.ttf) |
| **Xanh Mono** | Display, citações | Mono com itálico de serifa; muito peculiar. Yellow Type Foundry | Sim (`latin`) | 400 | 60 KB + 64 KB | n/d | [TTF](https://github.com/google/fonts/raw/main/ofl/xanhmono/XanhMono-Regular.ttf) |
| **Recursive** | Tudo em uma | Eixos MONO e CASL: mesma fonte vai de técnica a casual. Grande | Sim (`latin`) | 300-900 | **2,4 MB** | 56 KB | [TTF](https://github.com/google/fonts/raw/main/ofl/recursive/Recursive%5BCASL,CRSV,MONO,slnt,wght%5D.ttf) |
| **Monaspace** | Código | 5 famílias compatíveis; "texture healing". **RFN** | Não verificado | variável | **1,4 a 2,6 MB por família** | n/d | [zip 4,2 MB](https://github.com/githubnext/monaspace/releases/download/v1.400/monaspace-variable-v1.400.zip) |
| **Redaction** | Voz da IA | Nasceu de exposição sobre justiça criminal (MoMA PS1, 2019). Texto se degrada | Só `latin` listado; não verificado | 400, 700 | zip 1,7 MB | n/d | [zip](https://www.redaction.us/static/redaction.zip) |

**Redaction, atenção:** o [Fontsource](https://fontsource.org/fonts/redaction/about) diz OFL + LGPL 2.1; a página da [MCKL](https://www.mckltype.com/essays/Redaction-essay) orienta baixar "for personal use". Abra o zip e leia a licença antes de usar. O contexto também pesa: é uma fonte sobre encarceramento nos EUA, não um efeito neutro.

**Oportunidade brasileira:** Gabarito é a única da lista com autoria brasileira confirmada. O README dela também cita a [Harbor Type](https://www.harbortype.com/) (Henrique Beier, Porto Alegre). Encomendar uma fonte de título a tipógrafo brasileiro, ou uma variante de Gabarito (OFL e sem RFN permitem), daria uma marca que ninguém copia só instalando.

---

## 5. Nome

### 5.1 Tendências

A evidência é fraca: quase tudo é blog de marketing de domínios. O que se sustenta: Crunchbase aponta alta de nomes evocativos e de palavra única de dicionário ([Crunchbase](https://news.crunchbase.com/venture/startup-names-trends-cool/)); "AI" no nome está perdendo força; grafias criativas caem; o sufixo `-ify.ai` virou piada ([Product Hunt](https://www.producthunt.com/p/nicejourney/please-stop-naming-your-startup-something-ify-ai-brief-rant-from-a-professional-copywriter), s/d); [NameStation, 2026](https://www.namestation.com/blog/domain-naming-trends-2026) fala em nomes curtos e marcáveis. Em software, classes 9 e 42 de Nice são as relevantes ([Gerben](https://www.gerbenlaw.com/?p=81), [INPI](https://www.gov.br/inpi/en/services/trademarks/classification-trademarks)).

**Como evitar nome tomado.** Palavra inglesa curta está esgotada. Dos 33 nomes que testei, palavras técnicas ou de ciência caíram quase todas, e várias já tinham IA de memória por perto. Palavras de origem brasileira e tupi sobreviveram muito mais. Esse é o nosso fosso: nomes baratos para nós e caros para quem não é daqui.

**Regra de voz em pt-BR [I].** "Nós" significa *nodes*, *knots* e *we*. "Rede" é rede de pesca, de dormir e de computadores. Esses dois trocadilhos servem para título de página e texto, não como nome.

### 5.2 Os 12 nomes

Legenda: **Reg.** = crates.io, npm, PyPI e Homebrew Cask (livre = todos 404). **Dom.** = resultado RDAP: ✓ não encontrado (provavelmente livre, confirmar no registrador e em preço premium), ✗ registrado. Forma ASCII usada para binário e domínio. Verificado em 2026-10-31.

| # | Nome | Conceito | Reg. | .com | .app | .ai | .dev | .com.br | Conflito visto | Veredito |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | **Pororoca** | Onda de até 4 m que sobe o Amazonas quando mar e rio se encontram ([Wikipedia](https://en.wikipedia.org/wiki/Pororoca)). Sua nota encontra o que você já sabia. Nome mais memorável da lista | livre | ✗ | ✓ | ✗ | ✓ | ✗ | Nenhum produto achado; `.com` e `.ai` sem site aparente | **Sobrevive** |
| 2 | **Várzea** (`varzea`) | Planície que a cheia inunda e fertiliza com sedimento. O uso inunda, a memória fica. Também é "futebol de várzea": humor e chão | livre | ✗ | ✓ | ✓ | ✗ | ✗ | Nenhum em software. Em inglês lê-se bem, mas perde o acento | **Sobrevive** |
| 3 | **Igapó** (`igapo`) | Floresta alagada de água preta ([Wikipedia](https://en.wikipedia.org/wiki/Igap%C3%B3)). A floresta funciona debaixo d'água: profundo e calmo. Bom para CLI | livre | ✗ | ✓ | ✓ | ✗ | ✗ | `.com.br` com título de empresa (Remaza), sem relação | **Sobrevive** |
| 4 | **Piracema** | Migração dos peixes rio acima para desovar ([Wikipedia](https://en.wikipedia.org/wiki/Piracema)). Ideias voltam à origem para gerar mais | livre | ✗ | ✓ | ✓ | ✓ | ✗ | Piracema Comercial (fitas de arquear), outro setor | **Sobrevive** |
| 5 | **Preamar** | Maré alta, o ponto de maior cheia. Bonito em português, opaco em inglês | livre | ✗ | ✓ | ✓ | ✓ | ✗ | Série da HBO América Latina ([Wikipedia](https://en.wikipedia.org/wiki/Preamar)) | Sobrevive com ressalva |
| 6 | **Bilro** | Peça da renda de bilros: alfinetes são nós, fios são arestas, o desenho aparece cruzando. Grafo artesanal e brasileiro | livre | ✗ | ✓ | ✓ | ✓ | ✗ | **Bilro Tecnologia** em `.com.br`, setor próximo | Amarelo, checar INPI |
| 7 | **Mangue** | Manguebeat: lama fértil, antena, "caranguejos com cérebro". Melhor conceito | livre | ✗ | ✗ | ✗ | ✗ | ✗ | **Mangue Tech** (software ESG) em `.ai` | Amarelo, classe conflita |
| 8 | **Garoa** | Chuva fina e constante de São Paulo: aprendizado que acumula sem você notar | livre | ✗ | ✗ | ✓ | ✗ | ✗ | **Garoa Hacker Clube** ([site](https://garoa.net.br)), justo o público técnico; agência em `.com.br` | Amarelo |
| 9 | **Lastro** | Peso que estabiliza o barco e dá valor à moeda. Ótimo em português | livre | ✗ | ✗ | ✗ | ✗ | ✗ | Empresa em `.com.br`; `.ai` leva a uma IA imobiliária. Nada livre | Amarelo-vermelho |
| 10 | **Remanso** | Trecho calmo do rio onde tudo assenta | livre | ✗ | ✗ | ✓ | ✓ | ✗ | App de finanças em `remanso.app` (espanhol) | Amarelo |
| 11 | **Cordel** | Folhetos pendurados em barbante: nós em fios | livre | ✗ | ✗ | ✗ | ✓ | ✗ | **cordel.ai**, inspeção ferroviária por IA | Cai |
| 12 | **Vereda** | Caminho que se abre andando; Guimarães Rosa. Minha favorita de conceito | livre | ✗ | ✗ | ✗ | ✗ | ✗ | **vereda.ai**, estúdio de ferramentas de IA; `vereda.dev`, biblioteca | Cai |

**Observação honesta:** os quatro sobreviventes são todos Amazônia e água. São coesos, o que é bom para um universo de marca, mas parecidos entre si e exigem aprender a pronunciar fora do Brasil. Se o público é brasileiro técnico, isso é vantagem. Para um mercado global, Pororoca é o que melhor se sustenta como som.

**Descartados na triagem** (motivo, tudo em 2026-10-31):

| Nome | Por que caiu |
|---|---|
| Aluvia | `aluvia.app`: "the friend who truly remembers you". Colisão direta de conceito |
| Trama | Crate `trama` é parser de vault do Obsidian; `trama.app` existe |
| Noz | Crate `noz` é biblioteca de GUI; npm tomado. Pena: "noz" + "nós" + "knows" era ótimo |
| Curupira | PyPI: agentes locais de IA; `curupira.app`: cortes com IA no PC |
| Jabuti | `jabuti.ai`: agentic AI |
| Prumo | `prumo.ai` e PyPI voltado a agentes |
| Talweg, Paiol | `talweg.ai` (infra de IA), `paiol.ai` (estratégia de IA) |
| Breu | `breu.app`: leitura rápida para iOS |
| Patuá | `patua.app`: finanças pessoais brasileiro. O conceito (bolsinha com texto junto ao corpo) merece virar nome de recurso |
| Varve, Gnomon, Líquen, Lichen, Alidade | Crates ou PyPI tomados, com domínios ocupados |
| Tarrafa, Maré, Brasa, Tatu, Urutau, Saúva | Produto ou pacote ativo em domínio ou registro |

---

## 6. Achado → o que muda na marca

| Achado | O que muda na marca |
|---|---|
| Sparkles: 0 de 107 associações com IA (NN/g) | Nada de ícone "IA". A IA se mostra por rastro e efeito no conteúdo |
| Modelos convergem até depois de proibidos (Anthropic) | Regra positiva, não lista de proibições: cada escolha visual deriva da metáfora |
| Creme + serifa itálica é o próximo clichê | Rejeitar o creme como fundo principal; se usar papel, que seja colorido ou claramente de um sistema |
| Inter é só 1,4-1,5% da web | Não vender "sem Inter" como diferencial. O inimigo é o default, não a fonte |
| EdgeMind usa ouro, violeta, Bricolage e Instrument | Excluir âmbar, Bricolage, Instrument e Spline. Terminal âmbar estilo Bloomberg sai, embora seja uma ótima referência |
| EdgeMind tem um gesto; IA invisível fora do grafo | Gesto proprietário e presença da IA dentro do editor, não só no grafo |
| EdgeMind rejeitou reforço por uso | Fazer disso a tese: "o que você usa engrossa, o que larga assenta". Dá a frase da marca, à la Zed e Ghostty |
| Decisão do rival: IA grava sozinha com diff, motivo e desfazer | Identidade precisa mostrar procedência e reversão como elementos visuais de primeira classe |
| "Nada é compartilhável" no EdgeMind | Peça compartilhável: relatório do que a IA aprendeu, impresso ou como pôster do próprio histórico |
| Grafo global é pôster; posição instável | Posição estável com significado. Aproveitar o pôster como momento raro, não como ferramenta diária |
| Curva delight-impacto (Family) | Encanto na abertura da semana, na revisão, no primeiro grafo; editor discreto |
| Arc: personalidade não paga curva | Marca ousada, primeiro minuto óbvio. Modo sóbrio sempre disponível |
| Ghostty: recusar falsa escolha | Frase candidata: recusar a escolha entre IA que aprende sozinha e IA em que você confia |
| Are.na: modelo é voz | Local-first vira postura escrita, não item de lista de recursos |
| Physarum, fósforo e relevo rodam em GPU, e há crates | A marca vira demonstração técnica do app nativo; em WebView isso é bem mais difícil de sustentar |
| Nomes brasileiros sobrevivem, técnicos não | Assumir o Brasil como vantagem de marca, não como sotaque a esconder |
| Gabarito tem autoria brasileira | Ao menos uma fonte com origem nacional; considerar encomenda |
| iA Writer pede para não embutir; Plex e Monaspace têm RFN | Decidir subset e nomes antes de fixar as fontes |
| RADAM serviu à ditadura; cordel é arte de pessoas reais | Usar a regra, não a imagem. Citar com contexto. Encomendar e creditar |

---

## 7. Três direções de identidade

Valores hex são propostas minhas, não amostras medidas. Os pares de texto passam em WCAG AA; a tabela está no fim desta seção.

### Direção A: **Bancada** (nome sugerido: Pororoca)

**Metáfora central.** O app é um instrumento de medição e a sua memória é um sinal. O uso excita, o tempo decai. A IA é o segundo canal do osciloscópio. Pororoca é, literalmente, uma onda.

**Paleta.**

| Papel | Hex |
|---|---|
| Tubo apagado (fundo) | `#070B09` |
| Painel | `#0F1613` |
| Retícula | `#1F3329` |
| Traço esmaecendo | `#2E7D57` |
| Canal 1, humano | `#5CFFA3` |
| Canal 2, IA | `#6FD3FF` |
| Núcleo do feixe, texto | `#E6FFF1` |
| Marcador de alerta | `#FF5A36` |
| Tema claro, papel térmico | `#ECEBE3` com tinta `#0F3D2A` |

**Tipografia.** Departure Mono (títulos, medidas, unidades), Commit Mono (UI e editor), Atkinson Hyperlegible Next (leitura longa). De 650 KB a 1 MB no binário, conforme os estilos de Commit Mono.

**Grafo.** Nós fixos sobre retícula com coordenadas estáveis. Quando a IA lê, um feixe percorre a aresta com persistência física: o rastro decai por constante de tempo real, então dá para ver o que aconteceu nos últimos segundos sem log. Nós sem uso esmaecem. Cursores de medição mostram Δ de memórias e sessões. Segundo modo: **pilha**, uma linha por sessão empilhada como o gráfico de Craft, com a mesma regra de oclusão. Vira o pôster do seu ano de trabalho.

**Gesto proprietário.** O botão de base de tempo: girar no trackpad volta o sinal no tempo.

**Presença da IA no editor.** Traço fino do canal 2 corre na margem da nota que está sendo lida.

**Risco.** "Terminal verde hacker". Antídoto: tratar como instrumento de laboratório da Tektronix, com unidades reais e retícula honesta, nunca Matrix. Pode intimidar. É também a direção mais fria.

### Direção B: **Várzea** (nome sugerido: Várzea)

**Metáfora central.** Cada sessão é uma cheia. A água sobe, a IA trabalha, a água baixa e deixa uma camada de sedimento. Conhecimento é terreno que se forma, e você lê o tempo nas camadas. Junta Atkins, Humboldt e Imhof, e tem origem brasileira no nome.

**Paleta.** O modo principal é **azul de cianótipo em tela cheia**: nem cinza escuro nem creme.

| Papel | Hex |
|---|---|
| Azul profundo, fundo de leitura | `#081D33` |
| Azul-da-prússia, fundo | `#0E2F4F` |
| Superfície | `#16436B` |
| Linhas, curvas de nível | `#7FA6C2` |
| Papel não exposto, texto | `#EEF3F4` |
| Sedimento, camadas | `#D9C7A3` |
| Laterita, IA agora | `#F07F4F` |
| Tema claro | fundo `#EEF3F4`, tinta `#0E2F4F`, laterita `#A8421D` |

**Tipografia.** Newsreader (leitura e títulos, com eixo óptico), Familjen Grotesk (UI), Sometype Mono (metadados e datas). Cerca de 1,3 MB, menos com subset.

**Grafo.** Não é novelo: é **corte geológico**. Eixo vertical é tempo e profundidade, horizontal são as facetas. Cada sessão é um estrato; memórias são fósseis na camada em que nasceram; ligações são veios que atravessam camadas. Rótulos seguem Humboldt: o nome fica escrito na profundidade em que a ideia vive, e a posição é o dado. Vista alternativa, **planta**: relevo Imhof em shader, altura é uso, curvas de nível no lugar de arestas. Posição é função de tempo e faceta, portanto estável, e funciona com muitos nós porque é terreno.

**Gesto proprietário.** A cheia: ao fechar a sessão, a água sobe e baixa em 2 segundos, e o sedimento novo fica. **Desfazer é tirar o estrato.** Procedência e reversão ficam visíveis sem nenhum painel.

**Presença da IA no editor.** Água fina subindo pela margem da nota; o texto acrescentado pela IA fica em tom de sedimento até você ler.

**Risco.** Parecer ilustração de museu, fora de um app técnico. Antídoto: densidade alta, mono nos dados, física real de água rasa. Fundo azul é decisão difícil para muitas horas de uso; precisa de teste.

### Direção C: **Mangue** (nome sugerido: Mangue, ou Piracema se o conflito pesar)

**Metáfora central.** Lama fértil com antena parabólica. Memória é ecossistema sujo e produtivo, não biblioteca limpa. **Humano e IA são duas chapas de impressão**: o que você escreveu sai em preto, o que a IA escreveu sai em tinta de cor, e o desalinhamento entre elas é a procedência. Traz "caranguejos com cérebro" como provocação, e nenhum concorrente pode dizer isso.

**Paleta.** Fundo de papel colorido como capa de cordel, escolhido pelo usuário.

| Papel | Hex |
|---|---|
| Tinta preta, humano | `#15120F` |
| Papel rosa-cordel (padrão) | `#F6A5C0` |
| Papel verde-mangue | `#7FD1AE` |
| Papel kraft | `#C9A87C` |
| Tinta da IA sobre papel | `#1F3A8F` |
| Destaque fluorescente, só em modo escuro | `#FF6B35` |
| Modo escuro | fundo `#15120F`, texto `#F6A5C0` |

**Tipografia.** Gabarito (títulos e UI, autoria brasileira), Literata (leitura), Redaction em trechos de baixa confiança da IA, **somente se** a licença e o contexto da fonte passarem. Plano B: Xanh Mono no lugar dela. Cerca de 2 MB antes de subset.

**Grafo.** Rede de transporte **Physarum** com milhões de agentes em compute shader. Notas são comida em posições fixas. Consultas e leituras depositam rastro, e rastro sem uso evapora: o caminho usado engrossa, à vista. Sai em meio-tom de 1 bit, com duas chapas defasadas em 1,5 px. Ao aproximar, a rede vira folhetos pendurados em barbante, com título legível. Não se desenham arestas individuais; desenha-se fluxo, então não há novelo.

**Gesto proprietário.** A **tiragem**: toda semana o app imprime um folheto de uma página, com capa, sobre o que a IA aprendeu. PDF, PNG ou plotter de verdade. Resolve "nada é compartilhável". Pela curva da Family, é o momento raro e marcante.

**Presença da IA no editor.** Texto da IA em segunda tinta, levemente fora de registro, até você revisar.

**Risco.** A mais ousada e a mais fácil de virar fantasia regional. Antídoto: encomendar a gravador vivo, não imitar. Rosa em tela inteira cansa; o texto de leitura precisa de fundo mais neutro. Depende de resolver Mangue Tech no INPI. Modo sóbrio obrigatório.

### Comparação

| | Bancada | Várzea | Mangue |
|---|---|---|---|
| Sensação | Precisão | Tempo profundo | Vitalidade |
| Uau imediato | Alto | Muito alto | Muito alto |
| Uso por 8 h | Bom | A testar | Precisa de modo neutro |
| Grafo resolve novelo | Parcial | **Sim** | Sim, perde precisão |
| Mostra aprendizado por uso | Decaimento | Camadas | **Rastro que engrossa** |
| Procedência e desfazer | Canais | **Estrato removível** | Chapas |
| Distância do EdgeMind | Boa | **Máxima** | Máxima |
| Risco de pastiche | Médio | Baixo | Alto |

**Minha recomendação [I]:** começar por **Várzea**. É a única em que a metáfora, por si só, resolve grafo estável, tempo, procedência e desfazer. Ela é a resposta direta à decisão do rival de escrever sem portão de aprovação. O rastro de Physarum de Mangue pode entrar como camada de "uso" sobre o relevo. Antes de fechar: renderizar uma única tela do grafo nas três direções, com dados reais e 5.000 nós, e testar leitura por uma hora no fundo azul.

### Contraste (WCAG)

Calculado sobre os pares principais de texto; AA exige 4,5:1 para texto normal.

| Dir. | Par | Hex | Razão |
|---|---|---|---|
| A | texto / tubo | `#E6FFF1` / `#070B09` | 18.8:1 |
| A | canal 1 / tubo | `#5CFFA3` / `#070B09` | 15.4:1 |
| A | canal 2 / painel | `#6FD3FF` / `#0F1613` | 10.9:1 |
| A | alerta / tubo | `#FF5A36` / `#070B09` | 6.4:1 |
| A | traço esmaecendo / tubo (só gráfico, não texto; 3:1 basta) | `#2E7D57` / `#070B09` | 3.9:1 |
| A | tinta / papel térmico | `#0F3D2A` / `#ECEBE3` | 10.2:1 |
| B | texto / azul | `#EEF3F4` / `#0E2F4F` | 12.2:1 |
| B | texto / azul profundo | `#EEF3F4` / `#081D33` | 15.2:1 |
| B | texto / superfície | `#EEF3F4` / `#16436B` | 9.2:1 |
| B | sedimento / azul | `#D9C7A3` / `#0E2F4F` | 8.2:1 |
| B | laterita / azul | `#F07F4F` / `#0E2F4F` | 5.1:1 |
| B | linhas / azul | `#7FA6C2` / `#0E2F4F` | 5.3:1 |
| B | laterita clara / claro | `#A8421D` / `#EEF3F4` | 5.4:1 |
| C | tinta / rosa | `#15120F` / `#F6A5C0` | 9.8:1 |
| C | tinta / verde | `#15120F` / `#7FD1AE` | 10.3:1 |
| C | tinta / kraft | `#15120F` / `#C9A87C` | 8.3:1 |
| C | IA / rosa | `#1F3A8F` / `#F6A5C0` | 5.4:1 |
| C | IA / verde | `#1F3A8F` / `#7FD1AE` | 5.6:1 |
| C | IA / kraft | `#1F3A8F` / `#C9A87C` | 4.6:1 |
| C | rosa / escuro | `#F6A5C0` / `#15120F` | 9.8:1 |
| C | fluorescente / escuro | `#FF6B35` / `#15120F` | 6.6:1 |

---

## 8. Próximos passos

1. Rodada de busca web por nome para Pororoca, Várzea, Igapó, Piracema e Bilro, incluindo App Store e GitHub.
2. Pesquisar nas classes 9 e 42 do INPI e no USPTO, com ajuda jurídica antes de registrar.
3. Registrar logo os domínios candidatos: `pororoca.app`, `varzea.app`, `varzea.ai`, `igapo.app`, depois de confirmar disponibilidade e preço.
4. Ler a licença dentro de `redaction.zip`.
5. Fazer protótipo em wgpu de uma tela do grafo por direção.
