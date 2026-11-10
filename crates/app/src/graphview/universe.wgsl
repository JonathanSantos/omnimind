// Universo. Corpos são cartões voltados para a câmera; ligações são segmentos abertos na tela.
// Mesmo alvo opaco dos outros grafos: soma luz no escuro, tira no claro.
struct Frame {
  right: vec4f,
  up: vec4f,
  fwd: vec4f,
  eye: vec4f,
  res: vec2f,
  focal: f32,
  light: f32,
  fog: vec2f,
  pad: vec2f,
  pal: array<vec4f, 8>,
}

struct Draw {
  open: f32,
  spread: f32,
  dim: f32,
  pad: f32,
}

@group(0) @binding(0) var<uniform> f: Frame;
@group(0) @binding(1) var<uniform> d: Draw;

const NEAR: f32 = 4.0;

// x e y em unidades do espaço diante da câmera, z é a profundidade.
fn to_view(p: vec3f) -> vec3f {
  let q = p - f.eye.xyz;
  return vec3f(dot(q, f.right.xyz), dot(q, f.up.xyz), dot(q, f.fwd.xyz));
}

// Pixels a partir do centro, y para cima.
fn to_px(v: vec3f) -> vec2f {
  return v.xy * (f.focal / v.z) * f.res.y * 0.5;
}

fn to_clip(px: vec2f) -> vec4f {
  return vec4f(px.x * 2.0 / f.res.x, px.y * 2.0 / f.res.y, 0.5, 1.0);
}

// Longe fica mais apagado: é o que dá profundidade sem depender de sombra.
fn fog(z: f32) -> f32 {
  return mix(1.0, 0.34, clamp((z - f.fog.x) / max(f.fog.y - f.fog.x, 1.0), 0.0, 1.0));
}

fn ink(c: vec3f, a: f32) -> vec4f {
  return vec4f(mix(c, 1.0 - c, f.light) * a, 1.0);
}

struct BodyIn {
  @builtin(vertex_index) vi: u32,
  @location(0) pos: vec3f,
  @location(1) size: f32,
  @location(2) theme: f32,
  @location(3) alpha: f32,
  @location(4) delay: f32,
  @location(5) kind: f32,
}

struct BodyOut {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
  @location(1) a: f32,
  @location(2) half_px: f32,
  @location(3) c: vec3f,
  @location(4) kind: f32,
}

@vertex
fn body_vs(v: BodyIn) -> BodyOut {
  var corners = array<vec2f, 6>(vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(-1.0, 1.0), vec2f(-1.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0));
  let uv = corners[v.vi];
  var o: BodyOut;
  // Poeira de fundo (3) não participa da abertura: já estava lá.
  let dust = v.kind > 2.5;
  let view = to_view(v.pos * select(d.spread, 1.0, dust));
  var r = clamp((d.open - v.delay) / 0.1, 0.0, 1.0);
  r = r * r * (3.0 - 2.0 * r);
  if dust {
    r = 1.0;
  }
  if view.z < NEAR {
    o.pos = vec4f(2.0, 2.0, 2.0, 1.0);
    return o;
  }
  let px = f.focal / view.z * f.res.y * 0.5;
  let half = max(select(v.size * px, v.size * f.res.y / 1800.0, dust) * (0.35 + 0.65 * r), 1.6);
  o.pos = to_clip(to_px(view) + uv * (half + 1.0));
  o.uv = uv * (half + 1.0) / half;
  o.half_px = half;
  o.a = v.alpha * r * d.dim * fog(view.z);
  o.c = f.pal[i32(v.theme)].rgb;
  o.kind = v.kind;
  return o;
}

@fragment
fn body_fs(i: BodyOut) -> @location(0) vec4f {
  let r = length(i.uv);
  let h = i.half_px;
  var a = 0.0;
  if i.kind < 0.5 {
    // Estrela: miolo firme e halo.
    let core = clamp((0.34 - r) * h + 0.5, 0.0, 1.0);
    let halo = pow(max(1.0 - r, 0.0), 2.4) * 0.5;
    a = max(core, halo);
  } else if i.kind < 1.5 {
    // Planeta: disco menor e um anel fino.
    let core = clamp((0.26 - r) * h + 0.5, 0.0, 1.0);
    let ring = clamp(1.0 - abs(r - 0.66) * h * 0.85, 0.0, 1.0) * 0.8;
    let halo = pow(max(1.0 - r, 0.0), 2.6) * 0.22;
    a = max(max(core, ring), halo);
  } else if i.kind < 2.5 {
    // Brilho e névoa.
    a = pow(max(1.0 - r, 0.0), 2.0);
  } else {
    a = clamp((1.0 - r) * h, 0.0, 1.0);
  }
  return ink(i.c, clamp(a * i.a, 0.0, 1.0));
}

struct LinkIn {
  @builtin(vertex_index) vi: u32,
  @location(0) a: vec3f,
  @location(1) theme_a: f32,
  @location(2) b: vec3f,
  @location(3) theme_b: f32,
  @location(4) alpha: f32,
  @location(5) width: f32,
  @location(6) delay: f32,
}

struct LinkOut {
  @builtin(position) pos: vec4f,
  @location(0) edge: f32,
  @location(1) half_w: f32,
  @location(2) a: f32,
  @location(3) c: vec3f,
}

@vertex
fn link_vs(v: LinkIn) -> LinkOut {
  // Para cada canto: em que ponta está, e de que lado.
  var ends = array<vec2f, 6>(vec2f(0.0, -1.0), vec2f(1.0, -1.0), vec2f(0.0, 1.0), vec2f(0.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0));
  let e = ends[v.vi];
  var o: LinkOut;
  var va = to_view(v.a * d.spread);
  var vb = to_view(v.b * d.spread);
  if va.z < NEAR && vb.z < NEAR {
    o.pos = vec4f(2.0, 2.0, 2.0, 1.0);
    return o;
  }
  // Uma ponta atrás da câmera: corta o segmento no plano próximo.
  if va.z < NEAR {
    va = mix(va, vb, (NEAR - va.z) / (vb.z - va.z));
  }
  if vb.z < NEAR {
    vb = mix(vb, va, (NEAR - vb.z) / (va.z - vb.z));
  }
  let pa = to_px(va);
  let pb = to_px(vb);
  let along = pb - pa;
  let l = max(length(along), 0.0001);
  let n = vec2f(-along.y, along.x) / l;
  let scale = f.res.y / 1800.0;
  let w = max(v.width * scale, 1.0);
  let h = w * 0.5;
  o.pos = to_clip(mix(pa, pb, e.x) + n * e.y * (h + 1.0));
  o.edge = e.y * (h + 1.0);
  o.half_w = h;
  var r = clamp((d.open - v.delay) / 0.12, 0.0, 1.0);
  r = r * r * (3.0 - 2.0 * r);
  o.a = v.alpha * r * d.dim * fog(mix(va.z, vb.z, e.x)) * min(v.width * scale, 1.0);
  o.c = mix(f.pal[i32(v.theme_a)].rgb, f.pal[i32(v.theme_b)].rgb, e.x);
  return o;
}

@fragment
fn link_fs(i: LinkOut) -> @location(0) vec4f {
  let e = clamp(i.half_w - abs(i.edge) + 0.5, 0.0, 1.0);
  return ink(i.c, clamp(i.a * e, 0.0, 1.0));
}
