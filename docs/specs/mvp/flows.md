# Fluxos

## Primeira vez

1. Abre o app sem cofre: Boas-vindas, com um grafo de mentira ao lado.
2. **Começar** cria `~/Omnimind`. **Abrir uma pasta** serve para uma pasta de Obsidian. **Ver um exemplo** abre um cofre fictício.
3. O app vai para o Grafo. Em cofre vazio, a tela oferece **Escrever uma nota** e **Conectar sua IA**.
4. Ajustes → Conexões → **Conectar projeto…** roda `omnimind connect` no projeto escolhido e avisa quando terminar. **Copiar comando** dá a mesma coisa para rodar à mão. Em Projetos, o interruptor desliga e religa; vale a partir da próxima sessão, e o cofre não muda.
5. Na primeira sessão da IA, o grafo acende.

Com cofre, o app abre direto no Grafo.

## Uma sessão com a IA

| Momento | O que acontece | O que aparece |
|---|---|---|
| Início | Hook entrega o briefing, com cortes declarados | Nada a fazer |
| Busca e leitura | Evento no diário | No Grafo, o caminho até o item acende. Na barra lateral, "Claude Code leu..." |
| Pessoa corrige | Hook grava o sinal literal, antes de qualquer IA | Nada, ao vivo. Conta em Métricas, em Correções suas |
| IA grava | Portão decide: grava, funde ou retém | No Grafo, o item nasce dentro do tema. Na barra lateral, "Claude Code aprendeu..." |
| Fim | Sono leve em fatias | Aviso: "A sessão trouxe 2 memórias." |

## Revisar sem aprovar

Não há inbox. Em Atividade, cada sessão mostra o que mudou, o motivo e o antes e depois. **Desfazer** tira uma mudança. **Desfazer sessão** tira a sessão inteira, tudo ou nada.

Memórias retidas pelo portão ficam em revisão e nunca chegam à IA. Aparecem em Memórias e em Métricas → Para revisar. No painel de detalhe, **Aprovar** só surge depois que o corpo carregou, com os invisíveis à mostra.

## Quando a IA aprende errado

1. Perfil: clique na frase e corrija. Vale como autoridade da pessoa.
2. Atividade: desfaça, com o motivo à vista.
3. Memórias, Grafo ou Métricas: abra o painel e **Arquivar**, que é reversível.

## Saber o que está acontecendo

Métricas responde: o que é mais lido, que temas importam, o que a IA procurou e não achou, quem usa o cofre. Cada linha leva a uma ação: abrir a nota, abrir a memória, revisar, criar a nota que faltou.

## Melhorar o cofre

Em Métricas → Para revisar, a linha **Para melhorar com a IA** tem **Copiar /sonhar**. A pessoa cola na IA dela, que gasta os tokens dela, e a skill percorre a agenda: notas nunca lidas, contradições, memórias envelhecidas, lacunas.

## Texto seguro

| Transição | Regra |
|---|---|
| Parar de digitar | Grava em 350 ms; teto de 2 s |
| Trocar de nota | Grava, confirma, só então abre |
| Perder foco | Grava |
| Mudança externa, sem edição | Recarrega mantendo o cursor |
| Mudança externa com edição ou gravação em voo | Conflito, com duas saídas: **Manter a minha** e **Guardar as duas** |
| Segundo evento durante conflito | Só atualiza o texto de fora |
| Sair em conflito | Guarda a sua numa nota irmã |
| Falha ao gravar | Texto segue aberto, tenta de novo |
