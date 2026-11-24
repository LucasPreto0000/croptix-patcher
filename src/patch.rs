use serde_json::Value;
use sha2::{Digest, Sha256};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
const NAMES: [&str; 2] = ["katamari.js", "manifest.json"];
const START: &str = "'croptix-rust-fixes:v3';\n";
const END: &str = "'croptix-rust-fixes:end';\n";
fn payload() -> String {
    format!("{START}{}{END}", include_str!("../assets/runtime.js"))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("Não foi possível ler {}: {e}", path.display()))
}
fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
fn root(folder: &Path) -> Result<PathBuf, String> {
    if !folder.is_dir() {
        return Err("A pasta informada não existe.".into());
    }
    let root = fs::canonicalize(folder).map_err(|e| e.to_string())?;
    for name in NAMES {
        let file = root.join(name);
        if !file.is_file() {
            return Err(format!(
                "Arquivo esperado não encontrado: {name}. Se houver dist, selecione essa pasta."
            ));
        }
        not_redirected(&file)?;
    }
    Ok(root)
}
struct Lock {
    _file: fs::File,
}
impl Lock {
    fn acquire(root: &Path) -> Result<Self, String> {
        let file = root.join(".croptix-rust.lock");
        if file.exists() {
            not_redirected(&file)?;
        }
        let handle = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .access_mode(0x40000000 | 0x10000)
            .share_mode(0)
            .custom_flags(0x04000000)
            .open(&file)
            .map_err(|_| {
                "A pasta está em uso por outro patcher ou não permite gravação.".to_string()
            })?;
        Ok(Self { _file: handle })
    }
}
fn not_redirected(path: &Path) -> Result<(), String> {
    if fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .file_attributes()
        & 0x400
        != 0
        || fs::canonicalize(path).map_err(|e| e.to_string())? != path
    {
        return Err(format!(
            "Backup ou arquivo redirecionado por link: {}",
            path.display()
        ));
    }
    Ok(())
}
fn transaction(root: &Path, before: &[Vec<u8>; 2], after: &[Vec<u8>; 2]) -> Result<(), String> {
    let mut staged: Vec<(usize, PathBuf)> = vec![];
    let mut written: Vec<usize> = vec![];
    let result = (|| {
        for i in 0..2 {
            if before[i] == after[i] {
                continue;
            }
            let temporary = root.join(format!(".croptix-{}-{i}.tmp", stamp()));
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .map_err(|e| e.to_string())?;
            staged.push((i, temporary));
            file.write_all(&after[i]).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
        }
        for (i, temp) in &staged {
            let target = root.join(NAMES[*i]);
            not_redirected(&target)?;
            if read(&target)? != before[*i] {
                return Err(format!("{} foi alterado por outro programa.", NAMES[*i]));
            }
            fs::rename(temp, &target).map_err(|e| e.to_string())?;
            written.push(*i);
        }
        Ok(())
    })();
    let recovery = if result.is_err() {
        rollback(root, before, after, &written)
    } else {
        Ok(())
    };
    for (_, temp) in staged {
        let _ = fs::remove_file(temp);
    }
    recovery?;
    result
}
fn separate(name: &Value) -> bool {
    name.as_str()
        .is_some_and(|s| matches!(s.rsplit(['/', '\\']).next(), Some("pip.js" | "pip1.js")))
}
fn rollback(
    root: &Path,
    before: &[Vec<u8>; 2],
    after: &[Vec<u8>; 2],
    written: &[usize],
) -> Result<(), String> {
    for &i in written.iter().rev() {
        let target = root.join(NAMES[i]);
        not_redirected(&target)?;
        if read(&target)? != after[i] {
            return Err(format!(
                "{} foi atualizado por outro programa; arquivo preservado.",
                NAMES[i]
            ));
        }
        let mut file = tempfile::Builder::new()
            .prefix(".croptix-recovery-")
            .tempfile_in(root)
            .map_err(|e| e.to_string())?;
        (|| {
            file.write_all(&before[i]).map_err(|e| e.to_string())?;
            file.as_file().sync_all().map_err(|e| e.to_string())?;
            not_redirected(&target)?;
            if read(&target)? != after[i] {
                return Err("Arquivo alterado durante a recuperação.".into());
            }
            file.persist(&target)
                .map(|_| ())
                .map_err(|e| format!("Não foi possível recuperar {}: {e}", NAMES[i]))
        })()?;
    }
    Ok(())
}
fn base_source(main: &str, prefix: &str) -> Result<String, String> {
    if main.starts_with(prefix) {
        return Ok(main[prefix.len()..].into());
    }
    let previous = format!(
        "'croptix-rust-fixes:v1';\n{}{END}",
        include_str!("../assets/runtime-v1.js")
    );
    if main.starts_with(&previous) {
        return Ok(main[previous.len()..].into());
    }
    if let Some(end) = main
        .find(END)
        .filter(|_| main.starts_with("'croptix-rust-fixes:v2';\n"))
    {
        let (old, source) = main.split_at(end + END.len());
        if hash(old.as_bytes())
            == "95a528efcf5cec2295d77f4d54dba4732e6e102469659ae72df78cbc7b2e92a8"
        {
            return Ok(source.into());
        }
    }
    if main.starts_with("'croptix-rust-fixes:") {
        return Err(
            "O patch existente foi editado ou é de outra versão. Use o katamari.js original."
                .into(),
        );
    }
    for (start, end) in [
        (
            "// <CROPTIX-INDEPENDENT-FIXES v1>",
            "// </CROPTIX-INDEPENDENT-FIXES>",
        ),
        ("// <CROPTIX-PATCHER-PIP1>", "// </CROPTIX-PATCHER-PIP1>"),
    ] {
        if main.starts_with(start) {
            let at = main.find(end).ok_or("Patch anterior incompleto.")?;
            let head = &main[..at];
            if start.contains("INDEPENDENT") {
                let (_, rest) = head.split_once('\n').ok_or("Patch anterior inválido.")?;
                let (header, code) = rest.split_once('\n').ok_or("Patch anterior inválido.")?;
                let expected = header
                    .trim()
                    .strip_prefix("// SHA256: ")
                    .ok_or("Assinatura anterior ausente.")?;
                if hash(code.as_bytes()) != expected {
                    return Err("O patch anterior foi editado. Nenhum arquivo alterado.".into());
                }
            }
            let tail = &main[at + end.len()..];
            return Ok(tail
                .strip_prefix("\r\n")
                .or_else(|| tail.strip_prefix('\n'))
                .ok_or("Patch anterior inválido.")?
                .into());
        }
    }
    if main.contains("croptix.integrated-fixes.installed")
        || main.contains("croptix.pip1.installed")
    {
        return Err("Correções já incorporadas sem bloco reconhecido.".into());
    }
    Ok(main.into())
}
pub fn apply_logged(folder: &Path, log: &mut dyn FnMut(String)) -> Result<bool, String> {
    log(format!("[PASTA] {}", folder.display()));
    let root = root(folder)?;
    let before = [read(&root.join(NAMES[0]))?, read(&root.join(NAMES[1]))?];
    let main = std::str::from_utf8(&before[0]).map_err(|_| "katamari.js não é UTF-8.")?;
    let manifest_text = std::str::from_utf8(&before[1])
        .map_err(|_| "manifest.json não é UTF-8.")?
        .trim_start_matches('\u{feff}');
    let mut manifest: Value = serde_json::from_str(manifest_text)
        .map_err(|_| "manifest.json inválido. Nenhum arquivo alterado.")?;
    let scripts = manifest
        .get("content_scripts")
        .and_then(Value::as_array)
        .ok_or("Manifesto sem content_scripts.")?;
    if !scripts.iter().any(|s| {
        s["world"] == "MAIN"
            && s["js"]
                .as_array()
                .is_some_and(|a| a.iter().any(|n| n == "katamari.js"))
    }) {
        return Err("Katamari deve ser carregado no mundo MAIN.".into());
    }
    if !main.contains("player-controls-root") || !main.contains("nextEpisodeVM") {
        return Err("Katamari incompatível com os controles esperados.".into());
    }
    log("[OK] katamari.js e manifest.json validados.".into());
    let prefix = payload();
    let base = base_source(main, &prefix)?;
    let mut changed = false;
    for s in manifest["content_scripts"].as_array_mut().unwrap() {
        if let Some(a) = s.get_mut("js").and_then(Value::as_array_mut) {
            let len = a.len();
            a.retain(|n| !separate(n));
            changed |= len != a.len();
        }
    }
    if let Some(entries) = manifest
        .get_mut("web_accessible_resources")
        .and_then(Value::as_array_mut)
    {
        for e in entries {
            if let Some(a) = e.get_mut("resources").and_then(Value::as_array_mut) {
                let len = a.len();
                a.retain(|n| !separate(n));
                changed |= len != a.len();
            }
        }
    }
    let after = [
        format!("{prefix}{base}").into_bytes(),
        if changed {
            format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap()).into_bytes()
        } else {
            before[1].clone()
        },
    ];
    if before == after {
        for fix in [
            "PiP com legendas",
            "Próximo episódio",
            "Avanço sem espera",
            "Controles discretos",
            "CC automático desativado em áudio dublado",
        ] {
            log(format!("[JÁ APLICADO] {fix}"));
        }
        log("[OK] Nenhum arquivo alterado. Nenhum novo backup criado.".into());
        return Ok(false);
    }
    let _lock = Lock::acquire(&root)?;
    for i in 0..2 {
        if read(&root.join(NAMES[i]))? != before[i] {
            return Err("Arquivos alterados durante a validação.".into());
        }
    }
    transaction(&root, &before, &after)?;
    for i in 0..2 {
        log(format!(
            "[{}] {}",
            if before[i] != after[i] {
                "ALTERADO"
            } else {
                "SEM ALTERAÇÃO"
            },
            NAMES[i]
        ));
    }
    for fix in [
        "PiP com legendas",
        "Próximo episódio",
        "Avanço sem espera",
        "Controles discretos",
        "CC automático desativado em áudio dublado",
    ] {
        log(format!(
            "[{}] {fix}",
            if main.starts_with(&prefix) {
                "JÁ APLICADO"
            } else {
                "APLICADO"
            }
        ));
    }
    Ok(true)
}
pub fn restore_logged(folder: &Path, log: &mut dyn FnMut(String)) -> Result<(), String> {
    log(format!("[PASTA] {}", folder.display()));
    let root = root(folder)?;
    let _lock = Lock::acquire(&root)?;
    let backups = root.join(".croptix-rust-backups");
    if !backups.exists() {
        return Err("Nenhum backup deste patcher encontrado.".into());
    }
    not_redirected(&backups)?;
    let mut candidates: Vec<PathBuf> = fs::read_dir(&backups)
        .map_err(|_| "Nenhum backup deste patcher encontrado.")?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("snapshot-")
        })
        .collect();
    let before = [read(&root.join(NAMES[0]))?, read(&root.join(NAMES[1]))?];
    candidates.sort();
    for snapshot in candidates.iter().rev() {
        let validated: Result<[Vec<u8>; 2], String> = (|| {
            not_redirected(snapshot)?;
            for name in ["katamari.js", "manifest.json", "info.json"] {
                not_redirected(&snapshot.join(name))?;
            }
            let info: Value = serde_json::from_slice(&read(&snapshot.join("info.json"))?)
                .map_err(|_| "Backup inválido.".to_string())?;
            if info["folder"] != root.to_string_lossy().as_ref() {
                return Err("Backup de outra pasta.".into());
            }
            let after = [
                read(&snapshot.join(NAMES[0]))?,
                read(&snapshot.join(NAMES[1]))?,
            ];
            for i in 0..2 {
                if info["before"][i] != hash(&after[i]) || info["after"][i] != hash(&before[i]) {
                    return Err("Backup não corresponde aos arquivos atuais.".into());
                }
            }
            Ok(after)
        })();
        if let Ok(after) = validated {
            log(format!("[BACKUP] {}", snapshot.display()));
            transaction(&root, &before, &after)?;
            for i in 0..2 {
                log(format!(
                    "[{}] {}",
                    if before[i] != after[i] {
                        "RESTAURADO"
                    } else {
                        "SEM ALTERAÇÃO"
                    },
                    NAMES[i]
                ));
            }
            return Ok(());
        }
    }
    Err("Nenhum backup válido corresponde aos arquivos atuais. Eles foram editados/atualizados ou o backup está incompleto.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SOURCE: &str =
        "const controls = 'player-controls-root', nextEpisodeVM = {}, quality = '1080p';";
    const MANIFEST: &str = r#"{"name":"Teste","content_scripts":[{"world":"MAIN","js":["pip1.js","katamari.js"]}],"web_accessible_resources":[{"resources":["pip.js","fonts/*"]}]}"#;

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(NAMES[0]), SOURCE).unwrap();
        fs::write(dir.path().join(NAMES[1]), MANIFEST).unwrap();
        dir
    }

    #[test]
    fn applies_current_payload_and_never_creates_a_backup() {
        let dir = fixture();
        assert!(apply_logged(dir.path(), &mut |_| {}).unwrap());
        let source = fs::read_to_string(dir.path().join(NAMES[0])).unwrap();
        assert_eq!(source, format!("{}{SOURCE}", payload()));
        assert!(source.contains("installDubCaptionPolicy"));
        let manifest: Value =
            serde_json::from_slice(&fs::read(dir.path().join(NAMES[1])).unwrap()).unwrap();
        assert_eq!(manifest["content_scripts"][0]["js"], json!(["katamari.js"]));
        assert_eq!(
            manifest["web_accessible_resources"][0]["resources"],
            json!(["fonts/*"])
        );
        assert!(!apply_logged(dir.path(), &mut |_| {}).unwrap());
        assert!(restore_logged(dir.path(), &mut |_| {}).is_err());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn migrates_known_prefix_and_rejects_tampered_prefix() {
        let dir = fixture();
        let file = dir.path().join(NAMES[0]);
        let legacy = format!(
            "'croptix-rust-fixes:v1';\n{}{END}{SOURCE}",
            include_str!("../assets/runtime-v1.js")
        );
        fs::write(&file, &legacy).unwrap();
        assert!(apply_logged(dir.path(), &mut |_| {}).unwrap());
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            format!("{}{SOURCE}", payload())
        );
        let invalid = legacy.replace("let pipSession = null", "let pipSession = undefined");
        fs::write(&file, &invalid).unwrap();
        assert!(apply_logged(dir.path(), &mut |_| {}).is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), invalid);
    }

    #[test]
    fn invalid_manifest_leaves_files_intact() {
        let dir = fixture();
        fs::write(dir.path().join(NAMES[1]), "{broken").unwrap();
        assert!(apply_logged(dir.path(), &mut |_| {}).is_err());
        assert_eq!(
            fs::read_to_string(dir.path().join(NAMES[0])).unwrap(),
            SOURCE
        );
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn failed_second_replacement_rolls_back_first() {
        let dir = fixture();
        let root = root(dir.path()).unwrap();
        let manifest = root.join(NAMES[1]);
        // An open handle that allows reads but denies renaming the manifest.
        let _handle = fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&manifest)
            .unwrap();
        assert!(apply_logged(dir.path(), &mut |_| {}).is_err());
        assert_eq!(fs::read_to_string(root.join(NAMES[0])).unwrap(), SOURCE);
        assert_eq!(fs::read_to_string(&manifest).unwrap(), MANIFEST);
        assert_eq!(fs::read_dir(root).unwrap().count(), 2);
    }

    #[test]
    fn restores_matching_legacy_snapshot_without_changing_it() {
        let dir = fixture();
        let root = root(dir.path()).unwrap();
        apply_logged(&root, &mut |_| {}).unwrap();
        let snapshot = root.join(".croptix-rust-backups").join("snapshot-1");
        fs::create_dir_all(&snapshot).unwrap();
        for (name, bytes) in NAMES.into_iter().zip([SOURCE, MANIFEST]) {
            fs::write(snapshot.join(name), bytes).unwrap();
        }
        let info = json!({
            "folder": root.to_string_lossy(),
            "before": [hash(SOURCE.as_bytes()), hash(MANIFEST.as_bytes())],
            "after": NAMES.map(|name| hash(&fs::read(root.join(name)).unwrap())),
        });
        fs::write(snapshot.join("info.json"), info.to_string()).unwrap();
        restore_logged(&root, &mut |_| {}).unwrap();
        assert_eq!(fs::read_to_string(root.join(NAMES[0])).unwrap(), SOURCE);
        assert_eq!(fs::read_to_string(root.join(NAMES[1])).unwrap(), MANIFEST);
        assert_eq!(
            fs::read_to_string(snapshot.join("info.json")).unwrap(),
            info.to_string()
        );
        assert!(restore_logged(&root, &mut |_| {}).is_err());
    }
}
