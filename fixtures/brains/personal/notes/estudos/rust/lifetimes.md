---
title: Lifetimes, exercícios
tags: [rust, estudo]
---
# Lifetimes, exercícios

Fiz os exercícios do capítulo 10 em dois dias. Resumo do que aprendi:

- Lifetime não muda quanto tempo algo vive, só descreve relações entre referências.
- A assinatura `fn maior<'a>(x: &'a str, y: &'a str) -> &'a str` diz que o retorno vive no máximo tanto quanto o menor dos dois.
- Struct com referência precisa de parâmetro de lifetime. Quando começo a espalhar `'a` por tudo, em geral é sinal de que a struct deveria ser dona do dado.

Erro que mais cometi: devolver referência para uma variável local. A correção é devolver `String`.
