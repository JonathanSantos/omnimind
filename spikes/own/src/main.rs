use bytemuck::{Pod, Zeroable};
use cosmic_text::{Attrs, Buffer, CacheKey, Family, FontSystem, Metrics, Shaping, SwashCache, SwashContent};
use rustc_hash::FxHashMap;
use std::time::Instant;
use wgpu::*;

const W: u32 = 2560;
const H: u32 = 1600;
const ATLAS: u32 = 2048;
const FMT: TextureFormat = TextureFormat::Bgra8UnormSrgb;

#[repr(C)] #[derive(Clone, Copy, Pod, Zeroable)] struct Globals { size: [f32;2], cam: [f32;2], zoom: f32, time: f32, _p: [f32;2] }
#[repr(C)] #[derive(Clone, Copy, Pod, Zeroable)] struct Node { pos: [f32;2], r: f32, c: [f32;4] }
#[repr(C)] #[derive(Clone, Copy, Pod, Zeroable)] struct Edge { a: [f32;2], b: [f32;2], c: [f32;4] }
#[repr(C)] #[derive(Clone, Copy, Pod, Zeroable)] struct Glyph { xy: [f32;2], wh: [f32;2], uvxy: [f32;2], uvwh: [f32;2], c: [f32;4] }

struct Graph { p: Vec<[f32;2]>, v: Vec<[f32;2]>, e: Vec<(u32,u32)>, deg: Vec<u32>, label: Vec<String> }
fn rnd(s: &mut u64) -> f32 { *s ^= *s << 13; *s ^= *s >> 7; *s ^= *s << 17; ((*s >> 40) as f32) / (1u64<<24) as f32 }
fn make_graph(n: usize) -> Graph {
    let mut s = 88172645463325252u64;
    let words = ["decisão","rota","preço","marca","teste","ação","memória","hábito","revisão","órgão","maçã","coração","índice","fluxo","sessão"];
    let mut g = Graph { p: vec![], v: vec![[0.;2];n], e: vec![], deg: vec![0;n], label: vec![] };
    for i in 0..n {
        g.p.push([(rnd(&mut s)-0.5)*1600., (rnd(&mut s)-0.5)*1000.]);
        g.label.push(format!("{} {}", words[i%words.len()], i));
        if i>0 { for _ in 0..2 { let j=(rnd(&mut s).powi(2)*i as f32) as usize; g.e.push((i as u32,j as u32)); g.deg[i]+=1; g.deg[j]+=1; } }
    }
    g
}
// grid-repulsion step (spike only)
fn step(g: &mut Graph) {
    let n=g.p.len(); let cell=60f32; let mut grid: FxHashMap<(i32,i32),Vec<u32>> = FxHashMap::default();
    for i in 0..n { grid.entry(((g.p[i][0]/cell) as i32,(g.p[i][1]/cell) as i32)).or_default().push(i as u32); }
    let mut f=vec![[0f32;2];n];
    for i in 0..n { let (cx,cy)=((g.p[i][0]/cell) as i32,(g.p[i][1]/cell) as i32);
        for dx in -1..=1 { for dy in -1..=1 { if let Some(l)=grid.get(&(cx+dx,cy+dy)) { for &j in l { let j=j as usize; if j==i {continue}
            let d=[g.p[i][0]-g.p[j][0],g.p[i][1]-g.p[j][1]]; let r2=d[0]*d[0]+d[1]*d[1]+1.; let k=900./r2; f[i][0]+=d[0]*k; f[i][1]+=d[1]*k; }}}}
        f[i][0]-=g.p[i][0]*0.002; f[i][1]-=g.p[i][1]*0.002; }
    for &(a,b) in &g.e { let (a,b)=(a as usize,b as usize); let d=[g.p[b][0]-g.p[a][0],g.p[b][1]-g.p[a][1]]; let l=(d[0]*d[0]+d[1]*d[1]).sqrt()+1e-3; let k=(l-40.)*0.01/l; f[a][0]+=d[0]*k;f[a][1]+=d[1]*k;f[b][0]-=d[0]*k;f[b][1]-=d[1]*k; }
    for i in 0..n { g.v[i][0]=(g.v[i][0]+f[i][0])*0.85; g.v[i][1]=(g.v[i][1]+f[i][1])*0.85; g.p[i][0]+=g.v[i][0]; g.p[i][1]+=g.v[i][1]; }
}

struct Text { fs: FontSystem, sc: SwashCache, map: FxHashMap<CacheKey,(u16,u16,u16,u16,i16,i16)>, x:u32,y:u32,row:u32, pending: Vec<(u32,u32,u32,u32,Vec<u8>)> }
impl Text {
    fn glyph(&mut self, k: CacheKey) -> Option<(u16,u16,u16,u16,i16,i16)> {
        if let Some(v)=self.map.get(&k) { return Some(*v) }
        let img=self.sc.get_image_uncached(&mut self.fs,k)?;
        if img.content!=SwashContent::Mask { return None }
        let (w,h)=(img.placement.width,img.placement.height);
        if self.x+w+1>=ATLAS { self.x=0; self.y+=self.row+1; self.row=0; }
        let e=(self.x as u16,self.y as u16,w as u16,h as u16,img.placement.left as i16,img.placement.top as i16);
        if w>0&&h>0 { self.pending.push((self.x,self.y,w,h,img.data)); }
        self.x+=w+1; self.row=self.row.max(h); self.map.insert(k,e); Some(e)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2000);
    let t0=Instant::now();
    let inst=Instance::new(InstanceDescriptor{backends:Backends::METAL,..InstanceDescriptor::new_without_display_handle()});
    let adapter=pollster::block_on(inst.request_adapter(&RequestAdapterOptions::default())).unwrap();
    let (dev,q)=pollster::block_on(adapter.request_device(&DeviceDescriptor::default())).unwrap();
    println!("gpu init {:?} {}",t0.elapsed(),adapter.get_info().name);
    let sh=dev.create_shader_module(ShaderModuleDescriptor{label:None,source:ShaderSource::Wgsl(include_str!("shader.wgsl").into())});
    let tex=dev.create_texture(&TextureDescriptor{label:None,size:Extent3d{width:W,height:H,depth_or_array_layers:1},mip_level_count:1,sample_count:1,dimension:TextureDimension::D2,format:FMT,usage:TextureUsages::RENDER_ATTACHMENT|TextureUsages::COPY_SRC,view_formats:&[]});
    let view=tex.create_view(&Default::default());
    let atlas=dev.create_texture(&TextureDescriptor{label:None,size:Extent3d{width:ATLAS,height:ATLAS,depth_or_array_layers:1},mip_level_count:1,sample_count:1,dimension:TextureDimension::D2,format:TextureFormat::R8Unorm,usage:TextureUsages::TEXTURE_BINDING|TextureUsages::COPY_DST,view_formats:&[]});
    let ub=dev.create_buffer(&BufferDescriptor{label:None,size:32,usage:BufferUsages::UNIFORM|BufferUsages::COPY_DST,mapped_at_creation:false});
    let samp=dev.create_sampler(&SamplerDescriptor{mag_filter:FilterMode::Nearest,min_filter:FilterMode::Nearest,..Default::default()});
    let bgl=dev.create_bind_group_layout(&BindGroupLayoutDescriptor{label:None,entries:&[
        BindGroupLayoutEntry{binding:0,visibility:ShaderStages::VERTEX_FRAGMENT,ty:BindingType::Buffer{ty:BufferBindingType::Uniform,has_dynamic_offset:false,min_binding_size:None},count:None},
        BindGroupLayoutEntry{binding:1,visibility:ShaderStages::FRAGMENT,ty:BindingType::Texture{sample_type:TextureSampleType::Float{filterable:true},view_dimension:TextureViewDimension::D2,multisampled:false},count:None},
        BindGroupLayoutEntry{binding:2,visibility:ShaderStages::FRAGMENT,ty:BindingType::Sampler(SamplerBindingType::Filtering),count:None}]});
    let av=atlas.create_view(&Default::default());
    let bg=dev.create_bind_group(&BindGroupDescriptor{label:None,layout:&bgl,entries:&[BindGroupEntry{binding:0,resource:ub.as_entire_binding()},BindGroupEntry{binding:1,resource:BindingResource::TextureView(&av)},BindGroupEntry{binding:2,resource:BindingResource::Sampler(&samp)}]});
    let pl=dev.create_pipeline_layout(&PipelineLayoutDescriptor{label:None,bind_group_layouts:&[Some(&bgl)],immediate_size:0});
    let blend=BlendState{color:BlendComponent{src_factor:BlendFactor::One,dst_factor:BlendFactor::OneMinusSrcAlpha,operation:BlendOperation::Add},alpha:BlendComponent::OVER};
    let pipe=|vs:&str,fs:&str,attrs:&[VertexAttribute],stride:usize|dev.create_render_pipeline(&RenderPipelineDescriptor{label:None,layout:Some(&pl),
        vertex:VertexState{module:&sh,entry_point:Some(vs),compilation_options:Default::default(),buffers:&[Some(VertexBufferLayout{array_stride:stride as u64,step_mode:VertexStepMode::Instance,attributes:attrs})]},
        primitive:Default::default(),depth_stencil:None,multisample:Default::default(),
        fragment:Some(FragmentState{module:&sh,entry_point:Some(fs),compilation_options:Default::default(),targets:&[Some(ColorTargetState{format:FMT,blend:Some(blend),write_mask:ColorWrites::ALL})]}),multiview_mask:None,cache:None});
    let np=pipe("node_vs","node_fs",&vertex_attr_array![0=>Float32x2,1=>Float32,2=>Float32x4],28);
    let ep=pipe("edge_vs","edge_fs",&vertex_attr_array![0=>Float32x2,1=>Float32x2,2=>Float32x4],32);
    let tp=pipe("text_vs","text_fs",&vertex_attr_array![0=>Float32x2,1=>Float32x2,2=>Float32x2,3=>Float32x2,4=>Float32x4],48);
    let cap=(n*3).max(1024) as u64;
    let nb=dev.create_buffer(&BufferDescriptor{label:None,size:cap*28,usage:BufferUsages::VERTEX|BufferUsages::COPY_DST,mapped_at_creation:false});
    let eb=dev.create_buffer(&BufferDescriptor{label:None,size:cap*32,usage:BufferUsages::VERTEX|BufferUsages::COPY_DST,mapped_at_creation:false});
    let tb=dev.create_buffer(&BufferDescriptor{label:None,size:cap*48*14,usage:BufferUsages::VERTEX|BufferUsages::COPY_DST,mapped_at_creation:false});

    let t=Instant::now();
    let fs=FontSystem::new();
    println!("font system {:?} ({} faces)",t.elapsed(),fs.db().len());
    let mut tx=Text{fs,sc:SwashCache::new(),map:Default::default(),x:0,y:0,row:0,pending:vec![]};
    let mut g=make_graph(n);
    for _ in 0..200 { step(&mut g); }

    // shape all labels once; cached layouts = glyph runs
    let t=Instant::now();
    let m=Metrics::new(22.,28.);
    let mut runs: Vec<Vec<(CacheKey,f32,f32)>>=vec![]; let mut widths=vec![];
    for l in &g.label {
        let mut b=Buffer::new(&mut tx.fs,m);
        b.set_size(None,None);
        b.set_text(l,&Attrs::new().family(Family::SansSerif),Shaping::Advanced,None);
        b.shape_until_scroll(&mut tx.fs,false);
        let mut r=vec![]; let mut w=0f32;
        for lr in b.layout_runs(){ for gl in lr.glyphs { let p=gl.physical((0.,0.),1.0); r.push((p.cache_key,p.x as f32,p.y as f32)); } w=w.max(lr.line_w); }
        runs.push(r); widths.push(w);
    }
    println!("shape {} labels {:?}",n,t.elapsed());

    let frames=300usize; let (mut cpu,mut gpu,mut bad)=(vec![],vec![],0);
    let mut nv:Vec<Node>=Vec::with_capacity(n); let mut ev:Vec<Edge>=Vec::with_capacity(g.e.len()); let mut gv:Vec<Glyph>=vec![];
    let mut grid: FxHashMap<(i32,i32),()> = Default::default();
    let order:Vec<usize>={ let mut o:Vec<usize>=(0..n).collect(); o.sort_by_key(|&i|std::cmp::Reverse(g.deg[i])); o };
    let mut last=None;
    for fr in 0..frames {
        let t=Instant::now();
        let time=fr as f32/60.; let zoom=0.9+0.4*(time*0.7).sin();
        let cam=[60.*(time*0.5).sin(),40.*(time*0.3).cos()];
        q.write_buffer(&ub,0,bytemuck::bytes_of(&Globals{size:[W as f32,H as f32],cam,zoom,time,_p:[0.;2]}));
        nv.clear(); ev.clear(); gv.clear(); grid.clear();
        for i in 0..n { let r=2.5+(g.deg[i] as f32).sqrt()*1.6; let h=(i%7) as f32/7.; nv.push(Node{pos:g.p[i],r,c:[0.45+h*0.5,0.8-h*0.3,0.55,1.]}); }
        for &(a,b) in &g.e { ev.push(Edge{a:g.p[a as usize],b:g.p[b as usize],c:[0.35,0.5,0.45,0.28]}); }
        // greedy label grid
        let mut shown=0;
        for &i in &order {
            let s=[(g.p[i][0]-cam[0])*zoom+W as f32*0.5,(g.p[i][1]-cam[1])*zoom+H as f32*0.5];
            if s[0]<0.||s[1]<0.||s[0]>W as f32||s[1]>H as f32 {continue}
            let w=widths[i]; let x0=s[0]+8.; let y0=s[1]-14.;
            let (cw,ch)=(48.,30.);
            let (c0,c1)=((x0/cw) as i32,((x0+w)/cw) as i32); let r=(y0/ch) as i32;
            if (c0..=c1).any(|c|grid.contains_key(&(c,r))) {continue}
            for c in c0..=c1 { grid.insert((c,r),()); }
            shown+=1;
            for &(k,x,y) in &runs[i] { if let Some((ax,ay,aw,ah,l,tt))=tx.glyph(k) { if aw==0 {continue}
                gv.push(Glyph{xy:[x0+x+l as f32,y0+22.+y-tt as f32],wh:[aw as f32,ah as f32],uvxy:[ax as f32/ATLAS as f32,ay as f32/ATLAS as f32],uvwh:[aw as f32/ATLAS as f32,ah as f32/ATLAS as f32],c:[0.9,0.92,0.85,1.]});
            }}
        }
        for (x,y,w,h,d) in tx.pending.drain(..) {
            q.write_texture(TexelCopyTextureInfo{texture:&atlas,mip_level:0,origin:Origin3d{x,y,z:0},aspect:TextureAspect::All},&d,TexelCopyBufferLayout{offset:0,bytes_per_row:Some(w),rows_per_image:None},Extent3d{width:w,height:h,depth_or_array_layers:1});
        }
        q.write_buffer(&nb,0,bytemuck::cast_slice(&nv)); q.write_buffer(&eb,0,bytemuck::cast_slice(&ev)); q.write_buffer(&tb,0,bytemuck::cast_slice(&gv));
        let mut enc=dev.create_command_encoder(&Default::default());
        { let mut p=enc.begin_render_pass(&RenderPassDescriptor{label:None,color_attachments:&[Some(RenderPassColorAttachment{view:&view,depth_slice:None,resolve_target:None,ops:Operations{load:LoadOp::Clear(Color{r:0.012,g:0.016,b:0.02,a:1.}),store:StoreOp::Store}})],depth_stencil_attachment:None,timestamp_writes:None,occlusion_query_set:None,multiview_mask:None});
          p.set_bind_group(0,&bg,&[]);
          p.set_pipeline(&ep); p.set_vertex_buffer(0,eb.slice(..)); p.draw(0..6,0..ev.len() as u32);
          p.set_pipeline(&np); p.set_vertex_buffer(0,nb.slice(..)); p.draw(0..6,0..nv.len() as u32);
          p.set_pipeline(&tp); p.set_vertex_buffer(0,tb.slice(..)); p.draw(0..6,0..gv.len() as u32); }
        let c=t.elapsed();
        let sub=q.submit([enc.finish()]);
        dev.poll(PollType::Wait{submission_index:Some(sub),timeout:None}).unwrap();
        let f=t.elapsed(); cpu.push(c.as_secs_f64()*1e3); gpu.push(f.as_secs_f64()*1e3); if f.as_secs_f64()>0.0166 {bad+=1}
        last=Some(shown);
    }
    let pct=|v:&mut Vec<f64>,p:f64|{v.sort_by(|a,b|a.partial_cmp(b).unwrap()); v[((v.len()-1) as f64*p) as usize]};
    println!("nodes {} edges {} labels shown {:?} glyphs {} frames>16.6ms: {}/{}",n,g.e.len(),last,gv.len(),bad,frames);
    println!("cpu p50 {:.2} p99 {:.2} | cpu+gpu p50 {:.2} p99 {:.2} ms",pct(&mut cpu,0.5),pct(&mut cpu,0.99),pct(&mut gpu,0.5),pct(&mut gpu,0.99));

    // readback
    let bytes=(W*4).next_multiple_of(256)*H;
    let rb=dev.create_buffer(&BufferDescriptor{label:None,size:bytes as u64,usage:BufferUsages::MAP_READ|BufferUsages::COPY_DST,mapped_at_creation:false});
    let mut enc=dev.create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(tex.as_image_copy(),TexelCopyBufferInfo{buffer:&rb,layout:TexelCopyBufferLayout{offset:0,bytes_per_row:Some((W*4).next_multiple_of(256)),rows_per_image:None}},Extent3d{width:W,height:H,depth_or_array_layers:1});
    q.submit([enc.finish()]);
    rb.slice(..).map_async(MapMode::Read,|_|{});
    dev.poll(PollType::wait_indefinitely()).unwrap();
    let slice=rb.slice(..); let data=slice.get_mapped_range().unwrap(); let data=&data[..];
    let stride=(W*4).next_multiple_of(256) as usize; let mut out=Vec::with_capacity((W*H*4) as usize);
    for y in 0..H as usize { for x in 0..W as usize { let p=&data[y*stride+x*4..y*stride+x*4+4]; out.extend_from_slice(&[p[2],p[1],p[0],255]); } }
    let f=std::fs::File::create(args.get(2).map(|s|s.as_str()).unwrap_or("out.png")).unwrap();
    let mut e=png::Encoder::new(std::io::BufWriter::new(f),W,H); e.set_color(png::ColorType::Rgba); e.write_header().unwrap().write_image_data(&out).unwrap();
}
