# Golden set

`fixtures/brains/{personal,work}`: cópia dos brains de exemplo, sem `.index`, `.journal`, `.edgemind.lock`, `.trash`, `events/` e `inbox/`.

Perguntas:

- `fixtures/brains/*/schema/evals.md`: direta, morfologia e paráfrase, vindas dos brains de exemplo, com aliases gerados por IA na ingestão do EdgeMind.
- `fixtures/evals/<brain>.md`: abstenção e atualização, escritas para o Omnimind.
- `fixtures/evals/heldout-<brain>.md`: perguntas escritas por outro agente, que o autor do índice não leu. Quem mexer em busca não deve abrir esses arquivos.

Formato: `- pergunta -> título esperado` sob `## Classe`. Em `Atualização`: `- pergunta -> título vigente || título que não pode vencer`. Em `Abstenção`: `- pergunta`.
