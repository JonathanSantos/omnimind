struct Globals { size: vec2<f32>, scale: f32, time: f32 };
@group(0) @binding(0) var<uniform> g: Globals;
@group(1) @binding(0) var mask: texture_2d<f32>;
@group(1) @binding(1) var color_atlas: texture_2d<f32>;
@group(1) @binding(2) var samp: sampler;
@group(2) @binding(0) var image: texture_2d<f32>;
@group(2) @binding(1) var image_samp: sampler;

const QUAD = array<vec2<f32>,6>(vec2(0.,0.),vec2(1.,0.),vec2(1.,1.),vec2(0.,0.),vec2(1.,1.),vec2(0.,1.));

fn clip_pos(p: vec2<f32>) -> vec4<f32> {
  return vec4(p.x / g.size.x * 2. - 1., 1. - p.y / g.size.y * 2., 0., 1.);
}
fn clip_cov(p: vec2<f32>, c: vec4<f32>) -> f32 {
  let s = g.scale;
  let a = clamp((p - c.xy) * s + .5, vec2(0.), vec2(1.));
  let b = clamp((c.xy + c.zw - p) * s + .5, vec2(0.), vec2(1.));
  return a.x * a.y * b.x * b.y;
}

struct RectIn {
  @location(0) bounds: vec4<f32>, @location(1) clip: vec4<f32>, @location(2) fill: vec4<f32>,
  @location(3) fill2: vec4<f32>, @location(4) border: vec4<f32>, @location(5) params: vec4<f32>,
};
struct RectV {
  @builtin(position) pos: vec4<f32>, @location(0) p: vec2<f32>,
  @location(1) @interpolate(flat) bounds: vec4<f32>, @location(2) @interpolate(flat) clip: vec4<f32>,
  @location(3) @interpolate(flat) fill: vec4<f32>, @location(4) @interpolate(flat) fill2: vec4<f32>,
  @location(5) @interpolate(flat) border: vec4<f32>, @location(6) @interpolate(flat) params: vec4<f32>,
};
@vertex fn rect_vs(@builtin(vertex_index) i: u32, r: RectIn) -> RectV {
  let pad = r.params.z * 2. + 1.;
  let p = r.bounds.xy - pad + QUAD[i] * (r.bounds.zw + pad * 2.);
  var o: RectV; o.pos = clip_pos(p); o.p = p; o.bounds = r.bounds; o.clip = r.clip;
  o.fill = r.fill; o.fill2 = r.fill2; o.border = r.border; o.params = r.params; return o;
}
fn sd_round(p: vec2<f32>, h: vec2<f32>, r: f32) -> f32 {
  let q = abs(p) - h + r;
  return length(max(q, vec2(0.))) + min(max(q.x, q.y), 0.) - r;
}
@fragment fn rect_fs(v: RectV) -> @location(0) vec4<f32> {
  let h = v.bounds.zw * .5;
  let c = v.bounds.xy + h;
  let radius = min(v.params.x, min(h.x, h.y));
  let d = sd_round(v.p - c, h, radius);
  let blur = v.params.z;
  var col = v.fill;
  let ang = v.params.w;
  if (ang != 0.) {
    let dir = vec2(cos(ang), sin(ang));
    let ext = abs(dir.x) * h.x + abs(dir.y) * h.y;
    let t = clamp(dot(v.p - c, dir) / (ext * 2.) + .5, 0., 1.);
    col = mix(v.fill, v.fill2, t);
  }
  var cov: f32;
  if (blur > 0.) {
    let t = clamp((d + blur) / (blur * 2.), 0., 1.);
    cov = 1. - t * t * (3. - 2. * t);
  } else {
    cov = clamp(.5 - d * g.scale, 0., 1.);
  }
  let bw = v.params.y;
  if (bw > 0.) {
    let inner = clamp(.5 - (d + bw) * g.scale, 0., 1.);
    col = mix(v.border, col, inner);
  }
  return col * cov * clip_cov(v.p, v.clip);
}

struct GlyphIn {
  @location(0) bounds: vec4<f32>, @location(1) uv: vec4<f32>, @location(2) clip: vec4<f32>,
  @location(3) color: vec4<f32>, @location(4) kind: f32,
};
struct GlyphV {
  @builtin(position) pos: vec4<f32>, @location(0) p: vec2<f32>, @location(1) uv: vec2<f32>,
  @location(2) @interpolate(flat) clip: vec4<f32>, @location(3) @interpolate(flat) color: vec4<f32>,
  @location(4) @interpolate(flat) kind: f32,
};
@vertex fn glyph_vs(@builtin(vertex_index) i: u32, r: GlyphIn) -> GlyphV {
  let q = QUAD[i];
  let p = r.bounds.xy + q * r.bounds.zw;
  var o: GlyphV; o.pos = clip_pos(p); o.p = p; o.uv = r.uv.xy + q * r.uv.zw;
  o.clip = r.clip; o.color = r.color; o.kind = r.kind; return o;
}
@fragment fn glyph_fs(v: GlyphV) -> @location(0) vec4<f32> {
  let c = clip_cov(v.p, v.clip);
  if (v.kind > .5) {
    return textureSample(color_atlas, samp, v.uv) * v.color.a * c;
  }
  var a = textureSample(mask, samp, v.uv).r;
  // Contraste de glifo: engrossa texto escuro em fundo claro, afina o claro no escuro.
  let lum = dot(v.color.rgb, vec3(.3, .59, .11)) / max(v.color.a, .001);
  a = pow(a, mix(.82, 1.08, lum));
  return v.color * a * c;
}

struct SegIn {
  @location(0) ab: vec4<f32>, @location(1) clip: vec4<f32>, @location(2) color: vec4<f32>,
  @location(3) color2: vec4<f32>, @location(4) width: f32,
};
struct SegV {
  @builtin(position) pos: vec4<f32>, @location(0) p: vec2<f32>,
  @location(1) @interpolate(flat) ab: vec4<f32>, @location(2) @interpolate(flat) clip: vec4<f32>,
  @location(3) @interpolate(flat) c1: vec4<f32>, @location(4) @interpolate(flat) c2: vec4<f32>,
  @location(5) @interpolate(flat) w: f32,
};
@vertex fn seg_vs(@builtin(vertex_index) i: u32, r: SegIn) -> SegV {
  let a = r.ab.xy; let b = r.ab.zw;
  let d = normalize(b - a + vec2(1e-5, 0.));
  let n = vec2(-d.y, d.x);
  let e = r.width * .5 + 1.;
  let q = QUAD[i] * 2. - 1.;
  let p = mix(a, b, QUAD[i].x) + d * q.x * e + n * q.y * e;
  var o: SegV; o.pos = clip_pos(p); o.p = p; o.ab = r.ab; o.clip = r.clip; o.c1 = r.color; o.c2 = r.color2; o.w = r.width; return o;
}
@fragment fn seg_fs(v: SegV) -> @location(0) vec4<f32> {
  let a = v.ab.xy; let b = v.ab.zw;
  let pa = v.p - a; let ba = b - a;
  let h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-5), 0., 1.);
  let d = length(pa - ba * h) - v.w * .5;
  let cov = clamp(.5 - d * g.scale, 0., 1.);
  return mix(v.c1, v.c2, h) * cov * clip_cov(v.p, v.clip);
}

struct ImgV {
  @builtin(position) pos: vec4<f32>, @location(0) p: vec2<f32>, @location(1) uv: vec2<f32>,
  @location(2) @interpolate(flat) clip: vec4<f32>, @location(3) @interpolate(flat) tint: vec4<f32>,
};
@vertex fn image_vs(@builtin(vertex_index) i: u32, @location(0) rect: vec4<f32>, @location(1) clip: vec4<f32>, @location(2) tint: vec4<f32>) -> ImgV {
  let q = QUAD[i];
  let p = rect.xy + q * rect.zw;
  var o: ImgV; o.pos = clip_pos(p); o.p = p; o.uv = q; o.clip = clip; o.tint = tint; return o;
}
@fragment fn image_fs(v: ImgV) -> @location(0) vec4<f32> {
  return textureSample(image, image_samp, v.uv) * v.tint * clip_cov(v.p, v.clip);
}
