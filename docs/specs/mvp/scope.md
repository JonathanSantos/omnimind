# Escopo do MVP

## A aposta

Dá para abandonar a aprovação manual e confiar mais na memória, desde que ela seja **visível, explicável, reversível e defendida em código**. Se a pessoa nunca precisar abrir uma caixa de entrada e ainda souber o que a IA sabe, a aposta está provada.

## Entra

| Item | Problema | Por quê |
|---|---|---|
| Notas Markdown com preview ao vivo, wikilinks, acentos | 7 | Sem escrever bem, a memória morre |
| Núcleo: memórias em disco, diário, caminho único de escrita | 3, 4 | Base da confiança |
| Portão de escrita determinístico com quarentena | 3 | Sem humano antes de gravar, o portão é o código |
| Busca própria com cobertura honesta | 1, 5 | Vazio nunca mente |
| MCP com 4 tools, hooks e skill para Claude Code; MCP para Copilot | 1, 2 | É por aqui que a memória ajuda |
| Captura de correção sem LLM | 2, 8 | "Aprende sem pedir" |
| Sono leve automático, `/sonhar` a pedido | 4, 8 | Melhora sozinho, sem gasto próprio |
| Grafo vivo, em Feixes e Raízes | 5 | O uau, e a leitura do que a IA faz |
| Atividade com diff, motivo e desfazer | 3, 5, 8 | Substitui a inbox |
| Perfil editável | 2, 3 | Mostra o que a IA acha de você; editar é corrigir |
| Métricas reais, tiradas do diário | 5, 8 | Mostra o que é lido, o que falta e o que revisar |
| Vários cofres isolados | 6 | Um servidor, um cofre, sem tool que aceite caminho |
| Curva de correções repetidas, no núcleo | 8 | Prova de que melhora; a tela mostra só Correções suas |

## Fica de fora, e por quê

| Item | Motivo |
|---|---|
| Acessibilidade | Fora de escopo por pedido; ver `docs/next-steps.md` |
| Windows e Linux | macOS primeiro; `gfx` usa wgpu e o custo fica em `platform.rs` e no backend |
| Zoom, deslocamento e arrastar no grafo | Fora do redesenho; ver `docs/next-steps.md` |
| Importar de Obsidian com conversão | Abrir a pasta já funciona |
| Tabelas renderizadas, math, Mermaid, imagens | Não provam a aposta |
| Tiragem semanal em PDF | Boa ideia de compartilhamento, depois do uso real |
| Sync e colaboração | Fora das travas |

## Critérios de pronto

1. Uma pessoa nova instala o `.app`, cria um cofre, conecta o Claude Code, conversa, corrige a IA, e na sessão seguinte a IA acerta sem que ela peça para lembrar.
2. As provas de `docs/qa/limits.md` passam no app final, com números reais. A de efeitos deixou de se aplicar com o redesenho.
3. Travas medidas com 5.000 notas.
4. Nenhuma perda de texto no gerador aleatório nem nas saídas (`Quit`, fechar, trocar de nota).
5. Escrita hostil em qualquer campo não chega à resposta da busca.
6. Evidência de melhora ao longo de sessões reais de IA, dentro de US$ 3.
7. QA independente de produto e de design, com os problemas tratados ou declarados.
