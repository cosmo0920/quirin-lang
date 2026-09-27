use std::{fs, process::Command};

#[test]
fn renamed_archive_bytes_still_warn_under_a_legacy_alias() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let archive = root.join("docs/reports/remaining-v07/probes/d02_boundary_record.qr");
    let bytes = fs::read(&archive).unwrap();
    let directory = std::env::temp_dir().join(format!(
        "quirin-archived-extension-alias-{}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).unwrap();
    let legacy = directory.join("d02_boundary_record.qcore");
    fs::write(&legacy, &bytes).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("check")
        .arg(&legacy)
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(
        stderr.contains("warning[legacy-source-extension]"),
        "{stderr}"
    );
    assert!(stderr.contains("Q000"), "{stderr}");
    assert_eq!(fs::read(&archive).unwrap(), bytes);
    assert_eq!(fs::read(&legacy).unwrap(), bytes);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn runtime_and_contract_aliases_have_visible_warning_and_matching_outcomes() {
    let dir = std::env::temp_dir().join(format!("quirin-extension-cli-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    for (stem, source, suffix) in [
        ("runtime", "fn main() -> i32 { return 0; }", "qr"),
        ("contract", "module contract; extern fn f() -> unit;", "qv"),
    ] {
        let old = dir.join(format!("{stem}.qcore"));
        let new = dir.join(format!("{stem}.{suffix}"));
        fs::write(&old, source).unwrap();
        fs::write(&new, source).unwrap();
        let run = |p: &std::path::Path| {
            Command::new(env!("CARGO_BIN_EXE_quirin"))
                .arg("check")
                .arg(p)
                .output()
                .unwrap()
        };
        let legacy = run(&old);
        let current = run(&new);
        assert!(legacy.status.success(), "{legacy:?}");
        assert!(current.status.success(), "{current:?}");
        assert_eq!(legacy.stdout, current.stdout);
        assert!(
            String::from_utf8_lossy(&legacy.stderr).contains("warning[legacy-source-extension]")
        );
        assert!(!String::from_utf8_lossy(&current.stderr).contains("legacy-source-extension"));
    }
    fs::remove_dir_all(dir).unwrap();
}
