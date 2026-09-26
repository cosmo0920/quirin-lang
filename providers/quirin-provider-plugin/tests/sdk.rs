//! Card W45-USER-CONTRACT-PLUGINS: the SDK on its example user crate
//! (`tests/fixtures/example-user-crate/quirin-provider-acme`) and its example
//! checker (`src/bin/example_checker.rs`).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_schemas::PLUGIN_EVIDENCE_VERSION_V2;
use qcore_schemas::plugin::{
    PLUGIN_PROTOCOL_VERSION, PluginId, ProposedVerdict, TargetProfile, UnitIdentity,
    VerificationRequestV2, VerificationResponseV2, VerifierCapability,
};
use quirin_provider_plugin::{load_path, package, template, user_crate::UserCrate};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "quirin-plugin-sdk-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/example-user-crate")
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::write(&target, fs::read(entry.path()).unwrap()).unwrap();
        }
    }
}

#[test]
fn the_example_crate_reads_and_generates_its_user_load_path() {
    let krate = UserCrate::read(&example().join("quirin-provider-acme")).unwrap();
    assert_eq!(krate.name, "quirin-provider-acme");
    assert_eq!(krate.namespace, "acme");
    assert_eq!(
        krate.modules.keys().collect::<Vec<_>>(),
        ["acme/pool.qcore"]
    );
    assert_eq!(krate.manifest.checkers.len(), 1);
    let manifest =
        load_path::user_load_path("../providers/contract-load-path.json", &example(), &[krate])
            .unwrap();
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["extends"], "../providers/contract-load-path.json");
    let record = &manifest["crates"][0];
    assert_eq!(record["contracts"], "quirin-provider-acme/contracts");
    assert_eq!(record["modules"], serde_json::json!(["acme/pool.qcore"]));
    assert!(
        record.get("resource_view").is_none(),
        "the loader decides views"
    );
    assert_eq!(
        manifest["checkers"][0]["package"],
        "quirin-provider-acme/checkers/pool-check"
    );
    assert_eq!(manifest["overrides"], serde_json::json!([]));
}

#[test]
fn a_crate_outside_the_load_path_directory_or_malformed_is_refused() {
    let krate = UserCrate::read(&example().join("quirin-provider-acme")).unwrap();
    let elsewhere = scratch("elsewhere");
    assert!(load_path::user_load_path("x.json", &elsewhere, std::slice::from_ref(&krate)).is_err());
    assert!(
        load_path::user_load_path("x.json", &example(), &[krate.clone(), krate]).is_err(),
        "a crate listed twice"
    );
    for (file, edit, expected) in [
        (
            "Cargo.toml",
            ("quirin-provider-acme", "acme"),
            "quirin-provider-NAME",
        ),
        ("src/lib.rs", ("\"acme\";", "\"other\";"), "derived"),
        (
            "quirin-plugin.json",
            ("acme/pool.qcore", "acme/absent.qcore"),
            "does not own",
        ),
        (
            "quirin-plugin.json",
            ("checkers/pool-check", "../pool-check"),
            "without traversal",
        ),
        (
            "quirin-plugin.json",
            (
                "\"acme_pool_get\", \"acme_pool_put\"",
                "\"acme_pool_put\", \"acme_pool_get\"",
            ),
            "sorted",
        ),
    ] {
        let root = scratch("malformed");
        let directory = root.join("quirin-provider-acme");
        copy_tree(&example().join("quirin-provider-acme"), &directory);
        let path = directory.join(file);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains(edit.0), "{file}");
        fs::write(&path, text.replacen(edit.0, edit.1, 1)).unwrap();
        let error = UserCrate::read(&directory).unwrap_err();
        assert!(error.contains(expected), "{file}: {error}");
    }
    let root = scratch("stray");
    let directory = root.join("quirin-provider-acme");
    copy_tree(&example().join("quirin-provider-acme"), &directory);
    fs::write(directory.join("contracts/acme/notes.txt"), "x").unwrap();
    assert!(
        UserCrate::read(&directory)
            .unwrap_err()
            .contains("unexpected contract asset")
    );
}

#[test]
fn a_new_crate_from_the_template_is_a_valid_crate_held_for_review() {
    let root = scratch("template");
    let directory = root.join("quirin-provider-widget-pool");
    template::instantiate(&directory, "quirin-provider-widget-pool").unwrap();
    let krate = UserCrate::read(&directory).unwrap();
    assert_eq!(krate.namespace, "widget_pool");
    let text = &krate.modules["widget_pool/example.qcore"];
    assert!(text.contains("extern fn widget_pool_release("));
    assert!(text.contains("requires resource_review_pending("));
    assert!(template::instantiate(&directory, "quirin-provider-widget-pool").is_err());
    assert!(template::instantiate(&root.join("x"), "widget").is_err());
}

#[test]
fn the_example_checker_packages_verifies_and_answers_a_claim() {
    let root = scratch("package");
    let output = root.join("pool-check");
    let installed = package::build(
        Path::new(env!("CARGO_BIN_EXE_quirin-plugin-example-checker")),
        &output,
    )
    .unwrap();
    assert_eq!(installed.plugin_id.as_str(), "acme.pool_check");
    assert_eq!(package::verify(&output).unwrap(), installed);
    // A package whose installed manifest was edited no longer verifies.
    let path = output.join(package::INSTALLED_MANIFEST);
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replace("\"timeout_ms\": 5000", "\"timeout_ms\": 6000"),
    )
    .unwrap();
    assert!(package::verify(&output).unwrap_err().contains("differs"));
    fs::write(&path, text).unwrap();
    // The packaged checker answers one evidence-v2 frame with a Safe claim
    // and no evidence: what the audit's firewall reads as Pending.
    let request = VerificationRequestV2 {
        protocol_version: PLUGIN_PROTOCOL_VERSION,
        evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        request_id: "acme.pool_check:1".into(),
        plugin_id: PluginId::new("acme.pool_check").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "acme".into(),
            revision: "0.1.0".into(),
            profile: "user-contract".into(),
            dialect: "qcore-resource-v1".into(),
            platform: "any".into(),
            backend: "quirin-audit".into(),
        },
        unit: UnitIdentity {
            source_digest: "a".repeat(64),
            build_digest: "b".repeat(64),
            translation_unit: "quirin-provider-acme:acme/pool.qcore".into(),
        },
        facts: Vec::new(),
        assumptions: Default::default(),
        task_digest: "c".repeat(64),
        obligations: vec![qcore_schemas::obligation::Obligation {
            obligation_id: "resource_contracts:0".into(),
            capability: VerifierCapability::ResourceContracts,
            property: qcore_schemas::obligation::ObligationProperty::ParameterModeRespected,
            subject: qcore_schemas::obligation::ObligationSubject::Function {
                symbol: "acme_pool_get".into(),
                translation_unit: "quirin-provider-acme:acme/pool.qcore".into(),
            },
            scope: qcore_schemas::obligation::ObligationScope::WholeUnit,
            bounds: qcore_schemas::obligation::TaskBounds::single_path(),
            assumptions: Default::default(),
            evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        }],
        source_effect: None,
        counter_effect: None,
        reactor_program: None,
        teardown_source: None,
        call_use_source: None,
        cleanup_replay_source: None,
        composition_source: None,
    };
    let payload = serde_json::to_vec(&request).unwrap();
    let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
    frame.extend(payload);
    let mut child = Command::new(output.join("quirin-plugin-example-checker"))
        .current_dir(&output)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&frame).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let response: VerificationResponseV2 = serde_json::from_slice(&result.stdout[4..]).unwrap();
    assert_eq!(response.verdict, ProposedVerdict::Safe);
    assert!(response.evidence.is_none());
    assert_eq!(response.request_id, request.request_id);
}
