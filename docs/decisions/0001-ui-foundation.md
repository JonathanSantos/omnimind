# 0001. Base de UI: winit + wgpu + cosmic-text próprios

**Status:** adotada em 2026-10-31. Confirmada por `docs/research/frameworks.md`.

## Contexto

A interface tem de ser nativa, em Rust, por GPU, e o grafo é o momento de impacto: precisa de shaders, HDR, bloom e instancing sem pedir licença ao framework. O prompt exige quatro provas nesta ordem: editor, acentos, grafo, efeitos.

## O que foi medido

Tudo em M2, 2560×1600 ou 2880×1800, sem janela, com a máquina sob load 25 a 31 por outros processos.

| Prova | Resultado | Como reproduzir |
|---|---|---|
| Editor, nota de 50.188 caracteres | Teclada visível: CPU p50 1,25 ms, p99 3 ms, máximo 12,8 ms em 3 rodadas de 880 teclas, cada uma moldando 1 linha | `cargo run --release -p app --bin proof-editor` |
| Rolagem da mesma nota | p50 1,6 ms; p99 de 7,7 a 17,3 ms somando a espera síncrona pela GPU | idem |
| Acentos no layout real Brazilian Pro | `áãõéç''kê` com preedit e commit corretos, por `NSApp sendEvent:` com `NSEvent` vindo de `CGEvent` | `spikes/ime` |
| Grafo, 2.000 nós, 3.998 arestas, 5.158 glifos | CPU+GPU p50 2,45 ms, p99 6,32 ms; 1 de 300 quadros acima de 16,6 ms | `spikes/own` |
| Binário do spike de grafo | 7,1 MB | idem |

Primeira versão da prova do editor era inválida: a linha editada estava fora da tela, e o contador `shaped 0` denunciou. Corrigida com `reveal_caret`.

Montar `NSEvent` à mão com `characters: ""` não aciona a composição. O evento precisa vir de `CGEvent`. Antes disso o teste parecia reprovar o winit, sem culpa dele.

## Alternativas

| Opção | Por que não |
|---|---|
| GPUI | `gpui` no crates.io está em 0.2.2, de 2025-10-22, e `gpui_platform` não existe lá; o uso real é `gpui-pre` com pino exato. Sem gancho de shader no upstream, e o grafo é o coração do produto. Compilou e abriu janela na pesquisa (15 MB, 256 s, 1,6 GB), mas não passou pelas quatro provas: nenhuma foi feita nele. |
| egui 0.37 | Atual e com wgpu 30, mas sem shaping complexo nem bidi, e a issue #4439 de dead keys no macOS segue aberta |
| iced, Slint, Makepad, Freya, Dioxus nativo, Floem, Xilem | Resolvem widgets, e o produto quase não usa widgets padrão. iced ainda está em wgpu 27; demais motivos em `docs/research/frameworks.md` |

## Decisão

Base própria em camadas: `gfx` (cena instanciada, atlas de glifos, HDR, bloom), `ed` (modelo do editor sem GPU), `graph` (física em thread própria), `app` (telas).

Custos aceitos: construir rolagem, listas e campos de texto, e renderizar preedit. Mitigação: o modelo imediato reduz cada widget a poucas linhas, e o harness roda o app real sem janela.

## O que reabre esta decisão

- `winit` falhar com IME real, o que levaria a uma `NSView` própria com `NSTextInputClient`, como recomenda a pesquisa de interface.
- Falta de seleção/IME pesar mais do que o ganho de shaders próprios.
