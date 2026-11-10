# Próximos passos

Para continuar sem contexto: leia `README.md` em `docs/`, depois `specs/mvp/scope.md` e `decisions/`. Convenção: código, status e nomes de arquivo em inglês; comentários e interface em português. Veja `decisions/0006-redesign-omnimind.md` antes de mexer no grafo.

## Decisão aberta

A IA não renomeia nem arquiva memória da pessoa, mesmo a pedido no chat. Isso protege contra nota maliciosa, mas custa um passo no app. Detalhes em `qa/ai-sessions.md`.

## Como rodar

```bash
./scripts/bundle.sh
```

```bash
open "dist/Omnimind.app"
```

```bash
cargo test --workspace
```

```bash
cargo run --release -p app --bin proof-graph
```

```bash
cargo run --release -p app --bin shots -- docs/brand/screens
```

```bash
cargo run --release -p app --bin make-icon
```

```bash
python3 -m http.server 4173 --directory site
```

Provas na janela real: `--probe-ime`, `--probe-quit notes/a.md`, `--measure 240`, `--idle`, sempre com `-NSAppSleepDisabled YES`.

## Windows, Linux e site

Veja `decisions/0007-universe-platforms-and-follow-up-frame.md`. Falta:

1. Abrir o app numa máquina Windows e numa Linux. No CI o Ubuntu já compila e passa os testes sem GPU; o Windows compila.
2. Ver os jobs de macOS do CI rodarem com `macos-latest`. Uma tag `v*` gera a release, que nunca rodou.
3. O site está no ar em <https://jonathansantos.github.io/omnimind/>. Ele é público e o repositório é privado: os botões de código e de download só funcionam para quem tem acesso.
4. Ícone no executável de Windows e leitura da preferência de reduzir movimento fora do macOS.
5. Zoom, arrasto e pinça de Feixes e Raízes na janela real; ver `decisions/0008-graph-zoom-rings-and-reading-path.md`.

## Projetos

Veja `decisions/0009-project-is-a-note.md`. Falta:

1. A pessoa decidir a migração do cofre real: `omnimind projects --brain ~/Omnimind` mostra, `--apply` aplica.
2. Registrar os projetos sem nota, em Ajustes.
3. Separar os projetos dentro do grupo também em Raízes.
4. Podar os eventos de ferramenta antigos do diário, se o tamanho virar problema.

## Decisões pendentes da pessoa

1. **Fontes.** O protótipo aprovado usou Hanken Grotesk e IBM Plex Mono. O app segue com Familjen Grotesk e Sometype Mono, porque a troca exige baixar fontes. Falta aprovar o download.
2. **Nome visível de `/sonhar`.** O comando fica como está. A tela ainda mostra "Copiar /sonhar".
3. **Marca.** Checar o nome Omnimind em INPI e USPTO. Conflitos conhecidos em `brand/README.md`. O nome ficou por enquanto.

## Prioridade

1. **Sessão real com hooks.** O teste de IA roda com `--setting-sources ""`, e os hooks entram simulados. Falta `omnimind connect claude-code` num projeto real, em Claude Code interativo.
2. **Copilot de verdade.** `connect copilot` existe e nunca foi usado num VS Code real.
3. **Refazer a avaliação de IA.** `qa/ai-sessions` e o portão hostil não rodaram depois da renomeação.
4. **Zoom e deslocamento no grafo.** Não existem. Com milhares de itens, não há como chegar perto de uma região.
5. **Grafo de cofre vazio.** Hoje a tela é só texto e dois botões, sem desenho. Falta decidir o que a pessoa vê antes da primeira nota.
6. **Reservadas e teto de 5% do `/sonhar`.** Só há o teto de 40 memórias por sessão.
7. **Aliases no sono leve.** Paráfrase depende de aliases gravados pela IA. Três de 21 acertos fora da amostra vieram como `ok`; o resto veio como `weak`.
8. **Portão hostil.** Cinco de 108 variações ainda passam, e a lista é amostra. Senha nua digitada numa busca vai ao diário. Regra é triagem: quem protege de verdade é a origem, a quarentena e o desfazer.
9. **Atalhos de edição de texto no Perfil e na busca.** Campos simples têm só digitar, apagar, setas.
10. **Abas, arrastar nota para pasta, filtro por tipo de memória no grafo.** Faltam para quem vem do EdgeMind.
11. **Atalho e comando para Ajustes.** É a única tela fora de ⌘1 a ⌘6 e da paleta.
12. **Windows e Linux.** `gfx` só liga o backend Metal; `platform.rs`, `probe.rs` e o gancho de saída são macOS.
13. **Tiragem semanal:** um PNG do grafo com o que a IA aprendeu.

## Fora de escopo por decisão

**Acessibilidade:** fora deste MVP por pedido. Não há leitor de tela nem foco visível completo. Reduzir movimento só tira a abertura do grafo. Precisa virar prioridade antes de qualquer lançamento amplo, e com GPU própria é trabalho de verdade: exige expor a árvore de acessibilidade do macOS à mão.

## Problemas conhecidos

- `kill -9` ou queda de energia podem perder até 350 ms de digitação. Quit, fechar, trocar de nota e perder foco não perdem.
- A thread de dados faz polling a cada 250 ms em vez de usar FSEvents.
- Cada mudança refaz a visão inteira do cofre: 516 ms em 5.000 notas, fora do quadro, limitado a 15% de um núcleo.
- `journal::read_all` cresce com a história. Sem compactação de diário.
- Memória editada à mão que declara `authority: person` sem evento no diário perde peso e ganha `unverified_authority`, mas continua servida se não parecer hostil.
- Frase de instrução dentro de nota comum sai do briefing, mas ainda volta na busca, marcada e com aviso de que é dado.
- Memória correta pode nascer como `agent` quando a citação sustenta menos da metade do texto: conservador de propósito.
- A tag `project/` vem do `cwd` da sessão: quem fala de outro projeto ali herda a tag errada.
- `session-start` passa de 50 ms com dezenas de milhares de eventos no diário.
- Nome do tema vem da pasta da nota ou da tag `project/`; o resto cai em "Avulsos".
- Com oito temas ou mais, só os seis maiores têm nome e cor próprios; o resto vai para "Outros".
- As sete cores de tema não passam em todas as checagens de daltonismo. O nome sempre aparece, mas a cor sozinha não distingue.
- Feixes com 10.000 itens passa de 16,67 ms em p99: 23,53 ms, já a 1,5x.
- A curva de correções repetidas existe no núcleo, e nenhuma tela a mostra. Métricas só conta as correções.

## Não verificado

- Caminho interativo do Copilot, em VS Code.
- Hooks reais do Claude Code interativo.
- Avaliação de IA e portão hostil depois da renomeação. Prova de editor, acentos e Quit real depois do redesenho. Ver `qa/limits.md`.
- QA independente de produto e de design depois do redesenho.
- Migração da pasta privada em cofre real: só há teste automático.
- Remoção da entrada MCP antiga por `connect`: sem teste automático para esse caso.
- Se os clientes mandam `content` e `structuredContent` juntos ao modelo.
- Linux e Windows: nem compilados.
- Volumes de rede e pastas sincronizadas por nuvem.
- Tela ProMotion de 120 Hz e monitor externo. Só a tela de 60 Hz do M2.
- Horas de leitura nos dois temas.
- Interação à mão na janela real: os fluxos passam pelo harness e pelos eventos injetados; clique com mouse de verdade não foi exercitado por uma pessoa.
- Ver um exemplo, Conectar projeto e Trocar de cofre pelo seletor nativo de pastas.
- GPUI não passou pelas quatro provas: a comparação foi por pesquisa e compilação.

## Depois da revisão independente

- A revisão de código não foi repetida depois das correções.
- Se um cofre tiver `.varzea/` e `.omnimind/` ao mesmo tempo, a pasta antiga fica abandonada. A migração só age quando a nova não existe.
- A curva de correções repetidas saiu da interface junto com a tela antiga. O cálculo continua no núcleo.
