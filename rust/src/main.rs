#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod patch;
mod picker;
mod ui;
fn main() {
    let args:Vec<String>=std::env::args().skip(1).collect();
    let result=match args.as_slice(){
        [flag,folder] if flag=="--apply"=>patch::apply(std::path::Path::new(folder)).map(|_|()),
        [flag,folder] if flag=="--restore"=>patch::restore(std::path::Path::new(folder)),
        [flag,folder] if flag=="--picker-self-test"=>picker::self_test(folder),
        []=>ui::run(),
        _=>Err("Argumentos inválidos.".into())
    };
    if let Err(error)=result{eprintln!("{error}");std::process::exit(1)}
}
