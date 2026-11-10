# Base de UI para app desktop 100% Rust (sem WebView, GPU, macOS primeiro)

Pesquisa de 2026-10-31. Máquina de medição: MacBook Apple M2, 8 núcleos, 24 GB, macOS (Darwin 25.6), `rustc 1.99.1`, **apenas Command Line Tools, sem Xcode** (`xcrun metal` não existe aqui, o que importa para o GPUI).

## 1. Resposta curta

**Vale prova prática, nesta ordem:**

1. **Base própria: `winit 0.30.13` + `wgpu 30.0.1` + `cosmic-text`/`glyphon`.** É o único candidato em que (3), (4) e (5) foram exercitados aqui: 2.000 nós, 6.000 arestas, 2.000 rótulos e bloom, offscreen a 2560x1600, com PNG e tempos de quadro (p50 de 10 a 12 ms, mas p99 de 20 a 27 ms, seção 5.3). O custo é escrever o editor e os widgets.
2. **GPUI via `gpui-kit 0.8.1` (`gpui-pre =0.3.10`).** Melhor resposta para (1) e (2): camada AppKit própria, editor pronto, Markdown pronto, menus e diálogos nativos. Compila sem Xcode com `runtime_shaders`. O risco está em (3)/(4): upstream **não tem gancho de shader**, e o caminho é renderizar em textura própria e passar um `CVPixelBuffer`/IOSurface para o elemento `surface()`. A variante `gpui-ce` + `custom-gpu` (wgpu 29, sem gpui-kit) compilou e abriu aqui, e é o plano B dentro desta mesma prova.
3. **egui 0.37 como controle de baixo custo.** É o framework mais em dia (wgpu 30, release em 2026-10-15), com `egui-wgpu` para passes próprios, `egui_kittest` para (5) e layout de texto barato (0,3 ms para 50 mil chars). Falta confirmar (2), e o desempenho vem de não fazer shaping complexo nem bidi.

**Não recomendados agora:** iced 0.14 (wgpu 27, sem release desde 2025-12, `master` depende de forks), Dioxus/Blitz (IME incompleto, beta/alpha), Slint (shaders só por textura externa com API `unstable-*`, licença tripla), Xilem/Masonry (sem release há um ano, autodeclarado experimental), Floem (crates.io de 2024, `main` em fork de winit), Makepad (só por git, 168 MB de repo, sem wgpu), Freya (Skia, sem wgpu; shaders SkSL e `CodeEditor` sem preedit).

Critérios de aceitação das provas na seção 11. Nenhum candidato teve teclas mortas testadas à mão nesta pesquisa: **(2) continua sem verificação empírica em todos**, e é o primeiro teste a fazer.

## 2. Método e limites

- Versões, datas e licenças vêm da **API do crates.io** e da **API do GitHub**, consultadas em 2026-10-31.
- Código, changelogs e READMEs lidos direto nos repositórios ou nas fontes baixadas pelo Cargo. Issues citadas foram abertas e lidas uma a uma, quando indicado.
- Medições feitas aqui, em diretório temporário, sem tocar no projeto. Cada "hello" em build limpo, `--release`, com `strip = true` e `lto = "thin"`. Todos os binários de janela listados na seção 4 abriram e ficaram 3 a 4 s no ar sem abortar; **o conteúdo das janelas não foi inspecionado**.
- **A cota de busca web da sessão esgotou no meio do trabalho.** Nenhuma busca geral foi feita: todas as fontes são primárias (crates.io, GitHub), localizadas por nome ou pela busca de issues do próprio GitHub (interrompida também nesse ponto). Pode haver discussões em blogs/fóruns não consideradas.
- `[medido]` marca o que foi executado aqui. `[README]` marca o que foi lido, mas não compilado. Tempos de build sofreram alguma concorrência com outras tarefas; use como ordem de grandeza.

## 3. Comparativo

Datas no formato AAAA-MM-DD. Estrelas e atividade em 2026-10-31.

| Candidato | Versão (data) | Licença | Manutenção | IME macOS | Shaders custom | Texto rico | Headless/imagem/eventos |
|---|---|---|---|---|---|---|---|
| **GPUI upstream** | `gpui` 0.2.2 (2025-10-22, **parado**); `gpui-pre` 0.3.10 (2026-10-26, snapshot `zed@08cbb88`); Zed v1.26.2 (2026-10-28) | Apache-2.0 (`editor`, `markdown`, `rope` do Zed são **GPL-3.0-or-later**) | Altíssima, 92k estrelas; API instável, snapshots quase semanais | Própria: `NSTextInputClient` em `gpui_macos`, com casos de tecla morta documentados no código | **Sem pipeline público.** `SurfaceSource` só aceita `CVPixelBuffer` | Runs estilizados; CoreText no macOS; sem bidi em `text_system` | `TestPlatform`, `simulate_keystrokes/input`, `HeadlessAppContext::capture_screenshot`; `render_to_image` aborta em janela viva ([#66344](https://github.com/zed-industries/zed/pull/66344) aberto) |
| **gpui-kit / gpui-component** | 0.8.1 (2026-10-26) | Apache-2.0 | Semanal, 18,8k estrelas; usado no Longbridge Pro | Herda GPUI; [#3456](https://github.com/longbridge/gpui-kit/issues/3456) (Cmd+Return durante tecla morta) aberto | Mesmo limite do GPUI | Editor com rope e Tree-sitter ("200K linhas" segundo o README), Markdown/HTML nativos, tabelas | Janelas headless, ponteiro/teclado, `gpui-fps` para quadros |
| **gpui-ce** (fork) | 0.3.0 (2026-10-20) | Apache-2.0 | Comunitário ativo, 1,4k estrelas | Fork do GPUI | **`custom-gpu`**: `WgpuContextHandle`, `WgpuRenderTarget` (exige `macos-wgpu`, **wgpu 29**) | **Parley** (bidi), API diverge do upstream | `headless_app_context`, `visual_test` |
| **gpui-fast** (fork) | 0.1.12 (2026-10-26) | Apache-2.0 | Longbridge, 315 estrelas, "experimental" | Fork do GPUI | Sem gancho conhecido | Igual GPUI | Igual GPUI; compatível com gpui-kit via feature |
| **Dioxus native / Blitz** | Dioxus 0.7.10 (2026-07-30), 0.8.0-alpha.3 (2026-10-16); Blitz 0.2.x e 0.3.0-beta.4 (2026-10-16) | MIT OU Apache-2.0 | Ativa; Blitz "beta" | **Incompleto**: PRs [#844](https://github.com/DioxusLabs/blitz/pull/844), [#1084](https://github.com/DioxusLabs/blitz/pull/1084), [#1086](https://github.com/DioxusLabs/blitz/pull/1086) abertos; `DomEventData::Ime` descartado | Textura wgpu em `<canvas>` (`wgpu_texture`) | HTML/CSS completo via Parley; viewer Markdown `rdme` | `blitz-test-harness`, `screenshot.rs` (vello_cpu), `native-headless` |
| **iced** | 0.14.0 (2025-12-07), `iced_winit` 0.14.1 (2026-09-13) | MIT | Ativa no `master`, 31,9k estrelas; `master` em `0.15.0-dev` | Suporte desde 0.14; [#1544](https://github.com/iced-rs/iced/issues/1544) e [#2674](https://github.com/iced-rs/iced/issues/2674) abertos; [#3232](https://github.com/iced-rs/iced/issues/3232) panic em preedit CJK | **Melhor integrado:** widget `Shader`, trait `Primitive` com `prepare`/`draw`/`render` | `rich_text`, `markdown` incremental, `text_editor` (cosmic-text 0.15) | `iced_test`: `Simulator` com `click`, `typewrite`, `snapshot` |
| **Slint** | 1.18.1 (2026-09-21) | GPL-3.0 OU royalty-free 2.0 OU comercial | Empresa, 24k estrelas, releases regulares | [#10862](https://github.com/slint-ui/slint/issues/10862) e [#10861](https://github.com/slint-ui/slint/issues/10861) abertos; [#3811](https://github.com/slint-ui/slint/issues/3811) API pública ausente | `unstable-wgpu-30`: textura externa ou `set_rendering_notifier`; **sem shader na linguagem** | `TextEdit`; macro `@markdown` | `take_snapshot`, `dispatch_event`; backend de teste não gera pixels |
| **egui / eframe** | 0.37.0 (2026-10-15) | MIT OU Apache-2.0 | Muito ativa, 31,9k estrelas, MSRV 1.95 | Visuais novos ([#8083](https://github.com/emilk/egui/pull/8083), 2026-06-25); [#4439](https://github.com/emilk/egui/issues/4439) "dead keys broken on mac" aberto desde 2024 sem confirmação recente | `egui_wgpu::CallbackTrait` (`prepare`, `finish_prepare`, `paint`) | `LayoutJob`, `TextEdit`; **sem shaping complexo/bidi** | **`egui_kittest`**: AccessKit + snapshot wgpu |
| **Makepad** | crates.io 1.0.0 (2025-05-13); `main` 2.0.0, commit em 2026-10-31 | MIT OU Apache-2.0 | Muito ativa no git, sem release | Corrigido em 2026-09: [#1223](https://github.com/makepad/makepad/pull/1223), [#1227](https://github.com/makepad/makepad/pull/1227) | DSL própria de shaders; backend Metal próprio; **sem pass wgpu** | Famílias `rich_text`, `code_editor` | Backend headless **sem raster**; validação real por grab em GPU |
| **Floem** | 0.2.0 (2024-11-14); `main` até 2026-09-24 | MIT | Lenta, 4,3k estrelas | PR [#1028](https://github.com/lapce/floem/pull/1028) e [#1024](https://github.com/lapce/floem/issues/1024) abertos | `post_paint`; `main` com vger/vello/Skia | Editor próprio com rope (usado no Lapce) | `HeadlessHarness` (eventos e hit-test, sem PNG evidente) |
| **Xilem/Masonry** | 0.4.0 (2025-10-29); `main` ativo | Apache-2.0 | Ativa no git; "experimental" | PRs [#1692](https://github.com/linebender/xilem/pull/1692), [#1700](https://github.com/linebender/xilem/pull/1700) abertos | Vello fechado; sem API de pass próprio encontrada | Parley; `TextArea` | `masonry_testing::TestHarness` com `render() -> RgbaImage` |
| **Freya** | 0.4.3 (2026-08-30), 0.5.0-rc.13 (2026-10-25) | MIT | Ativa, 3,3k estrelas, RC semanal | `Input` ok; [#2248](https://github.com/marc2332/freya/issues/2248) `CodeEditor` sem preedit | SkSL via `RuntimeEffect`; `RenderCallback` em canvas Skia; **sem wgpu** | Skia paragraph, `freya-markdown`, `freya-code-editor` | `freya-testing`: `write_text`, `press_key`, `render_to_file` |
| **Base própria** | winit 0.30.13, wgpu 30.0.1, cosmic-text 0.19.0, glyphon 0.12.0 | Permissivas | Ver seções 6 a 8 | winit 0.30.13 (seção 6) | Total | A escrever | Total: offscreen [medido] |

### Matriz de versões (aqui nascem conflitos)

| Consumidor | wgpu | winit | texto |
|---|---|---|---|
| iced 0.14.0 | **27** | 0.30 | cosmic-text 0.15 |
| iced `master` (0.15-dev) | 30 | **fork por git** | **fork por git** |
| egui 0.37 | **30** | 0.30.13 | próprio |
| Slint 1.18.1 | 29 ou 30 (`unstable-*`) | 0.30.2+ | parley 0.11 (opcional) |
| vello 0.11.0 | 30 | n/a | n/a |
| glyphon 0.12.0 | 30 | n/a | cosmic-text 0.19 |
| gpui-pre 0.3.10 | Metal direto no macOS; `gpui-pre-wgpu` usa 30 | AppKit próprio | CoreText; cosmic-text 0.19 no Linux |
| gpui-ce 0.3.0 | **29** | AppKit próprio | parley |
| Dioxus 0.8-alpha / Blitz 0.3-beta | 30 | **`=0.31.0-beta.3`** | parley 0.12 |
| Dioxus 0.7.10 | via anyrender 0.6 | 0.30.11+ | blitz 0.2 |
| Xilem 0.4.0 / `main` | vello 0.6 / 28 | 0.30.12+ | parley 0.6 / 0.8 |
| Floem 0.2.0 / `main` | 22 / 27 | fork `floem-winit` | cosmic-text 0.12 / parley 0.7 |

## 4. Medições de compilação e binário `[medido]`

Build limpo em release, com `strip` e ThinLTO, de uma janela mínima. `target` é o tamanho total após o build. `crates` é a contagem no `Cargo.lock`.

| Projeto | Build (s) | Binário | `target` | crates |
|---|---|---|---|---|
| `wgpu_metal` (offscreen PNG, sem janela) | 27 | 4,75 MB | 258 MB | 91 (árvore) |
| `wgpu_default` (features padrão) | 28 | 4,75 MB | 258 MB | idem |
| iced 0.14 (`wgpu` apenas) | 43 | 8,31 MB | 498 MB | 274 |
| eframe 0.37 (`wgpu`, sem glow) | 131 | 10,21 MB | 571 MB | 352 |
| Slint 1.18.1 (padrão, FemtoVG) | 135 | 10,61 MB | 826 MB | 551 |
| gpui-pre 0.3.10 (`font-kit`, `runtime_shaders`) | 125 | 6,77 MB | 891 MB | 603 |
| gpui-kit 0.8.1 (hello com botão) | 256 | 15,06 MB | 1.627 MB | 795 |
| gpui-ce 0.3.0 + `custom-gpu`, `macos-wgpu` (exemplo oficial) | 191 | 17,10 MB | 1.995 MB | 737 |
| dioxus-native 0.7.10 | 187 | 21,65 MB | 1.307 MB | 670 |
| **base** winit + wgpu + glyphon (janela e texto) | 90 | 7,50 MB | 466 MB | 223 |
| xilem 0.4.0 | 129 | 9,76 MB | 674 MB | 407 |
| freya 0.4.3 (Skia) | 75 | 18,37 MB | 1.224 MB | 454 |

Floem e Makepad não foram medidos: Floem publicado usa wgpu 22 de 2024, e Makepad exige dependência por git de um repositório de 168 MB.

O `target` foi medido antes de qualquer limpeza, somente em release; um build de desenvolvimento soma espaço equivalente ou maior. Freya compila rápido e tem binário grande, compatível com uso de Skia pré-compilado (não confirmado no log).

Dependências de apoio, cada uma isolada num `main` mínimo com uso real da API:

| Projeto | Build (s) | Binário | crates |
|---|---|---|---|
| `rusqlite` 0.41.1 `bundled`, consulta FTS5 com `remove_diacritics` | 31 | 2,09 MB | 23 |
| `tantivy` 0.27.0 (`mmap`, `lz4-compression`, `stemmer`) | 48 | 5,21 MB | 143 |
| `rmcp` 3.8.0 servidor stdio | 43 | 2,48 MB | 85 |

**Leitura:** o wgpu sozinho custa 4,75 MB, com naga incluso. O `gpui-pre` não usa wgpu no macOS e fica abaixo de iced e egui, mas o gpui-kit dobra o binário e leva 4 min e 1,6 GB de `target`. A base própria é a mais leve em compilação.

## 5. Medições de desempenho `[medido]`

### 5.1 Texto: documento Markdown em pt-BR com emoji e trechos RTL

Largura 800 px, fontes de sistema, release, sem builds meus em paralelo (outros processos do sistema seguiam ativos, com load em torno de 10). Com a fila de builds rodando, os mesmos testes ficaram de 2 a 6 vezes mais lentos, o que dá uma ideia do pior caso.

| Operação | 50 mil chars | 500 mil chars |
|---|---|---|
| `pulldown-cmark` 0.13.6, parse completo | 0,11 ms | 1,3 ms |
| `ropey` 1.6.1, inserção ou remoção | 0,35 µs | 0,36 µs |
| cosmic-text 0.19, layout do documento inteiro | 65 ms | 710 ms |
| cosmic-text, somente viewport de 900 px | 1,9 ms | 3,2 ms |
| cosmic-text, reshape de 1 parágrafo (cerca de 400 chars) | 0,42 ms | 0,45 ms |
| parley 0.12, documento inteiro | 11 ms | 97 ms |
| parley, somando parágrafos separados | 13 ms | 117 ms |
| parley, 1 parágrafo | 0,07 ms | 0,07 ms |
| egui 0.37, galley inteiro (sem shaping complexo) | 0,3 ms | 1,2 ms |
| egui, novo galley após editar 1 parágrafo | 0,01 ms | 0,05 ms |
| Criação de contexto de fontes (partida) | cosmic 13 a 20 ms; parley 28 a 34 ms | |

**Conclusões:**

- **Requisito (1) é questão de arquitetura, não de biblioteca.** `pulldown-cmark` e `ropey` nem aparecem no orçamento de 16 ms. O gargalo é layout, e só se feito no documento inteiro.
- **Um único `Layout` do Parley para a nota inteira gasta 11 ms a cada tecla**, dois terços do quadro, e passa de 16 ms com a máquina ocupada (medi 20 a 80 ms assim). Faça layout por parágrafo: 0,07 ms. Isso atinge qualquer framework que use Parley em bloco único, caso provável de `textarea` no Blitz e `TextArea` no Masonry (não medidos).
- **cosmic-text é lazy por linha**: viewport em 2 ms e parágrafo em 0,4 ms. Layout completo é 6 vezes mais lento que o Parley. Barra de rolagem precisa de altura estimada.
- **Parley é cerca de 6 vezes mais rápido que cosmic-text** por parágrafo e no documento. Ambos escalam de forma aproximadamente linear.
- **egui é 100 vezes mais rápido porque faz menos**: não há shaping complexo nem bidi, e `layout_job` divide por parágrafo com memoização. Para pt-BR basta; para RTL, não. O custo de desenhar e tesselar o `TextEdit` não está nesses números.

### 5.2 Offscreen wgpu

Triângulo por WGSL, cópia para buffer e PNG em 74 ms de ponta a ponta no M2, incluindo criação do device. Linhas de Cargo na seção 12.

### 5.3 Grafo: requisitos (3), (4) e (5) na base própria

Offscreen, 2560x1600, RGBA16F com bloom (4 passes de blur a meia resolução) e composição, 300 quadros, com todas as posições e arestas reescritas a cada quadro e `glyphon` preparando 2.000 rótulos. Código em WGSL próprio, sem framework. Três execuções, sem builds meus em paralelo:

| Métrica | Resultado |
|---|---|
| Criação de 4 pipelines | 0,9 a 1,0 ms |
| Shaping de 2.000 rótulos curtos | 47 a 56 ms |
| CPU por quadro, p50 / p99 | 2,1 a 3,5 ms / 3,7 a 6,8 ms |
| CPU + GPU por quadro, p50 / p99 | 10,0 a 11,7 ms / 19,8 a 27,2 ms |
| PNG final | 7,7 MB, inspecionado: nós, arestas, rótulos com acentos e brilho corretos |

**Leitura:** o p50 cabe em 60 fps a uma resolução maior que a de uso normal, e a CPU sobra. O **p99 passa de 16,7 ms**, portanto 60 fps sustentados ainda não estão provados. A medição espera a GPU quadro a quadro, roda em paralelo com o compositor do sistema, e sem qualquer otimização (bundles, culling com `rstar`, bloom em menos passes). O custo dominante é GPU, por preenchimento e blur. A prova prática precisa repetir isso em janela real, e o resultado para eventos injetados (5) está limitado a dados sintéticos, sem camada de UI.

## 6. winit e IME no macOS

**Versões:** `0.30.13` (2026-03-02) é a estável. `0.31.0-beta.4` (2026-10-24), com beta desde 2025-11. Quase todo o ecossistema está em 0.30; só Dioxus 0.8-alpha/Blitz adotaram a 0.31.

### Como uma tecla morta atravessa o winit 0.30.13

Lido em [`view.rs` v0.30.13](https://github.com/rust-windowing/winit/blob/v0.30.13/src/platform_impl/macos/view.rs):

1. `set_ime_allowed(true)` é obrigatório. Sem ele, `interpretKeyEvents` nunca roda e não há composição.
2. `´` chama `setMarkedText` e gera `Ime::Preedit("´", …)`.
3. `a` chama `insertText("á")` e gera `Ime::Preedit("", None)` seguido de `Ime::Commit("á")`.
4. Texto normal sem marked text **não** vira `Commit`: `insertText` só emite com `hasMarkedText` verdadeiro, e a tecla segue como `KeyboardInput` com `.text`. **O app precisa tratar as duas vias.**

### Bugs relevantes

| Item | Estado | Impacto para pt-BR |
|---|---|---|
| [#4537](https://github.com/rust-windowing/winit/issues/4537) setas suprimidas durante composição de tecla morta | Aberto (2026-03) | Médio: seta depois de `´` não move o cursor |
| [#3617](https://github.com/rust-windowing/winit/issues/3617) `selected_range` e `replacement_range` ignorados | Aberto | Baixo para pt-BR; o `TODO` segue em 0.30.13 |
| [#4807](https://github.com/rust-windowing/winit/issues/4807) Kotoeri perde conversão de kanji | Aberto (2026-10) | Nenhum para pt-BR; afeta japonês |
| [#3925](https://github.com/rust-windowing/winit/issues/3925) `insertText` sem preedit perdido | Fechado em 2026-09-02, mas o código de 0.30.13 ainda exige marked text; não localizei a versão da correção | Baixo; afeta SKK e pontuação CJK |
| [#2040](https://github.com/rust-windowing/winit/issues/2040) dead keys na 0.25 | Aberto, antigo (2021) | Obsoleto na prática: API de teclado mudou |
| [#4519](https://github.com/rust-windowing/winit/pull/4519) reescrita de IME no macOS | PR aberto, rascunho de pesquisa | É o que resolveria o conjunto; sem previsão |

### Mudanças da 0.31 que afetam seu código

- **Não há mais `Ime::Preedit("")` antes de `Ime::Commit`**; `Commit` limpa o preedit ([#4735](https://github.com/rust-windowing/winit/issues/4735)). Lógica que depende do preedit vazio quebra.
- Novo `Ime::DeleteSurrounding`, `Window::request_ime_update` atômico, `ImeHints`. `set_ime_allowed`, `set_ime_cursor_area` e `set_ime_purpose` foram **depreciados**.
- **macOS: winit deixa de registrar `NSApplicationDelegate`**, então você instala o seu. Muda a estratégia da seção 9.
- `ApplicationHandler` usa `dyn ActiveEventLoop`, `resumed` vira `can_create_surfaces`.
- Corrigido na beta.4: cliques encaminhados ao AppKit, marked text descartado ao desabilitar IME. Character Viewer sem commit: corrigido só em `unreleased`.

### Como apps reais fazem

| App | Abordagem |
|---|---|
| **Alacritty** ([`event.rs`](https://github.com/alacritty/alacritty/blob/master/alacritty/src/event.rs)) | winit puro. `Commit` vai para `paste`, `Preedit` vira estado de desenho, `Enabled`/`Disabled` ligam o modo. Modelo mais simples a copiar |
| **Rio** v0.5.28 | **Fork `rio-window`**. Faz correções próprias de SKK ([#1845](https://github.com/raphamorim/rio/pull/1845)) e porta o controle do Ghostty, em que `insertText:` só acumula e `keyDown` decide no fim ([#1847](https://github.com/raphamorim/rio/pull/1847), aberto) |
| **Zed/GPUI** | **Sem winit.** `NSTextInputClient` completo em Objective-C runtime; durante composição, a tecla vai primeiro ao input handler; comentários listam casos como `" space`, `" backspace` |
| **COSMIC** | Fork de iced + winit. IME no cosmic-edit é recente e mínimo ([#539](https://github.com/pop-os/cosmic-edit/pull/539), 2026-04; [#624](https://github.com/pop-os/cosmic-edit/issues/624) preedit não desenha sem redraw) |
| **Lapce** | Via Floem e `floem-winit`. Problemas com português foram fechados ([#1689](https://github.com/lapce/lapce/issues/1689), 2023) |

**Padrão observado:** projetos sérios acabam com fork ou camada própria. Quem precisa de IME perfeito sai do winit; para pt-BR, winit puro provavelmente basta, mas falta o teste manual.

## 7. wgpu

**Versão:** 30.0.1 (2026-08-22). `trunk` tem mudanças não publicadas. Breaking de 30.0: `VertexState.buffers` agora é `&[Option<…>]`, inteiros em shader I/O exigem `@interpolate(flat)`, `SurfaceConfiguration.color_space`. Em uso real em 30.0.1: `get_mapped_range()` retorna `Result`, e `SurfaceTexture::present()` deu lugar a `queue.present(frame)`.

- **Offscreen para PNG `[medido]`:** textura `RENDER_ATTACHMENT | COPY_SRC` em `Rgba8UnormSrgb`, `copy_texture_to_buffer` para buffer `MAP_READ`, `map_async`, `device.poll(PollType::wait_indefinitely())`, depois `png`. `bytes_per_row` precisa ser múltiplo de 256.
- **Só Metal `[medido]`:** binários **idênticos, 4.753.424 bytes**, com features padrão e com `default-features = false, features = ["std","parking_lot","metal","wgsl"]`. No macOS, `dx12` e `vulkan` já são cortados por `cfg`. Enxugar features não reduz nada aqui; só importa ao compilar para Linux e Windows. Remover `wgsl` também não ajuda: o `wgpu` ativa `wgpu-core/wgsl` em nativo.
- **Pipeline cache: no-op no Metal.** `wgpu-hal` em `metal/device.rs` devolve uma estrutura vazia em `create_pipeline_cache`. O changelog o introduziu para Vulkan ([#5319](https://github.com/gfx-rs/wgpu/pull/5319)). No Metal, dependa do cache de shaders do próprio sistema. Criar 4 pipelines custou cerca de 1 ms na medição da seção 5.3, portanto não é problema de partida neste porte.
- **Render bundles:** suportados, re-executáveis, e a documentação diz que "often" são mais eficientes. Bom para arestas e nós estáticos do grafo. Medir antes de adotar: instancing com um único draw já custa pouco.

## 8. Texto: cosmic-text, parley, swash + harfrust

| | cosmic-text 0.19.0 (2026-04-22) | parley 0.12.0 (2026-10-09) | swash 0.2.10 + harfrust 0.15.0 |
|---|---|---|---|
| Shaping | harfrust **0.5.2** (`Cargo.lock`) | harfrust **0.12** (`Cargo.lock`) | harfrust 0.15 acompanha HarfBuzz 15; swash também faz shaping |
| Fallback | Próprio, com listas do Chromium/Firefox, por caractere | `fontique`: enumeração e fallback, com mmap lazy | **Nada**: você implementa |
| Bidi | Sim | Sim, via ICU4X | Você implementa |
| Edição | `Editor` e `Buffer` por linha | `PlainEditor` | Você implementa |
| Rasterização | swash; `glyphon` para wgpu | Fora: vello, ou glifo + swash | swash |
| Modelo | **Por linha, lazy, incremental** | **Bloco inteiro**; [#432](https://github.com/linebender/parley/issues/432) incrementalidade aberta; `parley_engine` novo | Só peças |
| Risco | Release parada desde abril; iced usa fork | API muda a cada release; [#763](https://github.com/linebender/parley/issues/763) regressão de 20x corrigida | Muito trabalho |

Usar cosmic-text e parley juntos põe duas versões de `harfrust` (0.5.2 e 0.12) e de `skrifa` (0.40 e 0.44) no binário `[medido]`.

**Recomendação de texto:** para a base própria, **começar com cosmic-text + glyphon** para ter editor e atlas prontos. Manter o layout do preview atrás de uma interface, porque o Parley por parágrafo é 8 vezes mais rápido por bloco e tem futuro melhor. swash + harfrust diretamente não se justifica.

**Vello:** a organização mudou. O `vello` 0.11 compute é agora descrito como pesquisa/experimental, e `vello_gpu` 0.3.0 (2026-10-02, Sparse Strips; `vello_hybrid` parou na 0.2.0 em 2026-08-07, aparentemente seu antecessor) é o que pretende virar o renderizador de produção; `vello_cpu` é o mais maduro. Para 2.000 rótulos, instancing próprio e atlas de glifos é mais simples que qualquer Vello.

## 9. Nativo no macOS

| Crate | Versão (data) | Notas |
|---|---|---|
| `muda` | 0.21.2 (2026-10-08) | `menu.init_for_nsapp()`; eventos por `MenuEvent::receiver`, repassar a `EventLoopProxy` |
| `rfd` | **0.17.2** (2026-01-12) | Armadilha: o crates.io lista 0.16.1 como "newest" por ser backport de 2026-10-22; use 0.17 |
| `tray-icon` | 0.26.1 (2026-10-07) | Exige event loop na main thread; crie o ícone na main thread. Já traz `muda ^0.21` |
| `objc2` | 0.6.5 (2026-10-05) | `objc2-app-kit` 0.3.2. Licenças `MIT` (objc2) e `Zlib OR Apache-2.0 OR MIT` (app-kit) |

Slint fixa `muda ^0.19` e Freya `tray-icon ^0.21`: duas versões no grafo, com menus concorrentes se misturar. GPUI tem menus e diálogos próprios (`cx.set_menus`, `prompt_for_paths`); para bandeja existe `gpui-tray` 0.3.0.

### `applicationShouldTerminate:` e `terminate:`

- **winit 0.30.13:** registra `WinitApplicationDelegate` (nome confirmado em `app_state.rs`), que só implementa `applicationDidFinishLaunching:` e `applicationWillTerminate:`. Não dá para trocar o delegate sem perder eventos. Solução: depois do launch, `class_addMethod` para `applicationShouldTerminate:` nessa classe.
- **winit 0.31:** "Winit guarantees that it will not register an application delegate", e o exemplo oficial em `winit-appkit/src/lib.rs` usa `define_class!` e `setDelegate`. Muito mais limpo.
- **GPUI:** `GPUIApplicationDelegate` (confirmado em `gpui-pre-macos` 0.3.10) não implementa `applicationShouldTerminate:`. O projeto Cap adiciona o método via `class_addMethod` em [`apps/desktop-gpui/src/platform.rs`](https://github.com/CapSoftware/Cap/blob/main/apps/desktop-gpui/src/platform.rs), com verificação do nome da classe e guarda contra instalação dupla. Copiar esse desenho.
- **Atalho simples:** item Quit próprio no `muda` (não o `PredefinedMenuItem::quit`) para salvar e sair. Cobre Cmd+Q, mas **não** Dock, logout e `Sair` do sistema, que passam por `terminate:`. Para garantir persistência é preciso o delegate. Responder `NSTerminateLater` e depois chamar `replyToApplicationShouldTerminate:`.

### `.app` sem Xcode

Script validado nesta máquina `[medido]`: `iconutil` gerou `.icns`, `plutil -lint` passou, `codesign --verify` retornou *valid on disk* e o binário dentro do bundle rodou. O linker já assina ad-hoc o executável (`flags=adhoc,linker-signed`); o `codesign --force --deep --sign -` sela o bundle. `spctl --assess` responde **rejected**, como esperado sem Developer ID.

```sh
# 1. ícone a partir de PNG 1024x1024
mkdir -p App.iconset
for s in 16 32 128 256 512; do
  sips -z $s $s icon.png --out App.iconset/icon_${s}x${s}.png
  sips -z $((s*2)) $((s*2)) icon.png --out App.iconset/icon_${s}x${s}@2x.png
done
iconutil -c icns App.iconset -o App.icns

# 2. bundle
mkdir -p Rival.app/Contents/{MacOS,Resources}
cp target/release/rival Rival.app/Contents/MacOS/rival
cp App.icns Rival.app/Contents/Resources/
cp Info.plist Rival.app/Contents/

# 3. assinatura ad-hoc: refaça após qualquer alteração no bundle
codesign --force --deep --sign - Rival.app
codesign --verify --verbose Rival.app
```

`Info.plist` mínimo: `CFBundleExecutable`, `CFBundleIdentifier`, `CFBundleName`, `CFBundleIconFile`, `CFBundlePackageType=APPL`, `CFBundleShortVersionString`, `NSHighResolutionCapable=true`. `actool`, `iconutil`, `codesign` estão nas Command Line Tools.

`cargo-bundle` 0.12.0 (2026-09-20), não testado aqui, gera `.app` a partir de `[package.metadata.bundle]`, aceita `--binary-path`, universal por `-t` repetido. Não assina nem notariza. Assinatura ad-hoc serve para rodar localmente; distribuir para terceiros sem Gatekeeper reclamar exige Developer ID e notarização.

## 10. Crates de apoio

| Crate | Versão (data) | Licença | Notas |
|---|---|---|---|
| `ropey` | **1.6.1** (2023-10-18); 2.0.0-beta.1 (2025-08-02) | MIT | 1.x maduro e estável; repo ativo em 2026-09. Fique na 1.6 até a 2.0 sair |
| `pulldown-cmark` | 0.13.6 (2026-10-30) | MIT | 0,26 ms para 50 mil chars. Offsets via `into_offset_iter()`. Reparsar a cada tecla é viável |
| `notify` | 8.2.1 (2026-10-25); 9.0.0-rc.7 | **CC0-1.0** | `macos_fsevent` por padrão. Use `notify-debouncer-full`. Fique na 8.x |
| `tantivy` | 0.27.0 (2026-10-17) | MIT | Binário mínimo 5,21 MB, 2,5 vezes o do SQLite `[medido]`. Justifica-se só com ranking e tokenização avançados |
| `rusqlite` | 0.41.1 (2026-10-28) | MIT | `bundled` traz FTS5 ativo. `tokenize='unicode61 remove_diacritics 2'` achou `coração`/`ação` buscando `acao` `[medido]`. **Escolha padrão**: 2,09 MB e banco único também para metadados. Índice próprio não se paga |
| `rmcp` | **3.8.0** (2026-10-26) | Apache-2.0 | SDK oficial. MCP `2026-07-28`, compatível com `2025-11-25`. MSRV 1.88. Releases semanais. [Guia de migração 3.x](https://github.com/modelcontextprotocol/rust-sdk/discussions/969) |
| `fs4` | 1.1.0 (2026-04-28) | MIT OU Apache-2.0 | Feature `sync`. `std::fs::File::try_lock` compilou e rodou no rustc 1.99.1 `[medido]` e cobre o básico |
| `fd-lock` | 4.0.4 (2025-03-10) | MIT OU Apache-2.0 | Sem release há 19 meses; preferir `std` ou `fs4` |
| `arc-swap` | 1.9.3 (2026-10-18) | MIT OU Apache-2.0 | Publicar snapshot de leitura para a thread de render |
| `triple_buffer` | 9.0.0 (2026-02-22) | **MPL-2.0** | Copyleft por arquivo, aceitável como dependência; registre |
| `rstar` | 0.13.0 (2026-05-24) | MIT OU Apache-2.0 | Picking e culling do grafo |
| `petgraph` | 0.8.3 (2025-09-30) | MIT OU Apache-2.0 | `default-features = false` e só `stable_graph` |

## 11. Provas práticas

**P0, em todas (meio dia):** app mínimo com área de texto. Layout ABC ou Brasileiro no macOS, testar `´+a=á`, `~+a=ã`, `^+e=ê`, `` `+a=à ``, `´+b` (sem composição), `´+espaço`, seta durante composição, Cmd+Q durante composição, Character Viewer. **Elimina candidatos antes de qualquer outra coisa.**

| Prova | Critério de aceitação |
|---|---|
| **Editor** | Nota de 50 mil chars, digitação com preview ao vivo: p99 de tecla-até-quadro abaixo de 16 ms, reparse e relayout incrementais |
| **Grafo** | 2.000 nós, 6.000 arestas, rótulos, zoom e pan a 60 fps em janela real no M2, com p99 medido |
| **Efeitos** | Bloom e 10 mil partículas no mesmo pass ou compostos com a UI, sem tearing nem atraso de um quadro |
| **Harness** | Em processo, sem janela visível: script de eventos, PNG determinístico, tempo de quadro em CSV, inclusive com **preedit simulado** |

**Candidato 1, base própria:** risco concentrado em (1) e no custo de widgets. Reaproveitar `glyphon`, Taffy para layout e AccessKit. O harness sai barato porque o render já é offscreen.

**Candidato 2, GPUI:** implementar primeiro a ponte wgpu para `CVPixelBuffer` (IOSurface) e medir latência e cópia. Se a ponte for ruim, comparar com `gpui-ce` + `custom-gpu`. Fixar versão com `=` e prever trabalho a cada atualização de snapshot.

**Candidato 3, egui:** validar `TextEdit` com 50 mil chars e preview ao vivo. O layout é barato, mas o custo total inclui tessellation. Validar o P0, já que [#4439](https://github.com/emilk/egui/issues/4439) segue aberto. Se reprovar, usar iced como reserva, ciente do wgpu 27.

**Decisão:** se GPUI passar P0 e a ponte ficar abaixo de 1 quadro de latência, ele economiza meses de editor. Se a composição de efeitos com a UI for requisito central, a base própria vence.

## 12. Cargo.toml

### Compilou com rustc 1.99.1 em 2026-10-31 `[medido]`

Compilou e o binário abriu, salvo indicação. `Cargo.lock` não é portátil entre datas: use `=` e versionar o lock.

```toml
# Base própria (janela, texto com glyphon, e offscreen para PNG)
winit = { version = "=0.30.13", default-features = false, features = ["rwh_06"] }
wgpu = { version = "=30.0.1", default-features = false, features = ["std", "parking_lot", "metal", "wgsl"] }
glyphon = "=0.12.0"          # traz cosmic-text 0.19
pollster = "0.4"
bytemuck = { version = "1", features = ["derive"] }
png = "0.18"
```

```toml
# Texto e dados
cosmic-text = "=0.19.0"
parley = "=0.12.0"
pulldown-cmark = { version = "=0.13.6", default-features = false }
ropey = "=1.6.1"
vello = "=0.11.0"       # compilou; só criei a cena, sem renderizar. Compute ainda experimental
rusqlite = { version = "=0.41.1", features = ["bundled"] }
tantivy = { version = "=0.27.0", default-features = false, features = ["mmap", "lz4-compression", "stemmer"] }
rmcp = { version = "=3.8.0", features = ["server", "macros", "transport-io"] }
```

```toml
# Nativo: compilou; só construí os objetos, sem exercitar menu, diálogo ou bandeja
muda = "=0.21.2"
rfd = "=0.17.2"
tray-icon = "=0.26.1"
objc2 = "=0.6.5"
objc2-app-kit = "=0.3.2"
```

```toml
# GPUI upstream sem Xcode: runtime_shaders é obrigatório nesta máquina
gpui = { package = "gpui-pre", version = "=0.3.10" }
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.10", features = ["font-kit", "runtime_shaders"] }
```

```toml
# Kit completo: já pina gpui-pre =0.3.10 e ativa runtime_shaders no workspace dele
gpui-kit = "=0.8.1"
```

```toml
# gpui-ce com pass wgpu próprio. wgpu DEVE ser 29; exemplo custom_gpu.rs de HEAD compilou contra 0.3.0
gpui = { package = "gpui-ce", version = "=0.3.0" }
gpui_platform = { package = "gpui_ce_platform", version = "=0.3.0", features = ["font-kit", "macos-wgpu", "runtime_shaders"] }
gpui_ce_wgpu = { version = "=0.3.0", features = ["custom-gpu"] }
wgpu = "29"
```

```toml
iced = { version = "=0.14.0", default-features = false, features = ["wgpu", "thread-pool", "crisp"] }
```

```toml
eframe = { version = "=0.37.0", default-features = false, features = ["wgpu", "default_fonts", "accesskit"] }
```

```toml
slint = "=1.18.1"
```

```toml
dioxus = { version = "=0.7.10", default-features = false, features = ["native", "macro", "html", "hooks", "signals", "launch"] }
```

```toml
xilem = "=0.4.0"
winit = "0.30"
```

```toml
freya = "=0.4.3"
```

### Do README, não compilado aqui

```toml
slint = { version = "~1.18", features = ["unstable-wgpu-30"] }
gpui = { package = "gpui-fast", version = "0.1.0" }
gpui_platform = { package = "gpui-fast-platform", version = "0.1.0" }
makepad-widgets = { git = "https://github.com/makepad/makepad", branch = "main" }
floem = { git = "https://github.com/lapce/floem", branch = "main" }
freya = "0.5.0-rc.13"
gpui-kit = { version = "0.8", features = ["gpui-fast"] }
```

## 13. Armadilhas

1. **`gpui = "*"` do README do Zed não funciona como escrito.** `gpui_platform` **não existe** no crates.io e `gpui` 0.2.2 tem um ano. O uso real é `gpui-pre` com `package =` e pino exato, como faz o gpui-kit.
2. **Sem Xcode, GPUI falha ao compilar shaders** a menos que `runtime_shaders` esteja ativo (`xcrun metal` é chamado no `build.rs`). Com a feature, compila e abriu janela por 4 s sem abortar nesta máquina.
3. **`gpui-kit` não aceita caret em GPUI:** snapshots quebram API; o workspace usa `=0.3.10` por causa de [#3156](https://github.com/longbridge/gpui-kit/issues/3156).
4. **Licença:** `gpui` é Apache-2.0, mas `editor`, `markdown` e `rope` do Zed são GPL-3.0-or-later. Não copie deles em app fechado.
5. **`Window::render_to_image` aborta em janela macOS viva** por conflito com `CAMetalDisplayLink` ([#66344](https://github.com/zed-industries/zed/pull/66344), aberto). Use `HeadlessAppContext`.
6. **Skew de wgpu:** um app com egui (30), iced (27) e gpui-ce (29) compila três wgpu e não compartilha device.
7. **iced `master` não é publicável:** depende de forks por git de winit e cosmic-text.
8. **Bibliotecas em `unstable-*` no Slint** podem ser removidas em minor. Licença royalty-free exclui embarcados.
9. **`wgpu` 30 mudou assinaturas** que exemplos antigos usam; veja seção 7.
10. **`cosmic-text` 0.19 mudou `set_text`:** não recebe mais `FontSystem`, e setters ficaram lazy (`b.lines[i] = …` não marca sujo; use `line_layout`).
11. **Tray:** `tray-icon` exige event loop na main thread.
12. **winit 0.31 é migração, não upgrade:** eventos de IME, delegate e `ApplicationHandler` mudam juntos.
13. **`triple_buffer` é MPL-2.0** e `notify` é CC0-1.0: registre em auditoria de licença.
14. **`gpui-ce` exige `wgpu = "29"`** no seu código: com 30 os tipos não casam com `WgpuContextHandle`. Atualizar wgpu depende do fork.
15. **`macos-wgpu` troca o renderizador do GPUI no macOS** por um caminho opt-in novo. Não é o Metal testado em produção pelo Zed.
16. **Sem `set_ime_allowed(true)` não há composição** em winit, e fazer isso cedo demais ativa IME em toda a janela: ligue ao focar o editor. Trate `KeyboardInput.text` **e** `Ime::Commit`.
17. **Compilar com a máquina ocupada distorce benchmarks de texto** em até 6 vezes: as primeiras leituras daqui pareciam reprovar o Parley por isso. Rode harness com máquina ociosa e guarde p99.

## 14. Fontes

Todas consultadas em 2026-10-31.

**Registros:** [gpui](https://crates.io/crates/gpui), [gpui-pre](https://crates.io/crates/gpui-pre), [gpui-kit](https://crates.io/crates/gpui-kit), [gpui-ce](https://crates.io/crates/gpui-ce), [gpui-fast](https://crates.io/crates/gpui-fast), [iced](https://crates.io/crates/iced), [egui](https://crates.io/crates/egui), [slint](https://crates.io/crates/slint), [dioxus](https://crates.io/crates/dioxus), [blitz](https://crates.io/crates/blitz), [freya](https://crates.io/crates/freya), [xilem](https://crates.io/crates/xilem), [floem](https://crates.io/crates/floem), [makepad-widgets](https://crates.io/crates/makepad-widgets), [winit](https://crates.io/crates/winit), [wgpu](https://crates.io/crates/wgpu), [rmcp](https://crates.io/crates/rmcp).

**Repositórios e documentos:** [Zed `crates/gpui`](https://github.com/zed-industries/zed/tree/main/crates/gpui), [gpui-kit](https://github.com/longbridge/gpui-kit), [gpui-ce](https://github.com/gpui-ce/gpui-ce) e [`custom_gpu.rs`](https://github.com/gpui-ce/gpui-ce/blob/main/crates/gpui_wgpu/examples/custom_gpu.rs), [gpui-fast](https://github.com/longbridge/gpui-fast), [Blitz](https://github.com/DioxusLabs/blitz), [iced CHANGELOG](https://github.com/iced-rs/iced/blob/master/CHANGELOG.md), [`Primitive`](https://github.com/iced-rs/iced/blob/0.14.0/wgpu/src/primitive.rs), [`iced_test`](https://github.com/iced-rs/iced/blob/0.14.0/test/src/lib.rs), [Slint LICENSE](https://github.com/slint-ui/slint/blob/v1.18.1/LICENSE.md), [Slint `wgpu_30`](https://github.com/slint-ui/slint/blob/v1.18.1/api/rs/slint/lib.rs), [egui_kittest](https://github.com/emilk/egui/tree/0.37.0/crates/egui_kittest), [Makepad AGENTS.md](https://github.com/makepad/makepad/blob/main/AGENTS.md), [Floem headless](https://github.com/lapce/floem/blob/main/src/headless.rs), [masonry_testing](https://github.com/linebender/xilem/tree/main/masonry_testing), [freya-testing](https://github.com/marc2332/freya/blob/main/crates/freya-testing/src/lib.rs), [winit changelog 0.31](https://github.com/rust-windowing/winit/blob/master/winit/src/changelog/v0.31.md), [winit-appkit](https://github.com/rust-windowing/winit/blob/master/winit-appkit/src/lib.rs), [wgpu CHANGELOG](https://github.com/gfx-rs/wgpu/blob/trunk/CHANGELOG.md), [cosmic-text CHANGELOG](https://github.com/pop-os/cosmic-text/blob/main/CHANGELOG.md), [parley CHANGELOG](https://github.com/linebender/parley/blob/main/CHANGELOG.md), [harfrust](https://github.com/harfbuzz/harfrust), [Vello README](https://github.com/linebender/vello), [cargo-bundle](https://github.com/burtonageo/cargo-bundle), [rmcp](https://github.com/modelcontextprotocol/rust-sdk).
