---
title: Ownership notes
tags: [rust, estudo]
---
# Ownership notes

Each value has one owner. When the owner goes out of scope the value is dropped. Assigning a `String` moves it; assigning an `i32` copies it because `i32` is `Copy`.

Borrowing rule I keep forgetting: many shared references OR one mutable reference, never both at the same time. The borrow checker uses the last use of a reference, not the end of the block (non-lexical lifetimes), so a lot of old advice about extra scopes no longer applies.

What finally clicked: think of `&mut` as "exclusive", not "mutable". See [[Lifetimes, exercícios]].
