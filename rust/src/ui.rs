use std::{ptr, path::PathBuf};
use windows_sys::{core::*, Win32::{Foundation::*, Graphics::{Gdi::*, Dwm::*}, System::LibraryLoader::*, UI::{WindowsAndMessaging::*, Controls::*, HiDpi::*, Input::KeyboardAndMouse::*}}};
use crate::{patch,picker};
const BG:u32=0x00201c19;const CARD:u32=0x002b2623;const BORDER:u32=0x003e3833;const ORANGE:u32=0x002d6fff;const TEXT:u32=0x00f7f4f3;const MUTED:u32=0x00b5aaa3;const LOG:u32=0x00130f0d;
const DONE:u32=WM_APP+1;
fn fit_scale(dpi:f64,width:i32,height:i32)->f64{dpi.min((width-32).max(1) as f64/760.0).min((height-80).max(1) as f64/710.0)}
#[cfg(test)]
mod tests {
    #[test]
    fn console_actions_fit_small_screen_at_high_dpi() {
        for (width,height,dpi) in [(1366,728,1.25),(1920,1040,1.5),(1280,680,1.0)] {
            let scale=super::fit_scale(dpi,width,height);
            assert!(760.0*scale+32.0<=width as f64);
            assert!(710.0*scale+80.0<=height as f64,"botões precisam caber na área útil");
        }
    }
}
fn w(s:&str)->Vec<u16>{s.encode_utf16().chain(Some(0)).collect()}
struct State {path:HWND,log:HWND,select:HWND,apply:HWND,restore:HWND,font:HFONT,bold:HFONT,title:HFONT,brand:HFONT,small:HFONT,console:HFONT,bg:HBRUSH,card:HBRUSH,logbrush:HBRUSH,scale:f64,status:String,busy:bool}
impl State {fn s(&self,v:i32)->i32{(v as f64*self.scale).round() as i32}}
unsafe fn state<'a>(hwnd:HWND)->&'a State {unsafe{&*(GetWindowLongPtrW(hwnd,GWLP_USERDATA) as *const State)}}
unsafe fn state_mut<'a>(hwnd:HWND)->&'a mut State {unsafe{&mut *(GetWindowLongPtrW(hwnd,GWLP_USERDATA) as *mut State)}}
unsafe fn font(size:i32,bold:bool,scale:f64)->HFONT{unsafe{CreateFontW(-(size as f64*scale*96.0/72.0).round() as i32,0,0,0,if bold{700}else{400},0,0,0,DEFAULT_CHARSET as u32,OUT_DEFAULT_PRECIS as u32,CLIP_DEFAULT_PRECIS as u32,CLEARTYPE_QUALITY as u32,DEFAULT_PITCH as u32,w("Segoe UI").as_ptr())}}
unsafe fn fill(dc:HDC,rect:RECT,color:u32){unsafe{let brush=CreateSolidBrush(color);FillRect(dc,&rect,brush);DeleteObject(brush);}}
unsafe fn round(dc:HDC,rect:RECT,color:u32,border:u32,r:i32){unsafe{let brush=CreateSolidBrush(color);let pen=CreatePen(PS_SOLID,1,border);let oldb=SelectObject(dc,brush);let oldp=SelectObject(dc,pen);RoundRect(dc,rect.left,rect.top,rect.right,rect.bottom,r,r);SelectObject(dc,oldb);SelectObject(dc,oldp);DeleteObject(brush);DeleteObject(pen);}}
unsafe fn text(dc:HDC,s:&State,content:&str,x:i32,y:i32,width:i32,height:i32,font:HFONT,color:u32){unsafe{SelectObject(dc,font);SetTextColor(dc,color);SetBkMode(dc,TRANSPARENT as i32);let mut rect=RECT{left:s.s(x),top:s.s(y),right:s.s(x+width),bottom:s.s(y+height)};let t=w(content);DrawTextW(dc,t.as_ptr(),t.len() as i32-1,&mut rect,DT_LEFT|DT_TOP|DT_NOPREFIX);}}
unsafe fn paint(hwnd:HWND,dc:HDC){unsafe{
    let s=state(hwnd);let mut client=RECT::default();GetClientRect(hwnd,&mut client);fill(dc,client,BG);
    round(dc,RECT{left:s.s(32),top:s.s(30),right:s.s(88),bottom:s.s(86)},0x002e2722,BORDER,s.s(20));
    text(dc,s,"C",46,30,38,52,s.brand,ORANGE);text(dc,s,"CrOptix Patcher",106,28,420,40,s.title,TEXT);
    text(dc,s,"Seu player. Do seu jeito.",108,70,410,24,s.font,MUTED);
    round(dc,RECT{left:s.s(32),top:s.s(128),right:s.s(728),bottom:s.s(260)},CARD,BORDER,s.s(16));
    text(dc,s,"Pasta da extensão",52,144,470,26,s.bold,TEXT);text(dc,s,"Escolha a pasta com katamari.js e manifest.json.",52,174,638,24,s.small,MUTED);
    round(dc,RECT{left:s.s(52),top:s.s(204),right:s.s(554),bottom:s.s(244)},BG,BORDER,s.s(10));
    text(dc,s,"CORREÇÕES DO PLAYER",32,281,500,24,s.small,MUTED);
    let titles=["PiP com legendas","Próximo episódio","Avanço sem espera","Controles discretos"];
    let details=["Vídeo e legendas na mesma janela.","Botão alinhado ao lado do volume.","Setas e J/L respondem a cada repetição.","A barra fica oculta ao avançar."];
    for i in 0..4{let x=32+(i%2)*356;let y=310+(i/2)*62;round(dc,RECT{left:s.s(x),top:s.s(y),right:s.s(x+340),bottom:s.s(y+52)},CARD,BORDER,s.s(12));text(dc,s,"✓",x+14,y+14,26,30,s.bold,ORANGE);text(dc,s,titles[i as usize],x+42,y+7,288,24,s.bold,TEXT);text(dc,s,details[i as usize],x+42,y+30,288,21,s.small,MUTED);}
    text(dc,s,"CONSOLE",32,439,250,24,s.small,MUTED);
    round(dc,RECT{left:s.s(32),top:s.s(467),right:s.s(728),bottom:s.s(633)},LOG,BORDER,s.s(12));
    text(dc,s,&s.status,32,667,315,26,s.small,MUTED);
}}
unsafe fn child(hwnd:HWND,kind:&str,label:&str,style:u32,x:i32,y:i32,width:i32,height:i32,id:usize)->HWND{unsafe{
    let s=state(hwnd);let h=CreateWindowExW(0,w(kind).as_ptr(),w(label).as_ptr(),WS_CHILD|WS_VISIBLE|style,s.s(x),s.s(y),s.s(width),s.s(height),hwnd,id as HMENU,GetModuleHandleW(ptr::null()),ptr::null());SendMessageW(h,WM_SETFONT,s.font as usize,1);h
}}
unsafe fn get_text(hwnd:HWND)->String{unsafe{let len=GetWindowTextLengthW(hwnd);let mut buffer=vec![0u16;len as usize+1];GetWindowTextW(hwnd,buffer.as_mut_ptr(),buffer.len() as i32);String::from_utf16_lossy(&buffer[..len as usize])}}
unsafe fn result(hwnd:HWND,message:&str){unsafe{let s=state(hwnd);SetWindowTextW(s.log,w(message).as_ptr());InvalidateRect(hwnd,ptr::null(),1);}}
unsafe extern "system" fn proc(hwnd:HWND,message:u32,wp:WPARAM,lp:LPARAM)->LRESULT{unsafe{
    match message {
        WM_CREATE=>{
            let cs=&*(lp as *const CREATESTRUCTW);SetWindowLongPtrW(hwnd,GWLP_USERDATA,cs.lpCreateParams as isize);
            let input=child(hwnd,"EDIT","",WS_TABSTOP|ES_AUTOHSCROLL as u32,64,214,478,23,100);
            let hint=w("Selecione ou digite a pasta da extensão");SendMessageW(input,0x1501,0,hint.as_ptr() as isize);
            let browse=child(hwnd,"BUTTON","Selecionar pasta",WS_TABSTOP|BS_OWNERDRAW as u32,568,204,140,40,101);
            let log=child(hwnd,"EDIT","Selecione a extensão para começar.\r\n\r\n• Validação antes de modificar arquivos\r\n• Backup automático\r\n• Relatório de alterações e correções já aplicadas",ES_MULTILINE as u32|ES_READONLY as u32|ES_AUTOVSCROLL as u32|WS_TABSTOP,44,479,672,142,102);
            SendMessageW(log,WM_SETFONT,state(hwnd).console as usize,1);
            let restore=child(hwnd,"BUTTON","Restaurar backup",WS_TABSTOP|BS_OWNERDRAW as u32,368,655,168,40,103);
            let apply=child(hwnd,"BUTTON","Aplicar patch",WS_TABSTOP|BS_OWNERDRAW as u32,548,655,180,40,104);
            let s=state_mut(hwnd);s.path=input;s.select=browse;s.log=log;s.restore=restore;s.apply=apply;0
        }
        WM_PAINT=>{let mut ps=PAINTSTRUCT::default();let dc=BeginPaint(hwnd,&mut ps);paint(hwnd,dc);EndPaint(hwnd,&ps);0}
        WM_ERASEBKGND=>1,
        WM_CTLCOLOREDIT|WM_CTLCOLORSTATIC=>{let s=state(hwnd);let dc=wp as HDC;SetTextColor(dc,TEXT);SetBkColor(dc,if lp as HWND==s.log{LOG}else{BG});if lp as HWND==s.log{s.logbrush as isize}else{s.bg as isize}}
        WM_DRAWITEM=>{
            let draw=&*(lp as *const DRAWITEMSTRUCT);let s=state(hwnd);let dc=draw.hDC;let primary=draw.CtlID==104;let disabled=draw.itemState&ODS_DISABLED!=0;let pressed=draw.itemState&ODS_SELECTED!=0;
            fill(dc,draw.rcItem,BG);let color=if disabled{BORDER}else if primary{if pressed{0x004687ff}else{ORANGE}}else{CARD};
            round(dc,draw.rcItem,color,if primary{color}else{BORDER},s.s(10));SelectObject(dc,s.font);SetBkMode(dc,TRANSPARENT as i32);SetTextColor(dc,if disabled{MUTED}else if primary{BG}else{TEXT});
            let caption=w(&get_text(draw.hwndItem));let mut rect=draw.rcItem;DrawTextW(dc,caption.as_ptr(),caption.len() as i32-1,&mut rect,DT_CENTER|DT_VCENTER|DT_SINGLELINE);
            if draw.itemState&ODS_FOCUS!=0{rect.left+=4;rect.top+=4;rect.right-=4;rect.bottom-=4;DrawFocusRect(dc,&rect);}1
        }
        WM_COMMAND=>{
            let id=wp&0xffff;
            if id==101{let input=state(hwnd).path;let initial=get_text(input);match picker::select(hwnd,&initial){Ok(Some(path))=>{SetWindowTextW(input,w(&path).as_ptr());state_mut(hwnd).status="Pasta selecionada.".into();InvalidateRect(hwnd,ptr::null(),1);},Ok(None)=>{},Err(error)=>result(hwnd,&error)}}
            if id==103||id==104{
                if state(hwnd).busy{return 0}let path=get_text(state(hwnd).path).trim().trim_matches('"').to_string();let undo=id==103;
                if path.is_empty(){result(hwnd,"Selecione a pasta da extensão.");return 0}
                if undo&&MessageBoxW(hwnd,w("Restaurar o backup mais recente desta pasta?").as_ptr(),w("Restaurar backup").as_ptr(),MB_YESNO|MB_ICONQUESTION)!=IDYES{return 0}
                state_mut(hwnd).busy=true;state_mut(hwnd).status="Processando…".into();
                result(hwnd,if undo{"Validando backup…"}else{"Validando arquivos…"});
                let controls={let s=state(hwnd);[s.path,s.select,s.apply,s.restore]};for h in controls{EnableWindow(h,0);}InvalidateRect(hwnd,ptr::null(),1);
                let handle=hwnd as usize;std::thread::spawn(move||{
                    let start=std::time::Instant::now();let mut lines=vec![];
                    let outcome=if undo{patch::restore_logged(&PathBuf::from(&path),&mut |s|lines.push(s))}else{patch::apply_logged(&PathBuf::from(&path),&mut |s|lines.push(s)).map(|_|())};
                    let outcome=match outcome{Ok(())=>{lines.push(format!("[OK] Concluído em {} ms. Recarregue a extensão e a página.",start.elapsed().as_millis()));Ok(lines.join("\r\n"))},Err(error)=>{lines.push(format!("[ERRO] {error}"));Err(lines.join("\r\n"))}};
                    let data=Box::into_raw(Box::new(outcome));if PostMessageW(handle as HWND,DONE,0,data as isize)==0{drop(Box::from_raw(data));}
                });
            }0
        }
        DONE=>{let outcome=*Box::from_raw(lp as *mut Result<String,String>);state_mut(hwnd).busy=false;let controls={let s=state(hwnd);[s.path,s.select,s.apply,s.restore]};for h in controls{EnableWindow(h,1);}match outcome{Ok(message)=>{state_mut(hwnd).status="Concluído.".into();result(hwnd,&message)},Err(error)=>{state_mut(hwnd).status="Não aplicado.".into();result(hwnd,&error)}}0}
        WM_CLOSE=>{if !state(hwnd).busy{DestroyWindow(hwnd);}0}
        WM_DESTROY=>{PostQuitMessage(0);0}
        _=>DefWindowProcW(hwnd,message,wp,lp)
    }
}}
pub fn run()->Result<(),String>{unsafe{
    let _apartment=picker::init()?;SetProcessDPIAware();
    let mut work=RECT{left:0,top:0,right:GetSystemMetrics(SM_CXSCREEN),bottom:GetSystemMetrics(SM_CYSCREEN)};
    SystemParametersInfoW(SPI_GETWORKAREA,0,(&mut work as *mut RECT).cast(),0);
    let scale=fit_scale(GetDpiForSystem() as f64/96.0,work.right-work.left,work.bottom-work.top);
    let instance=GetModuleHandleW(ptr::null());let icon=LoadIconW(instance,1usize as PCWSTR);
    let classname=w("CrOptixRustPatcher");let class=WNDCLASSW{lpfnWndProc:Some(proc),hInstance:instance,hIcon:icon,hCursor:LoadCursorW(ptr::null_mut(),IDC_ARROW),lpszClassName:classname.as_ptr(),..Default::default()};
    if RegisterClassW(&class)==0{return Err("Não foi possível registrar a janela.".into())}
    let console=CreateFontW(-(10.0*scale*96.0/72.0).round() as i32,0,0,0,400,0,0,0,DEFAULT_CHARSET as u32,OUT_DEFAULT_PRECIS as u32,CLIP_DEFAULT_PRECIS as u32,CLEARTYPE_QUALITY as u32,DEFAULT_PITCH as u32,w("Consolas").as_ptr());
    let mut s=Box::new(State{path:ptr::null_mut(),log:ptr::null_mut(),select:ptr::null_mut(),apply:ptr::null_mut(),restore:ptr::null_mut(),font:font(10,false,scale),bold:font(11,true,scale),title:font(23,true,scale),brand:font(30,true,scale),small:font(9,false,scale),console,bg:CreateSolidBrush(BG),card:CreateSolidBrush(CARD),logbrush:CreateSolidBrush(LOG),scale,status:"Pronto para selecionar uma pasta.".into(),busy:false});
    let mut rect=RECT{left:0,top:0,right:s.s(760),bottom:s.s(710)};let style=WS_OVERLAPPED|WS_CAPTION|WS_SYSMENU|WS_MINIMIZEBOX;AdjustWindowRectEx(&mut rect,style,0,0);
    let sw=work.right-work.left;let sh=work.bottom-work.top;let width=rect.right-rect.left;let height=rect.bottom-rect.top;
    let hwnd=CreateWindowExW(0,classname.as_ptr(),w("CrOptix Patcher").as_ptr(),style,work.left+(sw-width)/2,work.top+(sh-height)/2,width,height,ptr::null_mut(),ptr::null_mut(),instance,(&mut *s as *mut State).cast());
    if hwnd.is_null(){return Err("Não foi possível criar a janela.".into())}
    let dark:i32=1;DwmSetWindowAttribute(hwnd,20,(&dark as *const i32).cast(),4);let corners:i32=2;DwmSetWindowAttribute(hwnd,33,(&corners as *const i32).cast(),4);ShowWindow(hwnd,SW_SHOW);UpdateWindow(hwnd);
    let mut message=MSG::default();while GetMessageW(&mut message,ptr::null_mut(),0,0)>0{if IsDialogMessageW(hwnd,&message)==0{TranslateMessage(&message);DispatchMessageW(&message);}}
    for object in [s.font,s.bold,s.title,s.brand,s.small,s.console,s.bg,s.card,s.logbrush]{DeleteObject(object);}Ok(())
}}
