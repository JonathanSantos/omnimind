struct Globals { size: vec2<f32>, cam: vec2<f32>, zoom: f32, time: f32, _p: vec2<f32> };
@group(0) @binding(0) var<uniform> g: Globals;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

const QUAD = array<vec2<f32>,6>(vec2(-1.,-1.),vec2(1.,-1.),vec2(1.,1.),vec2(-1.,-1.),vec2(1.,1.),vec2(-1.,1.));

fn to_clip(p: vec2<f32>) -> vec4<f32> {
  let s = (p - g.cam) * g.zoom + g.size * .5;
  return vec4(s.x / g.size.x * 2. - 1., 1. - s.y / g.size.y * 2., 0., 1.);
}
struct NV { @builtin(position) p: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) c: vec4<f32>, @location(2) r: f32 };
@vertex fn node_vs(@builtin(vertex_index) i: u32, @location(0) pos: vec2<f32>, @location(1) r: f32, @location(2) c: vec4<f32>) -> NV {
  let q = QUAD[i]; let k = 3.0;
  var o: NV; o.p = to_clip(pos + q * r * k); o.uv = q * k; o.c = c; o.r = r * g.zoom; return o;
}
@fragment fn node_fs(v: NV) -> @location(0) vec4<f32> {
  let d = length(v.uv);
  let aa = 1.0 / max(v.r, .5);
  let core = 1.0 - smoothstep(1.0 - aa, 1.0 + aa, d);
  let glow = exp(-d * 1.6) * .55 * (1.0 + .25 * sin(g.time * 2. + v.c.r * 40.));
  let a = core + (1. - core) * glow;
  return vec4(v.c.rgb * a, a);
}
struct EV { @builtin(position) p: vec4<f32>, @location(0) t: f32, @location(1) c: vec4<f32>, @location(2) w: f32 };
@vertex fn edge_vs(@builtin(vertex_index) i: u32, @location(0) a: vec2<f32>, @location(1) b: vec2<f32>, @location(2) c: vec4<f32>) -> EV {
  let q = QUAD[i]; let t = (q.x + 1.) * .5;
  let d = normalize(b - a + vec2(1e-5)); let n = vec2(-d.y, d.x);
  let w = 1.4 / g.zoom;
  var o: EV; o.p = to_clip(mix(a, b, t) + n * q.y * w); o.t = q.y; o.c = c; o.w = t; return o;
}
@fragment fn edge_fs(v: EV) -> @location(0) vec4<f32> {
  let a = (1. - smoothstep(.3, 1., abs(v.t))) * v.c.a;
  let pulse = smoothstep(.96, 1., sin(v.w * 10. - g.time * 3.)) * .6;
  return vec4(v.c.rgb * (a + pulse * a), a);
}
struct TV { @builtin(position) p: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) c: vec4<f32> };
@vertex fn text_vs(@builtin(vertex_index) i: u32, @location(0) xy: vec2<f32>, @location(1) wh: vec2<f32>, @location(2) uvxy: vec2<f32>, @location(3) uvwh: vec2<f32>, @location(4) c: vec4<f32>) -> TV {
  let q = (QUAD[i] + 1.) * .5;
  var o: TV; let s = xy + q * wh;
  o.p = vec4(s.x / g.size.x * 2. - 1., 1. - s.y / g.size.y * 2., 0., 1.); o.uv = uvxy + q * uvwh; o.c = c; return o;
}
@fragment fn text_fs(v: TV) -> @location(0) vec4<f32> {
  let a = textureSample(atlas, samp, v.uv).r * v.c.a;
  return vec4(v.c.rgb * a, a);
}
