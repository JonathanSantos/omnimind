---
title: Importar receitas da planilha
aliases: [migrar planilha, import spreadsheet]
tags: [projeto/cuscuz, procedimento]
---
# Importar receitas da planilha

Passo a passo que usei e que funciona:

1. Exportar a planilha como CSV em UTF-8. No formato padrão os acentos quebram.
2. Rodar `cuscuz import --dry-run receitas.csv` e olhar a lista de unidades desconhecidas.
3. Corrigir as unidades em `unidades.toml`. "Copo americano" são 190 ml.
4. Rodar de novo sem `--dry-run`.
5. Conferir 10 receitas no app, escolhidas ao acaso, antes de apagar qualquer coisa.

Na primeira rodada 12 receitas caíram porque tinham o rendimento escrito por extenso ("serve seis").
