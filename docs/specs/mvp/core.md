# Núcleo: dados, escrita, busca e ciclo de vida

Convenção: tudo que é código ou formato em disco fica em inglês; comentários, interface e texto de documento ficam em português.

Spec do crate `core` (biblioteca) usado pelo app, pelo servidor MCP e pelos hooks. O produto se chama Omnimind; a forma ASCII `omnimind` é usada em binário, pasta privada e identificadores.

Base: `docs/research/edgemind/*`, `docs/research/market.md`, `docs/research/technical.md`.

## 1. Princípios

1. **Disco é a verdade.** Notas e memórias são arquivos Markdown legíveis. Apagar o estado privado não perde conhecimento, só histórico e índice.
2. **Um único caminho de escrita.** App, MCP, CLI e hooks chamam `Brain::apply`. Não existe outra forma de alterar uma memória. Toda regra mora ali.
3. **Nada de índice compartilhado.** Cada processo mantém seu índice em RAM. O arquivo em disco é só um instantâneo para abrir rápido. Assim somem as classes de falha do EdgeMind: índice meio construído, dois indexadores, geração velha, leitor esperando escritor.
4. **Nunca chamar LLM.** O produto faz o que é determinístico. O que exige julgamento vira tarefa na `agenda`, executada pelo agente da pessoa, no turno dela e a pedido dela.
5. **Automático por padrão, reversível sempre.** A escrita não pede aprovação. Cada mudança gera um evento com o texto antes e depois, um motivo e a origem. Desfazer é outra mudança, nunca edição do histórico.
6. **A confiança vem da origem, não do texto.** O que a pessoa disse pesa mais que o que a IA deduziu, que pesa mais que o que veio de nota ou ferramenta. Consolidar herda o nível mais baixo das fontes.
7. **Silêncio é informação.** Toda resposta diz o quanto está completa. Vazio nunca significa "não existe" sem dizer o que foi coberto.

## 2. Pasta do brain

```
<brain>/
  brain.toml                  nome, id, idioma, política
  notes/**/*.md               da pessoa. A IA lê e nunca reescreve.
  memories/<type>/<slug>.md   destiladas. Editáveis à mão.
  .omnimind/                   privado, ignorado por git
    journal/<YYYY-MM>/<session>.jsonl  um segmento por processo
    snapshot.bin              índice serializado, descartável
    lock                      trava de escrita
    generation                um byte por escrita: aviso barato para os outros processos varrerem já
    ledger                    `id hash` por escrita do produto: prova de autoridade sem ler o diário
    tmp/                      temporários de escrita atômica, mesmo volume do destino
    plans/                    planos multi-arquivo idempotentes, retomados na próxima trava
    trash/<id>/               notas apagadas, reversíveis
```

- Notas podem estar em qualquer lugar do brain, exceto ocultos e `memories/`. Uma pasta do Obsidian abre sem mudança.
- `brain.toml` e `.gitignore` são criados em `init`. O `.gitignore` cobre `.omnimind/`. Cofre criado com a pasta privada do nome antigo migra uma vez em `Brain::open`: ver `decisions/0006-redesign-omnimind.md`. Transcrições nunca entram em nada versionado.
- Symlink nunca é seguido, dentro ou fora do brain: é pulado na varredura, recusado em leitura e em escrita, e contado em `refused_symlinks`. É mais estrito que "recusar o que sai do brain", de propósito.

### Memória

```markdown
---
id: 01JB7Y...            ULID estável; o nome do arquivo pode mudar
type: preference         decision | procedure | concept | episode | preference
title: Prefere commits pequenos
summary: Uma frase que basta para decidir se vale abrir.
aliases: [commits atômicos, small commits]
tags: [projeto/rota-certa, area/git]
authority: person        person | agent | external
origin:
  kind: correction       correction | request | session | note | tool | consolidation | import
  client: claude-code
  session: abc123
  quote: "não, faz commits menores"
  note: notes/projects/rota.md
  hash: 7f3a...
created: 2026-10-31T19:20:00Z
updated: 2026-10-31T19:20:00Z
supersedes: [01JB...]    opcional
expires: 2026-12-31      opcional
quarantine: instruction_like   só quando o portão decidiu
---
Corpo em linguagem descritiva: fato com data e origem, nunca ordem.
```

Compatível com a ideia do OKF, sem depender dele. Chave desconhecida no frontmatter é preservada em texto ao regravar. Estados **derivados, nunca gravados**:

| Estado | Regra | Efeito na leitura |
|---|---|---|
| `active` | nenhum dos abaixo | servida |
| `stale` | hash da nota de origem mudou, ou citação quebrada | servida com aviso e rebaixada |
| `expired` | passou de `expires` | só com `include_old` |
| `superseded` | outra memória ativa tem `supersedes` para ela | só com `include_old` |
| `quarantined` | campo `quarantine`, ou conteúdo suspeito na leitura, ou autoridade `external` sem promoção | nunca servida ao agente, nunca em briefing, fora da exportação |
| `archived` | `archived: <data>` no frontmatter | só com `include_old` |

A exclusão é imposta no código da leitura, não por instrução. A busca avisa quantas ficaram de fora: `"hidden": {"superseded": 2, "quarantined": 1}`.

Exceção deliberada a "nunca gravados": `quarantine` é gravado. O portão inspeciona campos que não vão para o arquivo (`reason`, `key`, tag recusada); sem o campo, o arquivo limpo voltaria a parecer ativo. A inspeção na leitura continua valendo para arquivo editado por fora. `promoted` + `promoted_hash` (impressão do conteúdo) tiram da quarentena só aquele texto: editou, volta.

**Toda leitura inspeciona**, em qualquer autoridade: `authority: person` escrito no arquivo não dispensa nada. Suspeita vira `quarantined` com motivo. Texto estranho escrito pela própria pessoa fica ativo porque `apply` o grava com `promoted_hash`.

**Autoridade do arquivo precisa de evento.** `person` só vale se o hash atual do arquivo veio de uma escrita do produto, registrada no diário. Sem isso, a memória é tratada como `agent` no ranking e no perfil, sai com `flags: ["unverified_authority"]`, e o briefing diz "arquivo sem registro". A mesma prova vale para `promoted_hash`: promoção forjada no arquivo não tira da quarentena. Efeito colateral aceito: apagar `.omnimind/` faz as memórias da pessoa perderem essa prova, sem perder texto.

Superseded, derivado a cada varredura: quem substitui não vale se estiver em quarentena, arquivada ou vencida. Desfazer a substituta devolve a antiga sem tocar nela.

## 3. Diário de eventos

Uma linha JSON por evento, em segmento próprio do processo. Hooks só anexam. Leitores unem os segmentos por `ts` + `seq`. `O_APPEND` com uma única chamada `write`; linha truncada no fim é ignorada e contada.

```json
{"ts":"...","seq":12,"session":"abc","client":"claude-code","ev":"wrote","target":"01JB...",
 "before":"...","after":"...","reason":"...","rule":"direct-correction","undoes":null}
```

| Evento | Quando |
|---|---|
| `wrote`, `merged`, `superseded`, `archived`, `restored`, `undid` | mudança em memória, sempre com antes e depois |
| `quarantined`, `promoted`, `rejected` | decisão do portão de escrita |
| `read`, `searched` | uso pelo agente, com consulta, resultados e completude |
| `cited`, `helped`, `hurt` | reforço explícito ou por sinal |
| `signal` | correção, pedido ou preferência capturados por hook |
| `external` | arquivo mudou fora do produto |
| `covered` | nota lida para destilar, com hash |
| `session` | início, fim e compactação, com cliente e projeto |
| `note` | nota gravada pelo produto (`saved`, `renamed`, `deleted`, `restored`), com hashes |
| `tool` | ferramenta usada, só nome e caminho; saída nunca entra no diário |

Todo evento tem `id` (ULID) e hora em microssegundos. Eventos de mudança trazem `before_hash` e `after_hash`, que formam a cadeia usada pelo desfazer. Segredo nunca entra: `reason`, consultas e sinais passam por redação; escrita com segredo gera `rejected` sem o conteúdo. Ao abrir um segmento, se a última linha ficou sem `\n` (`kill -9`), o processo começa com `\n`, para a linha cortada não colar na próxima.

`external` é emitido sob a trava, relendo o diário antes: vários processos que veem a mesma edição geram um evento só. Evento adiado cujo hash já não é o do arquivo é descartado. Memória sem evento correspondente (processo morto entre `rename` e anexar, ou arquivo criado à mão) vira `external` na reconciliação. Em nota, só depois da primeira varredura completa.

O diário alimenta o app ao vivo, a linha do tempo, o relatório de saúde e o aprendizado de aliases. O app segue os segmentos por `notify`; se o FSEvents perder algo, a releitura periódica corrige.

## 4. Caminho de escrita

```rust
Brain::apply(Change, Actor) -> Result<Applied>
Applied { outcome, id, rule, hash, event, warnings, neighbors, tags, tag_vocabulary, current_text }
Outcome::{ Written, Merged{into}, Superseded{ids}, Quarantined{reason}, Rejected{reason, hint}, Unchanged }
```

Regras que vieram do catálogo de falhas do EdgeMind e valem para o agente:

- `Actor` vem do transporte. Campo da requisição nunca concede autoridade.
- Agente só grava `person` com `origin_kind` `correction` ou `request`, sem achado suspeito, e com **sustentação**: `quote` de ao menos 20 caracteres, literal, de um `signal`, com o detector de instrução limpo no trecho citado, e ao menos 50% das palavras significativas de `title + summary` (depois do analisador) presentes na citação. Data e palavras de atribuição, como `pessoa` e `disse`, não contam. Faltou algo: grava como `agent`, com aviso em `warnings`. Sem `quote`, o núcleo usa o sinal recente que o texto cobre: isso registra a origem e fecha o sinal, mas a sustentação continua valendo.
- Editar memória da pessoa como agente exige a mesma sustentação. Senão `Rejected(cannot_edit_person_memory)`, com dica para gravar uma nova, que abre conflito na agenda. Arquivar memória da pessoa só pelo app ou pela CLI. Edição de agente com essa sustentação promove memória `agent` a `person`.
- Trocar o título renomeia o arquivo antes do evento, e desfazer segue o id até o arquivo atual. Depois de desfazer, o nome só volta a acompanhar o título na próxima troca.
- `supersedes` sobre memória de autoridade maior: `Rejected(cannot_supersede_higher_authority)`. Vale também para `origin_kind: consolidation`.
- Agente não reescreve, arquiva nem restaura memória retida: `Rejected(memory_held)`.
- `current_text` nunca traz memória retida nem segredo: nesses casos só o `hash`. O mesmo vale para `neighbors`, `tag_vocabulary` e agenda. Tag de memória retida não entra no vocabulário.
- Fonte `note` exige a nota lida por inteiro nesta sessão, no hash atual. O hash é calculado pelo produto.
- `preference` exige citação com sustentação, achada em sinal ou na nota.
- Memória `stale` da mesma nota e tipo, com tema próximo: `Rejected(must_supersede)`, com o id nos vizinhos.
- Mais de 40 memórias novas numa sessão de agente: `Rejected(session_growth_limit)`. Substituição não conta.
- Limites: título 200, resumo 400, corpo 20.000, 24 aliases, 12 tags.

Ordem fixa, sob trava de escrita (`try_lock` com espera curta; trava morre com o processo):

1. **Normaliza** campos curtos e calcula a chave de idempotência (`key` do cliente ou hash do conteúdo). Repetir a mesma escrita devolve `Unchanged` com o id original.
2. **Inspeciona o texto bruto**, em todos os campos, inclusive `reason`:
   - invisíveis **por classe Unicode**, não por lista: Cf, Co, Cc, Zl, Zp, Cn, branco Braille, formatação musical, seletores de variação fora de emoji;
   - disfarce: largura total, matemáticos, círculos, palavra que mistura alfabetos;
   - bloco `---` no corpo;
   - instrução, sobre um esqueleto: entidades HTML decodificadas, NFKC, sem acento, minúsculas, confusáveis comuns (`ı ɡ ɪ ɢ ɴ ᴏ ʀ ᴇ`, cirílico, grego), pontuação e hífen como espaço. Casa por proximidade: verbo de ignorar, esquecer, desconsiderar, não seguir ou sobrescrever a até 5 palavras de instrução, regra, prompt ou orientação, em PT e EN; ordem de ocultar da pessoa; shell (`curl -`, `wget http`, `nc -e`, `bash <(`, `python -c`, pipe para `sh` ou `tee`); exfiltração (`~/.ssh`, `id_rsa`, `.env` com enviar, copiar, post); marcadores (`<|system|>`, `[SYSTEM]`, `authority:person`); bloco base64 longo, ou curto que decodifique para instrução;
   - segredo (formatos conhecidos de chave e token, `senha=`).

   **Isto é triagem, não garantia.** Sinônimo novo, outra língua, ROT13 e paráfrase passam: dos 108 casos do QA, 5 ainda passam. Falso positivo também existe (`cargo test | tee log` numa memória de agente fica retido). A defesa de verdade é a origem: autoridade, quarentena e o aviso de que conteúdo é dado. Tags passam pelo mesmo detector com `-` e `/` trocados por espaço. Sinais, briefing e agenda usam o mesmo detector.
3. **Decide** por autoridade:
   - segredo: `Rejected`, sempre;
   - invisível ou instrução escrita pela pessoa: grava sem o invisível, com aviso e promoção própria;
   - invisível ou instrução em origem não-pessoa: `Quarantined`;
   - conteúdo que só repete a nota: `Rejected`, dica "a nota já é lida pela busca";
   - autoridade `external`: `Quarantined` até haver promoção.
4. **Compara e troca.** Toda alteração traz `base_hash`. Sem ele, ou divergente, `Rejected` com o texto atual.
5. **Procura colisão.** Funde só conteúdo idêntico depois de normalizar: título, resumo e corpo iguais, ou corpo novo vazio com título e resumo iguais. Mesmo título com corpo diferente **nunca** funde. Quase-duplicata (mesmo título, SimHash próximo, Jaccard ≥ 0,7) grava como memória nova e aparece na agenda como `near_duplicate`, ou `contradiction` se a heurística achar conflito. Nenhuma escrita some sem evento: fusão com alias ou tag novos gera `merged` com antes e depois, reversível; repetição sem nada novo responde `Unchanged`, e, se aquele texto já foi substituído, avisa em `warnings` em vez de ressuscitá-lo. A mesma `key` com outro conteúdo: `Rejected(key_reused)`.
6. **Herda taint:** consolidar memórias usa a menor autoridade entre as fontes. Memória `agent` citando nota não vira `person`.
7. **Grava** em arquivo temporário, `sync_all`, `rename`, e anexa o evento. O índice do próprio processo atualiza na hora.
8. **Devolve** o que aconteceu, a regra aplicada e o id. A resposta inclui memórias vizinhas (mesma nota, mesmo tema) para o agente ligar `supersedes` sem ter de procurar.

Teto de crescimento: mais de 40 memórias novas numa sessão pede resumo em vez de lista. Teste de aceitação: escrita hostil em todos os campos nunca chega a uma resposta de busca.

## 5. Busca

Índice próprio em memória, em Rust puro. Sem tantivy nem SQLite (SQLite é C, e a regra do produto é Rust puro; ver `docs/decisions/`).

- **Postings** por campo com BM25F: título 5, alias 4, nome do arquivo 3,5, tag 3, resumo 2, corpo 1.
- **Analisador:** minúsculas, sem acento, stemmer PT e EN (`rust-stemmers`), plurais irregulares PT, partes de identificador (`rotaCerta`, `rota_certa`), mais a forma literal com peso 2.
- **Consulta:** `queries: [1..3]`, cada uma com OR + bônus de todas as palavras; fusão por RRF. Correção de typo no vocabulário (distância 1), só quando nada volta.
- **Grafo:** bônus para vizinhos de wikilink a um salto do melhor resultado.
- **Rankeamento de memória:** `bm25 × peso_estado × peso_autoridade × ativacao`, com `ativacao` crescendo por uso útil e caindo por sessões sem uso. Exposição nunca reforça. Decaimento só ordena; arquivar é decisão separada.
- **Instantâneo** gravado por `rename` atômico após mudança relevante, ao sair, e a cada 30 s no máximo. Abertura: carrega instantâneo e reconcilia.

### Coerência antes de cada consulta

```
se passou mais de 250 ms desde a última varredura:
    percorre a árvore com stat (tamanho, mtime_ns, ctime_ns, inode)
    arquivo mudou: relê e reindexa
    tocado há ≤ 2 s da última leitura: relê, no máximo 128 por varredura; não conta como indexação
    arquivo sumiu: remove; mas segura 300 ms antes de concluir (editores que gravam por rename)
```

Durante a carência o arquivo sumido não é mostrado, e `coverage.unread` o conta. A primeira varredura do processo não tem carência: o instantâneo é que está velho.

**Ler o que outro processo escreveu.** Os 250 ms deixariam uma janela. Cada escrita do produto anexa um byte a `.omnimind/generation`; consultar é um `stat`. Tamanho mudou, varre na hora. Perder esse arquivo só devolve a janela de 250 ms. Edição por editor externo continua com até 250 ms.

**Orçamento.** Reconciliação com prazo (`reconcile_slice`, MCP) conta o prazo só a partir da fila, processa ao menos um arquivo por fatia e retoma sem refazer o passeio. Sem isso, sob carga o passeio gastava o prazo inteiro e nada era indexado; o benchmark pegou (`indexing_makes_progress_with_a_zero_budget`).

Sem daemon, sem watcher no núcleo. Medido com 5.000 arquivos sob load 25: varredura de 25 a 40 ms. Meta: busca total ≤ 100 ms. Toda resposta traz:

```json
"coverage": {"memories": 412, "notes": 5000, "hidden": {...}, "unread": 3, "indexing": 0, "scanned_ms_ago": 4}
```

`status`: `ok`; `weak`, quando nenhum resultado cobre ao menos metade do IDF das palavras exigidas, ou quando veio de correção de typo; `no_match`; `incomplete`, com `reason`, se há indexação em curso ou arquivo ilegível. `weak` devolve resultados e não finge certeza. A dica ao agente é ler os melhores antes de reformular.

Stopword não é descartada: pontua pelo IDF, mas não é exigida nem conta na cobertura. Em corpus pequeno, descartá-la custou 0,21 de paráfrase@5.

## 6. Ciclo de vida automático

### Sono leve (determinístico, sem IA)

Roda em fatias curtas, no fim de sessão do MCP e ao abrir o app. Nunca bloqueia resposta. Cada ação gera evento reversível.

| Ação | Regra |
|---|---|
| Funde duplicatas | Mesma regra da escrita: só conteúdo idêntico, mesma autoridade. Par cuja fusão a pessoa desfez fica marcado e não refunde |
| Aprende alias | busca sem clique → reformulação → leitura em ≤ 3 passos: a primeira consulta vira alias da memória lida. Três ocorrências para promover. Consulta que o detector barra não vira alias |
| Marca envelhecida | hash de origem mudou |
| Promove sinal | a mesma correção em sessões distintas vira memória `person` |
| Vigia lacuna | consulta repetida sem resultado vira item de agenda. Consulta hostil não vira item |
| Rebaixa | sem uso útil em N sessões do projeto: desce no ranking; depois sugere arquivar na agenda |

### Sono profundo (a pedido, com a IA da pessoa)

A tool `memory_agenda` devolve o trabalho que exige julgamento, em lotes pequenos, sempre com ids e citações:

- notas novas, alteradas ou com correção sem cobertura;
- memórias envelhecidas para revisar contra a fonte;
- contradições;
- quase-duplicatas;
- lacunas (consultas sem resposta);
- sinais ainda não destilados.

Título, detalhe e citação passam pelo mesmo filtro do briefing: o que ele barraria sai vazio, com o motivo em `flags`.

O agente resolve com `remember`. A cobertura é contada pelo produto, não pelo agente: a agenda só esvazia quando o hash da nota tem evento `covered`.

Reservadas: toda rodada tem perguntas guardadas (do histórico) que ela não vê. Encolher conta como melhora. Teto de 5% de crescimento por rodada. **Não implementado:** hoje só existe o teto de 40 por sessão.

## 7. Captura de correções

Sem LLM, em duas camadas:

1. **Hook `UserPromptSubmit`:** classificador de regras sobre a mensagem da pessoa (`não, `, `na verdade`, `errado`, `eu disse`, `pare de`, `sempre`, `nunca`, `prefiro`, `gosto de`, `de novo`). `pare de`, os pedidos de registro, como `anota` e `guarda`, e a pessoa dizendo quem é, como `me chamo`, valem no começo de qualquer oração. Memória da pessoa com essa citação abre o briefing em "Quem conversa com você agora". Sinal com `[REDACTED]` fica no diário, mas não é pendência. Se bater: anexa evento `signal` com o texto literal e devolve contexto factual breve ao agente, dizendo que a mensagem parece correção e que `remember` existe.
2. **Hook `Stop`/`SessionEnd`:** sinais não destilados da sessão ficam na agenda. Sinais já valem como memória fraca: entram no briefing como "a pessoa disse, em tal data: ...", com texto literal e autoridade `person`, antes mesmo de a IA destilar.

Falsos positivos custam pouco: o sinal é literal, curto, e some da agenda se a próxima escrita o cobrir ou se a pessoa desfizer.

## 8. Briefing

`briefing(cwd, client) -> texto ≤ 7.000 caracteres`, com cortes declarados. O hook usa `briefing_light`, que lê só `memories/` e sinais dos dois meses mais recentes, em paralelo, sem abrir o índice. Frase de instrução fica fora em qualquer autoridade. Ordem: correções recentes, preferências firmes, decisões do projeto, procedimentos, ponteiros. Só `active`. Cada linha traz data e origem, em tom de fato, e `[projeto]` quando a memória pertence a um. Com `cwd`, o corpo traz só o projeto atual e o que não tem projeto. De outros projetos entram até 12 títulos, sem episódios e com procedimentos primeiro, sob "Outros projetos: não valem aqui sem confirmar". Correção aberta de outro projeto fica nele. Sem `cwd`, nada é outro projeto. Termina dizendo quantas memórias ficaram de fora e como buscá-las.

**Projeto** é o que uma nota declara (`Grupo`, `Pasta`, `Apelidos`), não o nome da pasta. O `cwd` é resolvido por `Projects::current`: pasta declarada ou apelido levam ao projeto da nota, pasta desconhecida vale com o próprio nome, e pasta descartável não tem projeto. As tags são comparadas pelo nome canônico. O hook lê a lista em `.omnimind/projects.json`, gravada junto com o instantâneo. Regras completas em `decisions/0009-project-is-a-note.md`.

## 9. Testes obrigatórios do núcleo

- 28 danos ao estado privado: resposta idêntica ao índice recriado.
- Tempestade de processos: 8 escritores, 8 leitores, sem perda nem pânico.
- Oráculo diferencial: busca contra varredura do disco.
- Mesma edição com mesmo tamanho e `mtime`: detectada.
- Escrita hostil em cada campo: nunca servida.
- Fato substituído nunca sai na busca padrão.
- Rename de nota não sequestra link de outra nota.
- `kill -9` no meio da escrita: sem arquivo parcial; evento e arquivo coerentes na reconciliação.
- Golden set em PT-BR com direta, morfologia, paráfrase, abstenção e atualização. Medido nos brains de exemplo copiados, com perguntas que o autor do índice não viu.

## 10. Como rodar

```sh
export CARGO_TARGET_DIR=$PWD/target/core
cargo test -p omnimind-core -p omnimind-cli
cargo test -p omnimind-core --test golden golden_recall -- --nocapture
cargo test --release -p omnimind-cli --test bench -- --ignored --nocapture
```

O benchmark falha se estourar uma meta.

## 11. Números medidos

Última rodada em 2026-11-01, depois do novo detector, macOS arm64, release, load average de 7 a 11, com a máquina compartilhada. 5.000 notas de 3 KB e 500 memórias fictícias, vocabulário de 20.000 palavras em frequência tipo Zipf.

| Medida | Meta | Medido |
|---|---|---|
| Busca com varredura de `stat`, p50 / p95 | ≤ 100 ms | 38,6 / 67,4 ms |
| Busca em rajada, 2 reformulações, p50 / p95 | | 3,4 / 5,0 ms |
| 1ª busca após escrita, mesmo processo, p50 / p95 | ≤ 100 ms | 0,1 / 0,1 ms |
| 1ª busca após escrita, outro processo, p50 / p95 | ≤ 100 ms | 31,3 / 37,3 ms |
| Escrita com `sync_all`, p50 / p95 | | 29,9 / 39,0 ms |
| Abertura sem índice (5.500 arquivos) | | 1,18 s (era 0,75–0,81 s: inspeção de segredo e detector novo) |
| Abertura com instantâneo + busca, processo inteiro, p50 | | 118,1 ms |
| `initialize` / primeira busca pronta | | 5,2 / 106,4 ms |
| RSS em repouso, em uso / recém-aberto | ≤ 150 MB | 103,5 / 100,7 MB |
| Instantâneo em disco | | 19,4 MB |
| Hook `session-start` com 560 memórias, p50 / p95 | < 50 ms | 24,1 / 35,7 ms |
| Hook `prompt`, p50 / p95 | < 50 ms | 2,0 / 2,2 ms |
| Binário | pequeno | 2,7 MB |

Sob load 46, em rodadas anteriores ao detector novo: abertura sem índice 2,2 s, escrita p50 de 96 ms, hook `session-start` p95 de 49,8 ms. O QA mediu `session-start` p50 de 69,8 ms com 429 memórias e 76 mil eventos; isso segue aberto. RSS de pico durante a gravação do instantâneo não foi medido.

### Golden set, recall@5

Brains de exemplo copiados: `personal` (18 notas, 9 memórias) e `work` (20 notas, 10 memórias). A importação preserva `aliases`, `supersedes` e a memória já velha.

| Classe | Perguntas vistas pelo autor do índice | Perguntas não vistas (held-out) | EdgeMind, mesmas perguntas |
|---|---|---|---|
| Direta | 9/9 = 1,000 | 12/12 = 1,000 | 1,000 |
| Morfologia | 8/8 = 1,000 | 12/12 = 1,000 | 1,000 |
| Paráfrase | 10/14 = 0,714 | 21/24 = 0,875 | 0,786 (11/14) |
| Abstenção | 12/12 = 1,000 | 11/12 = 0,917 | não medida |
| Atualização | 6/6 = 1,000 | não há | não medida |

Leitura honesta:

- **Fáceis:** direta e morfologia. **Difíceis:** paráfrase.
- As perguntas de `schema/evals.md` foram vistas ao ajustar a busca (a mudança de stopwords foi decidida olhando para elas), e abstenção e atualização foram escritas pelo próprio autor. Por isso o held-out existe.
- O held-out foi escrito por outro agente, que leu as memórias. Ele tem viés lexical possível, e cada memória é alvo de até 3 perguntas, porque os brains são pequenos. O 0,875 acima do 0,714 pode refletir perguntas mais fáceis, e não um índice melhor. Não é prova de paráfrase ≥ 0,9.
- A paráfrase depende dos `aliases` gerados por IA no EdgeMind. Sem bons aliases na escrita, o número cai.
- Abstenção conta `weak` ou `no_match`. `weak` ainda devolve resultados; quem abstém é o agente, ao ler o `status`.
- **`weak` discrimina pouco.** Dos 21 acertos de paráfrase do held-out, só 3 saíram com `ok`; os outros 18 acertaram com `weak`. Ou seja: o 0,917 de abstenção é barato, porque quase toda pergunta sem palavra-chave literal cai em `weak`. O sinal correto para o agente é "leia e confira", não "não existe".
- Controle positivo: `golden_fails_when_index_is_off` desliga o índice e exige zero resultados com o disco cheio.

## 12. O que não foi verificado

Falso positivo do detector em uso real, medido só no golden set (nenhuma memória importada retida); Claude Code real, Copilot CLI e VS Code reais (só o protocolo e os formatos documentados, contra o binário); se o cliente envia `content` e `structuredContent` ao modelo, caso em que uma resposta custaria o dobro de tokens; Windows e Linux (nem compilado); volumes de rede e pastas sincronizadas; queda de energia (o `kill -9` não a simula; a durabilidade de `rename` no APFS não é documentada); `ctime` em sistema sem esse campo; brains acima de 5.000 notas.
