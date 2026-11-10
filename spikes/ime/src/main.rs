// Prova de acentos: injeta NSEvents reais no NSApp e observa o que chega via winit Ime.
use objc2::rc::Retained; use objc2::runtime::{AnyClass, AnyObject}; use objc2::{msg_send, msg_send_id, class};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::c_void; use std::time::{Duration, Instant};
use winit::application::ApplicationHandler; use winit::event::*; use winit::event_loop::*; use winit::window::*; use winit::dpi::*;

#[link(name="Carbon",kind="framework")] extern "C" {
    fn TISCopyCurrentKeyboardLayoutInputSource()->*mut c_void;
    fn TISGetInputSourceProperty(s:*mut c_void,k:*const c_void)->*mut c_void;
    static kTISPropertyUnicodeKeyLayoutData:*const c_void;
    fn LMGetKbdType()->u8;
    fn UCKeyTranslate(l:*const c_void,code:u16,action:u16,mods:u32,kbd:u32,opts:u32,dead:*mut u32,max:usize,len:*mut usize,out:*mut u16)->i32;
}
#[link(name="CoreFoundation",kind="framework")] extern "C" { fn CFDataGetBytePtr(d:*mut c_void)->*const c_void; }
#[link(name="CoreGraphics",kind="framework")] extern "C" { fn CGEventCreateKeyboardEvent(src:*const c_void,k:u16,down:bool)->*mut c_void; fn CGEventSetFlags(e:*mut c_void,f:u64); fn CGEventPostToPid(pid:i32,e:*mut c_void); fn CGPreflightPostEventAccess()->bool; }

unsafe fn layout()->*const c_void{ let s=TISCopyCurrentKeyboardLayoutInputSource(); CFDataGetBytePtr(TISGetInputSourceProperty(s,kTISPropertyUnicodeKeyLayoutData)) }
fn translate(code:u16,shift:bool,dead:&mut u32)->String{ unsafe{ let mut o=[0u16;8]; let mut n=0; UCKeyTranslate(layout(),code,0,if shift{2}else{0},LMGetKbdType() as u32,0,dead,8,&mut n,o.as_mut_ptr()); String::from_utf16_lossy(&o[..n]) } }
fn dead_keys()->Vec<(u16,bool,String)>{ let mut v=vec![]; for c in 0..51u16 { for s in [false,true] { let mut d=0; translate(c,s,&mut d); if d!=0 { let shown=translate(49,false,&mut d); v.push((c,s,shown)); } } } v }
fn find(ch:&str)->Option<(u16,bool)>{ for c in 0..51u16 { for s in [false,true]{ let mut d=0; if translate(c,s,&mut d)==ch && d==0 {return Some((c,s));} } } None }

unsafe fn nsstr(s:&str)->Retained<AnyObject>{ let cls=class!(NSString); let cstr=std::ffi::CString::new(s).unwrap(); msg_send_id![cls,stringWithUTF8String:cstr.as_ptr()] }
#[repr(transparent)] #[derive(Clone,Copy)] struct CgRef(*mut c_void);
unsafe impl objc2::Encode for CgRef { const ENCODING: objc2::Encoding = objc2::Encoding::Pointer(&objc2::Encoding::Struct("__CGEvent",&[])); }
#[repr(C)] #[derive(Clone,Copy)] struct P{x:f64,y:f64}
unsafe impl objc2::Encode for P { const ENCODING: objc2::Encoding = objc2::Encoding::Struct("CGPoint",&[f64::ENCODING,f64::ENCODING]); }
unsafe fn key(win:isize,kc:u16,shift:bool,down:bool){
    let _=win;
    let cg=CGEventCreateKeyboardEvent(std::ptr::null(),kc,down);
    CGEventSetFlags(cg,if shift{0x20000}else{0});
    if std::env::var("POST").is_ok(){ CGEventPostToPid(std::process::id() as i32,cg); return }
    let e:Option<Retained<AnyObject>>=msg_send_id![class!(NSEvent),eventWithCGEvent:CgRef(cg)];
    let app:*mut AnyObject=msg_send![class!(NSApplication),sharedApplication];
    let _:()=msg_send![app,sendEvent:&*e.unwrap()];
}

struct A{w:Option<Window>,num:isize,step:usize,script:Vec<(u16,bool)>,next:Instant,out:String,pre:String,log:Vec<String>,ks:Vec<String>,expected:String}
impl ApplicationHandler for A{
    fn resumed(&mut self,el:&ActiveEventLoop){ if self.w.is_some(){return}
        let w=el.create_window(Window::default_attributes().with_title("spike-ime").with_inner_size(LogicalSize::new(320,120))).unwrap();
        w.set_ime_allowed(true); w.set_ime_cursor_area(PhysicalPosition::new(0,0),PhysicalSize::new(10,10));
        if let RawWindowHandle::AppKit(h)=w.window_handle().unwrap().as_raw(){ unsafe{ let v=h.ns_view.as_ptr() as *mut AnyObject; let nw:*mut AnyObject=msg_send![v,window]; self.num=msg_send![nw,windowNumber]; let app:*mut AnyObject=msg_send![class!(NSApplication),sharedApplication]; let _:()=msg_send![app,activateIgnoringOtherApps:true]; } }
        self.w=Some(w); self.next=Instant::now()+Duration::from_millis(800);
    }
    fn window_event(&mut self,_:&ActiveEventLoop,_:WindowId,e:WindowEvent){ match e {
        WindowEvent::Ime(Ime::Preedit(s,_))=>{self.log.push(format!("preedit[{s}]"));self.pre=s;}
        WindowEvent::Ime(Ime::Commit(s))=>{self.log.push(format!("commit[{s}]"));self.out.push_str(&s);self.pre.clear();}
        WindowEvent::Ime(Ime::Enabled)=>self.log.push("ime-on".into()),
        WindowEvent::Ime(Ime::Disabled)=>self.log.push("ime-off".into()),
        WindowEvent::KeyboardInput{event,..} if event.state==ElementState::Pressed =>{ if let Some(t)=event.text { self.log.push(format!("key[{t}]")); self.ks.push(t.to_string()); self.out.push_str(&t);} }
        _=>{} } }
    fn about_to_wait(&mut self,el:&ActiveEventLoop){
        let now=Instant::now();
        if now>=self.next { self.next=now+Duration::from_millis(40);
            if self.step==0 { unsafe{ let a:*mut AnyObject=msg_send![class!(NSApplication),sharedApplication]; let act:objc2::runtime::Bool=msg_send![a,isActive]; let kw:*mut AnyObject=msg_send![a,keyWindow]; let ic:*mut AnyObject=msg_send![class!(NSTextInputContext),currentInputContext]; println!("active {} keyWindow {:?} inputCtx {:?}",act.as_bool(),kw,ic);} }
            if self.step<self.script.len(){ let (k,s)=self.script[self.step]; unsafe{ key(self.num,k,s,true); key(self.num,k,s,false);} self.step+=1; }
            else if now>self.next-Duration::from_millis(1) && self.step==self.script.len(){ self.step+=1; self.next=now+Duration::from_millis(300);} 
            else { println!("log: {}",self.log.join(" ")); println!("esperado: {:?}",self.expected); println!("obtido:   {:?}",self.out); println!("{}",if self.out==self.expected{"PASS"}else{"FAIL"}); el.exit(); }
        }
        el.set_control_flow(ControlFlow::WaitUntil(self.next));
    }
}
fn main(){
    let dk=dead_keys(); println!("dead keys: {:?}",dk);
    let acute=dk.iter().find(|d|d.2.contains('´')||d.2.contains('\'')).map(|d|(d.0,d.1));
    let tilde=dk.iter().find(|d|d.2.contains('~')).map(|d|(d.0,d.1));
    let circ=dk.iter().find(|d|d.2.contains('^')).map(|d|(d.0,d.1));
    let (ac,ti)=(acute.expect("acute"),tilde.expect("tilde"));
    let a=find("a").unwrap(); let c=find("c").unwrap(); let e=find("e").unwrap(); let o=find("o").unwrap(); let sp=find(" ").unwrap(); let k=find("k").unwrap();
    let mut s=vec![ac,a,ti,a,ti,o,ac,e,ac,c,ac,sp,ac,k]; let mut exp=String::from("áãõéç''k");
    if let Some(c2)=circ { s.push(c2); s.push(e); exp.push('ê'); }
    let el=EventLoop::new().unwrap(); el.set_control_flow(ControlFlow::Poll);
    unsafe{println!("post access {}",CGPreflightPostEventAccess());}
    let mut app=A{w:None,num:0,step:0,script:s,next:Instant::now(),out:String::new(),pre:String::new(),log:vec![],ks:vec![],expected:exp};
    el.run_app(&mut app).unwrap();
}
