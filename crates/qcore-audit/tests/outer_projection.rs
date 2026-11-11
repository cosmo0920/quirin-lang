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

//! Card OUTER-CONTRACT (gateway rule 31, contract specification C04.20): the
//! runtime projection. Unit vectors for the facts, the contributions and the
//! join (bound to `QCore.OuterContract`'s `projectionVectors`), and an end
//! to end run of the no-build route on a self-contained fixture: the static
//! report is byte-identical with and without the projection, the loader's pin
//! decides whether the contract covers anything, and the reach is read from
//! Clang's AST of the loader's unit.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_audit::outer_selection::{
    Fact, ScenarioContract, contribution, custody_facts, null_facts, runtime_verdict,
};
use qcore_c_gateway::outer_record_graph::Reach;
use qcore_contract::outer_contract::{EnvelopeLine, MemberEnvelope};
use qcore_schemas::verdict::Verdict;
use serde_json::{Value, json};

fn contract(in_force: bool, reach: Reach) -> ScenarioContract {
    ScenarioContract {
        name: "app_plugin".into(),
        identity: "app_plugin@abc".into(),
        in_force,
        envelope: vec![
            EnvelopeLine::Member {
                tag: "instance".into(),
                path: "p".into(),
                envelope: MemberEnvelope::Kept,
            },
            EnvelopeLine::Member {
                tag: "collector".into(),
                path: "cb".into(),
                envelope: MemberEnvelope::Registered,
            },
            EnvelopeLine::Premise {
                name: "app_closed".into(),
            },
        ],
        reach,
    }
}

fn all() -> Reach {
    Reach::All {
        witness: "void".into(),
    }
}

fn stable(key: &str) -> Fact {
    Fact::Member {
        kind: "stable",
        key: key.into(),
    }
}

#[test]
fn rk_loader_does_not_vouch_for_fluent_bit_instance_members() {
    let sources = std::collections::BTreeMap::from([
        (
            "fluent_bit/outer.qv".into(),
            include_str!(
                "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/outer.qv"
            )
            .into(),
        ),
        (
            "librdkafka/outer.qv".into(),
            include_str!(
                "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/outer.qv"
            )
            .into(),
        ),
    ]);
    let loaded = qcore_contract::outer_contract::load_outer_contract_sources(&sources).unwrap();
    let scenario = |name: &str| ScenarioContract {
        name: name.into(),
        identity: loaded.identity(name).unwrap(),
        in_force: true,
        envelope: loaded.contracts[name].envelope.clone(),
        reach: all(),
    };
    let fluent_bit = scenario("flb_external_plugin");
    let rk = scenario("rk_plugin_library");
    for tag in ["input", "output", "filter", "processor"] {
        let fact = stable(&format!("flb_{tag}_instance.p"));
        assert_eq!(
            contribution(&fact, &fluent_bit).unwrap().verdict,
            Verdict::Pending
        );
        assert_eq!(contribution(&fact, &rk).unwrap().verdict, Verdict::Unknown);
        assert_eq!(
            runtime_verdict(
                Verdict::Safe,
                &BTreeSet::from([fact]),
                &[fluent_bit.clone(), rk.clone()]
            )
            .0,
            Verdict::Unknown
        );
    }
    assert_eq!(
        contribution(&stable("rd_kafka_broker_s.rkb_rk"), &rk)
            .unwrap()
            .verdict,
        Verdict::Pending
    );
}

#[test]
fn facts_are_read_from_the_dependency_record() {
    let null = json!({
        "contract_dependencies": ["stable:instance.p", "member:topic.rk", "inferred:f", "scope_fn"],
        "premises": [{"name": "rk_no_plugin_library"}, {"name": "asserted_allocation"}],
    });
    assert_eq!(
        null_facts(&null),
        BTreeSet::from([
            stable("instance.p"),
            Fact::Member {
                kind: "member",
                key: "topic.rk".into()
            },
            Fact::Premise {
                name: "rk_no_plugin_library".into()
            },
        ])
    );
    let dependencies = json!({
        "declarations": ["setter", "dumper"],
        "run_set_members": ["conf.log_cb"],
        "stated_members": ["op.cb"],
    });
    let premises = json!({
        "dumper": [{"name": "rk_no_plugin_library"}, {"name": "literal_argument"}],
    });
    assert_eq!(
        custody_facts(&dependencies, &premises),
        BTreeSet::from([
            Fact::RunSet {
                key: "conf.log_cb".into()
            },
            Fact::Member {
                kind: "stated",
                key: "op.cb".into()
            },
            Fact::Premise {
                name: "rk_no_plugin_library".into()
            },
        ])
    );
    assert!(null_facts(&json!({"contract_dependencies": ["f", "result:g"]})).is_empty());
}

#[test]
fn a_covered_fact_is_pending_and_an_uncovered_one_unknown() {
    let in_force = contract(true, all());
    let covered = contribution(&stable("instance.p"), &in_force).unwrap();
    assert_eq!(covered.verdict, Verdict::Pending);
    assert!(
        covered
            .reason
            .contains("rests on outer contract `app_plugin@abc`")
    );
    assert_eq!(
        qcore_schemas::reason::ReasonCategory::classify(&covered.reason).unwrap(),
        qcore_schemas::reason::ReasonCategory::OuterContract
    );
    let uncovered = contribution(&stable("instance.q"), &in_force).unwrap();
    assert_eq!(uncovered.verdict, Verdict::Unknown);
    assert!(uncovered.reason.contains("outer-uncovered"));
    // `#registered` covers a run set, never a stable member.
    let registered_run_set = Fact::RunSet {
        key: "collector.cb".into(),
    };
    assert_eq!(
        contribution(&registered_run_set, &in_force)
            .unwrap()
            .verdict,
        Verdict::Pending
    );
    assert_eq!(
        contribution(&stable("collector.cb"), &in_force)
            .unwrap()
            .verdict,
        Verdict::Unknown
    );
    let premise = Fact::Premise {
        name: "app_closed".into(),
    };
    assert_eq!(
        contribution(&premise, &in_force).unwrap().verdict,
        Verdict::Pending
    );
    // A contract not in force covers nothing.
    let held = contract(false, all());
    assert_eq!(
        contribution(&stable("instance.p"), &held).unwrap().verdict,
        Verdict::Unknown
    );
    // An unreachable record contributes nothing; a premise is always reached.
    let narrow = contract(true, Reach::Records(BTreeSet::from(["config".to_owned()])));
    assert!(contribution(&stable("instance.p"), &narrow).is_none());
    assert_eq!(
        contribution(&premise, &narrow).unwrap().verdict,
        Verdict::Pending
    );
}

#[test]
fn the_runtime_verdict_is_the_join_and_never_promotes() {
    let scenario = [contract(true, all())];
    let none = BTreeSet::new();
    for verdict in Verdict::ALL {
        // A row with no closed-world fact is unchanged.
        assert_eq!(runtime_verdict(verdict, &none, &scenario).0, verdict);
        for facts in [
            BTreeSet::from([stable("instance.p")]),
            BTreeSet::from([stable("instance.q")]),
            BTreeSet::from([stable("instance.p"), stable("instance.q")]),
        ] {
            let (runtime, reasons) = runtime_verdict(verdict, &facts, &scenario);
            assert!(runtime >= verdict, "{verdict:?} -> {runtime:?}");
            assert_eq!(reasons.len(), facts.len());
        }
    }
    assert_eq!(
        runtime_verdict(
            Verdict::Safe,
            &BTreeSet::from([stable("instance.p")]),
            &scenario
        )
        .0,
        Verdict::Pending
    );
    assert_eq!(
        runtime_verdict(
            Verdict::Safe,
            &BTreeSet::from([stable("instance.p"), stable("instance.q")]),
            &scenario
        )
        .0,
        Verdict::Unknown
    );
    assert_eq!(
        runtime_verdict(
            Verdict::Unsafe,
            &BTreeSet::from([stable("instance.p")]),
            &scenario
        )
        .0,
        Verdict::Unsafe
    );
}

static NEXT: AtomicUsize = AtomicUsize::new(0);

const OUTER: &str = r#"module contract;
extern fn load_plugin(path: ptr<mut, u8>, config: ptr<mut, u8>) -> i32
contract {
    requires outer_contract_v1;
    quant #outer [[contract]] app_plugin, "a loaded plugin stores into no instance's p";
    quant #outer where #config in #loaded [[runtime]] when #result == 0;
    quant #outer #loaded: instance.p |-> #kept;
};
"#;

const RESOURCE: &str = "module contract;
extern fn dispose(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires resource_revision(flexible);
    ensures normal resource_released(p, nonnull);
    ensures normal resource_return_none();
};
";

const SOURCE: &str = "struct plugin { int flags; };
struct instance { struct plugin *p; int id; };
struct unrelated { int x; };
struct config { struct instance *in; void *ctx; };

int load_plugin(char *path, struct config *config)
{
    return (path == 0 || config == 0) ? -1 : 0;
}

int plain(int x)
{
    return x + 1;
}
";

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-outer-projection-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("tree")).unwrap();
        fs::write(root.join("tree/app.c"), SOURCE).unwrap();
        let contracts = root.join("providers/quirin-provider-alpha/contracts/alpha");
        fs::create_dir_all(&contracts).unwrap();
        fs::write(contracts.join("outer.qv"), OUTER).unwrap();
        fs::write(contracts.join("heap.qv"), RESOURCE).unwrap();
        fs::write(
            root.join("providers/contract-load-path.json"),
            serde_json::to_string_pretty(&json!({"schema_version": 1, "crates": [
                {"name": "quirin-provider-alpha", "namespace": "alpha", "version": "0.1.0",
                 "contracts": "quirin-provider-alpha/contracts", "dependencies": [],
                 "modules": ["alpha/heap.qv", "alpha/outer.qv"], "published": {},
                 "resource_view": ["alpha/heap.qv"], "bounds_view": [],
                 "outer_view": ["alpha/outer.qv"]},
            ]}))
            .unwrap(),
        )
        .unwrap();
        Self(root)
    }

    fn quirin(&self, args: &[&str], projection: bool) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quirin-audit"));
        command
            .current_dir(&self.0)
            .args(args)
            .env_remove("QCORE_BOUNDS_PROPERTY")
            .env_remove("QCORE_NULL_PROPERTY");
        if projection {
            command.env("QCORE_OUTER_PROJECTION", self.0.join("out/outer.json"));
        } else {
            command.env_remove("QCORE_OUTER_PROJECTION");
        }
        command.output().unwrap()
    }

    /// Run the no-build route; the report's bytes.
    fn audit(&self, projection: bool) -> Vec<u8> {
        let _ = fs::remove_dir_all(self.0.join("out"));
        fs::create_dir_all(self.0.join("out")).unwrap();
        stdout(&self.quirin(
            &[
                "contracts",
                "--source-root",
                "tree",
                "app.c",
                "--load-path",
                "providers/contract-load-path.json",
                "--report",
                "out/report.json",
            ],
            projection,
        ));
        fs::read(self.0.join("out/report.json")).unwrap()
    }

    fn projection(&self) -> Value {
        serde_json::from_slice(&fs::read(self.0.join("out/outer.json")).unwrap()).unwrap()
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
    eprintln!("skipping Clang-dependent outer projection test");
    false
}

#[test]
fn the_static_report_is_identical_and_the_pin_and_reach_decide() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    let without = fixture.audit(false);
    assert!(!fixture.0.join("out/outer.json").exists());
    // No lock: the loader is held, so the contract covers nothing and reaches
    // every record.
    let with = fixture.audit(true);
    assert_eq!(without, with, "the static report moved");
    let projection = fixture.projection();
    assert_eq!(projection["rule"], "31");
    assert_eq!(projection["contracts"][0]["in_force"], false);
    assert_eq!(projection["loaders"][0]["pin_hold"]["reason"], "no-lock");
    assert_eq!(
        projection["scenarios"]["all"]["contracts"][0]["reach"]["kind"],
        "all-records"
    );
    // No row rests on a closed-world fact: every scenario equals the static one.
    assert_eq!(
        projection["scenarios"]["all"]["custody"],
        projection["static"]["custody"]
    );
    assert_eq!(projection["scenarios"]["all"]["moved"], json!([]));
    // Pinned: in force, and the reach is read from the loader's own unit,
    // where `void *ctx` reaches every record.
    stdout(&fixture.quirin(
        &[
            "pin",
            "--outer",
            "--source-root",
            "tree",
            "--load-path",
            "providers/contract-load-path.json",
        ],
        false,
    ));
    let pinned = fixture.audit(true);
    assert_eq!(without, pinned, "the static report moved");
    let projection = fixture.projection();
    assert_eq!(projection["contracts"][0]["in_force"], true);
    let reach = &projection["loaders"][0]["reach"];
    assert_eq!(reach["reach"]["kind"], "all-records", "{reach:#}");
    assert!(
        reach["reach"]["witness"]
            .as_str()
            .unwrap()
            .contains("void pointer"),
        "{reach:#}"
    );
    assert!(
        projection["contracts"][0]["identity"]
            .as_str()
            .unwrap()
            .starts_with("app_plugin@")
    );
}

#[test]
fn a_typed_graph_reaches_only_what_its_members_reach() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    let typed = SOURCE.replace(" void *ctx;", "");
    fs::write(fixture.0.join("tree/app.c"), typed).unwrap();
    stdout(&fixture.quirin(
        &[
            "pin",
            "--outer",
            "--source-root",
            "tree",
            "--load-path",
            "providers/contract-load-path.json",
        ],
        false,
    ));
    fixture.audit(true);
    let projection = fixture.projection();
    let reach = &projection["loaders"][0]["reach"]["reach"];
    assert_eq!(reach["kind"], "records", "{reach:#}");
    assert_eq!(reach["records"], json!(["config", "instance", "plugin"]));
}
