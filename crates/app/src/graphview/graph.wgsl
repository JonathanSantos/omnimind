// Feixes e Raízes. Fitas e pontos num alvo opaco: soma no escuro, subtrai no claro.
struct Frame {
  res: vec2f,
  scale: f32,
  light: f32,
  span: f32,
  head: f32,
  // Deslocamento do desenho na tela, em pixels: é o arrasto.
  offset: vec2f,
  // Escala de espessuras e pontos. Cresce menos que `scale` no zoom, para a linha não virar faixa.
  wscale: f32,
  // Sete fatias de tema e, por último, a cor neutra da poeira.
  pal: array<vec4f, 8>,
}

struct Draw {
  p: f32,
  use_delay: f32,
  hi: f32,
  hi_a: f32,
  add: f32,
  rev: f32,
  head_gain: f32,
  dim: f32,
  soft: f32,
}

@group(0) @binding(0) var<uniform> f: Frame;
@group(0) @binding(1) var<uniform> d: Draw;

struct LineIn {
  @location(0) pos: vec2f,
  @location(1) normal: vec2f,
  @location(2) side: f32,
  @location(3) along: f32,
  @location(4) delay: f32,
  @location(5) width: f32,
  @location(6) alpha: f32,
  @location(7) themes: vec2f,
}

struct LineOut {
  @builtin(position) pos: vec4f,
  @location(0) along: f32,
  @location(1) reveal: f32,
  @location(2) edge: f32,
  @location(3) half_w: f32,
  @location(4) alpha: f32,
  @location(5) c0: vec3f,
  @location(6) c1: vec3f,
}

fn to_clip(px: vec2f) -> vec4f {
  return vec4f(px.x * 2.0 / f.res.x, -px.y * 2.0 / f.res.y, 0.0, 1.0);
}

@vertex
fn line_vs(v: LineIn) -> LineOut {
  var o: LineOut;
  let w = v.width * f.wscale + d.add * d.hi;
  let h = max(w, 1.0) * 0.5;
  o.pos = to_clip(v.pos * f.scale + f.offset + v.normal * v.side * (h + 1.0));
  o.along = v.along;
  o.edge = v.side * (h + 1.0);
  o.half_w = h;
  o.reveal = select(d.p, (d.p - v.delay) / f.span, d.use_delay > 0.5);
  o.alpha = mix(v.alpha, d.hi_a, d.hi) * min(w, 1.0);
  o.c0 = f.pal[i32(v.themes.x)].rgb;
  o.c1 = f.pal[i32(v.themes.y)].rgb;
  return o;
}

// Cor e cobertura de um ponto da fita. Negativo: ainda não foi revelado.
fn line_ink(i: LineOut) -> vec4f {
  let a = select(i.along, 1.0 - i.along, d.rev > 0.5);
  let gap = a - i.reveal;
  if gap > 0.0 {
    return vec4f(-1.0);
  }
  let head = smoothstep(-f.head, 0.0, gap) * d.head_gain;
  var e = clamp(i.half_w - abs(i.edge) + 0.5, 0.0, 1.0);
  // Brilho: a mesma fita, larga e sem borda, mais forte no meio.
  if d.soft > 0.5 {
    e = pow(clamp(1.0 - abs(i.edge) / max(i.half_w, 1.0), 0.0, 1.0), 1.7);
  }
  let al = clamp(i.alpha * e * d.dim * (1.0 + head), 0.0, 1.0);
  return vec4f(mix(i.c0, i.c1, i.along), al);
}

// Destaques: somam luz no escuro e tiram no claro.
@fragment
fn line_fs(i: LineOut) -> @location(0) vec4f {
  let k = line_ink(i);
  if k.a < 0.0 {
    discard;
  }
  return vec4f(mix(k.rgb, 1.0 - k.rgb, f.light) * k.a, 1.0);
}

// Base: cobre. Ligações empilhadas chegam à cor do tema e param ali.
@fragment
fn line_over_fs(i: LineOut) -> @location(0) vec4f {
  let k = line_ink(i);
  if k.a < 0.0 {
    discard;
  }
  return vec4f(k.rgb * k.a, k.a);
}

struct DotIn {
  @builtin(vertex_index) vi: u32,
  @location(0) pos: vec2f,
  @location(1) size: f32,
  @location(2) delay: f32,
  @location(3) alpha: f32,
  @location(4) theme: f32,
  @location(5) kind: f32,
}

struct DotOut {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
  @location(1) a: f32,
  @location(2) s: f32,
  @location(3) c: vec3f,
  @location(4) kind: f32,
}

@vertex
fn dot_vs(v: DotIn) -> DotOut {
  var corners = array<vec2f, 6>(vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(-1.0, 1.0), vec2f(-1.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0));
  let uv = corners[v.vi];
  var r = clamp((d.p - v.delay) / 0.09, 0.0, 1.0);
  r = r * r * (3.0 - 2.0 * r);
  // O anel precisa de um pouco mais de corpo que o disco para ler no mesmo tamanho.
  let grow = select(1.0, 1.22, v.kind > 0.5);
  // `add` aumenta o ponto inteiro: é o que faz do mesmo item um halo em volta do miolo.
  let s = max(v.size * 2.0 * grow * (1.0 + d.add) * f.wscale * (0.4 + 0.6 * r), 1.5) + 1.5;
  var o: DotOut;
  o.kind = v.kind;
  o.pos = to_clip(v.pos * f.scale + f.offset + uv * s * 0.5);
  o.uv = uv;
  o.a = v.alpha * r * d.dim;
  o.s = s;
  o.c = f.pal[i32(v.theme)].rgb;
  return o;
}

fn dot_cover(i: DotOut) -> f32 {
  let r = length(i.uv);
  let h = i.s * 0.5;
  var a = clamp((1.0 - r) * h, 0.0, 1.0);
  // Nota: anel com um miolo pequeno. Muito pequeno não dá para ler o furo, e fica disco.
  if i.kind > 0.5 && d.soft < 0.5 && i.s > 9.0 {
    // Anel fino e miolo pequeno, como o planeta do Universo: a nota se distingue sem pesar mais que a memória.
    let ring = a * clamp((r - 0.76) * h + 0.5, 0.0, 1.0);
    let core = clamp((0.32 - r) * h + 0.5, 0.0, 1.0);
    a = max(ring, core);
  }
  if d.soft > 0.5 {
    a = pow(max(1.0 - r, 0.0), 2.2);
  }
  return a * i.a * select(1.0, d.hi_a, d.soft > 0.5 && d.hi_a > 0.0);
}

@fragment
fn dot_fs(i: DotOut) -> @location(0) vec4f {
  let a = dot_cover(i);
  return vec4f(mix(i.c, 1.0 - i.c, f.light) * a, 1.0);
}

@fragment
fn dot_over_fs(i: DotOut) -> @location(0) vec4f {
  let a = clamp(dot_cover(i), 0.0, 1.0);
  return vec4f(i.c * a, a);
}
