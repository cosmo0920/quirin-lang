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

//! Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (gateway rules 14f–14j),
//! end to end on the contracts route (`quirin-audit contracts`) with the bounds
//! property selected, over a self-contained fixture modelled on chunkio's
//! header accessors (no provider contract and no corpus pin moves):
//!
//! * **header bodies (14f):** a `static inline` header definition has no row;
//!   its body is checked in every unit that includes the header and published
//!   as evidence for its declaration (`body_evidence` in the selection
//!   record), with the Safe roll-up;
//! * **cross-unit refutation (14h):** a declaration whose main-source body
//!   refutes it in one unit is withdrawn from a caller in another unit by the
//!   second phase (the caller moves Safe → Unknown, never Unsafe), and a body
//!   that does not refute leaves no table and no second phase;
//! * **tips (14i):** `pin --bounds --track-tip` records a tip entry; in force,
//!   it is a named assumption with an `info` notice; with other sites or an
//!   unresolvable ref it is held (`contract-pin-mismatch`), with a `warning`;
//!   the tip decision is bound to `QCore.BoundsTransport.tipVectors`.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const BOUNDS: &str = "module contract;

extern fn hget(map: ptr<mut, u8>) -> u8
contract {
    requires bounds_contract_v1;
    quant checks map, Idx #i: 2 <= #i && #i < 4;
    ensures normal bounds_frame_reads(map);
};

extern fn pick(p: ptr<mut, u8>) -> u8
contract {
    requires bounds_contract_v1;
    quant checks p, Idx #i: 0 <= #i && #i < 2;
    ensures normal bounds_frame_reads(p);
};
";

// A resource module beside it, so the resource selection is not empty.
const RESOURCE: &str = "module contract;
extern fn dispose(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires resource_revision(flexible);
    ensures normal resource_released(p, nonnull);
    ensures normal resource_return_none();
};
";

const HEADER: &str = "static inline unsigned char hget(char *map)
{
    return (unsigned char) (map[2] ^ map[3]);
}
";

const USE_HEADER: &str = "#include \"meta.h\"
unsigned char header_caller(void)
{
    char buffer[4];
    buffer[2] = 1;
    buffer[3] = 2;
    return hget(buffer);
}
";

const PICK_REFUTES: &str = "unsigned char pick(char *p)
{
    return (unsigned char) p[5];
}
";

const PICK_KEEPS: &str = "unsigned char pick(char *p)
{
    return (unsigned char) p[1];
}
";

const USE_PICK: &str = "unsigned char pick(char *p);
unsigned char pick_caller(void)
{
    char two[2];
    two[0] = 1;
    two[1] = 2;
    return pick(two);
}
";

struct Fixture(PathBuf);

impl Fixture {
    fn new(pick: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-bounds-transport-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("tree")).unwrap();
        fs::write(root.join("tree/meta.h"), HEADER).unwrap();
        fs::write(root.join("tree/use_header.c"), USE_HEADER).unwrap();
        fs::write(
            root.join("tree/also_header.c"),
            USE_HEADER.replace("header_caller", "second_caller"),
        )
        .unwrap();
        fs::write(root.join("tree/pick.c"), pick).unwrap();
        fs::write(root.join("tree/use_pick.c"), USE_PICK).unwrap();
        let contracts = root.join("providers/quirin-provider-alpha/contracts/alpha");
        fs::create_dir_all(&contracts).unwrap();
        fs::write(contracts.join("meta.qv"), BOUNDS).unwrap();
        fs::write(contracts.join("heap.qv"), RESOURCE).unwrap();
        fs::write(
            root.join("providers/contract-load-path.json"),
            serde_json::to_string_pretty(&json!({"schema_version": 1, "crates": [
                {"name": "quirin-provider-alpha", "namespace": "alpha", "version": "0.1.0",
                 "contracts": "quirin-provider-alpha/contracts", "dependencies": [],
                 "modules": ["alpha/heap.qv", "alpha/meta.qv"], "published": {},
                 "resource_view": ["alpha/heap.qv"], "bounds_view": ["alpha/meta.qv"]},
            ]}))
            .unwrap(),
        )
        .unwrap();
        Self(root)
    }

    fn quirin(&self, args: &[&str], bounds: bool, tips: Option<&str>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quirin-audit"));
        command
            .current_dir(&self.0)
            .args(args)
            .env_remove("QUIRIN_TIP_REPOSITORIES");
        if bounds {
            command.env("QCORE_BOUNDS_PROPERTY", self.0.join("out/bounds.jsonl"));
        } else {
            command.env_remove("QCORE_BOUNDS_PROPERTY");
        }
        if let Some(tips) = tips {
            command.env("QUIRIN_TIP_REPOSITORIES", tips);
        }
        command.output().unwrap()
    }

    fn pin(&self, extra: &[&str]) -> String {
        let mut args = vec![
            "pin",
            "--bounds",
            "--source-root",
            "tree",
            "--load-path",
            "providers/contract-load-path.json",
        ];
        args.extend_from_slice(extra);
        stdout(&self.quirin(&args, false, None))
    }

    /// The bounds rows by function, the selection record and the console.
    fn audit(
        &self,
        tips: Option<&str>,
    ) -> (std::collections::BTreeMap<String, Value>, Value, String) {
        let _ = fs::remove_dir_all(self.0.join("out"));
        fs::create_dir_all(self.0.join("out")).unwrap();
        let console = stdout(&self.quirin(
            &[
                "contracts",
                "--source-root",
                "tree",
                "use_header.c",
                "also_header.c",
                "pick.c",
                "use_pick.c",
                "--load-path",
                "providers/contract-load-path.json",
                "--report",
                "out/report.json",
            ],
            true,
            tips,
        ));
        let rows = fs::read_to_string(self.0.join("out/bounds.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .map(|row| {
                (
                    row["function"].as_str().unwrap().to_owned(),
                    row["bounds"].clone(),
                )
            })
            .collect();
        let record =
            serde_json::from_slice(&fs::read(self.0.join("out/bounds.contracts.json")).unwrap())
                .unwrap();
        (rows, record, console)
    }

    fn vcs(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(self.0.join("tree"))
            .args([
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "init.defaultBranch=main",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "{args:?}: {output:?}");
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn commit(&self, message: &str) {
        self.vcs(&["add", "-A"]);
        self.vcs(&["commit", "-q", "-m", message]);
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
    eprintln!("skipping Clang-dependent bounds transport test");
    false
}

fn verdict(rows: &std::collections::BTreeMap<String, Value>, f: &str) -> String {
    rows[f]["verdict"].as_str().unwrap().to_owned()
}

/// Rule 14f: the header body has no row and is published as evidence for its
/// declaration from both including units; rule 14h: no refutation, no table.
#[test]
fn a_header_body_is_evidence_for_its_declaration_and_not_a_row() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new(PICK_KEEPS);
    fixture.pin(&[]);
    let (rows, record, console) = fixture.audit(None);
    assert!(!rows.contains_key("hget"), "a header definition has no row");
    assert_eq!(verdict(&rows, "header_caller"), "Safe", "{rows:#?}");
    assert_eq!(verdict(&rows, "pick_caller"), "Safe", "{rows:#?}");
    let hget = &record["body_evidence"]["declarations"]["hget"];
    assert_eq!(hget["body_standing"], "validated", "{hget:#}");
    assert_eq!(hget["units_examined"], 2, "{hget:#}");
    assert_eq!(
        hget["clauses"],
        json!({
            "bounds_frame_reads(map)": "validated",
            "quant checks map, Idx #i: 2 <= #i && #i < 4": "validated",
        })
    );
    // Both header callers, `pick_caller` and `pick` itself (relative to its
    // own declaration) rest only on validated declarations.
    assert_eq!(
        record["body_evidence"]["safe_rollup"]["resting_only_on_validated_declarations"], 4,
        "{:#}",
        record["body_evidence"]
    );
    assert!(record.get("refutation_table").is_none(), "{record:#}");
    assert!(
        console.contains("bounds body evidence: 2 declaration(s): 2 validated"),
        "{console}"
    );
}

/// Rule 14h: `pick`'s body (in `pick.c`) accesses `p[5]`, outside its check:
/// the second phase withdraws the declaration from `use_pick.c`'s caller,
/// which moves Safe → Unknown (never Unsafe); the declaration is a declared
/// assumption with one failing unit.
#[test]
fn a_body_refuting_its_declaration_withdraws_it_in_every_unit() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new(PICK_REFUTES);
    fixture.pin(&[]);
    let (rows, record, _) = fixture.audit(None);
    assert_eq!(
        record["refutation_table"]["refuted"],
        json!(["pick"]),
        "{record:#}"
    );
    assert_eq!(verdict(&rows, "pick_caller"), "Unknown", "{rows:#?}");
    assert!(
        rows["pick_caller"]["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r.as_str().unwrap().contains("states no bounds contract")),
        "{:#}",
        rows["pick_caller"]
    );
    assert_eq!(verdict(&rows, "pick"), "Unknown");
    let pick = &record["body_evidence"]["declarations"]["pick"];
    assert_eq!(pick["body_standing"], "declared-assumption", "{pick:#}");
    assert_eq!(pick["units_failed"], 1, "{pick:#}");
    // The header declaration is untouched.
    assert_eq!(verdict(&rows, "header_caller"), "Safe");
}

/// Rule 14i: a bounds tip entry, end to end.
#[test]
fn a_bounds_tip_entry_is_a_named_assumption_in_force_and_held_otherwise() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new(PICK_KEEPS);
    fixture.vcs(&["init", "-q"]);
    fixture.commit("base");
    fixture.vcs(&["checkout", "-q", "-b", "fix"]);
    fixture.pin(&[]);
    let tracked = fixture.pin(&["--function", "hget", "--track-tip", "fix"]);
    assert!(
        tracked.contains("tracks the tip `refs/heads/fix`"),
        "{tracked}"
    );
    assert!(
        tracked.contains("targets a future, unreleased release"),
        "{tracked}"
    );

    // In force: the audited sites are the tip's.
    let (rows, record, console) = fixture.audit(None);
    let outcome = record["pins"]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["function"] == "hget")
        .unwrap()
        .clone();
    assert_eq!(outcome["standing"], "tip-in-force", "{outcome:#}");
    assert_eq!(outcome["notice"]["severity"], "info", "{outcome:#}");
    assert_eq!(
        record["pins"]["assumptions"]["tip_tracked"][0]["name"],
        "tip-tracked:hget"
    );
    assert!(
        console.contains("bounds contract notice [info]"),
        "{console}"
    );
    assert_eq!(verdict(&rows, "header_caller"), "Safe");

    // The tip moves away from the audited source: held, a warning.
    fs::write(
        fixture.0.join("tree/meta.h"),
        HEADER.replace("map[2] ^ map[3]", "map[3] ^ map[2]"),
    )
    .unwrap();
    fixture.commit("tip moves");
    fixture.vcs(&["checkout", "-q", "main"]);
    let (rows, record, console) = fixture.audit(None);
    let outcome = record["pins"]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["function"] == "hget")
        .unwrap()
        .clone();
    assert_eq!(outcome["reason"], "source-changed", "{outcome:#}");
    assert_eq!(outcome["notice"]["severity"], "warning", "{outcome:#}");
    assert_eq!(record["pins"]["assumptions"]["tip_tracked"], json!([]));
    assert!(
        console.contains("bounds contract notice [warning]"),
        "{console}"
    );
    assert_eq!(verdict(&rows, "header_caller"), "Pending", "{rows:#?}");
    assert_eq!(
        record["body_evidence"]["declarations"]["hget"]["body_standing"],
        "held"
    );

    // The ref is gone: unresolved, held.
    fixture.vcs(&["branch", "-q", "-D", "fix"]);
    let (rows, record, _) = fixture.audit(None);
    let outcome = record["pins"]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["function"] == "hget")
        .unwrap()
        .clone();
    assert_eq!(outcome["reason"], "tip-unresolved", "{outcome:#}");
    assert_eq!(verdict(&rows, "header_caller"), "Pending", "{rows:#?}");
}

/// `QCore.BoundsTransport.tipVectors`: (text matches, resolved, sites equal,
/// reason), against the C06.2 decision a bounds tip entry is compared by.
#[test]
fn tip_vectors_match_lean() {
    use qcore_audit::contract_revision::{ContractLock, LOCK_VERSION, PinStanding, pin_standing};
    use qcore_audit::contract_tip::{TipEntry, TipRepository, TipResolver, TipReview};
    let vectors = [
        (true, true, true, None),
        (true, true, false, Some("source-changed")),
        (true, false, true, Some("tip-unresolved")),
        (false, true, true, Some("contract-changed")),
        (false, false, false, Some("contract-changed")),
    ];
    assert_eq!(vectors.len(), 5, "tip_vectors_count");
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new(PICK_KEEPS);
    fixture.vcs(&["init", "-q"]);
    fixture.commit("base");
    let head = fixture.vcs(&["rev-parse", "HEAD"]);
    let tree = fixture.0.join("tree");
    let sites = qcore_audit::c_identity::SourceTree::read_sites(
        &tree,
        &std::collections::BTreeSet::from(["hget".to_owned()]),
    )
    .unwrap()
    .sites["hget"]
        .clone();
    let digest = |n: u64| format!("{n:064x}");
    for (text, resolved, same, expected) in vectors {
        let entry = TipEntry {
            function: "hget".into(),
            module: "alpha/meta.qv".into(),
            contract_sha256: digest(if text { 7 } else { 8 }),
            pinned_at: None,
            repository: TipRepository {
                name: "tree".into(),
                url: None,
                path: Some(tree.display().to_string()),
            },
            reference: if resolved {
                "refs/heads/main".into()
            } else {
                "refs/heads/absent".into()
            },
            release_label: None,
            reviewed: TipReview {
                commit: head.clone(),
                sites: sites.clone(),
            },
        };
        let lock = ContractLock {
            lock_version: LOCK_VERSION,
            generated_by: "quirin-audit pin".into(),
            note: String::new(),
            owner: "quirin-provider-alpha".into(),
            normalisation: "c-tokens-v1".into(),
            fluent_bit_revision: None,
            pinned: Vec::new(),
            flexible: Vec::new(),
            tip: vec![entry],
        };
        let audited = if same { sites.clone() } else { Vec::new() };
        let standing = pin_standing(
            Some(&lock),
            "hget",
            &digest(7),
            &audited,
            "quirin-provider-alpha:alpha/meta.qv",
            &mut TipResolver::default(),
        );
        let found = match &standing {
            PinStanding::Mismatch { reason, .. } => Some(reason.as_str()),
            _ => None,
        };
        assert_eq!(found, expected, "{text} {resolved} {same}: {standing:?}");
    }
}

/// Rule 14j: inclusion and inheritance (G05a) stay outside the bounds
/// vocabulary. A bounds module that includes another and inherits a bounds
/// declaration is accepted by neither loader, so the generated views leave it
/// out (the probe), and a load path that names it in a bounds view refuses the
/// whole selection rather than load it un-elaborated.
#[test]
fn an_inheriting_bounds_module_never_reaches_a_bounds_view() {
    let fixture = Fixture::new(PICK_KEEPS);
    let contracts = fixture
        .0
        .join("providers/quirin-provider-alpha/contracts/alpha");
    fs::write(
        contracts.join("child.qv"),
        "module contract;\n#include \"quirin-provider-alpha:alpha/meta.qv\";\n\
         extern fn hget2(m: ptr<mut, u8>) -> u8\ncontract inherit hget;\n",
    )
    .unwrap();
    let load_path = fixture.0.join("providers/contract-load-path.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&load_path).unwrap()).unwrap();
    manifest["crates"][0]["modules"] = json!(["alpha/child.qv", "alpha/heap.qv", "alpha/meta.qv"]);
    manifest["crates"][0]["published"] =
        json!({"alpha/meta.qv": qcore_schemas::sha256_hex(BOUNDS.as_bytes())});
    fs::write(&load_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
    let probed = qcore_audit::providers::probe_resource_views(&load_path).unwrap();
    let views = &probed["crates"]["quirin-provider-alpha"];
    assert_eq!(views["bounds_view"], json!(["alpha/meta.qv"]), "{probed:#}");
    assert_eq!(
        views["resource_view"],
        json!(["alpha/heap.qv"]),
        "{probed:#}"
    );
    manifest["crates"][0]["bounds_view"] = json!(["alpha/child.qv", "alpha/meta.qv"]);
    fs::write(&load_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
    let selection =
        qcore_contract::load_path_manifest::load_contract_load_path_manifest(&load_path)
            .map_err(|e| e.to_string())
            .and_then(|manifest| qcore_audit::bounds_selection::load_views(&manifest).map(|_| ()));
    assert!(selection.is_err(), "{selection:?}");
}
