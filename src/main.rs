#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod patch;
mod ui;

use std::{env, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => ui::run(),
        [command, folder] if command == "--apply" || command == "--restore" => {
            let result = operate(Path::new(folder), command == "--restore", &mut |line| {
                println!("{line}");
            });
            match result {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("[ERRO] {error}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("Uso: croptix-patcher [--apply | --restore] <pasta>");
            ExitCode::from(2)
        }
    }
}

fn operate(path: &Path, restore: bool, log: &mut dyn FnMut(String)) -> Result<(), String> {
    if restore {
        patch::restore_logged(path, log)?;
    } else {
        patch::apply_logged(path, log)?;
    }
    log("[OK] Concluído. Recarregue a extensão e a página.".into());
    Ok(())
}
