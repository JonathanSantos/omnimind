<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/diagrams/banner-dark.svg">
    <img src="docs/brand/diagrams/banner-light.svg" alt="Omnimind: a memória das suas IAs de código, em arquivos seus. Corrija uma vez. Na próxima sessão, a IA já sabe. Ao lado, um grafo em forma de raízes cresce do centro, em sete cores." width="100%">
  </picture>
</p>

<p align="center">
  <code>100% Rust</code> · <code>macOS</code> · <code>Windows</code> · <code>Linux</code> · <code>GPU nativa</code> · <code>local</code> · <code>sem nuvem</code> · <code>sem embeddings</code> · <code>Claude Code</code> · <code>GitHub Copilot</code>
</p>

<p align="center">
  <a href="https://jonathansantos.github.io/omnimind/"><b>Site do projeto: jonathansantos.github.io/omnimind</b></a>
</p>

<p align="center">
  <a href="#o-que-ele-faz">O que faz</a> ·
  <a href="#as-telas">Telas</a> ·
  <a href="#os-fluxos">Fluxos</a> ·
  <a href="#como-funciona-por-dentro">Por dentro</a> ·
  <a href="#números">Números</a> ·
  <a href="#instalar">Instalar</a> ·
  <a href="#estado-atual">Estado atual</a>
</p>

---

## O que ele faz

Toda sessão de IA começa do zero. Você repete o gerenciador de pacotes, o formato do commit e aquela decisão de duas semanas atrás. O Omnimind fica entre a IA e uma pasta sua: entrega o que importa no início da sessão, registra o que você corrige e mostra tudo o que foi gravado, com desfazer.

| Palavra | O que é |
|---|---|
| **Nota** | O que você escreve, em Markdown |
| **Memória** | O que a IA aprende, com data, origem e autoridade |
| **Cofre** | A pasta que guarda as duas. Um arquivo comum, sem conta e sem nuvem |
| **Projeto** | O que uma nota declara, com grupo e pasta. A memória de um projeto não vaza para outro |

**Três ideias sustentam o produto:**

- **A correção não depende da IA.** Um hook guarda sua frase literal antes de o modelo decidir qualquer coisa.
- **A IA não fala por você sem prova.** Só vira "disse a pessoa" o que uma citação literal, registrada no diário, sustenta.
- **Tudo é reversível.** Cada mudança fica no diário, e uma sessão inteira se desfaz com um clique.

## As telas

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: light)" srcset="docs/brand/screens/15-light-graph.png">
    <img src="docs/brand/screens/01-graph-bundles.png" alt="Tela Grafo do Omnimind em Feixes: temas ao redor de um círculo, com ligações agrupadas" width="100%">
  </picture>
</p>

<table>
  <tr>
    <td width="50%"><img src="docs/brand/screens/24-graph-universe.png" alt="Grafo em Universo: notas e memórias como corpos em três dimensões"><br><b>Universo.</b> O cofre em três dimensões. Assuntos parecidos ficam perto. Arraste para girar, role para aproximar.</td>
    <td width="50%"><img src="docs/brand/screens/25-graph-universe-reading.png" alt="Universo durante uma leitura da IA"><br><b>Leitura ao vivo.</b> Quando a IA lê, uma luz viaja de um corpo ao seguinte e as ligações dele acendem.</td>
  </tr>
  <tr>
    <td width="50%"><img src="docs/brand/screens/03-graph-roots.png" alt="Grafo em Raízes"><br><b>Raízes.</b> O cofre como árvore. Ramos engrossam com o uso, e a leitura da IA acende o caminho. Nota é anel, memória é disco; role para aproximar.</td>
    <td width="50%"><img src="docs/brand/screens/05-notes.png" alt="Editor de notas"><br><b>Notas.</b> Editor nativo de Markdown, com preview ao vivo.</td>
  </tr>
  <tr>
    <td><img src="docs/brand/screens/06-memories.png" alt="Lista de memórias"><br><b>Memórias.</b> Tudo o que a IA guarda, por tipo e origem.</td>
    <td><img src="docs/brand/screens/08-activity.png" alt="Atividade por sessão"><br><b>Atividade.</b> O que cada sessão mudou, com desfazer.</td>
  </tr>
  <tr>
    <td><img src="docs/brand/screens/09-metrics.png" alt="Métricas"><br><b>Métricas.</b> O que é lido, o que você corrigiu e o que ficou sem resposta.</td>
    <td><img src="docs/brand/screens/12-profile.png" alt="Perfil"><br><b>Perfil.</b> O que a IA sabe de você. Clique numa frase para corrigir.</td>
  </tr>
  <tr>
    <td><img src="docs/brand/screens/13-settings.png" alt="Ajustes com a lista de projetos"><br><b>Ajustes.</b> Cada projeto com seu interruptor.</td>
    <td><img src="docs/brand/screens/22-welcome.png" alt="Boas-vindas"><br><b>Boas-vindas.</b> Aparece só enquanto não existe um cofre.</td>
  </tr>
</table>

## Os fluxos

### 1. Do download à primeira sessão

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/diagrams/first-session-dark.svg">
    <img src="docs/brand/diagrams/first-session-light.svg" alt="Linha de cinco passos: instalar, começar, conectar, abrir sessão e lembrar" width="100%">
  </picture>
</p>

Conectar escreve a integração dentro do projeto. Nada vai para `~/.claude` sem `--global`, e suas configurações continuam lá.

### 2. Uma sessão por dentro

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/diagrams/session-dark.svg">
    <img src="docs/brand/diagrams/session-light.svg" alt="Sequência entre você, Claude Code, hooks, servidor MCP e cofre, do início ao fim da sessão" width="100%">
  </picture>
</p>

O servidor MCP nasce com a sessão e morre com ela. Com o app fechado, nada fica rodando.

### 3. De correção a memória

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/diagrams/correction-dark.svg">
    <img src="docs/brand/diagrams/correction-light.svg" alt="Fluxo vertical: a frase é gravada sem IA, passa por segredo, instrução e citação, e vira memória sua ou da IA" width="72%">
  </picture>
</p>

Se a IA não fizer nada, a frase literal entra no próximo resumo do mesmo jeito.

### 4. Vários projetos, uma memória

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/diagrams/projects-dark.svg">
    <img src="docs/brand/diagrams/projects-light.svg" alt="Projetos do cofre ligados por feixes às camadas do resumo inicial, com o projeto atual em destaque" width="100%">
  </picture>
</p>

Uma lição de um projeto ajuda em outro, com a origem citada, e a regra de um não vaza para o outro.

### 5. Ligar e desligar

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/diagrams/on-off-dark.svg">
    <img src="docs/brand/diagrams/on-off-light.svg" alt="Estados Ligado, Desligado e Pasta não encontrada, com interruptor e Remover" width="100%">
  </picture>
</p>

Desligar tira do projeto só o que o Omnimind colocou. O cofre não muda, e o estado vem dos arquivos do projeto, então a tela não mostra uma conexão que não existe.

### 6. Conferir e desfazer

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/diagrams/review-dark.svg">
    <img src="docs/brand/diagrams/review-light.svg" alt="O que a IA grava aparece em Atividade e segue, é corrigido no Perfil ou desfeito" width="100%">
  </picture>
</p>

### 7. Consolidar com `/sonhar`

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/diagrams/dream-dark.svg">
    <img src="docs/brand/diagrams/dream-light.svg" alt="Ciclo circular de /sonhar: agenda, ler, decidir e gravar, até gerar o relatório" width="72%">
  </picture>
</p>

## Como funciona por dentro

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/diagrams/architecture-dark.svg">
    <img src="docs/brand/diagrams/architecture-light.svg" alt="Claude Code e Copilot falam com hooks e servidor MCP; app e servidor usam o núcleo; tudo termina no cofre" width="100%">
  </picture>
</p>

**O disco é a verdade.** Não há banco de dados nem processo em segundo plano. Cada processo mantém um índice em memória, e o instantâneo é descartável. Se você editar um arquivo por fora, o Omnimind percebe.

### O cofre

```text
~/Omnimind/
├── brain.toml                nome e regras do cofre
├── notes/                    suas notas em Markdown
├── memories/
│   ├── decision/             o que foi decidido
│   ├── preference/           como você gosta de trabalhar
│   ├── procedure/            como se faz
│   ├── concept/              o que algo significa
│   └── episode/              o que aconteceu
└── .omnimind/                privado e descartável, salvo o diário
    ├── journal/              diário só de anexar, por sessão
    ├── connections.json      projetos ligados a este cofre
    ├── live/                 sessão atual de cada cliente
    └── snapshot.bin          índice pronto para abrir rápido
```

### As quatro ferramentas da IA

| Ferramenta | Para quê |
|---|---|
| `memory_search` | Buscar em notas e memórias, com origem, estado e cobertura |
| `memory_read` | Abrir o texto inteiro. Segredos saem redigidos |
| `memory_write` | Gravar, editar, substituir, arquivar |
| `memory_agenda` | Listar o trabalho de consolidação pendente |

### O caminho de escrita

App, CLI, MCP e hooks passam por um único `Brain::apply`:

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/diagrams/write-path-dark.svg">
    <img src="docs/brand/diagrams/write-path-light.svg" alt="As oito etapas da escrita, de pedido a diário e índice" width="100%">
  </picture>
</p>

A autoridade vem do transporte e do diário, nunca de um campo da requisição. Uma memória marcada à mão como sua perde esse status se o arquivo não tiver vindo de uma escrita do produto.

### A busca, sem embeddings

| Etapa | Como |
|---|---|
| Índice | Invertido em memória, com BM25F. Título e aliases pesam mais que o corpo |
| Idioma | Stemming em português e inglês, com dobra de acentos |
| Reformulações | De 1 a 3 consultas, fundidas por RRF |
| Sinônimos | A IA grava `aliases` em português e inglês |
| Erro de digitação | Correção só quando nada voltou |
| Honestidade | `ok`, `weak`, `no_match` ou `incomplete`, com a cobertura |

### Crates

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/diagrams/crates-dark.svg">
    <img src="docs/brand/diagrams/crates-light.svg" alt="Dependências: app usa gfx, ed, graph e núcleo; a linha de comando usa o núcleo" width="100%">
  </picture>
</p>

### O grafo

`crates/graph` é geometria pura: recebe itens e ligações e devolve fitas de triângulos. O `GraphView` desenha com wgpu numa textura fora da tela e só redesenha quando algo muda.

| Forma | Técnica | Referência |
|---|---|---|
| **Feixes** | Ligações agrupadas por tema | Hierarchical edge bundling, Holten 2006 |
| **Raízes** | Cada tema cresce do centro | Space colonization, Runions 2007 |

## Números

Medidos num MacBook Air M2, com cofre fictício de 5.000 notas e 800 memórias.

| Medida | Limite | Medido |
|---|---|---|
| App instalado | 100 MB | **12 MB** |
| Primeiro quadro | 2 s | **0,59 s** |
| Memória do app | 500 MB | **143 MB** |
| Servidor da IA em repouso | 150 MB | **102,6 MB** |
| Busca, p95 | 100 ms | **32,8 ms** |
| Janela real, 300 quadros, p99 | 60 fps | **17,05 ms** |
| Quadros em repouso | 0 | **0** |

### Com o Sonnet 5.5, em cenários fictícios

| Teste | Com Omnimind | Sem memória |
|---|---|---|
| Correções lembradas sem repetir | 5 de 5 | 0 de 4 |
| 32 projetos, auditoria manual | 9 de 9 | 2 de 9, mais 3 parciais |
| Layout: cores do sistema | 46 de 46 | 0 de 45 |

A amostra é pequena, os cenários foram escritos por nós e o grupo sem memória não podia ler o repositório. Os resultados mostram que o mecanismo funciona, não quanto ele vai render no seu dia a dia. Detalhes em [`docs/qa/ai-sessions.md`](docs/qa/ai-sessions.md).

## Instalar

Os pacotes prontos de macOS, Windows e Linux saem do GitHub Actions a cada tag `v*` e ficam na [página de releases](https://github.com/JonathanSantos/omnimind/releases/latest). O macOS é a plataforma de referência. Os pacotes de Windows e Linux compilam e linkam, mas ainda não foram abertos numa máquina real.

Para gerar a partir do código, no macOS: um Mac com Apple Silicon, Rust estável e o Claude Code ou o GitHub Copilot. No Windows e no Linux, `scripts/package.sh` monta a pasta com os dois binários.

**1. Gerar o app.**

```bash
./scripts/bundle.sh
```

**2. Copiar para Aplicativos.** Apague a cópia anterior antes: copiar por cima invalida a assinatura.

```bash
rm -rf /Applications/Omnimind.app && ditto dist/Omnimind.app /Applications/Omnimind.app
```

**3. Abrir e clicar em Começar.** O cofre nasce em `~/Omnimind`.

```bash
open /Applications/Omnimind.app
```

**4. Ligar a IA.** Em todos os projetos de uma vez:

```bash
/Applications/Omnimind.app/Contents/MacOS/omnimind connect claude-code --global --brain ~/Omnimind
```

Ou só no projeto da pasta atual:

```bash
/Applications/Omnimind.app/Contents/MacOS/omnimind connect claude-code --brain ~/Omnimind --project .
```

Use `--print` para ver o que mudaria sem gravar nada. Cada arquivo alterado ganha uma cópia de segurança ao lado, e a ordem das chaves das suas configurações é mantida.

**Desligar** tira só o que o Omnimind colocou:

```bash
/Applications/Omnimind.app/Contents/MacOS/omnimind disconnect claude-code --global
```

**Testes:**

```bash
cargo test --workspace
```

O passo a passo com imagens está no [guia em PDF](docs/guide/guide.pdf), também disponível em [HTML](docs/guide/index.html) para abrir no navegador.

## Estado atual

Versão inicial, feita para uso pessoal em Mac com Apple Silicon.

| Área | Estado |
|---|---|
| Windows e Linux | Compilam e empacotam. Falta abrir o app e rodar os testes numa máquina real |
| Hooks e MCP no Claude Code | Testados em sessões reais automáticas |
| Modo interativo do Claude Code | Falta testar |
| Convivência com a memória nativa | Falta testar |
| GitHub Copilot | Conecta, mas falta testar no produto real |
| Duas sessões simultâneas no mesmo projeto | Podem se misturar no diário |
| Ligação global | Funciona, mas não aparece em Ajustes |
| Restaurar nota apagada | A nota vai para a lixeira do cofre, mas o app e `undo` ainda não a trazem de volta |
| Nome | Provisório |

## Documentação

| Documento | Conteúdo |
|---|---|
| [Site do projeto](https://jonathansantos.github.io/omnimind/) | Visão geral, diagramas animados e as três vistas do grafo |
| [Guia de uso](docs/guide/guide.pdf) | Instalar, conectar, usar, ligar e desligar, com imagens |
| [Arquitetura](docs/specs/mvp/architecture.md) | Threads, grafo e multi-processo |
| [Núcleo](docs/specs/mvp/core.md) | Cofre, diário, escrita e busca |
| [MCP e hooks](docs/specs/mvp/mcp.md) | Ferramentas e `connect` |
| [Medidas](docs/qa/limits.md) | Limites e como foram medidos |
| [Sessões com IA](docs/qa/ai-sessions.md) | Todas as avaliações reais |
| [Decisões](docs/decisions/) | Por que cada escolha foi feita |

## Licença

[MIT](LICENSE). As fontes embutidas, Familjen Grotesk e Sometype Mono, seguem a SIL Open Font License 1.1; os textos estão em [`assets/fonts`](assets/fonts).
