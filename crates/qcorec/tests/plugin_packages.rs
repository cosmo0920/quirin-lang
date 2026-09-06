use qcore_compiler::{PluginEntrypoint, PluginId};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Fixture {
    root: PathBuf,
    package: PathBuf,
    config: PathBuf,
}
impl Fixture {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("qcore-cli-package-{name}-{}", std::process::id()));
        let package = root.join("package");
        std::fs::create_dir_all(&package).unwrap();
        let mut manifest = qcore_compiler::first_party_manifests().remove(0);
        manifest.plugin_id = PluginId::new("example.cli").unwrap();
        manifest.entrypoint = PluginEntrypoint::Executable {
            path: "provider".into(),
        };
        manifest.package_digest =
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into();
        // Deliberately not executable: verification must read it without launching.
        std::fs::write(package.join("provider"), "").unwrap();
        std::fs::write(
            package.join("verifier-plugin.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        Self {
            config: root.join("plugins.json"),
            root,
            package,
        }
    }
    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_qcorec"))
            .args(args)
            .output()
            .unwrap()
    }
    fn enable(&self) -> Output {
        self.cli(&[
            "plugins",
            "enable",
            self.package.to_str().unwrap(),
            "--config",
            self.config.to_str().unwrap(),
        ])
    }
    fn list(&self) -> Output {
        self.cli(&["plugins", "list", "--config", self.config.to_str().unwrap()])
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn succeeds(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}
fn fails(output: Output, message: &str) {
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(message),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn package_selection_is_explicit_pinned_and_reversible() {
    let f = Fixture::new("selection");
    succeeds(f.cli(&["plugins", "verify", f.package.to_str().unwrap()]));
    succeeds(f.enable());
    assert!(succeeds(f.list()).contains("example.cli"));
    assert!(!succeeds(f.list()).contains("qcore.libevent"));
    let original = std::fs::read(&f.config).unwrap();
    let manifest = f.package.join("verifier-plugin.json");
    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    json["package_version"] = "9.9.9".into();
    std::fs::write(&manifest, serde_json::to_vec(&json).unwrap()).unwrap();
    fails(f.list(), "changed since enable");
    assert_eq!(std::fs::read(&f.config).unwrap(), original);
    // Explicit re-enable is the package upgrade/review decision.
    succeeds(f.enable());
    succeeds(f.cli(&[
        "plugins",
        "disable",
        "example.cli",
        "--config",
        f.config.to_str().unwrap(),
    ]));
    assert!(succeeds(f.list()).is_empty());
    // Disabled packages need not remain installed, and never enter dispatch.
    std::fs::remove_file(f.package.join("provider")).unwrap();
    assert!(succeeds(f.list()).is_empty());
    let input = f.root.join("source.qcore");
    std::fs::write(&input, "fn main() -> i32 { return 1; }").unwrap();
    succeeds(f.cli(&[
        "check",
        "--plugins",
        f.config.to_str().unwrap(),
        input.to_str().unwrap(),
    ]));
    std::fs::write(&input, "verify scope s { plugin example.cli profile default capabilities { resource_contracts }; }\nfn main() -> i32 { return 1; }").unwrap();
    fails(
        f.cli(&[
            "check",
            "--plugins",
            f.config.to_str().unwrap(),
            input.to_str().unwrap(),
        ]),
        "plugin is not installed",
    );
}

#[test]
fn package_tampering_never_updates_the_selection_file() {
    let f = Fixture::new("tampering");
    succeeds(f.enable());
    let original = std::fs::read(&f.config).unwrap();
    std::fs::write(f.package.join("provider"), "changed").unwrap();
    fails(f.enable(), "digest mismatch");
    fails(f.list(), "digest mismatch");
    assert_eq!(std::fs::read(&f.config).unwrap(), original);
    fails(
        f.cli(&[
            "plugins",
            "disable",
            "example.absent",
            "--config",
            f.config.to_str().unwrap(),
        ]),
        "not configured",
    );
    assert_eq!(std::fs::read(&f.config).unwrap(), original);
}

#[test]
fn invalid_configuration_has_no_default_registry_fallback() {
    let f = Fixture::new("config");
    fails(f.list(), "plugins.json");
    succeeds(f.enable());
    let original: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&f.config).unwrap()).unwrap();
    for kind in 0..4 {
        let mut value = original.clone();
        match kind {
            0 => value["version"] = 2.into(),
            1 => value["ignored"] = true.into(),
            2 => {
                let duplicate = value["packages"][0].clone();
                value["packages"].as_array_mut().unwrap().push(duplicate);
            }
            _ => value["packages"][0]["path"] = "relative/path".into(),
        }
        let bytes = serde_json::to_vec(&value).unwrap();
        std::fs::write(&f.config, &bytes).unwrap();
        assert!(!f.list().status.success());
        assert!(!f.enable().status.success());
        assert_eq!(std::fs::read(&f.config).unwrap(), bytes);
    }
}

#[cfg(unix)]
#[test]
fn package_symlinks_cannot_escape_the_package_root() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new("escape");
    let external = f.root.join("external");
    std::fs::write(&external, "").unwrap();
    std::fs::remove_file(f.package.join("provider")).unwrap();
    symlink(&external, f.package.join("provider")).unwrap();
    fails(
        f.cli(&["plugins", "verify", f.package.to_str().unwrap()]),
        "escapes package directory",
    );
    assert!(!Path::new(&f.config).exists());
}

#[cfg(unix)]
#[test]
fn configured_source_scope_dispatches_only_the_enabled_process() {
    use qcore_compiler::{PluginManifest, SupportedTarget, VerifierCapability};
    use std::collections::BTreeSet;
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new("dispatch");
    let executable = f.package.join("provider");
    std::fs::write(
        &executable,
        "#!/bin/sh\nprintf invoked > invoked\nexit 23\n",
    )
    .unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    let manifest_path = f.package.join("verifier-plugin.json");
    let mut manifest: PluginManifest =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    manifest.package_digest =
        "b825beca93d78556d05b77634bcc85da3c2ea8217cea3f80c38f8f6ef1edf6a1".into();
    manifest.capabilities = BTreeSet::from([VerifierCapability::ObservationHooks]);
    manifest.evidence_versions = BTreeSet::from([1, 2]);
    manifest.required_observation_hooks = BTreeSet::from(["resize".into()]);
    manifest.certificate_checker = "qcore.contract_observation.v2".into();
    manifest.targets = vec![SupportedTarget {
        library: "example".into(),
        revisions: BTreeSet::from(["source".into()]),
        profiles: BTreeSet::from(["default".into()]),
        dialects: BTreeSet::from(["qcore".into()]),
        platforms: BTreeSet::from([std::env::consts::OS.into()]),
        backends: BTreeSet::from(["native".into()]),
    }];
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    succeeds(f.enable());
    let input = f.root.join("source.qcore");
    std::fs::write(&input, "extern fn resize() -> unit contract { requires size > 0; ensures normal live == 1; };\nverify scope s { plugin example.cli profile default capabilities { observation_hooks }; }\nfn main() -> i32 { return 7; }").unwrap();
    let output = f.cli(&[
        "check",
        "--plugins",
        f.config.to_str().unwrap(),
        input.to_str().unwrap(),
    ]);
    assert!(
        !output.status.success(),
        "provider failure cannot certify the source"
    );
    assert_eq!(
        std::fs::read_to_string(f.package.join("invoked")).unwrap(),
        "invoked"
    );
    std::fs::remove_file(f.package.join("invoked")).unwrap();
    succeeds(f.cli(&[
        "plugins",
        "disable",
        "example.cli",
        "--config",
        f.config.to_str().unwrap(),
    ]));
    fails(
        f.cli(&[
            "check",
            "--plugins",
            f.config.to_str().unwrap(),
            input.to_str().unwrap(),
        ]),
        "plugin is not installed",
    );
    assert!(!f.package.join("invoked").exists());
}
