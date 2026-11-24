use std::{env, fs, path::PathBuf};

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    println!("cargo:rerun-if-changed=assets/croptix.ico");
    let icon = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("assets/croptix.ico");
    let resource = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("croptix.rc");
    let icon = icon.to_str().expect("UTF-8 icon path").replace('\\', "/");
    fs::write(&resource, format!("1 ICON \"{icon}\"\n")).expect("write icon resource");
    embed_resource::compile(&resource, embed_resource::NONE)
        .manifest_required()
        .expect("embed CrOptix icon");

    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        // GPUI imports TaskDialogIndirect, which requires Common Controls v6.
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
    }
}
