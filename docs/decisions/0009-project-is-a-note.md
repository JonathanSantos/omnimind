# 0009. Projeto é o que uma nota declara, com grupo acima dele

**Status:** implementada em 2026-11-10, a pedido da pessoa. Muda a spec do núcleo (`docs/specs/mvp/core.md`, escrita e resumo inicial) e o agrupamento do grafo.

## Contexto

Projeto era o nome da pasta onde a sessão rodou. No cofre pessoal, com 63 notas e 114 memórias, isso deu quatro problemas:

- **Dois mundos.** As 45 notas de `notes/projetos/` formavam o tema "Projetos". Os projetos novos existiam só como tag de memória, cada um um tema do mesmo nível. Nenhum tinha nota, e nenhuma nota tinha memórias agrupadas com ela.
- **Pasta descartável virou projeto.** Uma pasta `scratch-…` carregava 29 memórias: decisões de um produto, resumos de nove outros projetos (destilados das notas deles) e regras gerais da pessoa.
- **Regra geral presa a um projeto.** O resumo inicial entregava regras de processo da pessoa como coisa de outro projeto, que "não vale aqui sem confirmar".
- **Doze temas para sete cores.** Cada projeto novo empurrava outro para "Outros".

As notas de projeto da pessoa já declaravam `Grupo` e `Pasta`. O padrão existia; o produto não lia.

## Decisão

- **A nota declara o projeto.** `- **Grupo:** X` e `- **Pasta:** \`~/…\`` no cabeçalho, ou `group`/`folder` no frontmatter. `Apelidos` junta nomes antigos. O slug é o nome do arquivo. Tudo em `crates/core/src/projects.rs`, puro.
- **A pasta só leva até o projeto.** Sessão dentro da pasta declarada, ou numa pasta com o nome ou apelido do projeto, é do projeto. Pasta desconhecida vale como projeto provisório, com o nome dela, como antes. Pasta descartável (`scratch-*`, `/tmp`, `/var/folders`) nunca é projeto.
- **Uma memória, um projeto.** Quem decide, nesta ordem: a nota de onde ela foi destilada; a tag escrita que aponta para um projeto com nota; a pasta da sessão.
- **Em pasta descartável:** o título `projeto: assunto` leva ao projeto; regra e preferência ficam gerais; o resto guarda o nome da pasta, para não valer em todo lugar até a pessoa decidir.
- **Só a pessoa move.** `Change::SetProject` muda o projeto de uma memória ou a torna geral. Fica no diário e se desfaz como qualquer mudança. No app: "Mudar" no painel da memória, e "Projetos sem nota" em Ajustes, com Criar nota e Mover.
- **Apelido não exige migração.** O resumo e o grafo comparam pelo nome canônico: `project/toca-discos` conta como `vitrola` no dia em que a nota declarar o apelido.
- **Grafo em dois níveis.** A cor é o grupo. Dentro dele, cada projeto é um subtema: feixe próprio com nome em Feixes, sistema em volta da nota no Universo. A ficha "Projeto" troca a cor para o projeto, como era.
- **`omnimind projects`** mostra os projetos e o que as regras mudariam nas memórias que já existem. Só aplica com `--apply`.

Junto, quatro melhorias menores:

- **Título que é só uma resposta** ("Sim, usar a tabela") gera aviso na escrita e item `title` na agenda, para o `/sonhar` reescrever.
- **Diário.** O hook de ferramenta só registra o que mexe num arquivo do cofre. Antes, cada `Bash` e cada `Read` do projeto da pessoa entrava: 10.095 de 10.561 eventos.
- **Ligação por semelhança.** O índice liga cada item aos dois mais parecidos pelos termos raros que dividem, quando não há tag nem link entre eles. Só em cofres de até 3.000 itens.
- **Ligação global** aparece em Ajustes e na barra lateral.

## Alternativas

- **Pasta desconhecida sem projeto nenhum.** Era a proposta inicial. Memória sem projeto vale em todo projeto: uma decisão dita numa pasta nova vazaria para todos os resumos. O nome provisório isola até a pessoa registrar.
- **Achar o projeto por citação no texto.** Há projetos com nome de palavra comum: citar no meio de uma frase não prova nada. Só o começo do título, no formato `projeto:`, conta.
- **Recusar título sem contexto.** A regra é uma triagem com falso positivo. Recusar perderia memória boa; avisar e pôr na agenda não perde nada.
- **Apagar do diário os eventos de ferramenta antigos.** O diário é só de acréscimo e há vários processos gravando. Parou de crescer; o passado fica.

## O que não foi verificado

- O instantâneo mudou de formato (3). Processos antigos ainda abertos, como servidores MCP de sessões em andamento, refazem o próprio instantâneo: é correto, mas nunca foi medido com as duas versões ao mesmo tempo.
- As regras nunca rodaram no cofre real, só em cópias. A migração das memórias antigas espera a decisão da pessoa.
- A semelhança por texto foi conferida a olho em 28 pares do cofre pessoal, não medida contra um gabarito.
- Raízes não separa os projetos dentro do grupo: só Feixes e Universo fazem isso.
