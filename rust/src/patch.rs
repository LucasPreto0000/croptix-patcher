use std::{fs, io::Write, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::os::windows::fs::OpenOptionsExt;
const NAMES: [&str;2] = ["katamari.js","manifest.json"];
const START: &str = "'croptix-rust-fixes:v2';\n";
const END: &str = "'croptix-rust-fixes:end';\n";
fn payload() -> String { format!("{START}{}{END}",include_str!("../runtime.js")) }
fn hash(bytes: &[u8]) -> String { format!("{:x}",Sha256::digest(bytes)) }
fn read(path: &Path) -> Result<Vec<u8>,String> { fs::read(path).map_err(|e|format!("Não foi possível ler {}: {e}",path.display())) }
fn write(path: &Path,bytes: &[u8]) -> Result<(),String> { fs::write(path,bytes).map_err(|e|format!("Não foi possível gravar {}: {e}",path.display())) }
fn stamp() -> u128 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() }
fn root(folder: &Path) -> Result<PathBuf,String> {
    if !folder.is_dir() {return Err("A pasta informada não existe.".into())}
    let root=fs::canonicalize(folder).map_err(|e|e.to_string())?;
    for name in NAMES {
        let file=root.join(name);
        if !file.is_file(){return Err(format!("Arquivo esperado não encontrado: {name}. Se houver dist, selecione essa pasta."))}
        if fs::canonicalize(&file).map_err(|e|e.to_string())? != file {return Err(format!("Arquivo redirecionado por link: {name}"))}
    }
    Ok(root)
}
struct Lock { _file: fs::File }
impl Lock {
    fn acquire(root:&Path)->Result<Self,String>{
        let file=root.join(".croptix-rust.lock");
        if file.exists(){not_redirected(&file)?;}
        let handle=fs::OpenOptions::new().write(true).create(true).access_mode(0x40000000|0x10000).share_mode(0).custom_flags(0x04000000).open(&file).map_err(|_|"A pasta está em uso por outro patcher ou não permite gravação.".to_string())?;
        Ok(Self{_file:handle})
    }
}
fn not_redirected(path:&Path)->Result<(),String>{
    if fs::canonicalize(path).map_err(|e|e.to_string())? != path {return Err(format!("Backup ou arquivo redirecionado por link: {}",path.display()))}Ok(())
}
fn transaction(root:&Path,before:&[Vec<u8>;2],after:&[Vec<u8>;2])->Result<(),String>{
    let mut staged: Vec<(usize,PathBuf)>=vec![];
    let mut written:Vec<usize>=vec![];
    let result=(||{
        for i in 0..2 {
            if before[i]==after[i]{continue}
            let temporary=root.join(format!(".croptix-{}-{i}.tmp",stamp()));
            let mut file=fs::OpenOptions::new().create_new(true).write(true).open(&temporary).map_err(|e|e.to_string())?;
            staged.push((i,temporary));file.write_all(&after[i]).map_err(|e|e.to_string())?;file.sync_all().map_err(|e|e.to_string())?;
        }
        for (i,temp) in &staged {
            let target=root.join(NAMES[*i]);
            if read(&target)?!=before[*i]{return Err(format!("{} foi alterado por outro programa.",NAMES[*i]))}
            fs::rename(temp,&target).map_err(|e|e.to_string())?;written.push(*i);
        }
        Ok(())
    })();
    let recovery=if result.is_err(){rollback(root,before,after,&written)}else{Ok(())};
    for (_,temp) in staged {let _=fs::remove_file(temp);}
    recovery?;
    result
}
fn separate(name:&Value)->bool {name.as_str().is_some_and(|s|matches!(s.rsplit(['/', '\\']).next(),Some("pip.js"|"pip1.js")))}
fn rollback(root:&Path,before:&[Vec<u8>;2],after:&[Vec<u8>;2],written:&[usize])->Result<(),String>{
    for &i in written.iter().rev(){
        let target=root.join(NAMES[i]);
        if read(&target)?!=after[i]{return Err(format!("{} foi atualizado por outro programa; arquivo preservado. Use o backup para recuperar a versão anterior.",NAMES[i]))}
        let staged=root.join(format!(".croptix-recovery-{}.tmp",stamp()));
        let result=(||{
            let mut file=fs::OpenOptions::new().create_new(true).write(true).open(&staged).map_err(|e|e.to_string())?;
            file.write_all(&before[i]).map_err(|e|e.to_string())?;file.sync_all().map_err(|e|e.to_string())?;drop(file);
            if read(&target)?!=after[i]{return Err("Arquivo alterado durante a recuperação; use o backup.".into())}
            fs::rename(&staged,&target).map_err(|e|format!("Não foi possível recuperar {}. Use o backup: {e}",NAMES[i]))
        })();let _=fs::remove_file(staged);result?;
    }Ok(())
}
fn base_source(main:&str,prefix:&str)->Result<String,String>{
    if main.starts_with(prefix){return Ok(main[prefix.len()..].into())}
    let previous=format!("'croptix-rust-fixes:v1';\n{}{END}",include_str!("../runtime-v1.js"));
    if main.starts_with(&previous){return Ok(main[previous.len()..].into())}
    if main.starts_with("'croptix-rust-fixes:"){return Err("O patch existente foi editado ou é de outra versão. Restaure o backup antes de aplicar.".into())}
    for (start,end) in [("// <CROPTIX-INDEPENDENT-FIXES v1>","// </CROPTIX-INDEPENDENT-FIXES>"),("// <CROPTIX-PATCHER-PIP1>","// </CROPTIX-PATCHER-PIP1>")] {
        if main.starts_with(start) {
            let at=main.find(end).ok_or("Patch anterior incompleto.")?;
            let head=&main[..at];
            if start.contains("INDEPENDENT") {
                let (_,rest)=head.split_once('\n').ok_or("Patch anterior inválido.")?;
                let (header,code)=rest.split_once('\n').ok_or("Patch anterior inválido.")?;
                let expected=header.trim().strip_prefix("// SHA256: ").ok_or("Assinatura anterior ausente.")?;
                if hash(code.as_bytes())!=expected{return Err("O patch anterior foi editado. Nenhum arquivo alterado.".into())}
            }
            let tail=&main[at+end.len()..];
            return Ok(tail.strip_prefix("\r\n").or_else(||tail.strip_prefix('\n')).ok_or("Patch anterior inválido.")?.into());
        }
    }
    if main.contains("croptix.integrated-fixes.installed")||main.contains("croptix.pip1.installed"){return Err("Correções já incorporadas sem bloco reconhecido.".into())}
    Ok(main.into())
}
pub fn apply(folder:&Path)->Result<bool,String>{apply_logged(folder,&mut |_|{})}
pub fn apply_logged(folder: &Path, log:&mut dyn FnMut(String)) -> Result<bool, String> {
    log(format!("[PASTA] {}",folder.display()));
    let root=root(folder)?;
    let before=[read(&root.join(NAMES[0]))?,read(&root.join(NAMES[1]))?];
    let main=std::str::from_utf8(&before[0]).map_err(|_|"katamari.js não é UTF-8.")?;
    let manifest_text=std::str::from_utf8(&before[1]).map_err(|_|"manifest.json não é UTF-8.")?.trim_start_matches('\u{feff}');
    let mut manifest:Value=serde_json::from_str(manifest_text).map_err(|_|"manifest.json inválido. Nenhum arquivo alterado.")?;
    let scripts=manifest.get("content_scripts").and_then(Value::as_array).ok_or("Manifesto sem content_scripts.")?;
    if !scripts.iter().any(|s|s["world"]=="MAIN"&&s["js"].as_array().is_some_and(|a|a.iter().any(|n|n=="katamari.js"))){return Err("Katamari deve ser carregado no mundo MAIN.".into())}
    if !main.contains("player-controls-root")||!main.contains("nextEpisodeVM"){return Err("Katamari incompatível com os controles esperados.".into())}
    log("[OK] katamari.js e manifest.json validados.".into());
    let prefix=payload();let base=base_source(main,&prefix)?;
    let mut changed=false;
    for s in manifest["content_scripts"].as_array_mut().unwrap(){
        if let Some(a)=s["js"].as_array_mut(){let len=a.len();a.retain(|n|!separate(n));changed|=len!=a.len();}
    }
    if let Some(entries)=manifest["web_accessible_resources"].as_array_mut(){for e in entries{if let Some(a)=e["resources"].as_array_mut(){let len=a.len();a.retain(|n|!separate(n));changed|=len!=a.len();}}}
    let after=[format!("{prefix}{base}").into_bytes(),if changed {format!("{}\n",serde_json::to_string_pretty(&manifest).unwrap()).into_bytes()}else{before[1].clone()}];
    if before==after{
        for fix in ["PiP com legendas","Próximo episódio","Avanço sem espera","Controles discretos"]{log(format!("[JÁ APLICADO] {fix}"));}
        log("[OK] Nenhum arquivo alterado. Nenhum novo backup criado.".into());return Ok(false)
    }
    let _lock=Lock::acquire(&root)?;
    for i in 0..2{if read(&root.join(NAMES[i]))?!=before[i]{return Err("Arquivos alterados durante a validação.".into())}}
    let backups=root.join(".croptix-rust-backups");if backups.exists(){not_redirected(&backups)?;}else{fs::create_dir(&backups).map_err(|e|e.to_string())?;}
    not_redirected(&backups)?;
    let snapshot=backups.join(format!("snapshot-{}",stamp()));fs::create_dir(&snapshot).map_err(|e|e.to_string())?;
    for i in 0..2{write(&snapshot.join(NAMES[i]),&before[i])?;}
    let info=json!({"folder":root.to_string_lossy(),"created":stamp().to_string(),"before":[hash(&before[0]),hash(&before[1])],"after":[hash(&after[0]),hash(&after[1])]});
    write(&snapshot.join("info.json"),serde_json::to_string(&info).unwrap().as_bytes())?;
    log(format!("[BACKUP] {}",snapshot.display()));
    transaction(&root,&before,&after)?;
    for i in 0..2{log(format!("[{}] {}",if before[i]!=after[i]{"ALTERADO"}else{"SEM ALTERAÇÃO"},NAMES[i]));}
    for fix in ["PiP com legendas","Próximo episódio","Avanço sem espera","Controles discretos"]{log(format!("[{}] {fix}",if main.starts_with(&prefix){"JÁ APLICADO"}else{"APLICADO"}));}
    Ok(true)
}
pub fn restore(folder:&Path)->Result<(),String>{restore_logged(folder,&mut |_|{})}
pub fn restore_logged(folder: &Path, log:&mut dyn FnMut(String)) -> Result<(), String> {
    log(format!("[PASTA] {}",folder.display()));
    let root=root(folder)?;let _lock=Lock::acquire(&root)?;
    let backups=root.join(".croptix-rust-backups");
    if !backups.exists(){return Err("Nenhum backup deste patcher encontrado.".into())}not_redirected(&backups)?;
    let mut candidates:Vec<PathBuf>=fs::read_dir(&backups).map_err(|_|"Nenhum backup deste patcher encontrado.")?.filter_map(|e|e.ok().map(|e|e.path())).filter(|p|p.is_dir()&&p.file_name().unwrap().to_string_lossy().starts_with("snapshot-")&&p.join("info.json").is_file()).collect();
    let before=[read(&root.join(NAMES[0]))?,read(&root.join(NAMES[1]))?];
    candidates.sort();
    for snapshot in candidates.iter().rev(){
        let validated: Result<[Vec<u8>;2], String>=(||{
            not_redirected(snapshot)?;for name in ["katamari.js","manifest.json","info.json"]{not_redirected(&snapshot.join(name))?;}
            let info:Value=serde_json::from_slice(&read(&snapshot.join("info.json"))?).map_err(|_|"Backup inválido.".to_string())?;
            if info["folder"]!=root.to_string_lossy().as_ref(){return Err("Backup de outra pasta.".into())}
            let after=[read(&snapshot.join(NAMES[0]))?,read(&snapshot.join(NAMES[1]))?];
            for i in 0..2 {if info["before"][i]!=hash(&after[i])||info["after"][i]!=hash(&before[i]){return Err("Backup não corresponde aos arquivos atuais.".into())}}
            Ok(after)
        })();
        if let Ok(after)=validated{
            log(format!("[BACKUP] {}",snapshot.display()));
            transaction(&root,&before,&after)?;
            for i in 0..2{log(format!("[{}] {}",if before[i]!=after[i]{"RESTAURADO"}else{"SEM ALTERAÇÃO"},NAMES[i]));}
            return Ok(())
        }
    }
    Err("Nenhum backup válido corresponde aos arquivos atuais. Eles foram editados/atualizados ou o backup está incompleto.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    const MAIN: &str = "(()=>{ const playerControls='player-controls-root'; const nextEpisodeVM={}; })();";
    #[test] fn upgrades_previous_rust_patch_and_restores_it_exactly() {
        let dir=fixture();
        let old=format!("'croptix-rust-fixes:v1';\n{}{}{}",include_str!("../runtime-v1.js"),END,MAIN);
        fs::write(dir.path().join("katamari.js"),&old).unwrap();
        assert!(apply(dir.path()).unwrap());
        let updated=fs::read_to_string(dir.path().join("katamari.js")).unwrap();
        assert_eq!(updated.matches("function browserFixes").count(),1);
        assert!(updated.ends_with(MAIN));
        assert!(!apply(dir.path()).unwrap());
        restore(dir.path()).unwrap();assert_eq!(fs::read_to_string(dir.path().join("katamari.js")).unwrap(),old);
    }
    #[test] fn refuses_edited_previous_rust_patch() {
        let dir=fixture();let old=format!("'croptix-rust-fixes:v1';\n{}{}{}",include_str!("../runtime-v1.js"),END,MAIN).replace("let pipSession = null","let pipSession = undefined");
        fs::write(dir.path().join("katamari.js"),&old).unwrap();
        assert!(apply(dir.path()).is_err());assert_eq!(fs::read_to_string(dir.path().join("katamari.js")).unwrap(),old);
    }
    fn fixture() -> tempfile::TempDir {
        let dir=tempfile::tempdir().unwrap();
        fs::write(dir.path().join("katamari.js"),MAIN).unwrap();
        fs::write(dir.path().join("manifest.json"),r#"{"content_scripts":[{"world":"MAIN","js":["pip1.js","katamari.js"]}],"web_accessible_resources":[{"resources":["pip1.js","fonts/*"]}]}"#).unwrap();dir
    }
    #[test] fn report_matches_actual_files_and_reapplication() {
        let dir=fixture();let mut events=vec![];
        assert!(apply_logged(dir.path(),&mut |line|events.push(line)).unwrap());
        assert!(events.iter().any(|s|s=="[ALTERADO] katamari.js"));
        assert!(events.iter().any(|s|s=="[ALTERADO] manifest.json"));
        let backup=events.iter().find_map(|s|s.strip_prefix("[BACKUP] ")).expect("backup deve ser informado");
        assert!(Path::new(backup).join("info.json").is_file());
        events.clear();assert!(!apply_logged(dir.path(),&mut |line|events.push(line)).unwrap());
        assert!(!events.iter().any(|s|s.starts_with("[ALTERADO]")||s.starts_with("[BACKUP]")));
        assert!(events.iter().any(|s|s=="[JÁ APLICADO] PiP com legendas"));
        events.clear();restore_logged(dir.path(),&mut |line|events.push(line)).unwrap();
        assert!(events.iter().any(|s|s=="[RESTAURADO] katamari.js"));
        assert!(events.iter().any(|s|s=="[RESTAURADO] manifest.json"));
        assert_eq!(fs::read_to_string(dir.path().join("katamari.js")).unwrap(),MAIN);
    }
    #[test] fn applies_without_external_scripts() {
        let dir=fixture();assert!(apply(dir.path()).unwrap());
        let js=fs::read_to_string(dir.path().join("katamari.js")).unwrap();
        assert!(js.contains("requestPictureInPicture"));assert!(js.ends_with(MAIN));
        assert!(!js.lines().any(|l|l.trim_start().starts_with("//")));
        let m:serde_json::Value=serde_json::from_slice(&fs::read(dir.path().join("manifest.json")).unwrap()).unwrap();
        assert_eq!(m["content_scripts"][0]["js"],serde_json::json!(["katamari.js"]));
    }
    #[test] fn repeat_is_noop_and_restore_is_exact() {
        let dir=fixture();let m=fs::read(dir.path().join("manifest.json")).unwrap();
        apply(dir.path()).unwrap();let first=fs::read(dir.path().join("katamari.js")).unwrap();
        assert!(!apply(dir.path()).unwrap());assert_eq!(fs::read(dir.path().join("katamari.js")).unwrap(),first);
        restore(dir.path()).unwrap();assert_eq!(fs::read(dir.path().join("katamari.js")).unwrap(),MAIN.as_bytes());
        assert_eq!(fs::read(dir.path().join("manifest.json")).unwrap(),m);
    }
    #[test] fn malformed_manifest_changes_nothing() {
        let dir=fixture();fs::write(dir.path().join("manifest.json"),"{broken").unwrap();
        assert!(apply(dir.path()).is_err());assert_eq!(fs::read_to_string(dir.path().join("katamari.js")).unwrap(),MAIN);
        assert!(!dir.path().join(".croptix-rust-backups").exists());
    }
    #[test] fn updated_main_cannot_be_restored() {
        let dir=fixture();apply(dir.path()).unwrap();fs::write(dir.path().join("katamari.js"),"new version").unwrap();
        assert!(restore(dir.path()).is_err());assert_eq!(fs::read_to_string(dir.path().join("katamari.js")).unwrap(),"new version");
    }
    #[test] fn incomplete_folder_changes_nothing() {
        let dir=tempfile::tempdir().unwrap();fs::write(dir.path().join("katamari.js"),MAIN).unwrap();
        assert!(apply(dir.path()).is_err());assert_eq!(fs::read_to_string(dir.path().join("katamari.js")).unwrap(),MAIN);
        assert!(!dir.path().join(".croptix-rust-backups").exists());
    }
    #[test] fn corrupt_backup_cannot_be_restored() {
        let dir=fixture();apply(dir.path()).unwrap();let patched=fs::read(dir.path().join("katamari.js")).unwrap();
        let snapshot=fs::read_dir(dir.path().join(".croptix-rust-backups")).unwrap().next().unwrap().unwrap().path();
        fs::write(snapshot.join("katamari.js"),"corrupted").unwrap();
        assert!(restore(dir.path()).is_err());assert_eq!(fs::read(dir.path().join("katamari.js")).unwrap(),patched);
    }
    #[test] fn legacy_node_block_is_migrated_and_restorable() {
        let dir=fixture();let code=";(function(){})();\n";
        let old=format!("// <CROPTIX-INDEPENDENT-FIXES v1>\n// SHA256: {}\n{}// </CROPTIX-INDEPENDENT-FIXES>\n{MAIN}",hash(code.as_bytes()),code);
        fs::write(dir.path().join("katamari.js"),&old).unwrap();apply(dir.path()).unwrap();
        let patched=fs::read_to_string(dir.path().join("katamari.js")).unwrap();assert!(patched.ends_with(MAIN));assert!(!patched.contains("CROPTIX-INDEPENDENT"));
        restore(dir.path()).unwrap();assert_eq!(fs::read_to_string(dir.path().join("katamari.js")).unwrap(),old);
    }
    #[test] fn backup_junction_is_rejected() {
        let dir=fixture();let outside=tempfile::tempdir().unwrap();
        let script=format!("New-Item -ItemType Junction -Path '{}' -Target '{}' | Out-Null",dir.path().join(".croptix-rust-backups").display(),outside.path().display());
        let status=std::process::Command::new("powershell.exe").args(["-NoProfile","-Command",&script]).status().unwrap();assert!(status.success());
        assert!(apply(dir.path()).is_err());assert_eq!(fs::read_to_string(dir.path().join("katamari.js")).unwrap(),MAIN);
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(),0);
    }
    #[test] fn failed_newer_snapshot_does_not_block_restore() {
        let dir=fixture();apply(dir.path()).unwrap();
        let newer=dir.path().join(".croptix-rust-backups").join("snapshot-9999999999999999999999");fs::create_dir(&newer).unwrap();
        let old=fs::read_dir(dir.path().join(".croptix-rust-backups")).unwrap().map(|e|e.unwrap().path()).find(|p|p!=&newer).unwrap();
        for name in NAMES{fs::copy(old.join(name),newer.join(name)).unwrap();}
        let mut info:Value=serde_json::from_slice(&fs::read(old.join("info.json")).unwrap()).unwrap();info["after"][1]=json!("failed transaction");
        fs::write(newer.join("info.json"),serde_json::to_vec(&info).unwrap()).unwrap();
        restore(dir.path()).unwrap();assert_eq!(fs::read_to_string(dir.path().join("katamari.js")).unwrap(),MAIN);
    }
    #[test] fn rollback_is_exact_if_target_is_unchanged() {
        let dir=fixture();let before=[MAIN.as_bytes().to_vec(),vec![]];let after=[b"patched".to_vec(),vec![]];
        fs::write(dir.path().join("katamari.js"),&after[0]).unwrap();rollback(dir.path(),&before,&after,&[0]).unwrap();
        assert_eq!(fs::read_to_string(dir.path().join("katamari.js")).unwrap(),MAIN);
    }
    #[test] fn rollback_preserves_a_concurrent_update() {
        let dir=fixture();let before=[MAIN.as_bytes().to_vec(),vec![]];let after=[b"patched".to_vec(),vec![]];
        fs::write(dir.path().join("katamari.js"),"external update").unwrap();assert!(rollback(dir.path(),&before,&after,&[0]).is_err());
        assert_eq!(fs::read_to_string(dir.path().join("katamari.js")).unwrap(),"external update");
    }
}
