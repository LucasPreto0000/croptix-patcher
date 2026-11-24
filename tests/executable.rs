use std::{fs, process::Command};

#[test]
fn executable_applies_and_reapplies_to_a_fixture() {
    let directory = tempfile::tempdir().unwrap();
    let source = "const controls = 'player-controls-root', nextEpisodeVM = {};";
    let manifest = r#"{"content_scripts":[{"js":["katamari.js"],"world":"MAIN"}]}"#;
    let player_path = directory.path().join("katamari.js");
    let manifest_path = directory.path().join("manifest.json");
    fs::write(&player_path, source).unwrap();
    fs::write(&manifest_path, manifest).unwrap();

    let run = || {
        let result = Command::new(env!("CARGO_BIN_EXE_croptix-patcher"))
            .arg("--apply")
            .arg(directory.path())
            .output()
            .unwrap();
        assert!(result.status.success(), "{result:?}");
    };
    run();
    let patched = fs::read_to_string(&player_path).unwrap();
    assert!(patched.contains("installDubCaptionPolicy"));
    assert!(patched.ends_with(source));
    run();
    assert_eq!(fs::read_to_string(player_path).unwrap(), patched);
    assert_eq!(fs::read_to_string(manifest_path).unwrap(), manifest);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}
