use std::{ffi::c_void, path::Path, ptr};
use windows_sys::{core::GUID, Win32::{Foundation::HWND, System::Com::*}};
const CLASS:GUID=GUID{data1:0xdc1c5a9c,data2:0xe88a,data3:0x4dde,data4:[0xa5,0xa1,0x60,0xf8,0x2a,0x20,0xae,0xf7]};
const IID:GUID=GUID{data1:0x42f85136,data2:0xdb7e,data3:0x439c,data4:[0x85,0xf1,0xe4,0x07,0x5d,0x13,0x5f,0xc8]};
const SHELL:GUID=GUID{data1:0x43826d1e,data2:0xe718,data3:0x42ee,data4:[0xbc,0x55,0xa1,0xe2,0x61,0xc3,0x7b,0xfe]};
const OPTIONS:u32=0x20|0x40|0x800|0x8;
struct Com(*mut c_void);
impl Drop for Com {fn drop(&mut self){unsafe{let call:unsafe extern "system" fn(*mut c_void)->u32=std::mem::transmute(self.method(2));call(self.0);}}}
impl Com {unsafe fn method(&self,n:usize)->*const c_void{unsafe{*(*(self.0 as *mut *mut *const c_void)).add(n)}}}
fn wide(s:&str)->Vec<u16>{s.encode_utf16().chain(Some(0)).collect()}
fn ok(hr:i32)->Result<(),String>{if hr<0{Err(format!("Falha no seletor do Explorador: 0x{:08X}",hr as u32))}else{Ok(())}}
pub struct Apartment;
impl Drop for Apartment {fn drop(&mut self){unsafe{CoUninitialize();}}}
pub fn init()->Result<Apartment,String>{unsafe{ok(CoInitializeEx(ptr::null(),COINIT_APARTMENTTHREADED as u32))?;Ok(Apartment)}}
fn configured(initial:&str)->Result<Com,String>{unsafe{
    let mut pointer=ptr::null_mut();ok(CoCreateInstance(&CLASS,ptr::null_mut(),CLSCTX_INPROC_SERVER,&IID,&mut pointer))?;
    let dialog=Com(pointer);
    let get:unsafe extern "system" fn(*mut c_void,*mut u32)->i32=std::mem::transmute(dialog.method(10));
    let set:unsafe extern "system" fn(*mut c_void,u32)->i32=std::mem::transmute(dialog.method(9));
    let mut flags=0;ok(get(pointer,&mut flags))?;ok(set(pointer,flags|OPTIONS))?;
    let title:unsafe extern "system" fn(*mut c_void,*const u16)->i32=std::mem::transmute(dialog.method(17));
    let label:unsafe extern "system" fn(*mut c_void,*const u16)->i32=std::mem::transmute(dialog.method(18));
    ok(title(pointer,wide("Selecionar pasta da extensão CrOptix").as_ptr()))?;ok(label(pointer,wide("Selecionar pasta").as_ptr()))?;
    if Path::new(initial).is_dir(){
        let mut item=ptr::null_mut();
        let full=std::path::absolute(initial).map_err(|e|e.to_string())?;
        ok(windows_sys::Win32::UI::Shell::SHCreateItemFromParsingName(wide(&full.to_string_lossy()).as_ptr(),ptr::null_mut(),&SHELL,&mut item))?;
        let item=Com(item);
        let folder:unsafe extern "system" fn(*mut c_void,*mut c_void)->i32=std::mem::transmute(dialog.method(12));
        ok(folder(pointer,item.0))?;
    }
    Ok(dialog)
}}
fn filepath(item:Com)->Result<String,String>{unsafe{
    let method:unsafe extern "system" fn(*mut c_void,u32,*mut *mut u16)->i32=std::mem::transmute(item.method(5));
    let mut name=ptr::null_mut();ok(method(item.0,0x80058000,&mut name))?;
    let mut len=0;while *name.add(len)!=0{len+=1}
    let result=String::from_utf16_lossy(std::slice::from_raw_parts(name,len));CoTaskMemFree(name.cast());Ok(result)
}}
pub fn select(owner:HWND,initial:&str)->Result<Option<String>,String>{unsafe{
    let dialog=configured(initial)?;
    let show:unsafe extern "system" fn(*mut c_void,HWND)->i32=std::mem::transmute(dialog.method(3));
    let hr=show(dialog.0,owner);if hr as u32==0x800704c7{return Ok(None)}ok(hr)?;
    let get:unsafe extern "system" fn(*mut c_void,*mut *mut c_void)->i32=std::mem::transmute(dialog.method(20));
    let mut item=ptr::null_mut();ok(get(dialog.0,&mut item))?;Ok(Some(filepath(Com(item))?))
}}
pub fn self_test(folder:&str)->Result<(),String>{unsafe{
    let _apartment=init()?;let dialog=configured(folder)?;
    let get:unsafe extern "system" fn(*mut c_void,*mut u32)->i32=std::mem::transmute(dialog.method(10));
    let mut flags=0;ok(get(dialog.0,&mut flags))?;if flags&OPTIONS!=OPTIONS{return Err("Opções do seletor incorretas.".into())}
    let get:unsafe extern "system" fn(*mut c_void,*mut *mut c_void)->i32=std::mem::transmute(dialog.method(13));
    let mut item=ptr::null_mut();ok(get(dialog.0,&mut item))?;
    let found=std::fs::canonicalize(filepath(Com(item))?).map_err(|e|e.to_string())?;
    if found!=std::fs::canonicalize(folder).map_err(|e|e.to_string())?{return Err("Pasta inicial incorreta.".into())}Ok(())
}}
#[cfg(test)] mod tests {
    #[test] fn configures_explorer_for_an_existing_folder(){let temp=tempfile::tempdir().unwrap();super::self_test(temp.path().to_str().unwrap()).unwrap();}
}
