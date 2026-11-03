//! Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS (gateway rule 12r): bounds
//! contract modules transported to an audit route from a provider crate's
//! bounds view, pinned by `quirin-audit pin --bounds`, and held when their pin
//! is not in force, end to end on the command line.
//!
//! A self-contained fixture tree modelled on chunkio's header accessors
//! (`get_meta_len` reads `map[22]` and `map[23]`; `set_meta_len` writes them)
//! on one temporary load path; no provider contract and no corpus pin moves.
//! The pin decision is bound to `QCore.BoundsPinHold`'s `decisionVectors` and
//! the standing to its `standingVectors`.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_audit::contract_revision::{
    ContractLock, LOCK_VERSION, PinStanding, PinnedEntry, ReviewedIdentity, ReviewedSource,
    pin_standing,
};
use qcore_audit::contract_tip::TipResolver;
use qcore_schemas::contract_revision::PinMismatchReason;
use serde_json::{Value, json};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const BOUNDS: &str = "module contract;

extern fn get_meta_len(map: ptr<mut, u8>) -> u16
contract {
    requires bounds_contract_v1;
    quant checks map, Idx #i: 22 <= #i && #i < 24;
    ensures normal bounds_frame_reads(map);
};

extern fn set_meta_len(map: ptr<mut, u8>, len: u16) -> unit
contract {
    requires bounds_contract_v1;
    quant checks map, Idx #i: 22 <= #i && #i < 24;
    ensures normal bounds_frame_writes(map);
};
";

// A resource module beside it, so the resource selection is not empty; bound
// by name (flexible), as the synthetic API has no body.
const RESOURCE: &str = "module contract;
extern fn dispose(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires resource_revision(flexible);
    ensures normal resource_released(p, nonnull);
    ensures normal resource_return_none();
};
";

const SOURCE: &str = "unsigned short get_meta_len(char *map)
{
    return (unsigned short) (((unsigned char) map[22] << 8) | (unsigned char) map[23]);
}

void set_meta_len(char *map, unsigned short len)
{
    map[22] = (char) (len >> 8);
    map[23] = (char) (len & 0xFF);
}

unsigned short full_header(void)
{
    char header[24];
    set_meta_len(header, 3);
    return get_meta_len(header);
}

unsigned short short_header(void)
{
    char header[16];
    return get_meta_len(header);
}
";

const PREDICATE: &str = "22 <= #i && #i < 24;";

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-bounds-pin-hold-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("tree")).unwrap();
        fs::write(root.join("tree/meta.c"), SOURCE).unwrap();
        let contracts = root.join("providers/quirin-provider-alpha/contracts/alpha");
        fs::create_dir_all(&contracts).unwrap();
        fs::write(contracts.join("meta.qv"), BOUNDS).unwrap();
        fs::write(contracts.join("heap.qv"), RESOURCE).unwrap();
        let fixture = Self(root);
        fixture.load_path(&["alpha/heap.qv"], &["alpha/meta.qv"]);
        fixture
    }

    fn load_path(&self, resource: &[&str], bounds: &[&str]) {
        fs::write(
            self.0.join("providers/contract-load-path.json"),
            serde_json::to_string_pretty(&json!({"schema_version": 1, "crates": [
                {"name": "quirin-provider-alpha", "namespace": "alpha", "version": "0.1.0",
                 "contracts": "quirin-provider-alpha/contracts", "dependencies": [],
                 "modules": ["alpha/heap.qv", "alpha/meta.qv"], "published": {},
                 "resource_view": resource, "bounds_view": bounds},
            ]}))
            .unwrap(),
        )
        .unwrap();
    }

    fn quirin(&self, args: &[&str], bounds: bool) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quirin-audit"));
        command.current_dir(&self.0).args(args);
        if bounds {
            command.env("QCORE_BOUNDS_PROPERTY", self.0.join("out/bounds.jsonl"));
        } else {
            command.env_remove("QCORE_BOUNDS_PROPERTY");
        }
        command.output().unwrap()
    }

    fn audit_with(&self, bounds: bool) -> Output {
        let _ = fs::remove_dir_all(self.0.join("out"));
        fs::create_dir_all(self.0.join("out")).unwrap();
        self.quirin(
            &[
                "contracts",
                "--source-root",
                "tree",
                "meta.c",
                "--load-path",
                "providers/contract-load-path.json",
                "--report",
                "out/report.json",
            ],
            bounds,
        )
    }

    /// The bounds verdict of each function, and the selection record.
    fn audit(
        &self,
    ) -> (
        std::collections::BTreeMap<String, (String, Vec<String>)>,
        Value,
    ) {
        stdout(&self.audit_with(true));
        let rows = fs::read_to_string(self.0.join("out/bounds.jsonl")).unwrap();
        let verdicts = rows
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .map(|row| {
                (
                    row["function"].as_str().unwrap().to_owned(),
                    (
                        row["bounds"]["verdict"].as_str().unwrap().to_owned(),
                        row["bounds"]["reasons"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|r| r.as_str().unwrap().to_owned())
                            .collect(),
                    ),
                )
            })
            .collect();
        (verdicts, self.read("out/bounds.contracts.json"))
    }

    fn pin(&self, extra: &[&str]) -> Output {
        let mut args = vec![
            "pin",
            "--source-root",
            "tree",
            "--load-path",
            "providers/contract-load-path.json",
        ];
        args.extend_from_slice(extra);
        self.quirin(&args, false)
    }

    fn read(&self, path: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(path)).unwrap()).unwrap()
    }

    fn edit(&self, path: &str, old: &str, new: &str) {
        let path = self.0.join(path);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains(old), "{old}");
        fs::write(path, text.replace(old, new)).unwrap();
    }

    fn lock(&self) -> PathBuf {
        self.0
            .join("providers/quirin-provider-alpha/bounds-contracts.lock")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stdout(output: &Output) -> String {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn clang_available() -> bool {
    if qcore_c_gateway::clang_command().is_ok() {
        return true;
    }
    assert!(
        std::env::var_os("QCORE_CLANG").is_none(),
        "explicit QCORE_CLANG is invalid"
    );
    eprintln!("skipping Clang-dependent bounds pin test");
    false
}

fn verdict(rows: &std::collections::BTreeMap<String, (String, Vec<String>)>, f: &str) -> String {
    rows[f].0.clone()
}

fn reasons(rows: &std::collections::BTreeMap<String, (String, Vec<String>)>, f: &str) -> String {
    rows[f].1.join(" | ")
}

fn pin_reasons(record: &Value) -> Vec<(String, Option<String>)> {
    record["pins"]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| {
            (
                o["function"].as_str().unwrap().to_owned(),
                o["reason"].as_str().map(str::to_owned),
            )
        })
        .collect()
}

/// Unpinned (no lock): both declarations are held; every function relying on
/// them is Pending or Unknown, never Safe, and the too-small header is never
/// accused. Pinned: in force; the bodies and the full header are Safe and the
/// short header is a (fixture) violation.
#[test]
fn an_unpinned_bounds_declaration_is_held_and_a_pin_puts_it_in_force() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    let (rows, record) = fixture.audit();
    assert_eq!(record["selection"], "all-bounds-views");
    assert_eq!(record["modules"][0]["module"], "alpha/meta.qv", "{record}");
    assert_eq!(
        pin_reasons(&record),
        vec![
            ("get_meta_len".to_owned(), Some("no-lock".to_owned())),
            ("set_meta_len".to_owned(), Some("no-lock".to_owned())),
        ]
    );
    assert_eq!(
        verdict(&rows, "get_meta_len"),
        "Pending",
        "{}",
        reasons(&rows, "get_meta_len")
    );
    assert_eq!(verdict(&rows, "set_meta_len"), "Pending");
    assert_eq!(
        verdict(&rows, "full_header"),
        "Pending",
        "{}",
        reasons(&rows, "full_header")
    );
    assert_eq!(
        verdict(&rows, "short_header"),
        "Unknown",
        "{}",
        reasons(&rows, "short_header")
    );
    assert!(reasons(&rows, "get_meta_len").contains("contract-pin-mismatch"));
    let unpinned_digest = record["environment_sha256"].clone();

    let pinned = stdout(&fixture.pin(&["--bounds"]));
    assert!(
        pinned.contains("quirin-provider-alpha (bounds): 2 pinned"),
        "{pinned}"
    );
    let lock = ContractLock::read(&fixture.lock()).unwrap();
    assert_eq!(lock.owner, "quirin-provider-alpha");
    assert_eq!(
        lock.pinned
            .iter()
            .map(|e| e.module.as_str())
            .collect::<Vec<_>>(),
        ["alpha/meta.qv", "alpha/meta.qv"]
    );
    stdout(&fixture.pin(&["--bounds", "--check"]));

    let (rows, record) = fixture.audit();
    assert_eq!(
        pin_reasons(&record),
        vec![
            ("get_meta_len".to_owned(), None),
            ("set_meta_len".to_owned(), None),
        ]
    );
    assert_ne!(record["environment_sha256"], unpinned_digest);
    assert_eq!(
        verdict(&rows, "get_meta_len"),
        "Safe",
        "{}",
        reasons(&rows, "get_meta_len")
    );
    assert_eq!(
        verdict(&rows, "set_meta_len"),
        "Safe",
        "{}",
        reasons(&rows, "set_meta_len")
    );
    assert_eq!(
        verdict(&rows, "full_header"),
        "Safe",
        "{}",
        reasons(&rows, "full_header")
    );
    assert_eq!(
        verdict(&rows, "short_header"),
        "Unsafe",
        "{}",
        reasons(&rows, "short_header")
    );
}

/// Negative controls: a changed body (a stale pin), a changed clause and a
/// missing entry each take the declaration out of force, holding it.
#[test]
fn a_changed_body_a_changed_clause_or_a_missing_entry_holds_the_declaration() {
    if !clang_available() {
        return;
    }
    // Changed body: the lock's identity is stale.
    let fixture = Fixture::new();
    stdout(&fixture.pin(&["--bounds"]));
    fixture.edit(
        "tree/meta.c",
        "(unsigned char) map[23]);",
        "(unsigned char) map[23] | 0);",
    );
    let check = fixture.pin(&["--bounds", "--check"]);
    assert_eq!(check.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&check.stdout)
            .contains("x get_meta_len: not in force (source-changed)"),
        "{}",
        String::from_utf8_lossy(&check.stdout)
    );
    let (rows, record) = fixture.audit();
    assert_eq!(
        pin_reasons(&record),
        vec![
            ("get_meta_len".to_owned(), Some("source-changed".to_owned())),
            ("set_meta_len".to_owned(), None),
        ]
    );
    assert_eq!(verdict(&rows, "get_meta_len"), "Pending");
    assert_eq!(verdict(&rows, "set_meta_len"), "Safe");
    assert_ne!(verdict(&rows, "full_header"), "Safe");
    assert_ne!(verdict(&rows, "short_header"), "Unsafe");

    // Changed clause: the quant check's text is part of the pinned digest.
    let fixture = Fixture::new();
    stdout(&fixture.pin(&["--bounds"]));
    fixture.edit(
        "providers/quirin-provider-alpha/contracts/alpha/meta.qv",
        &format!("map, Idx #i: {PREDICATE}\n    ensures normal bounds_frame_reads"),
        "map, Idx #i: 22 <= #i && #i <= 23;\n    ensures normal bounds_frame_reads",
    );
    let (rows, record) = fixture.audit();
    assert_eq!(
        pin_reasons(&record),
        vec![
            (
                "get_meta_len".to_owned(),
                Some("contract-changed".to_owned())
            ),
            ("set_meta_len".to_owned(), None),
        ]
    );
    assert_eq!(verdict(&rows, "get_meta_len"), "Pending");
    // The lock no longer lists the current digest: --verify-contracts says so.
    let verify = fixture.quirin(
        &[
            "pin",
            "--verify-contracts",
            "--load-path",
            "providers/contract-load-path.json",
        ],
        false,
    );
    assert_eq!(verify.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&verify.stdout).contains(
            "quirin-provider-alpha (bounds): stale lock: `get_meta_len`'s contract text changed"
        ),
        "{}",
        String::from_utf8_lossy(&verify.stdout)
    );

    // Missing entry: only `set_meta_len` pinned.
    let fixture = Fixture::new();
    stdout(&fixture.pin(&["--bounds", "--function", "set_meta_len"]));
    let (rows, record) = fixture.audit();
    assert_eq!(
        pin_reasons(&record),
        vec![
            ("get_meta_len".to_owned(), Some("not-in-lock".to_owned())),
            ("set_meta_len".to_owned(), None),
        ]
    );
    assert_eq!(verdict(&rows, "get_meta_len"), "Pending");
}

/// Without the selecting variable nothing is read: no bounds files, and the
/// custody report is byte-identical to the one the bounds run writes.
#[test]
fn the_custody_report_is_unchanged_by_the_bounds_selection() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    stdout(&fixture.pin(&["--bounds"]));
    stdout(&fixture.audit_with(false));
    assert!(!fixture.0.join("out/bounds.jsonl").exists());
    assert!(!fixture.0.join("out/bounds.contracts.json").exists());
    let plain = fs::read(fixture.0.join("out/report.json")).unwrap();
    stdout(&fixture.audit_with(true));
    assert!(fixture.0.join("out/bounds.contracts.json").exists());
    assert_eq!(fs::read(fixture.0.join("out/report.json")).unwrap(), plain);
}

/// A bounds module is outside every resource view: naming it in a resource
/// selection refuses, and a load path declaring a module in both views, or a
/// bounds view naming a module the crate does not own, refuses.
#[test]
fn a_bounds_module_never_reaches_a_resource_selection() {
    let fixture = Fixture::new();
    let probe = qcore_audit::providers::probe_resource_views(
        &fixture.0.join("providers/contract-load-path.json"),
    )
    .unwrap();
    let alpha = &probe["crates"]["quirin-provider-alpha"];
    assert_eq!(alpha["resource_view"], json!(["alpha/heap.qv"]));
    assert_eq!(alpha["bounds_view"], json!(["alpha/meta.qv"]));

    let refused = fixture.quirin(
        &[
            "contracts",
            "--source-root",
            "tree",
            "meta.c",
            "--load-path",
            "providers/contract-load-path.json",
            "--provider",
            "quirin-provider-alpha:alpha/meta.qv",
            "--report",
            "out/report.json",
        ],
        false,
    );
    assert!(!refused.status.success());

    fixture.load_path(&["alpha/heap.qv", "alpha/meta.qv"], &["alpha/meta.qv"]);
    let both = qcore_contract::load_path_manifest::load_contract_load_path_manifest(
        &fixture.0.join("providers/contract-load-path.json"),
    );
    assert!(
        both.as_ref().is_err_and(|e| e
            .to_string()
            .contains("both its resource view and its bounds view")),
        "{both:?}"
    );
    fixture.load_path(&["alpha/heap.qv"], &["alpha/absent.qv"]);
    let unowned = qcore_contract::load_path_manifest::load_contract_load_path_manifest(
        &fixture.0.join("providers/contract-load-path.json"),
    );
    assert!(unowned.is_err(), "{unowned:?}");
}

/// `QCore.BoundsPinHold.decisionVectors`: the C06.2 decision a bounds
/// declaration is compared by (function 1, digest 7, audited identity 3;
/// identities are encoded as one site whose declaration digest is the number).
#[test]
fn decision_vectors_match_lean() {
    let digest = |n: u64| format!("{n:064x}");
    let identity = |n: u64| {
        vec![qcore_audit::c_identity::SourceSite {
            path: "meta.c".into(),
            kind: qcore_audit::c_identity::SiteKind::Definition,
            declaration_sha256: digest(n),
            body_sha256: None,
        }]
    };
    let entry = |function: &str, d: u64, ids: &[u64]| PinnedEntry {
        function: function.into(),
        module: "alpha/meta.qv".into(),
        contract_sha256: digest(d),
        pinned_at: None,
        identities: ids
            .iter()
            .enumerate()
            .map(|(i, n)| ReviewedIdentity {
                reviewed_at: vec![ReviewedSource {
                    kind: "git".into(),
                    revision: Some(format!("{i}")),
                }],
                sites: identity(*n),
            })
            .collect(),
        derivation: None,
    };
    let lock = |entries: Vec<PinnedEntry>| ContractLock {
        lock_version: LOCK_VERSION,
        generated_by: "quirin-audit pin".into(),
        note: String::new(),
        owner: "quirin-provider-alpha".into(),
        normalisation: "c-tokens-v1".into(),
        fluent_bit_revision: None,
        pinned: entries,
        flexible: Vec::new(),
        tip: Vec::new(),
    };
    use PinMismatchReason::{ContractChanged, NoLock, NotInLock, SourceChanged};
    let vectors: Vec<(Option<ContractLock>, Option<PinMismatchReason>)> = vec![
        (None, Some(NoLock)),
        (Some(lock(vec![])), Some(NotInLock)),
        (Some(lock(vec![entry("f2", 7, &[3])])), Some(NotInLock)),
        (
            Some(lock(vec![entry("f1", 8, &[3])])),
            Some(ContractChanged),
        ),
        (Some(lock(vec![entry("f1", 7, &[])])), Some(NotInLock)),
        (Some(lock(vec![entry("f1", 7, &[4])])), Some(SourceChanged)),
        (Some(lock(vec![entry("f1", 7, &[3])])), None),
        (Some(lock(vec![entry("f1", 7, &[4, 3])])), None),
        (
            Some(lock(vec![entry("f1", 8, &[4, 3])])),
            Some(ContractChanged),
        ),
    ];
    assert_eq!(vectors.len(), 9, "decision_vectors_count");
    let mut tips = TipResolver::default();
    for (lock, expected) in vectors {
        let standing = pin_standing(
            lock.as_ref(),
            "f1",
            &digest(7),
            &identity(3),
            "quirin-provider-alpha:alpha/meta.qv",
            &mut tips,
        );
        let found = match standing {
            PinStanding::Mismatch { reason, .. } => Some(reason),
            _ => None,
        };
        assert_eq!(found, expected, "{lock:?}");
    }
}
