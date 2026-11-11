// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use qcore_compiler::verifier_plugin::envelope;
use qcore_compiler::{PluginEntrypoint, PluginId, PluginManifest};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

// An executable package file is written by a separate process, so a sibling
// test's concurrent spawn cannot make its `execve` fail with `ETXTBSY`
// (`tests/support/process_fixture.rs`). Data files stay `fs::write`.
#[path = "../../../tests/support/process_fixture.rs"]
mod process_fixture;

/// Write a package executable carrying `manifest` as its embedded template, and
/// the installed manifest that binds to it. The file is never executed by
/// verification, so plain bytes carrying the envelope are a faithful stand-in
/// for a compiled provider: what the gate reads is exactly these bytes.
/// `mode` is `None` for those stand-ins (written as plain, non-executable
/// data) and `Some` for a package a test then launches, which is written
/// with that mode by the shared out-of-process writer.
fn install(
    package: &Path,
    manifest: &mut PluginManifest,
    executable_body: &str,
    mode: Option<u32>,
) {
    let mut template = manifest.clone();
    // An executable cannot embed its own hash; real templates carry the same
    // all-zero build placeholder, which the installed digest never equals.
    template.package_digest = "0".repeat(64);
    let mut bytes = executable_body.as_bytes().to_vec();
    bytes.extend(envelope(&serde_json::to_string_pretty(&template).unwrap()).into_bytes());
    match mode {
        None => std::fs::write(package.join("provider"), &bytes).unwrap(),
        Some(mode) => {
            process_fixture::write_executable_mode(&package.join("provider"), &bytes, mode)
        }
    }
    manifest.package_digest = qcore_compiler::verifier_plugin::sha256_hex(&bytes);
    std::fs::write(
        package.join("verifier-plugin.json"),
        serde_json::to_vec(manifest).unwrap(),
    )
    .unwrap();
}

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
        // Deliberately not executable: verification must read it without
        // launching, so the "executable" is just bytes carrying the template.
        install(&package, &mut manifest, "", None);
        Self {
            config: root.join("plugins.json"),
            root,
            package,
        }
    }
    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quirin"))
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
    let manifest_path = f.package.join("verifier-plugin.json");
    let installed: PluginManifest =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    // A schema-valid, digest-intact declaration edit is bound to the
    // executable's own template and refused by field name.
    let mut edited = installed.clone();
    edited.package_version = "9.9.9".into();
    std::fs::write(&manifest_path, serde_json::to_vec(&edited).unwrap()).unwrap();
    fails(
        f.list(),
        "installed manifest field `package_version` diverges",
    );
    assert_eq!(std::fs::read(&f.config).unwrap(), original);
    // A coherently rebuilt package at the same path binds, and is then caught by
    // the separate pinned-manifest comparison instead.
    let mut rebuilt = edited.clone();
    install(&f.package, &mut rebuilt, "rebuilt", None);
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

/// Provider discovery from an explicit selection must be deterministic: the
/// order packages happen to be enabled in may not change what `list` reports or
/// what the selection file records.
#[test]
fn discovery_order_is_independent_of_selection_order() {
    let f = Fixture::new("ordering");
    let ids = ["example.gamma", "example.alpha", "example.beta"];
    let mut packages = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        let package = f.root.join(format!("package-{id}"));
        std::fs::create_dir_all(&package).unwrap();
        let mut manifest = qcore_compiler::first_party_manifests().remove(0);
        manifest.plugin_id = PluginId::new(*id).unwrap();
        manifest.entrypoint = PluginEntrypoint::Executable {
            path: "provider".into(),
        };
        // Distinct declared libraries keep registration free of route conflicts.
        for target in &mut manifest.targets {
            target.library = format!("library{index}");
        }
        install(&package, &mut manifest, "", None);
        packages.push(package);
    }
    let listing = |config: &Path, order: &[usize]| {
        for index in order {
            succeeds(f.cli(&[
                "plugins",
                "enable",
                packages[*index].to_str().unwrap(),
                "--config",
                config.to_str().unwrap(),
            ]));
        }
        (
            succeeds(f.cli(&["plugins", "list", "--config", config.to_str().unwrap()])),
            std::fs::read(config).unwrap(),
        )
    };
    let forward = listing(&f.root.join("forward.json"), &[0, 1, 2]);
    let reverse = listing(&f.root.join("reverse.json"), &[2, 1, 0]);
    assert_eq!(forward.0, reverse.0);
    assert_eq!(forward.1, reverse.1);
    assert_eq!(
        forward
            .0
            .lines()
            .map(|line| line.split('\t').next().unwrap())
            .collect::<Vec<_>>(),
        ["example.alpha", "example.beta", "example.gamma"]
    );
    // Repeating the same query is stable too.
    let repeated = succeeds(f.cli(&[
        "plugins",
        "list",
        "--config",
        f.root.join("forward.json").to_str().unwrap(),
    ]));
    assert_eq!(repeated, forward.0);
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
    let f = Fixture::new("dispatch");
    let manifest_path = f.package.join("verifier-plugin.json");
    let mut manifest: PluginManifest =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
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
    // The shell body precedes the envelope, so `sh` exits before reaching it and
    // the template still sits in the file the gate reads.
    install(
        &f.package,
        &mut manifest,
        "#!/bin/sh\nprintf invoked > invoked\nexit 23\n",
        Some(0o755),
    );
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

/// The configured execution-profile route (card `P03-F`).
///
/// Before this, `linux-rlimit-v1` and `linux-confined-v1` were reachable only
/// from Rust, so every configured run was permanently on the historical
/// profile. This checks that a written selection reaches the registry, that
/// the default is unchanged and unwritten, and that a selection the host
/// cannot honor refuses instead of falling back.
#[test]
fn the_execution_profile_is_explicit_configuration_and_defaults_to_the_legacy_one() {
    let f = Fixture::new("profile-route");
    succeeds(f.enable());
    let written = std::fs::read_to_string(&f.config).unwrap();
    // The default is recorded, not written: an enable produces exactly the
    // bytes it produced before this field existed.
    assert!(!written.contains("\"execution\""), "{written}");
    let reported = succeeds(f.cli(&["plugins", "profile", "--config", f.config.to_str().unwrap()]));
    assert!(reported.contains("unenforced-legacy"), "{reported}");
    assert!(!reported.contains("confinement"), "{reported}");
    // With no configuration at all the answer is the same default.
    let bare = succeeds(f.cli(&["plugins", "profile"]));
    assert!(bare.contains("unenforced-legacy"), "{bare}");

    let mut config: serde_json::Value = serde_json::from_str(&written).unwrap();
    let config_path = f.config.clone();
    let selected = |value: &mut serde_json::Value, block: serde_json::Value| {
        value["execution"] = block;
        std::fs::write(&config_path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    };

    // A confining selection reaches the registry and is reported as selected.
    selected(
        &mut config,
        serde_json::json!({
            "version": 1,
            "profile": "linux-confined-v1",
            "confinement": {"base": "linux-runtime-defaults", "read_only": ["/etc/qcore"]}
        }),
    );
    let reported = succeeds(f.cli(&["plugins", "profile", "--config", f.config.to_str().unwrap()]));
    assert!(reported.contains("linux-confined-v1"), "{reported}");
    assert!(reported.contains("linux-runtime-defaults"), "{reported}");
    assert!(succeeds(f.list()).contains("example.cli"));

    // A policy under a profile that installs none is refused, and no registry
    // is built at all -- the same refusal the provider makes, made earlier.
    selected(
        &mut config,
        serde_json::json!({
            "version": 1,
            "profile": "linux-rlimit-v1",
            "confinement": {"base": "linux-runtime-defaults"}
        }),
    );
    fails(f.list(), "enforces no confinement policy");

    // A confining profile with no policy is refused in the other direction.
    selected(
        &mut config,
        serde_json::json!({"version": 1, "profile": "linux-confined-v1"}),
    );
    fails(f.list(), "requires an explicit confinement policy");

    // A relative admitted path would mean whatever the reader's directory
    // makes it mean, so it is refused rather than resolved.
    selected(
        &mut config,
        serde_json::json!({
            "version": 1,
            "profile": "linux-confined-v1",
            "confinement": {"base": "linux-runtime-defaults", "read_write": ["scratch"]}
        }),
    );
    fails(f.list(), "is relative");

    // An unknown selection version is refused rather than guessed.
    selected(
        &mut config,
        serde_json::json!({"version": 2, "profile": "unenforced-legacy"}),
    );
    fails(f.list(), "unsupported execution policy version");

    // An unknown key inside the block is refused too, so a misspelled profile
    // selection cannot be silently ignored into the default.
    selected(
        &mut config,
        serde_json::json!({"version": 1, "profil": "linux-confined-v1"}),
    );
    fails(f.list(), "unknown field");
}
