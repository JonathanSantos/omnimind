---
title: Modelo de dados das receitas
tags: [projeto/cuscuz]
---
# Modelo de dados das receitas

Receita tem `titulo`, `porcoes`, `tempo_min`, `origem` e `passos`. Ingrediente tem `nome`, `quantidade`, `unidade` e `escala`, que pode ser `linear`, `nao_escala` ou `limitada`.

A unidade é sempre gravada em gramas ou mililitros, e a interface converte para xícara e colher na hora de mostrar. Uma xícara são 240 ml. A colher de sopa são 15 ml. Isso evita a bagunça da planilha, onde cada linha usava uma medida.

Veja [[Escala de porções]] para o campo `escala`.
