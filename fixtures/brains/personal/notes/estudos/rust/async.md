---
title: Async with tokio
tags: [rust, estudo]
---
# Async with tokio

Notes from the tutorial. A future does nothing until polled. `tokio::spawn` needs `Send + 'static`, which is why holding a `MutexGuard` across `.await` fails to compile with the std mutex.

I do not need async in Cuscuz: SQLite calls are short and local. Leaving this here for later, probably a sync server. Decision so far is to stay sync and not pull tokio into the app.
