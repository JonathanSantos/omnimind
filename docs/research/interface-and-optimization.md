# Interface e otimização: técnicas concretas para um app desktop Rust + GPU

Pesquisa feita em 2026-10-31. Todas as fontes foram acessadas nessa data; a data entre parênteses é a de publicação, quando foi possível obtê-la.

Contexto: app desktop 100% Rust, GPU nativa (wgpu/Metal), meta de 60 fps ou mais, com grafo de milhares de nós, editor de texto e efeitos.

**Legenda de confiança**

| Marca | Significado |
|---|---|
| [P] | Fonte primária: código-fonte, documentação ou blog do próprio projeto, artigo original |
| [S] | Secundária: espelho de git, wiki gerada, engenharia reversa, trecho de busca sem leitura da página |
| [A] | Anedótica: relato em fórum, afirmação de crate pequena, número sem método |
| [R] | Recomendação ou dedução minha para este projeto, sem fonte direta |
| [M] | Lembrado de memória, não conferido nesta pesquisa |

As lacunas estão na seção 8. O orçamento de buscas acabou antes do fim, então alguns temas ficaram sem fonte.

---

## 1. Renderização de UI por GPU

### 1.1 Zed / GPUI

**Primitivos instanciados.** GPUI desenha só cinco primitivos: retângulos, sombras, texto, ícones e imagens. Cada um tem shader próprio, em vez de um renderizador vetorial genérico. Cada lista de primitivos vira uma chamada de desenho instanciada, com a caixa delimitadora como dois triângulos e o preenchimento no fragment shader. [P] [zed.dev/blog/videogame](https://zed.dev/blog/videogame) (2023-03-07)

- Orçamento: 8,33 ms por quadro a 120 Hz, incluindo atualização de estado, layout e escrita no framebuffer.
- Ordem por camada: sombras, depois retângulos, glifos, ícones e imagens. Assim um retângulo nunca cobre um glifo da mesma camada. Tooltips e afins abrem um novo `Layer`.
- Layout estilo Flutter: restrições descem, tamanhos sobem.

**SDF de retângulo arredondado.** Espelha o ponto para o quadrante superior direito, encolhe o retângulo pelo raio, calcula a distância e subtrai o raio. O pixel é preenchido com distância menor ou igual a zero. [P] (mesma fonte) Forma compacta usual [R]:

```wgsl
fn sd_round_rect(p: vec2f, half: vec2f, r: f32) -> f32 {
    let q = abs(p) - half + r;
    return length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - r;
}
// cobertura com AA analítico: clamp(0.5 - d, 0.0, 1.0), com d em pixels físicos
```

**Sombras sem passe de blur.** Técnica de Evan Wallace: para retângulos, solução fechada com `erf` em cada eixo, porque o blur é separável. Para retângulos arredondados não há forma fechada em 2D: faz a integral exata em x e aproxima y com 4 amostras em cerca de ±3σ. [P] (mesma fonte)

**Apresentação Metal (o post mais útil para este projeto).** [P] [zed.dev/blog/120fps](https://zed.dev/blog/120fps) (2024-02-07)

- Sem sincronização, o macOS estica o quadro anterior até o novo chegar.
- `wait_until_completed` bloqueava a thread principal. `wait_until_scheduled` reduziu o bloqueio, mas criou corrida entre CPU e GPU na mesma memória.
- Solução: pool de buffers de instância. Cada quadro pega um e devolve em `add_completed_handler` quando a GPU termina.
- Quadros abaixo de 4 ms ainda davam engasgos com repetição de tecla a 10 ms. A causa era o ProMotion reduzindo o clock do painel.
- `CADisplayLink`: apresentar um drawable a cada quadro mantém a taxa alta; pular um quadro deixa cair. GPUI redesenha quando sujo e, caso contrário, reapresenta a cena por 1 s depois de `last_input_timestamp`.
- Correção final de um limite de 60 fps: chamar `present_drawable` antes de `commit()`, remover `wait_until_scheduled` e desligar `presentsWithTransaction` em regime. `presentsWithTransaction` fica só para inicialização e redimensionamento.

**Estado atual.** O código atual usa `FrameSource` com `CAMetalDisplayLink` no macOS 14+ e `CVDisplayLink` nos anteriores. O link é desanexado (não pausado) para permitir desenho síncrono via `nextDrawable`. [P] [gpui_macos/src/window.rs](https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui_macos/src/window.rs)

Em dezembro de 2025, o Zed passou a apresentar quadros só durante entrada de alta frequência, como rolagem, e não após cada evento, reduzindo a GPU em repouso. Também ativou direct-to-display, que evita a composição. Há aí uma tensão com o achado de 2024, em que manter apresentação evitava queda do ProMotion: a prática atual restringe o custo ao momento em que ele importa. [P] [zed.dev/blog/quality-week-december-2025](https://zed.dev/blog/quality-week-december-2025) (dez. 2025, sem números)

**wgpu no Zed.** Em fevereiro de 2026, o Zed trocou Blade por wgpu só no Linux. Sobre Windows e macOS, um mantenedor escreveu que os renderizadores nativos "probably have better performance and wider compatibility than WGPU". Janela wgpu vazia: cerca de 100 MB contra 10 MB de GPUI no Windows, relato de um usuário que outro contestou. [P/A] [PR #46758](https://github.com/zed-industries/zed/pull/46758) (2026-02-13)

### 1.2 Outros projetos

| Projeto | Mecanismo | Como mantém o quadro | Números e fonte |
|---|---|---|---|
| **Figma** | Motor próprio baseado em tiles (2015); C++ para WASM; WebGPU desde 2025 | Memória pré-alocada evita pausas de GC; uniforms enviados em lote via `encodeDraw()`/`submit()`; bind groups reaproveitados; menos render passes | [2015-12-07](https://www.figma.com/blog/building-a-professional-design-tool-on-the-web/), [2025-09-18](https://www.figma.com/blog/figma-rendering-powered-by-webgpu/). Sem ganhos numéricos. Readback assíncrono acrescentaria centenas de ms ao startup, por isso o fallback é no meio da sessão. Planos: compute para blur, RenderBundles |
| **Linear** | Réplica local em IndexedDB; objetos em pool | Leitura sempre local; escrita aplica em memória e enfileira | Seção 5.6 |
| **Warp** | Rust + Metal; só 3 primitivos (retângulo, imagem, glifo) | Shaders pequenos, glifos rasterizados uma vez, poucas trocas de estado | Cerca de 200 linhas de shader; redesenho médio de 1,9 ms em uma semana; mais de 144 fps em 4K. [P] [How Warp Works](https://www.warp.dev/blog/how-warp-works) |
| **Ghostty** | Thread de leitura, de escrita e de render por terminal; parser com SIMD | Renderer nunca bloqueia no lock do terminal (`tryLock`); render guiado por wakeup, não por timer | Seção 5.5. [P] [README](https://raw.githubusercontent.com/ghostty-org/ghostty/main/README.md), [Thread.zig](https://raw.githubusercontent.com/ghostty-org/ghostty/main/src/renderer/Thread.zig) |
| **Alacritty** | Instancing em OpenGL; atlas 1024×1024 em prateleiras (shelf); lote de 65 536 instâncias | Damage por linha; dois passes (fundo e texto) sobre as mesmas instâncias | [P] [atlas.rs](https://raw.githubusercontent.com/alacritty/alacritty/master/alacritty/src/renderer/text/atlas.rs), [glsl3.rs](https://raw.githubusercontent.com/alacritty/alacritty/master/alacritty/src/renderer/text/glsl3.rs), [damage.rs](https://raw.githubusercontent.com/alacritty/alacritty/master/alacritty/src/display/damage.rs) |
| **Sublime Text** | OpenGL 4.1; lotes de glifos | Agrupar glifos foi a principal alavanca | Em 1440p com RX560: 52 ms sem lote; 17 ms (x4); 8 ms (x16); 3 ms (x8192). [P] [artigo](https://www.sublimetext.com/blog/articles/hardware-accelerated-rendering) |
| **Lapce / Floem** | Rust; wgpu via vger ou vello; Taffy para layout; fallback tiny-skia | Sinais de granularidade fina; árvore de views construída uma vez; listas virtuais | [P] [README Floem](https://raw.githubusercontent.com/lapce/floem/main/README.md). Núcleo de rope derivado do Xi. Sem benchmarks |
| **Rio** | Sugarloaf; WebGPU e variantes nativas Metal/Vulkan que dispensam wgpu | Damage e redraw agendado; `strategy = "game"` força contínuo para diagnóstico | [S] [sugarloaf](https://docs.rs/crate/sugarloaf/latest/source/src/sugarloaf.rs), [A] [discussão #1803](https://github.com/raphamorim/rio/discussions/1803). Sem números |
| **Vello** | Compute 2D; ramo novo "sparse strips" | Hybrid: caminho na CPU e rasterização fina na GPU, sem exigir compute | `vello_common`/`vello_cpu` 0.0.6 em 2026-01-15. A doc diz que o CPU está à frente do hybrid; máscaras e filtros só no CPU. [S] [vello_api](https://docs.rs/crate/vello_api/latest), [sparse_strips](https://skia.googlesource.com/external/github.com/linebender/vello/+/refs/heads/main/sparse_strips) |
| **Rive** | Pixel Local Storage; fallback por interlock e leitura/escrita; MSAA 4x em Qualcomm | Feathering calculado analiticamente na geometria, sem filtro de convolução | Curva gaussiana integrada; 50% de cobertura na borda; curvas exigem integral 2D. Sem medições. [P] [blog](https://rive.app/blog/how-rive-reinvented-feathering-for-the-vectorian-era) (2025-04-02), [docs](https://rive.app/docs/runtimes/cpp/renderers.md) |

Referência teórica: tiles de 16×16 px dão cerca de 12 mil tiles numa janela de 2048×1536; tigre do Ghostscript em cerca de 2,8 ms numa Iris 640; damage regions seguem úteis principalmente por energia. [P] [Levien, 2019](https://raphlinus.github.io/rust/graphics/gpu/2019/05/08/modern-2d.html)

### 1.3 O que adotar [R]

1. Renderizador próprio com poucos primitivos instanciados, não vetor genérico. Vello e parecidos ficam para ícones vetoriais rasterizados e cacheados.
2. Um pipeline e um buffer por tipo de primitivo, ordem fixa por camada, um `draw_indexed(0..6, 0, 0..n)` por lote.
3. Sombra por `erf`, sem blur em tempo real para UI.
4. Tiles ou textura offscreen por camada, de modo que digitar não redesenhe o grafo (seção 5.4).

---

## 2. Texto

### 2.1 Pilha Rust

| Peça | Papel | Estado |
|---|---|---|
| **HarfRust** | Shaping; port do HarfBuzz sobre read-fonts | Menos de 25% mais lento que HarfBuzz nas fontes comuns; sem `unsafe`; há versão 0.9.0. [P/S] [docs.rs](https://docs.rs/crate/harfrust/0.9.0), [README 0.8.0](https://docs.rs/crate/harfrust/0.8.0/source/README.md) |
| **cosmic-text** | Shaping (HarfRust), raster (swash), fontdb, layout e edição | README ainda lista como pendentes: wrap com indentação, ellipsize, copiar e colar. Conferir no código. [P] [README](https://raw.githubusercontent.com/pop-os/cosmic-text/main/README.md) |
| **Parley** | Layout rico, bidi, quebra de linha, seleção e edição | Fontique + HarfRust + Skrifa + ICU4X. [P] [README](https://raw.githubusercontent.com/linebender/parley/main/README.md) |
| **swash** | Rasterização, hinting, emoji colorido | Usado pelo cosmic-text |
| **glyphon** | Texto em wgpu como middleware: cosmic-text + etagere + wgpu | Desenha em render pass já existente. [P] [repo](https://github.com/grovesNL/glyphon) |

Não encontrei benchmark Parley contra cosmic-text. Escolha [R]: Parley se o editor precisar de texto rico com seleção; cosmic-text mais glyphon se o objetivo for chegar logo. Uma alternativa é usar CoreText, como o Zed, porque coincide com o texto nativo do macOS, ao custo de portabilidade.

### 2.2 Atlas de glifos

- **Só alfa.** Um canal basta, e o glifo é tingido por multiplicação. [P] [Zed](https://zed.dev/blog/videogame)
- **Empacotamento.** Zed usa `etagere`. Alacritty usa prateleiras simples de 1024×1024, cria novo atlas quando enche e limpa tudo em troca de fonte.
- **Lazy.** Rasterize na primeira falta de cache. Atlas cheio: crie outra textura. [P] [Warp](https://www.warp.dev/blog/adventures-text-rendering-kerning-glyph-atlases) (2022-07-27)
- **Posição subpixel.** O post de 2023 do Zed fala em até 16 variantes, mas o código atual usa `SUBPIXEL_VARIANTS_X = 4`, `SUBPIXEL_VARIANTS_Y = 1`. [P] [text_system.rs](https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui/src/text_system.rs) O Warp usa 3 (0,0 / 0,33 / 0,66). Aria Desires sugere 4 em x e nenhuma em y para escrita latina. [P] [Text Rendering Hates You](https://faultlore.com/blah/text-hates-you/)
- **Chave de cache.** `(fonte, tamanho, glyph_id, subpixel_x)`. Desenhe em `floor(x) + offset` e compense no shader o deslocamento já embutido no bitmap (Warp).
- **Texto animado.** Navegadores desligam o subpixel para evitar que o texto "trema" entre posições. Durante zoom de câmera, use posição inteira ou SDF. [P] (Aria)
- **Subpixel LCD.** Saiu de uso em telas Retina e está desligado por padrão no macOS; compõe mal com transparência. [P] (Aria) No Retina [R]: só escala de cinza. Fica mais simples e evita o dual-source blending do Alacritty.

### 2.3 Gamma e peso

- **Zed (Windows, Linux).** Porte da correção perceptual do DirectWrite via Windows Terminal: tabela de 13 entradas para gamma 1,0 a 2,2. No Linux, gamma padrão 1,8 e contraste em escala de cinza 1,0. [P] [PR #37167](https://github.com/zed-industries/zed/pull/37167) (2025-09-01), [S] [#38862](https://git.secluded.site/zed/commit/e72021a26bf3e7e31d736eaaea89d6914149a9bf) Não confirmei o equivalente no macOS.
- **Ghostty.** Desde a 1.1, mistura em Display P3 por padrão, reduzindo o escurecimento. `linear` é tecnicamente correto, mas deixa texto escuro fino demais e texto claro grosso demais; `linear-corrected` compensa e fica muito perto de `native`. [P] [notas 1.1.0](https://ghostty.org/docs/install/release-notes/1-1-0)
- **Stem darkening.** Pathfinder reproduz a dilatação do macOS e do FreeType. No FreeType, só nos drivers CFF e Type 1. [S] [pathfinder](https://github.com/pcwalton/pathfinder), [issue FreeType](https://gitlab.freedesktop.org/freetype/freetype/-/issues/470)
- **Lição geral [R].** Blend linear puro em texto parece errado. Use uma tabela de correção por luminância do texto e compare lado a lado com o TextEdit, em tema claro e escuro.

### 2.4 Bitmap, SDF, MSDF e curvas na GPU

| Técnica | Usar quando | Limites concretos |
|---|---|---|
| Bitmap + atlas | Texto do editor e UI, tamanho fixo | Uma entrada por tamanho e por subpixel. Melhor qualidade em corpo pequeno |
| SDF | Rótulos em zoom contínuo; o sigma.js usa | Cantos vivos arredondam. [P] [Dobbie](https://wdobbie.com/post/gpu-text-rendering-with-vector-textures/) |
| MSDF | Títulos e rótulos grandes em zoom | Faixa de distância 4 (exemplo do README); na tela nunca abaixo de 1, e abaixo de 2 o AA tende a falhar. Traço mais fino com 2 a 3 px no campo. Sem mipmaps. [P/A] [msdfgen](https://github.com/Chlumsky/msdfgen), [issue #117](https://github.com/Chlumsky/msdfgen/issues/117), [shaders](https://github.com/Blatko1/awesome-msdf/blob/main/shaders/README.md) |
| Slug | Texto em qualquer escala ou perspectiva | Patente em domínio público desde 2026-03-17; shaders de referência sob MIT; dilatação dinâmica de meio pixel. [P] [Lengyel](https://terathon.com/blog/decade-slug.html). Comentários sem benchmark veem pouco ganho em corpo pequeno sobre MSDF com fallback raster. [A] [HN via zeli](https://zeli.app/story/49908962) |

Recomendação [R]: bitmap no editor. Nos rótulos do grafo, bitmap em tamanhos discretos com SDF somente durante o zoom animado, trocando ao parar. Não use MSDF abaixo de cerca de 14 px físicos sem testar.

### 2.5 Cache de shaping

- **Zed `LineLayoutCache`.** Duas gerações: `previous_frame` e `current_frame`. Um acerto move a entrada para o quadro atual. `finish_frame` troca e limpa. Quem não for usado em dois quadros seguidos morre. Chave sem alocação via `CacheKeyRef`. Reuso por região com `reuse_layouts(range)`. [P] [line_layout.rs](https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui/src/text_system/line_layout.rs)
- **cosmic-text `ShapeRunCache`.** Chave: texto, atributos padrão e spans. Contador de idade com `trim(keep_ages)`. [P] [shape_run_cache.rs](https://raw.githubusercontent.com/pop-os/cosmic-text/main/src/shape_run_cache.rs)
- **Android.** Cacheia por palavra inferida. Perde shaping entre palavras, mas medir e desenhar sempre concordam. [P] [Levien, 2020](https://raphlinus.github.io/text/2020/10/26/text-layout.html)
- **Ghostty.** Reinicia periodicamente o LRU porque tempos de render pioravam com o uso. [S] [log do espelho](https://git.uoc.run.place/Applied-Software/ghostty/commits/commit/15e41959eb762600d9549877ccee49b01c298be5/src/font?page=4) Vale testar o cache em sessões longas.

### 2.6 Editor: buffer, virtualização e destaque

**Rope.**

| Dado | Valor | Fonte |
|---|---|---|
| Ropey, inserções incoerentes | Mais de 1,8 milhão/s | [P] [repo](https://github.com/cessen/ropey) |
| Ropey, inserções coerentes | Mais de 3,3 milhões/s | idem |
| Ropey, memória | 100 MB ocupam cerca de 110 MB; clone inicial de 8 bytes | idem |
| Ropey, edição em vários GB | Microssegundos de um dígito | idem |
| Zed, tamanho de chunk | Até 128 caracteres em release (`2 * CHUNK_BASE`, base 64) | [P] [zed-decoded](https://zed.dev/blog/zed-decoded-rope-sumtree) |
| Zed, snapshot | Incremento de contador; nós em `Arc` | idem |

O `SumTree` guarda em cada nó um resumo com bytes, UTF-16, linhas e linha mais longa, o que dá conversão offset↔linha em O(log n). Em cada edição, um snapshot vai para outra thread para o reparse. Sobre rope, Levien conclui que não há caso em que ele "performs badly". [P] [xi retrospective](https://raphlinus.github.io/xi/2020/06/27/xi-retrospective.html) (2020-06-27)

**Virtualização.** O CodeMirror 6 mantém um mapa de alturas: estima as linhas não medidas, corrige após medir e cobre o restante com um espaçador. O limite é que o custo cresce com o tamanho do viewport. [S] [DeepWiki](https://deepwiki.com/codemirror/view/6-layout-and-height-management), [A] [fórum](https://discuss.codemirror.net/t/viewport-issues-with-cm-6/3586) Para o caso de Rust [R]: um `SumTree` com altura acumulada por linha em cada nó, que dá `y→linha` e `linha→y` em O(log n), e medir só o viewport mais cerca de 1 tela de folga.

**Destaque.** Tree-sitter atualiza a árvore após uma edição em geralmente menos de 1 ms; do zero, um arquivo Rust de exemplo levou cerca de 6,48 ms. [S] [tree-sitter-rust](https://github.com/tree-sitter/tree-sitter-rust/wiki) O Zed roda em background sobre snapshot e guarda injeções em `SumTree`. [P] [syntax-aware-editing](https://zed.dev/blog/syntax-aware-editing) Regex é ordem de grandeza pior: syntect é cerca de 2500 vezes mais lento que `pulldown-cmark` em Markdown. [P] (xi retrospective)

**Lições do xi [P].** Separar frontend e núcleo em processos "was not a good idea": a rolagem levou meses. Não faça a thread de UI pedir texto por mensagem. Dê a ela um snapshot imutável, que resolve o mesmo problema sem assincronia na leitura.

**Regra de destaque [R].** Primeiro quadro após a tecla: texto com o estilo da linha anterior mais edição aplicada. Reparse em background chega 1 a 2 quadros depois. Nunca espere o parser antes de apresentar.

### 2.7 IME no macOS

**Como o winit faz** (fluxo em [view.rs](https://raw.githubusercontent.com/rust-windowing/winit/master/winit-appkit/src/view.rs), [P]):

1. `keyDown:` confere se a fonte de entrada mudou.
2. Reescreve o evento conforme `option_as_alt`.
3. Chama `interpretKeyEvents` antes de emitir `KeyboardInput`. A ordem é essencial.
4. O AppKit chama `setMarkedText`, `insertText` ou `doCommandBySelector`.
5. Só emite `KeyboardInput` se não houve IME, ou se `forward_key_to_app` foi marcado.

Estados: `Disabled`, `Ground`, `Preedit`, `Committed`. O último faz ignorar a entrada seguinte. Durante composição, `keyUp` é suprimido.

**Limites atuais do winit relevantes para um editor [P]:**

- `selectedRange` retorna sempre `NSNotFound`.
- `characterIndexForPoint:` retorna `0`.
- `replacementRange` é ignorado (há TODO no código), o que atrapalha reconversão e sugestões que trocam texto já inserido.
- Comentário no código: "IMEs are known to send both mid-surrogate and out-of-bounds offsets".

**Falhas conhecidas:**

| Sintoma | Causa | Fonte |
|---|---|---|
| Teclas mortas com Option falham | No macOS, passam pela via de IME, não pela entrada normal | [A] [Neovide #1866](https://github.com/neovide/neovide/issues/1866) |
| Seta anda 2 candidatos | `interpretKeyEvents` e o lado Rust tratam a mesma tecla | [A] [Warp #9709](https://upd.dev/warpdotdev/warp/issues/9709) (espelho, 2026) |
| Caractere vaza durante preedit | Checagem de atalho dispara callback de preedit; a equipe chama o conserto de paliativo e propõe rastrear composição na camada do app | [S] [Ghostty, ref. #5558](https://git.uoc.run.place/Applied-Software/ghostty/commit/0538d6db6a0e971f425a99e4e75f9203b5605e68.patch) |
| Atalho `jj` engole tecla do IME | Em fontes não ASCII, IME deve ter prioridade sobre keybinding | [S] [Zed #52192](https://git.secluded.site/zed/commit/ddc327fe035beb690d52a55c0d7645f0def233b8) |
| Crash em `set_marked_text` | Pinyin envia faixa maior que o texto; clamp em UTF-16 resolve | [P] [winit #4494](https://github.com/rust-windowing/winit/pull/4494) (mar. 2026) |
| Evento consumido pelo IME reaparece | Faltava marcá-lo como tratado | [P] [Chromium](https://codereview.chromium.org/2601513002) |

**Recomendação [R].** O GPUI registra uma `NSView` própria com `NSTextInputClient` completo. Para um editor de escrita, faça o mesmo: mantenha o winit para janela e eventos, ou substitua, mas tenha `NSView` própria via `objc2` com:

- `selectedRange`, `markedRange`, `attributedSubstringForProposedRange`, `firstRectForCharacterRange` e `characterIndexForPoint` reais.
- `replacementRange` respeitado.
- Um único caminho de despacho: chama `interpretKeyEvents` primeiro; só depois decide se a tecla vira comando.
- Estado de composição no editor, como região marcada no buffer, renderizada com sublinhado e fora do histórico de desfazer.
- Conversão UTF-16↔UTF-8 com clamp e tolerância a meio de surrogate.
- Testes com ABC Extended, US International, Pinyin, Kotoeri e Character Viewer, além de acento + vogal em layout brasileiro.

---

## 3. Grafos de alto desempenho

### 3.1 O que cada projeto faz

| Projeto | Física | Render | Números |
|---|---|---|---|
| **cosmos.gl / Cosmograph** | Tudo na GPU, em fragment e vertex shaders | WebGL 2 via luma.gl; picking por buffer de IDs com leitura não bloqueante | "Centenas de milhares" no README; cerca de 1 milhão com o app. 133 mil nós e 321 mil arestas em demonstração. [P] [repo](https://github.com/cosmosgl/graph), [Nightingale](https://nightingaledvs.com/how-to-visualize-a-graph-with-a-million-nodes/) |
| **Sigma.js** | Delegada (graphology) | Quads, atlas SDF para rótulos, picking em framebuffer com `pickingDownSizingRatio` | Buckets de profundidade: arestas, rótulos de aresta, nós, rótulos. [P] [rendering](https://www.sigmajs.org/concepts/rendering/) |
| **Graphistry** | ForceAtlas2 em OpenCL no servidor | WebGL no cliente, dados em Arrow | [P] [arquitetura](https://graphistry-admin-docs.readthedocs.io/en/latest/planning/architecture.html). Sem detalhes do layout |
| **GraphPU** | Rust + wgpu + WGSL (Vulkan e Metal), UI em egui | Compute | Afirma milhões em tempo real, sem número exato. [P] [repo](https://github.com/latentcat/graphpu) |
| **GPUGraphLayout** | FA2 com Barnes-Hut em CUDA | - | Acima de 40× sobre FA2 em CPU. [S] [repo](https://github.com/govertb/GPUGraphLayout) |
| **GraphWaGu** | Quadtree e travessia Barnes-Hut em compute WebGPU, sem recursão nem ponteiros | - | [S] [Eurographics PGV 2022](https://unpaywall.org/10.2312%2FPGV.20221067). PDF inacessível aqui |
| **vibe-graph-layout-gpu** | Rust + wgpu; quadtree na CPU, forças na GPU; θ = 0,8 | - | 1 mil nós a 185 fps; 9 mil a 110 fps; sem hardware informado. [A] [README](https://docs.rs/crate/vibe-graph-layout-gpu/latest/source/README.md) |

**Gephi Lite.** Pilha de sigma.js e graphology, com ForceAtlas2 em web worker [M para a composição exata]. A doc da graphology registra que o worker degradava o desempenho em alguns casos antes da v0.5.0, e o FA2 foi projetado para 10 a 10 mil nós. Não há limite publicado para o Gephi Lite. [P] [graphology](https://graphology.github.io/standard-library/layout-forceatlas2)

**Repulsão no cosmos.gl.** A abordagem tradicional não acelera na GPU por causa do acesso aleatório à memória. A versão atual troca o quadtree por pirâmide de grades (campo distante) mais campo próximo Monte Carlo, ponderado por Horvitz-Thompson para estimativa não enviesada. A vizinhança 3×3 mais fina fica fora da agregação grosseira, o que impede hubs de colapsarem. Grafos pequenos usam passe exato. [S] [DeepWiki](https://deepwiki.com/cosmosgl/graph/6.3.1-many-body-repulsion) e trechos da doc; não li o shader. Limitações registradas: ruído quando vários nós caem no mesmo quadrado da grade e `spaceSize` limitado. [P] (Nightingale)

### 3.2 O que reclamam do Obsidian

| Tamanho do cofre | Relato | Fonte |
|---|---|---|
| 9 mil notas | Lag só de deixar o grafo aberto em aba oculta | [A] [fórum](https://forum.obsidian.md/t/help-obsidian-lags-with-many-notes/82241) |
| 25 mil+ | Moderador diz que o uso prático é improvável | [A] [fórum](https://forum.obsidian.md/t/maximum-number-of-notes-in-vault/1509?page=5) |
| 35 mil | 5 minutos carregando e tela preta | [A] idem acima |
| 130 mil | Congela, até o grafo local com profundidade 1 | [A] [fórum](https://forum.obsidian.md/t/obsidian-graph-view-doesnt-work-for-a-large-vault/106287) |

Lições [R]: pausar simulação quando oculto; layout fora da thread de UI; índice incremental persistido em vez de reindexar a cada abertura; grafo local com orçamento de tamanho; primeira imagem imediata a partir de posições salvas. Plugins como [Speed Graph](https://community.obsidian.md/plugins/speed-graph) fazem layout em background justamente por isso. [A]

### 3.3 Algoritmos de layout

**ForceAtlas2.** Atração linear na distância; repulsão proporcional a `(deg+1)(deg+1)`; velocidade adaptativa por nó (swinging e traction). [P] [Jacomy et al., 2014](https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0098679)

| Parâmetro | Valor |
|---|---|
| Faixa de projeto | Redes livres de escala de 10 a 10 mil nós |
| Tolerância no Gephi | 0,1 até 5 mil nós; 1 até 50 mil; 10 acima |
| Constante de velocidade local | 0,1 |
| θ no benchmark | 1,2 (graphology usa 0,5 por padrão, com Barnes-Hut desligado) |
| Benchmark | Scaling 2,0; gravidade 0; 68 redes, de 5 a 23 133 nós |
| Tempo até 50% da qualidade | FA2 68 ms; Yifan Hu 98 ms; LinLog 134 ms; Fruchterman-Reingold 7 853 ms |
| Tempo até 90% da qualidade | Yifan Hu 333 ms; FA2 638 ms; LinLog 1 184 ms; FR 20 201 ms |

Defaults da graphology: `gravity 1`, `scalingRatio 1`, `slowDown 1`, `linLogMode false`, `adjustSizes false`. [P] [docs](https://graphology.github.io/standard-library/layout-forceatlas2)

**Multinível (sfdp).** Colapsa o grafo em níveis, resolve o mais grosso e refina, com octree para forças de longo alcance. Isso escapa de mínimos locais. [P] [Yifan Hu](https://yifanhu.net/SOFTWARE/SFDP/index.html). Defaults da implementação graph-tool: `cooling_step 0.95`, `adaptive_cooling True`, `theta 0.6`, `max_level 15`. [P] [graph-tool](https://graph-tool.skewed.de/doc/autosummary/graph_tool.draw.sfdp_layout.html) Os parâmetros do próprio artigo não foram verificados: o PDF não pôde ser lido.

**Incremental.** d3-force: `alphaDecay ≈ 0,0228` (300 iterações), `alphaMin 0,001`, `velocityDecay 0,4`. Reaquecer com `alphaTarget(0.3).restart()` ao arrastar é prática comum, não valor da doc. [P] [d3-force](https://d3js.org/d3-force/simulation)

**GPU com WGSL.** Não há recursão: a travessia usa pilha explícita ([vibe-graph](https://docs.rs/crate/vibe-graph-layout-gpu/0.1.1/source/src/shaders.rs)). Código Morton + radix sort na GPU construindo a árvore num único buffer é a família de técnica do trabalho seguinte ao GraphWaGu, que usa ordenação Hilbert ([PDF](https://par.nsf.gov//servlets/purl/10384648), [S], lido só por trecho de busca).

**Proposta para milhares de nós [R]:**

1. Até cerca de 2 mil nós: forças exatas O(n²) em compute são simples e sem artefatos. Meça.
2. De 2 mil a 100 mil: grade uniforme hierárquica ou Barnes-Hut com Morton, θ entre 0,8 e 1,2, em compute.
3. Layout inicial: multinível na CPU em background; ao terminar, entrega posições e a GPU refina.
4. Persistir posições por nó. Abertura sempre começa da última imagem.
5. Evento novo (nota, link): só o nó novo e o entorno de 1 a 2 saltos ficam móveis, o resto ancorado com rigidez alta. Reaquecimento global só por ação explícita.
6. Parar a simulação em `alpha < alphaMin` e tirar o compute do quadro. Grafo parado custa zero.

### 3.4 Rótulos sem sobreposição

**Sigma.js, grid por densidade.** Cada célula ordena candidatos por tamanho e mostra `ceil(density / ratio²)`. Defaults: `labelGridCellSize 100`, `labelDensity 1`, `labelRenderedSizeThreshold 6`. [P] [labels.ts](https://raw.githubusercontent.com/jacomyal/sigma.js/main/packages/sigma/src/core/labels.ts), [settings.ts](https://github.com/jacomyal/sigma.js/blob/6c1cf6d/packages/sigma/src/settings.ts). O grid limita a quantidade por célula, mas pelo algoritmo descrito não garante ausência de colisão entre células vizinhas [R].

**Mapbox, colisão em tela.** Grade em coordenadas de tela com células de 30 px, escolhidas por perfil de animações comuns: um círculo de 16 px cabe em 1 a 4 células. A colisão fica fora do caminho síncrono; latência de 200 ms é aceitável; o exemplo atualiza a cada cerca de 150 ms, com fade de 300 ms em shader. A opacidade fica por `crossTileID`, então o fade continua através de trocas de nível de zoom. [P] [wiki](https://github.com/mapbox/mapbox-gl-native/wiki/Collision-Detection) (fim de 2017)

- Fatiamento real: `COLLISION_TIME_BUDGET_MS = 2`. A checagem ocorre entre unidades, logo uma unidade grande pode estourar. [P] [pauseable_placement.ts](https://raw.githubusercontent.com/mapbox/mapbox-gl-js/main/src/style/pauseable_placement.ts)
- `GridIndex`: arrays planos, `seen` por geração para deduplicar sem realocar, `hitTest` com saída antecipada. [P] [grid_index.ts](https://raw.githubusercontent.com/mapbox/mapbox-gl-js/main/src/symbol/grid_index.ts)
- Padding de viewport tira trocas de colisão da área visível. [S] [MapLibre](https://devlop.bogorkab.go.id/kunker/node_modules/maplibre-gl/src/symbol/collision_index.ts)

**Cosmograph.** Seleciona os pontos mais importantes por peso ou grau, com limite de 30 ou 100 conforme a versão. [P] [docs](https://cosmograph.app/docs-lib/features/labels/)

**Algoritmo proposto [R]:**

```text
a cada 150 ms ou ao mudar de câmera, em fatias de 2 ms:
  candidatos = nós no viewport, com tamanho renderizado >= limiar
  ordenar por prioridade = selecionado > foco > vizinho do foco > centralidade
  somar bônus de histerese a quem já está visível
  para cada candidato:
    se a caixa não colide no GridIndex (células de 30 px): inserir e alvo = 1
    senão: alvo = 0
fade em shader: opacidade anda até o alvo em 200 a 300 ms, com relógio próprio
```

Um dado por rótulo: `opacidade_atual`, `alvo`, `t_inicio`. A GPU interpola sem o CPU tocar em nada, portanto o fade não exige reposicionar ninguém.

### 3.5 LOD e arestas

- **LOD por zoom.** yFiles descreve o padrão: detalhe em zoom alto, simplificação em zoom baixo; arestas têm menos opções, então tira setas e recortes. [P] [docs](https://docs.yworks.com/yfiles-html/dguide/large_graph_performance/)
- **Em movimento.** Sigma tem `hideEdgesOnMove` e `hideLabelsOnMove`. [P] [settings](https://www.sigmajs.org/api/sigma/settings/interfaces/settings/)
- **Agregação.** Zinsmaier et al. combinam acúmulo de arestas e agregação por densidade em cerca de 10⁷ nós e 10⁶ arestas interativos. [S] [artigo](https://graphics.uni-konstanz.de/publikationen/Zinsmaier2012InteractiveLevelDetail/Zinsmaier2012InteractiveLevelDetail.pdf) (2012)
- **Arestas instanciadas.** Um quad por instância, com largura perpendicular; uma única chamada, N instâncias. [P] [Terrell](https://wwwtyro.net/2019/11/18/instanced-lines.html) (2019-11-18). Expanda o quad em `largura/2 + 1 px` para o AA não cortar, e use SDF de cápsula com `smoothstep`. [S] [uzor](https://docs.rs/crate/uzor-render-wgpu-instanced/1.3.4/source/src/shaders.rs)
- **Densidade [R].** Opacidade inversamente proporcional à densidade de arestas na tela. Use blending aditivo sobre buffer HDR para somar em vez de pintar uma mancha.
- **Largura mínima [R].** Abaixo de 1 px físico, mantenha 1 px e reduza alfa em proporção. Linha fina com largura variável pisca.

### 3.6 Edge bundling

| Método | Parâmetros | Observação |
|---|---|---|
| FDEB | C = 6 ciclos; `S0 = 0,1`; `K = 0,1`; P começa em 1 e dobra; I começa em 90 e cai 2/3; compatibilidade 0,6. [P] [d3.ForceBundle](https://raw.githubusercontent.com/upphiminn/d3.ForceBundle/master/d3-ForceEdgeBundling.js). A netgraph usa limiar 0,05, 5 ciclos, 50 iterações ([docs](https://netgraph.readthedocs.io/en/stable/edge_layout.html)) | Compara todos os pares de arestas a cada iteração. Só em background ou GPU |
| Hammer (datashader) | `initial_bandwidth 0.05`, `decay 0.7`, `iterations 4` | Variante de Hurter et al. Aumentar banda ou decay exige mais iterações. [P] [datashader](https://datashader.org/user_guide/Networks.html) |

Recomendação [R]: bundling só como modo de visão, sobre arestas entre comunidades. Calcular fora do quadro e interpolar a polilinha de reta para curva em 300 ms. Subdivisões entram no vertex shader como pontos de controle em storage buffer.

### 3.7 Glow e bloom

**Filtro de Jimenez (padrão atual para bloom).** 13 amostras no downsample e tenda 3×3 no upsample (4 no centro, 2 nas bordas, 1 nos cantos, dividido por 16). 5 ou 6 mips bastam. Mistura com a cena em torno de 0,04 (faixa de 0,03 a 0,15). Média de Karis (`1/(1+luma)`) só na primeira redução evita cintilação. [P] [LearnOpenGL](https://learnopengl.com/Guest-Articles/2022/Phys.-Based-Bloom) (2022), sobre SIGGRAPH 2014.

**Dual Kawase.** Reduz e amplia, com banda limitada por trabalhar em texturas menores. Atribuído a Bjørge (SIGGRAPH 2015). O AOSP adotou Kawase no lugar de Gaussiano por visual parecido e melhor desempenho. [S] [AOSP](https://gerrit.omnirom.org/plugins/gitiles/android_frameworks_native/+/263152043dee4d2bce8df5e442aac88da4304127%5E%21) Kernels de memória [M]: descida com 4× centro + 4 diagonais em meio pixel, sobre 8; subida com 4 eixos (peso 1) + 4 diagonais (peso 2), sobre 12.

**Comparação em celular, 2026.** Mipmap bloom 10,29 ms; Kawase + HDR 11,49 ms; Gaussiano 14,12 ms. [S] [ETASR](https://etasr.com/index.php/ETASR/article/download/19101/6940) Portanto Kawase bate Gaussiano, mas não é necessariamente o mais rápido.

Recomendação [R]: desenhar nós e arestas em `Rgba16Float`, extrair só os canais que brilham, downsample a partir de 1/2 resolução, 4 ou 5 mips. Em repouso, cachear o resultado. Glow não precisa acompanhar o quadro quando nada mudou.

---

## 4. O que dá vida

### 4.1 Molas

**Conversão da Apple.** `massa = 1`, `rigidez = (2π ÷ duração)²`. [P] [WWDC23 10158](https://developer.apple.com/videos/play/wwdc2023/10158/)

O amortecimento aparece transcrito como `1 - 4π × bounce ÷ duration`, o que não fecha: com `bounce = 0` deveria dar o amortecimento crítico `4π ÷ duração`, igual ao ramo negativo. A leitura coerente é `(1 - bounce) × 4π ÷ duração`, isto é, `ζ = 1 - bounce`. [R] Essa correção é dedução minha; confirme no vídeo.

- `duration` é perceptual e estável. A duração de assentamento varia e não deve guiar a UI.
- Ao redirecionar, a mola herda a velocidade corrente. É isso que dá sensação de resposta física.
- Bounce vai de -1,0 a 1,0.

**Passo exato.** Use a solução fechada por `dt`, independente de taxa de quadros, com ramo separado para `ζ = 1`: `(A + Bt)·e^(−ω₀t)`. [A] [bezel-gpui](https://docs.rs/crate/bezel-gpui/latest/source/src/spring.rs), [uzor-animation](https://docs.rs/uzor-animation/1.0.3/aarch64-unknown-linux-gnu/src/uzor_animation/spring.rs.html), derivados de Juckett. Não reutilize matriz em cache com `dt` diferente. ProMotion muda o `dt` o tempo todo.

**Valores iniciais [R].** UI: duração 0,25 a 0,35 s, bounce 0 a 0,12. Painel entrando: 0,4 s, bounce 0. Nó ao ser solto no grafo: 0,5 s, bounce 0,15. Apple usa 0,5 s e bounce 0,2 como exemplo.

### 4.2 Interpolação de layout

cosmos.gl: `transitionDuration` 800 ms com `CubicInOut`; 0 faz troca imediata. [P] [repo](https://github.com/cosmosgl/graph)

Separe taxas [R]: a física roda em passo fixo (por exemplo 60 Hz), e o render interpola entre os dois últimos estados pelo tempo do quadro. A 120 Hz, a imagem fica suave mesmo com simulação a 30 ou 60 Hz. Isso custa um passo de atraso visual, aceitável em física, inaceitável em arraste: o nó sob o cursor segue o mouse direto, sem interpolação.

### 4.3 Partículas nas arestas

Não obtive fonte específica (busca esgotada). Desenho proposto [R]:

- Opção barata, primeira escolha: sem buffer. No fragment shader da aresta, `fract(s * freq - t * vel)` gera pulsos. Custa só o tempo de ativar a animação.
- Opção rica: storage buffer `{edge_id: u32, t: f32, vel: f32}`, compute atualiza `t`, vertex shader busca os extremos da aresta em storage e desenha sprite instanciado com mistura aditiva.
- Disparar só em arestas do foco ou de um evento. Teto: 1 a 5 mil partículas.
- Um pulso curto em ação do usuário, como "link criado", vale mais que movimento perpétuo. O loop contínuo mata a bateria (seção 5.3).

### 4.4 Câmera: zoom suave de van Wijk

Caminho ótimo por `(x, y, largura)` que afasta, viaja e aproxima. d3 usa `ρ = √2` por padrão; a implementação Polymaps usa 1,42 citando a seção 6 do artigo como resultado de teste com usuários. O interpolador expõe `duration`, a ser multiplicada por um fator V. [P] [d3-interpolate](https://d3js.org/d3-interpolate/zoom), [gist](https://gist.github.com/600144), artigo [van Wijk & Nuij](http://www.win.tue.nl/~vanwijk/zoompan.pdf)

Fórmulas [M], a conferir em [zoom.js](https://github.com/d3/d3-interpolate/blob/main/src/zoom.js):

```text
d = |c1 - c0|
b(i) = (w1² - w0² ± ρ⁴d²) / (2 · w_i · ρ² · d)      + para i=0, - para i=1
r(i) = ln(-b(i) + sqrt(b(i)² + 1))
S    = (r1 - r0) / ρ
u(s) = w0/(ρ²d) · (cosh(r0)·tanh(ρs + r0) - sinh(r0))
w(s) = w0 · cosh(r0) / cosh(ρs + r0)
caso d ≈ 0: S = ln(w1/w0)/ρ; w(s) = w0·exp(ρs)
duração em d3 = S · 1000 · ρ / √2 ms
```

Aplique com `s = S · easing(t)`. Limite a duração total a cerca de 400 a 800 ms [R]; navegar entre notas não pode parecer um passeio.

### 4.5 Ruído, fumaça, tinta, reaction-diffusion

**Fluido estilo Stam.** Defaults do WebGL-Fluid-Simulation: sim 128, tinta 1024, dissipação de densidade 1, de velocidade 0,2, pressão 0,8, 20 iterações de Jacobi, curl 30, splat 0,25. Passes: curl, vorticidade, divergência, limpa pressão, Jacobi ×20, subtrai gradiente, advecção de velocidade e de tinta. [P] [script.js](https://raw.githubusercontent.com/PavelDoGreat/WebGL-Fluid-Simulation/master/script.js)

Traduzindo: são cerca de 27 passes por quadro. Com compute, dá para reduzir [R]: sim a 64 ou 96, Jacobi a 8–12, e reaproveitar pressão do quadro anterior. Reservar para transição breve, nunca como fundo permanente com editor aberto.

**Reaction-diffusion (Gray-Scott).** `D_A = 1,0`, `D_B = 0,5`, `Δt = 1,0`; Laplaciano 3×3 com centro -1, vizinhos 0,2, diagonais 0,05. Padrão `f = 0,055`, `k = 0,062`; mitose `f = 0,0367`, `k = 0,0649`; coral `f = 0,0545`, `k = 0,062`. [P] [Karl Sims](https://www.karlsims.com/rd.html)

Implementação [R]: textura `Rg16Float`, dois buffers alternados, compute com 4 a 16 passos por quadro em 256×256 a 512×512, cor por mapa, depois congelar. Como cresce devagar, serve para um estado lento e orgânico, por exemplo "crescimento" de uma área do grafo.

**Domain warping e curl noise.** Página de Inigo Quilez não disponível neste acesso. Mantido como lacuna.

### 4.6 Grão e dithering

- **Dark UI é onde banding aparece.** O Zed aplicou ruído triangular após interpolar gradientes: cerca de ±2/255 em RGB e ±3/255 em alfa, eliminando banding em gradientes escuros de 5% a 13% de luminosidade. [S] [commit](https://git.secluded.site/zed/commit/a46858ac21ec43b1af94d8b2b9a5a8fde4ac4f01.patch), trecho de busca.
- **Distribuição.** Triangular corrige sem ruído excessivo; azul também vai bem; Bayer dá padrão visível. [P] [Wronski](https://bartwronski.com/2016/10/30/dithering-part-three-real-world-2d-quantization-dithering/) (2016-10-30). Base: Gjøl, Playdead, citado no [Blender](https://lists.blender.org/pipermail/bf-blender-cvs/2020-February/135857.html).
- **Grão [R].** Amplitude de 1/255 a 2/255 em luminância, aplicado no último passe. Em repouso congele a semente. Grão animado obrigaria redesenho contínuo.

### 4.7 TAA e motion blur

**TAA.** Jitter Halton (2,3), 8 amostras, peso da fonte 0,05, clamp 3×3 mais clip por variância com γ = 1, reprojeção por velocidade RG16F. [P] [Tardif](https://alextardif.com/TAA.html)

Conclusão [R]: não use em texto nem UI, e provavelmente não no grafo. Linhas finas em movimento são o caso de ghosting, e SDF analítico já resolve o AA de forma estável sem histórico. Se houver cena 3D ou efeito volumétrico, aplique só nela, antes de compor a UI.

**Motion blur.** Números de McGuire não obtidos. Para nós rápidos [R], alongue o quad na direção da velocidade e reduza o alfa proporcionalmente. Não precisa de reconstrução em tela. A 120 Hz, é menos necessário.

---

## 5. Agendamento

### 5.1 Pacing no macOS

**CAMetalDisplayLink.** macOS 14+. A Apple o indica para quando se precisa de mais controle da janela de tempo do quadro. O delegado `metalDisplayLink(_:needsUpdate:)` recebe drawable e prazos. [P] [doc](https://developer.apple.com/documentation/quartzcore/cametaldisplaylink.md)

- `preferredFrameLatency` só aceita 1,0 ou 2,0; a latência final pode ser maior em janelas no macOS. [P] [doc](https://developer.apple.com/documentation/quartzcore/cametaldisplaylink/preferredframelatency.md)
- `targetTimestamp` é o prazo para chamar `present()`. A GPU pode continuar depois disso, conforme a latência. [P] [doc](https://developer.apple.com/documentation/quartzcore/cametaldisplaylink/update/targettimestamp.md)
- Métodos que apresentam em tempo específico disparam assert com essa classe. [S] [resumo da doc](https://developer.apple.com/documentation/quartzcore/cametaldisplaylink/update.md)
- Chromium: `preferredFrameLatency = 1.0`, criado pausado, `framebufferOnly = YES`, `BGRA8Unorm`, `NSRunLoopCommonModes`, callback na mesma run loop. [P] [ca_metal_display_link_mac.mm](https://chromium.googlesource.com/chromium/src/+/main/ui/display/mac/ca_metal_display_link_mac.mm)

**Outras opções.** `CVDisplayLink` está obsoleto desde o macOS 15; substitutos: `NSView/NSWindow/NSScreen.displayLink(target:selector:)`. [S] [QEMU](https://gitlab.com/qemu-project/qemu/-/issues/2575) Rosetta limita a 60 Hz. [A] [fórum Apple](https://www.developer.apple.com/forums/thread/800540)

**ProMotion.** Um crate afirma que o macOS só sobe para 120 Hz quando algo no processo declara demanda de taxa, e que a apresentação com vsync do wgpu sozinha deixa o painel ocioso. A solução é um display link com faixa 60–120. [A] [teksilo-platform](https://docs.rs/crate/teksilo-platform/0.17.0/source/src/wake.rs), coerente com o achado do Zed.

**Fórum Apple sobre 120 Hz.** Quadro de 2 a 3 ms ainda rodava a cerca de 80 Hz, travado em `nextDrawable`. O DTS recomenda `CAMetalDisplayLink`, repensar `displaySyncEnabled` e diz que 2 drawables são aceitáveis no macOS se o orçamento for respeitado. Outro participante recomenda 3 e pedir o drawable o mais tarde possível. Sem causa-raiz confirmada. [A/P] [thread 763426](https://developer.apple.com/forums/thread/763426)

**macOS 27 (beta).** Painel alterna entre 120 e 60 Hz. Desvio padrão do intervalo de vsync: 4,093 ms com ProMotion, 0,391 ms forçado a 60 Hz. Fps mediano igual em ambos (59). Relatos com M4 Pro e M5 Pro; Apple foi informada; sem conserto registrado até a data da página. Em um relato, 70% de quadros saudáveis a 120 Hz contra 97,6% a 60 Hz. [A] [thread 840214](https://developer.apple.com/forums/thread/840214) Lição: fps mediano esconde pacing ruim.

### 5.2 Limites do wgpu no Metal

Do código atual [P] ([surface.rs](https://raw.githubusercontent.com/gfx-rs/wgpu/trunk/wgpu-hal/src/metal/surface.rs), [adapter.rs](https://raw.githubusercontent.com/gfx-rs/wgpu/trunk/wgpu-hal/src/metal/adapter.rs)):

| Item | Comportamento |
|---|---|
| Modos de apresentação | Só `Fifo` e `Immediate`. **Não há `Mailbox`** |
| Latência | `maximum_frame_latency` em `1..=2`; `maximumDrawableCount = N + 1` |
| `presentsWithTransaction` | Não configurável pela `SurfaceConfiguration`; lido da layer |
| `framebufferOnly` | Ativo só com uso exclusivo como alvo de cor e sem formatos alternativos |
| Oclusão | Retorna `SurfaceError::Occluded` para evitar hang de cerca de 1 s em `nextDrawable` |
| Formatos | `Bgra8Unorm`, `Bgra8UnormSrgb`, `Rgba16Float`; P3 disponível; `Rgb10a2Unorm` quando suportado |
| Display link | Sem integração que achei; o plugin bevy_metalfx usa `CAMetalLayer` próprio porque o `present` do wgpu não tem argumento de tempo. [A] [bevy_metalfx](https://docs.rs/crate/bevy_metalfx/latest/source/src/present/mod.rs) |

**Conflito a resolver [R].** `CAMetalDisplayLink` entrega o drawable; o wgpu o obtém com `nextDrawable` em `get_current_texture`. Os dois caminhos não combinam diretamente. O comentário do Zed confirma a tensão: o link é desanexado para permitir `nextDrawable`. Duas rotas:

1. **Simples:** usar `NSView.displayLink` (ou `CADisplayLink`) só como relógio, e o wgpu adquire normalmente com `Fifo` e `desired_maximum_frame_latency = 1`. Começar aqui e medir.
2. **Controle total:** registrar `CAMetalDisplayLink` e usar `wgpu-hal` para embrulhar `update.drawable.texture` como textura externa no quadro, ou escrever só o passe final em Metal direto. Fazer só se a rota 1 falhar nas medições. Prototipar antes; não verifiquei se a API atual permite isso limpo.

**Não usar [R]:** `Immediate` (fim do vsync sem ganho) e loop `request_redraw` sem relógio.

**Redimensionamento.** Desenhe de forma síncrona no evento de resize, para imagem e geometria chegarem juntas; o Zed usa `presentsWithTransaction` só aí. [P] [120fps](https://zed.dev/blog/120fps), [A] [inset-embedder-winit](https://docs.rs/crate/inset-embedder-winit/latest/source/src/frames.rs)

### 5.3 Sob demanda ou loop contínuo

Modelo de estados [R], a partir de Zed, Ghostty e Neovide:

| Estado | Entra quando | Relógio | Custo esperado |
|---|---|---|---|
| Parado | Nada sujo há mais de 1 s | Display link pausado, `ControlFlow::Wait` | Próximo de zero |
| Interativo | Entrada de alta frequência (rolagem, arraste, zoom, tecla repetida) | Display link 60–120, com permanência de cerca de 1 s | Alto, mas breve |
| Animando | Mola, fade ou física ativos | Display link até `alpha < alphaMin` e molas assentarem | Alto, com fim previsto |
| Sem foco | Janela visível sem foco | Sem animações decorativas; física pausa ou 30 Hz | Baixo |
| Oculto ou ocluído | Janela oculta | Nada de render; `utility` para tarefas | Zero em GPU |

Referências:

- Ghostty: a thread de render é acordada por mensagens. O timer só existe para animações, e ao ficar invisível tudo para. [P] [Thread.zig](https://raw.githubusercontent.com/ghostty-org/ghostty/main/src/renderer/Thread.zig)
- Neovide: 5 Hz sem foco, nos exemplos da doc. [P] [config](https://neovide.dev/configuration.html)
- Apple: não fazer polling; dar folga a timers; não atualizar conteúdo invisível; não esperar o App Nap. [P] [Energy guide](https://developer.apple.com/library/archive/documentation/Performance/Conceptual/power_efficiency_guidelines_osx/BestPractices.html) (arquivado)
- Cursor piscando [R]: não acorde o pipeline inteiro. Ghostty usa 600 ms de meio período. Redesenhe só o retângulo do caret. Para em foco perdido.

Toda animação precisa ter condição de parada. Um efeito decorativo sem fim transforma o app inteiro em loop contínuo.

### 5.4 Damage, camadas e fatiamento

**Damage por linha (Alacritty).** Cada linha guarda `left`/`right`, retângulos são fundidos, e `full` marca redesenho total. Rastreia a posição antiga e nova de cursor e seleção. Mantém 2 quadros. [P] [damage.rs](https://raw.githubusercontent.com/alacritty/alacritty/master/alacritty/src/display/damage.rs)

**Camadas [R].** O `CAMetalLayer` não preserva quadro, logo damage aqui é sobre recalcular menos, não sobre escrever menos pixels finais:

```text
camada grafo   -> textura offscreen, atualiza só com física ou câmera
camada efeitos -> bloom cacheado, sempre derivado da anterior
camada editor  -> instâncias reconstruídas por linha suja
composição     -> blit + UI + caret, sempre barato
```

Digitar não deve tocar nas duas primeiras camadas. Essa é a garantia concreta de que o grafo não atrapalha a escrita, e uma diferença direta contra o caso Obsidian de aba oculta.

**Coalescer entrada.** Chrome 60 entrega eventos contínuos (`mousemove`, `wheel`, `pointermove`) logo antes do rAF; discretos (`keydown`, `mousedown`) vão imediatamente, com os pendentes antes. Resultado: 35% menos hit tests. Mouses chegam a 2000 Hz. [P] [Chrome](https://developer.chrome.com/blog/aligning-input-events)

Regra [R]: fila por tipo; movimento colapsa no mais recente; tecla jamais colapsa. O hit test roda uma vez por quadro. Guarde os pontos intermediários só em traço livre.

**Fatiamento.** Modelo Mapbox: 2 ms por quadro, checagem entre unidades, retomada por índice. Dimensione a unidade para durar bem menos que o orçamento. [P]

**Picking sem travar.** Use mapa espacial na CPU (grade uniforme com as posições do quadro anterior) ou buffer de ID com readback assíncrono. O cosmos.gl usa leitura não bloqueante com `fenceSync`. [S] Com wgpu: `map_async` em buffer de staging, resultado 1 a 3 quadros depois [R]. Nunca espere `device.poll(Wait)` na thread de render.

### 5.5 Threads e estruturas sem lock

| Mecanismo | Quando | Números e notas |
|---|---|---|
| Snapshot `Arc` (rope) | Texto para parser, busca, render | Custo de incremento. [P] Zed |
| `triple_buffer` | Física ou layout → render; um produtor, um consumidor | Wait-free limitado; sem deadlock; só contenção de cache; MPL-2.0; versão 9.0.0. [P] [repo](https://github.com/HadrienG2/triple-buffer) |
| `arc-swap` | Configuração, tema, índice do grafo, leitura muito frequente | Leitura como `Mutex` sem disputa, quase sempre wait-free; escrita cara. Use `load`; `Cache` acelera mais. Doc não dá ns. [P] [docs](https://docs.rs/arc-swap/latest/arc_swap/docs/performance/index.html) |
| Fila limitada | Mensagens à render thread | Ghostty: 64 mensagens, mais `xev.Async` que coalesce wakeups |
| QoS macOS | Prioridades | Ghostty: `user_interactive` com foco; `user_initiated` sem foco; `utility` invisível |

**Topologia proposta [R]:**

```text
principal    : AppKit, entrada, IME; nunca bloqueia
render       : acorda no display link, lê snapshots, grava comandos
física       : passo fixo ou GPU; publica em triple buffer
texto/índice : tree-sitter, busca, links; pool com prioridade baixa
IO           : disco; grava com debounce
```

A entrada precisa sair da principal rápido. O xi mostra o oposto a evitar: leitura pedida por mensagem assíncrona.

### 5.6 Carregamento progressivo e UI otimista

**Figma.** Carrega a página atual e as dependências, depois o resto sob demanda. 33% mais rápido nos 5% mais lentos; 70% menos nós em memória; 33% menos OOM; decodificação paralela 40% mais rápida. Dependências implícitas viraram tipo de aresta. [P] [artigo](https://www.figma.com/blog/speeding-up-file-load-times-one-page-at-a-time/) (2024-05-22)

**Linear (engenharia reversa).** [S] [wzhudev](https://raw.githubusercontent.com/wzhudev/reverse-linear-sync-engine/main/README.md)

- Pool de objetos em memória, por UUID, com observabilidade.
- Bootstrap: `full`, `partial`, `local`. Estratégias por modelo: `instant`, `lazy`, `partial`.
- Escrita: muda em memória, guarda valor antigo, persiste na tabela `__transactions`, envia em lote, reverte se rejeitada.
- Banco local só muda após confirmação; pendências ficam separadas.
- `lastSyncId` global e monotônico detecta lacunas; delta rebaseia pendências sob last-writer-wins.

**Adaptação local-first para este app [R].** Escrita em memória dentro do quadro; journal em disco com fsync em lote (por exemplo a cada 250 ms a 1 s); nunca fsync na thread de entrada. Abertura em ordem: pixels da última sessão (posições + nota aberta) em menos de 100 ms, índice carregado em background, grafo refinado depois.

---

## 6. Sensação de escrita

### 6.1 Latência medida

| Medida | Valor | Fonte |
|---|---|---|
| Apple IIe, tecla → caractere | 30 ms | [P] [Dan Luu](https://danluu.com/input-lag/) |
| Máquinas modernas típicas | 70 a 200 ms | idem |
| Terminal.app, mediana ociosa | 6 ms (interna, tecla → captura) | [P] [term-latency](https://danluu.com/term-latency/) |
| Alacritty, mediana ociosa | 31 ms, em 2017 | idem |
| Teclado: Magic Keyboard | Cerca de 15 ms | [P] [keyboard-latency](https://danluu.com/keyboard-latency/) |
| Teclado: Logitech K360 | Cerca de 60 ms | idem |
| Polling 1000 Hz | Economiza só 3,5 ms em média | idem |
| Orçamento de hardware típico | Cerca de 26 ms; ideal 3 ms | [P] [Fatin](https://pavelfatin.com/typing-with-pleasure/) |
| Sublime, GVim, Atom (Windows, texto puro) | 8,2 / 0,9 / 49,4 ms | idem |
| IDEA normal e zero-latency | 24,7 e 2,9 ms | idem |
| Pintura do editor, alvo | 1 a 5 ms | idem |

Datas: Dan Luu cerca de 2017, Fatin cerca de 2015 [M]. Os números são antigos, mas o método continua válido.

Sobre percepção: toque abaixo de 10 ms é perceptível e muda a expectativa ([S] [TriboTouch](https://karan-ahuja.com/assets/docs/paper/tribotouch.pdf)); inking fica por volta de 50 ms ([S] [Annett, GI 2014](https://webdocs.cs.ualberta.ca/%7Ewfb/publications/C-2014-GI-Latency.pdf)). Dan Luu cita 10 a 20 ms como faixa em que tablets e VR parecem certos. Não encontrei estudo equivalente para teclado.

Afirmações como "Zed 2 ms contra VS Code 12 ms" ([A](https://www.gilricardo.com/blog/zed-editor-review-2026)) não têm método: não use.

**Interpretando "menos de 16 ms" [R].** Total tecla→fóton nunca fica em 16 ms: só o teclado leva 15 ms no melhor caso. A meta correta é o trecho sob controle do app:

```text
keyDown -> buffer + layout da linha + instâncias + present antes do prazo do próximo vsync
meta: 2 a 4 ms de CPU, nunca perder o próximo vsync, p99 < 8 ms
```

A 120 Hz, a espera média por vsync é de cerca de 4 ms; a 60 Hz, 8 ms. Por isso, em digitação, 120 Hz rende mais que qualquer otimização do layout.

### 6.2 Tipografia e foco

**iA Writer.**

- Duospace: `m`, `M`, `w`, `W` a 150% da largura; o resto monoespaçado. Sobre IBM Plex Mono. Dois largos juntos voltam ao ritmo. [P] [artigo](https://ia.net/topics/in-search-of-the-perfect-writing-font)
- Focus Mode: frase ou parágrafo ativo; o resto escurece. Typewriter: cursor fixo verticalmente, sem escurecer. Recomenda desligar ao editar porque o centro pode pular. [P] [docs](https://ia.net/writer/support/editor/focus-mode)
- Georgia 16 px como referência; 12 px pequeno demais; altura de linha em torno de 140%. Layouts adaptativos com poucos breakpoints; evita preto puro sobre branco. [P] [responsive typography](https://ia.net/topics/responsive-typography-the-basics)

**Typora.** Focus escurece tudo exceto linha ou bloco atual. Typewriter mantém o caret, com opção de centralizar só ao digitar, não ao clicar. [P] [docs](https://support.typora.io/Focus-and-Typewriter-Mode/)

**Obsidian Live Preview.** Decorations do CodeMirror 6 trocam a sintaxe por widgets quando o cursor não sobrepõe o trecho. [A] [fórum](https://forum.obsidian.md/t/markdown-formatting-characters-in-live-preview/50439). Reclamação recorrente: ao alternar sintaxe, as linhas deslocam e o cursor pula. [A] [fórum](https://forum.obsidian.md/t/to-hide-markdown-syntax-in-live-preview-so-it-doesn-t-show-formatting-markers/109155). O Atomic Editor corrige mantendo altura estável e tratando Markdown cru como fonte única. [A] [repo](https://github.com/kenforthewin/atomic-editor)

**Regras para o live preview [R]:**

1. Revelar sintaxe nunca muda altura de linha nem a coluna do texto que vem antes do cursor.
2. Marcadores ocultos com fonte de largura zero ou métrica reservada, com fade de 100 ms na revelação.
3. O texto sob o caret não reflui em digitação comum; títulos mudam de tamanho só ao sair da linha.

**Bear, Ulysses, Craft.** Sem dados: a busca acabou antes. Ficam como lacuna.

### 6.3 Cursor animado

Neovide: animação de 150 ms em geral e 40 ms em movimentos curtos de 1 a 2 caracteres, como ao digitar. Valores menores de trail adicionam atraso. [P] [config](https://neovide.dev/configuration.html)

Regras [R]:

- Em digitação, o caret salta no mesmo quadro do glifo, ou anima em até 40 ms.
- Saltos longos (clique, busca): mola de 80 a 120 ms.
- Ao digitar, reinicie a piscada, com o caret sólido por 500 a 600 ms.
- Com tecla repetida, sem animação.
- Rolagem com inércia: cancele a animação assim que chegar entrada nova, herdando velocidade.

---

## 7. Tabela final: técnica → o que adotar, com que parâmetros, como medir

| Técnica | O que adotar | Parâmetros iniciais | Como medir |
|---|---|---|---|
| Primitivos instanciados | Pipeline por tipo, ordem fixa | Rect, sombra, glifo, ícone, imagem, aresta, nó | Chamadas de desenho por quadro abaixo de 30; Metal System Trace |
| SDF de rect | Fragment shader analítico | Cobertura `clamp(0.5 - d, 0, 1)` | Comparar com referência a 2× em screenshot |
| Sombra | `erf` | 4 amostras em ±3σ | Tempo GPU contra blur; visual |
| Apresentação | `Fifo`, latência 1, relógio via `NSView.displayLink` | `desired_maximum_frame_latency = 1`, faixa 60–120 | Intervalo de vsync: alvo de desvio padrão abaixo de 1 ms; referência 0,39 contra 4,09 ms |
| ProMotion | Permanência em alta taxa após entrada | 1 s após última entrada | Fps real por histograma, não mediana |
| Redesenho sob demanda | Máquina de estados da seção 5.3 | Zero quadros em repouso | Aba Energia do Monitor de Atividade; `powermetrics`; contagem de quadros ociosos = 0 |
| Resize | Desenho síncrono com transação | Só no resize | Gravação a 240 fps, sem esticar |
| Atlas | Alfa, `etagere`, lazy | Atlas 1024 ou 2048; 4×1 subpixel | Taxa de acerto acima de 99%; ms de upload por quadro |
| Gamma de texto | Tabela por luminância | Gamma 1,8, contraste 1,0; ajustar contra TextEdit | Comparação A/B claro e escuro |
| Shaping | Cache de 2 gerações por linha | Reuso por região | Tempo de shaping por quadro em digitação, abaixo de 0,5 ms |
| Rope | `ropey` ou SumTree próprio | Snapshot por edição | Tempo de edição em arquivo de 100 MB |
| Virtualização | `SumTree` de alturas | Viewport + 1 tela | Tempo de abertura; instâncias por quadro |
| Highlight | Tree-sitter em background | Reparse após snapshot, sem esperar | Latência do reparse; quadros perdidos: 0 |
| IME | `NSView` própria, `NSTextInputClient` completo | `interpretKeyEvents` primeiro; clamp UTF-16 | Roteiro manual: ABC Extended, Pinyin, Kotoeri, Character Viewer |
| Física | Compute, exata pequena, grade/BH grande | θ 0,8–1,2; FA2 tolerância 0,1 até 5 mil nós | Tempo GPU por `timestamp query`, abaixo de 2 ms |
| Layout inicial | Multinível em background | graph-tool: cooling 0,95, θ 0,6 | Tempo até 90% da qualidade; stress médio |
| Incremental | Ancorar o resto, aquecer 1–2 saltos | `alphaMin 0,001`, `velocityDecay 0,4` | Deslocamento médio de nós não afetados |
| Rótulos | Prioridade + GridIndex + fade | Células de 30 px, 2 ms por fatia, 150 ms, fade 200–300 ms | Colisões por quadro; trocas de rótulo por segundo |
| LOD | Por tamanho em tela e movimento | Limiar 6 px; arestas simplificadas em movimento | Quadros acima do orçamento em pan rápido |
| Arestas | Quad instanciado com cápsula SDF | `largura/2 + 1 px`; mínimo 1 px | Tempo GPU com 100 mil arestas; aliasing visual |
| Bundling | Só sob demanda | FDEB C=6, K=0,1, S=0,1; hammer 0,05/0,7/4 | Tempo em background; legibilidade |
| Bloom | Jimenez em `Rgba16Float` | 4–5 mips; força 0,04; Karis no 1º | Tempo GPU em 2× resolução, abaixo de 1,5 ms |
| Blur de UI | Dual Kawase | 2–4 iterações [M] | Tempo GPU |
| Molas | Fechada, herdando velocidade | Duração 0,25–0,35 s; bounce até 0,12 | Teste com `dt` variável; continuidade da velocidade |
| Câmera | van Wijk | ρ = √2; limite 400–800 ms | Teste de uso; ms por quadro no zoom |
| Partículas | Primeiro shader; depois compute | Até 5 mil, só em foco | Tempo GPU; bateria com efeito ligado |
| Fundo vivo | Reaction-diffusion ou fluido, breve | RD `f 0,055`, `k 0,062`; fluido sim 64–96 | Watts extras; deve desligar sozinho |
| Dither | Triangular no último passe | ±2/255 RGB | Inspeção em gradiente escuro de 5–13% |
| Coalescência | Movimento por quadro; tecla imediata | Um hit test por quadro | Hit tests por segundo; fila vazia a cada quadro |
| Fatiamento | Mapbox | 2 ms, unidade pequena | Maior fatia, histograma |
| Threads | Snapshot, triple buffer, arc-swap | Ver 5.5 | Thread sanitizer; zero espera em lock na render |
| Camadas | Grafo offscreen, editor separado | Digitar não invalida grafo | Quadro de digitação com grafo em execução: sem aumento |
| Partida | Pixels primeiro, índice depois | Abaixo de 100 ms até primeira imagem [R] | Marcas `os_signpost` da abertura |
| Otimista | Memória + journal assíncrono | fsync em lote a cada 250 ms–1 s | Sem fsync na thread principal; teste de crash |
| Latência de digitação | Medir no CI | 200 caracteres, 150 ms de intervalo, como Fatin | Typometer; câmera de 1000 fps para o total; p50, p99, máximo |
| Regressão | Cenários fixos | Margem de 20% na nuvem; p50 e p90 de fps (Figma) | Bisseção em máquina física, como a Figma |

**Orçamento inicial de quadro a 120 Hz [R].** São dois orçamentos em paralelo, cada um abaixo de 8,33 ms, porque CPU e GPU trabalham em pipeline. CPU: até 3 ms (1 ms entrada e estado, 1 ms texto, 1 ms encode). GPU: até 4 ms (2 ms física, 2 ms render). A 60 Hz, a folga permite ativar efeitos mais caros, sempre que houver margem medida.

**Instrumentação mínima [R].** Marcar `os_signpost` em entrada, início do quadro, encode, `present` e fim da GPU. `MTLCommandBuffer` dá `GPUStartTime` e `GPUEndTime`; wgpu dá `timestamp` queries. Instruments com Metal System Trace mostra perdas de vsync. Tracy ou puffin cobrem a CPU. Metal HUD serve para checagem rápida, mas o relato do fórum mostra que ele pode divergir da medição real.

**Figma como modelo de disciplina.** Margem de 20% em VMs com GPU e laptops antigos para bisseção; guarda de desempenho em menos de 10 minutos por PR. [P] [Keeping Figma Fast](https://www.figma.com/blog/keeping-figma-fast/) (2023-08-29)

---

## 8. Lacunas e cautelas

**Limite de busca.** O orçamento de buscas se esgotou durante a pesquisa. Depois disso, só li páginas diretas já conhecidas. Para completar, é só pedir uma nova rodada.

Não obtido:

- Bear, Ulysses e Craft: nada verificado.
- Medições atuais com Typometer para Zed, Obsidian e Neovim. Só há testes antigos, avulsos ou sem método.
- Rio: internals e números do damage.
- Figma: como funcionam os tiles hoje. A única confirmação é de 2015.
- Hu (sfdp): PDF ilegível nesta máquina; parâmetros do artigo não conferidos.
- McGuire (motion blur): página vazia.
- Quilez (domain warp) e curl noise: página indisponível.
- Dual Kawase: tabela em ms do original, e os kernels vieram de memória.
- Partículas em arestas com compute: sem fonte; desenho meu.
- Gephi Lite: documentação própria não localizada. Números antigos do sigma (2015) não usados.
- Correção de gamma do Zed no macOS e stem darkening no Skia.

Pontos que exigem teste próprio antes de virar decisão:

- Rota de `CAMetalDisplayLink` com wgpu (seção 5.2).
- Se Parley ou cosmic-text aguenta o editor sob digitação com texto rico.
- `direct-to-display` no macOS 26/27 com bloom e transparência.
- Fragilidade do ProMotion em macOS 27: relatos são de beta e o estado final não foi verificado.
- Tudo marcado [R] e [A]. Em especial, as faixas numéricas sem fonte são pontos de partida.
