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

use super::*;
use serde_json::{Value, json};

const POLICY: &str = include_str!(
    "../../../../providers/quirin-provider-shared/contracts/library/descriptor_native_observation.qv"
);

fn operand(role: &str, kind: &str, id: i64, generation: u32) -> Value {
    json!({"role": role, "kind": kind, "id": id, "generation": generation})
}

fn row(call: &str, args: Value, result: i64, errno: i64, operands: Vec<Value>) -> Value {
    json!({"schema_version":1,"sequence":0,"pid":7,"thread":7,"call":call,
           "site":{"module":"target","offset":64},"args":args,"result":result,
           "errno":errno,"injected":false,"operands":operands})
}

fn start() -> Value {
    let mut start = row(
        "start",
        json!([]),
        0,
        0,
        vec![
            operand("inherited", "fd", 0, 1),
            operand("inherited", "fd", 1, 1),
            operand("inherited", "fd", 2, 1),
            json!({"role":"inherited","kind":"file","id":4096,"generation":1,"child":1}),
        ],
    );
    start["injections"] = json!([{"call":"socket","index":1,"error":24,"site_lo":0,"site_hi":0}]);
    start
}

fn exit(live: Vec<Value>) -> Value {
    let mut operands = vec![
        operand("live", "fd", 0, 1),
        operand("live", "fd", 1, 1),
        operand("live", "fd", 2, 1),
        operand("live", "file", 4096, 1),
    ];
    operands.extend(live);
    row("exit", json!([]), 0, 0, operands)
}

fn render(rows: Vec<Value>) -> String {
    rows.into_iter()
        .enumerate()
        .map(|(index, mut row)| {
            row["sequence"] = json!(index + 1);
            format!("{row}\n")
        })
        .collect()
}

fn check(rows: Vec<Value>) -> Result<DescriptorRunReport, String> {
    check_descriptor_observations(&render(rows), POLICY)
}

fn violations(report: &DescriptorRunReport, property: &str) -> usize {
    report.properties[property].violations
}

fn total_violations(report: &DescriptorRunReport) -> usize {
    report
        .properties
        .values()
        .map(|result| result.violations)
        .sum()
}

/// A lifecycle over every observed kind with no violation.
fn clean_run() -> Vec<Value> {
    let mut injected = row("socket", json!([2, 1, 0]), -1, 24, vec![]);
    injected["injected"] = json!(true);
    let mut fopen = row(
        "fopen",
        json!([]),
        8192,
        0,
        vec![
            operand("acquired", "file", 8192, 1),
            operand("derived", "fd", 5, 1),
        ],
    );
    fopen["path"] = json!("/etc/hostname");
    vec![
        start(),
        injected,
        row(
            "pipe",
            json!([]),
            0,
            0,
            vec![
                operand("acquired", "fd", 3, 1),
                operand("acquired", "fd", 4, 1),
            ],
        ),
        row("write", json!([4]), 0, 0, vec![operand("used", "fd", 4, 1)]),
        row(
            "close",
            json!([3]),
            0,
            0,
            vec![operand("released", "fd", 3, 1)],
        ),
        row(
            "close",
            json!([4]),
            0,
            0,
            vec![operand("released", "fd", 4, 1)],
        ),
        // Descriptor 3 is reused: a fresh generation.
        row(
            "socket",
            json!([2, 1, 0]),
            3,
            0,
            vec![operand("acquired", "fd", 3, 2)],
        ),
        row(
            "dup",
            json!([3]),
            6,
            0,
            vec![operand("used", "fd", 3, 2), operand("acquired", "fd", 6, 1)],
        ),
        row(
            "close",
            json!([6]),
            0,
            0,
            vec![operand("released", "fd", 6, 1)],
        ),
        fopen,
        row(
            "fileno",
            json!([]),
            5,
            0,
            vec![
                operand("used", "file", 8192, 1),
                operand("related", "fd", 5, 1),
            ],
        ),
        row(
            "fread",
            json!([]),
            0,
            0,
            vec![operand("used", "file", 8192, 1)],
        ),
        row(
            "fclose",
            json!([]),
            0,
            0,
            vec![
                operand("released", "file", 8192, 1),
                operand("released_child", "fd", 5, 1),
            ],
        ),
        row(
            "dlopen",
            json!([1]),
            12288,
            0,
            vec![operand("acquired", "dl_handle", 12288, 1)],
        ),
        row(
            "dlopen",
            json!([1]),
            12288,
            0,
            vec![operand("acquired", "dl_handle", 12288, 1)],
        ),
        row(
            "dlsym",
            json!([]),
            0,
            0,
            vec![operand("used", "dl_handle", 12288, 1)],
        ),
        row(
            "dlclose",
            json!([]),
            0,
            0,
            vec![operand("released", "dl_handle", 12288, 1)],
        ),
        row(
            "dlclose",
            json!([]),
            0,
            0,
            vec![operand("released", "dl_handle", 12288, 1)],
        ),
        row(
            "getaddrinfo",
            json!([2, 1]),
            0,
            0,
            vec![operand("acquired", "addrinfo", 16384, 1)],
        ),
        row(
            "freeaddrinfo",
            json!([]),
            0,
            0,
            vec![operand("released", "addrinfo", 16384, 1)],
        ),
        row("getaddrinfo", json!([2, 1]), -2, 0, vec![]),
        row(
            "mmap",
            json!([4096, 3, 1, 3, 0]),
            1_048_576,
            0,
            vec![
                operand("used", "fd", 3, 2),
                json!({"role":"acquired","kind":"mapping","id":1_048_576,"generation":1,"extent":4096}),
            ],
        ),
        row(
            "msync",
            json!([]),
            0,
            0,
            vec![operand("used", "mapping", 1_048_576 + 128, 0)],
        ),
        {
            let mut munmap = row(
                "munmap",
                json!([4096]),
                0,
                0,
                vec![
                    json!({"role":"released","kind":"mapping","id":1_048_576,"generation":1,"extent":4096}),
                ],
            );
            munmap["args"] = json!([4096]);
            munmap
        },
        row(
            "close",
            json!([3]),
            0,
            0,
            vec![operand("released", "fd", 3, 2)],
        ),
        exit(vec![]),
    ]
}

#[test]
fn clean_run_holds_every_property_over_every_kind() {
    let report = check(clean_run()).unwrap();
    assert_eq!(total_violations(&report), 0, "{:#?}", report.properties);
    assert_eq!(report.reused_identities, 1);
    assert_eq!(report.injected_failures, 1);
    assert_eq!(report.injections_configured, 1);
    assert_eq!(report.constructor_failures, 2);
    assert_eq!(report.releases["dl_handle_reference"], 1);
    assert_eq!(report.releases["dl_handle"], 1);
    assert_eq!(report.acquisitions["mapping"], 1);
    assert!(!report.static_certificate);
    // Only the inherited identities are live at exit.
    assert!(report.live_at_exit.iter().all(|live| live.inherited));
    for (property, _) in PROPERTIES {
        if *property != "counted_reference" && *property != "mapping_extent" {
            assert!(
                report.properties[*property].checks > 0,
                "{property} unexercised"
            );
        }
    }
    assert!(report.properties["counted_reference"].checks == 1);
    assert!(report.properties["mapping_extent"].checks == 1);
}

#[test]
fn double_close_violates_release_at_most_once() {
    let rows = vec![
        start(),
        row(
            "socket",
            json!([2, 1, 0]),
            3,
            0,
            vec![operand("acquired", "fd", 3, 1)],
        ),
        row(
            "close",
            json!([3]),
            0,
            0,
            vec![operand("released", "fd", 3, 1)],
        ),
        row(
            "close",
            json!([3]),
            -1,
            9,
            vec![operand("released", "fd", 3, 1)],
        ),
        exit(vec![]),
    ];
    let report = check(rows).unwrap();
    assert_eq!(violations(&report, "release_at_most_once"), 1);
    assert_eq!(total_violations(&report), 1);
}

#[test]
fn use_after_close_is_recorded() {
    let rows = vec![
        start(),
        row(
            "socket",
            json!([2, 1, 0]),
            3,
            0,
            vec![operand("acquired", "fd", 3, 1)],
        ),
        row(
            "close",
            json!([3]),
            0,
            0,
            vec![operand("released", "fd", 3, 1)],
        ),
        row("write", json!([3]), 0, 0, vec![operand("used", "fd", 3, 1)]),
        exit(vec![]),
    ];
    let report = check(rows).unwrap();
    assert_eq!(violations(&report, "no_use_after_release"), 1);
}

#[test]
fn closing_a_stream_descriptor_directly_violates_custody_then_release_once() {
    let rows = vec![
        start(),
        row(
            "fopen",
            json!([]),
            8192,
            0,
            vec![
                operand("acquired", "file", 8192, 1),
                operand("derived", "fd", 5, 1),
            ],
        ),
        row(
            "close",
            json!([5]),
            0,
            0,
            vec![operand("released", "fd", 5, 1)],
        ),
        row(
            "fclose",
            json!([]),
            -1,
            9,
            vec![
                operand("released", "file", 8192, 1),
                operand("released_child", "fd", 5, 1),
            ],
        ),
        exit(vec![]),
    ];
    let report = check(rows).unwrap();
    assert_eq!(violations(&report, "release_matches_kind"), 1);
    assert_eq!(violations(&report, "release_at_most_once"), 1);
}

#[test]
fn releasing_a_pointer_as_another_kind_is_a_kind_mismatch() {
    let rows = vec![
        start(),
        row(
            "opendir",
            json!([]),
            8192,
            0,
            vec![
                operand("acquired", "dir", 8192, 1),
                operand("derived", "fd", 5, 1),
            ],
        ),
        row(
            "fclose",
            json!([]),
            0,
            0,
            vec![
                operand("released", "file", 8192, 1),
                operand("released_child", "fd", 5, 1),
            ],
        ),
        exit(vec![]),
    ];
    let report = check(rows).unwrap();
    assert!(violations(&report, "release_matches_kind") >= 1);
}

#[test]
fn sentinel_release_is_an_invalid_operand_not_a_double_release() {
    let rows = vec![
        start(),
        row(
            "close",
            json!([-1]),
            -1,
            9,
            vec![operand("released", "fd", -1, 0)],
        ),
        exit(vec![]),
    ];
    let report = check(rows).unwrap();
    assert_eq!(violations(&report, "operand_identity_valid"), 1);
    assert_eq!(violations(&report, "release_at_most_once"), 0);
}

#[test]
fn fileno_must_return_the_stream_descriptor() {
    let rows = vec![
        start(),
        row(
            "fopen",
            json!([]),
            8192,
            0,
            vec![
                operand("acquired", "file", 8192, 1),
                operand("derived", "fd", 5, 1),
            ],
        ),
        row(
            "fileno",
            json!([]),
            1,
            0,
            vec![
                operand("used", "file", 8192, 1),
                operand("related", "fd", 1, 1),
            ],
        ),
        row(
            "fclose",
            json!([]),
            0,
            0,
            vec![
                operand("released", "file", 8192, 1),
                operand("released_child", "fd", 5, 1),
            ],
        ),
        exit(vec![]),
    ];
    let report = check(rows).unwrap();
    assert_eq!(violations(&report, "relation_holds"), 1);
}

#[test]
fn fdopen_adopts_and_fclose_releases_the_adopted_descriptor() {
    let rows = vec![
        start(),
        row(
            "open",
            json!([-100, 0, 0]),
            5,
            0,
            vec![operand("acquired", "fd", 5, 1)],
        ),
        row(
            "fdopen",
            json!([]),
            8192,
            0,
            vec![
                operand("used", "fd", 5, 1),
                operand("acquired", "file", 8192, 1),
                operand("adopted", "fd", 5, 1),
            ],
        ),
        row(
            "fclose",
            json!([]),
            0,
            0,
            vec![
                operand("released", "file", 8192, 1),
                operand("released_child", "fd", 5, 1),
            ],
        ),
        exit(vec![]),
    ];
    let report = check(rows).unwrap();
    assert_eq!(total_violations(&report), 0, "{:#?}", report.properties);
}

#[test]
fn partial_unmap_violates_the_mapping_extent() {
    let rows = vec![
        start(),
        row(
            "mmap",
            json!([8192, 3, 34, -1, 0]),
            1_048_576,
            0,
            vec![
                json!({"role":"acquired","kind":"mapping","id":1_048_576,"generation":1,"extent":8192}),
            ],
        ),
        row(
            "munmap",
            json!([4096]),
            0,
            0,
            vec![
                json!({"role":"released","kind":"mapping","id":1_048_576,"generation":1,"extent":4096}),
            ],
        ),
        exit(vec![]),
    ];
    let report = check(rows).unwrap();
    assert_eq!(violations(&report, "mapping_extent"), 1);
}

#[test]
fn live_generations_at_exit_are_reported_with_their_origin() {
    let mut open = row(
        "open",
        json!([-100, 0, 0]),
        3,
        0,
        vec![operand("acquired", "fd", 3, 1)],
    );
    open["path"] = json!("/etc/machine-id");
    let rows = vec![start(), open, exit(vec![operand("live", "fd", 3, 1)])];
    let report = check(rows).unwrap();
    assert_eq!(total_violations(&report), 0);
    let leaked: Vec<_> = report
        .live_at_exit
        .iter()
        .filter(|live| !live.inherited)
        .collect();
    assert_eq!(leaked.len(), 1);
    assert_eq!(leaked[0].origin_call, "open");
    assert_eq!(leaked[0].origin_path.as_deref(), Some("/etc/machine-id"));
    // An observer that omits the live descriptor disagrees with the replay.
    let mut open = row(
        "open",
        json!([-100, 0, 0]),
        3,
        0,
        vec![operand("acquired", "fd", 3, 1)],
    );
    open["path"] = json!("/etc/machine-id");
    let report = check(vec![start(), open, exit(vec![])]).unwrap();
    assert_eq!(violations(&report, "leak_set_well_defined"), 1);
}

#[test]
fn acquiring_a_live_identity_records_an_unobserved_release() {
    let rows = vec![
        start(),
        row(
            "socket",
            json!([2, 1, 0]),
            3,
            0,
            vec![operand("acquired", "fd", 3, 1)],
        ),
        row(
            "socket",
            json!([2, 1, 0]),
            3,
            0,
            vec![operand("acquired", "fd", 3, 2)],
        ),
        row(
            "close",
            json!([3]),
            0,
            0,
            vec![operand("released", "fd", 3, 2)],
        ),
        exit(vec![]),
    ];
    let report = check(rows).unwrap();
    assert_eq!(violations(&report, "generation_fresh"), 1);
}

#[test]
fn malformed_or_inconsistent_records_are_not_evidence() {
    // Observer generation stamp disagrees with the replay.
    let rows = vec![
        start(),
        row(
            "socket",
            json!([2, 1, 0]),
            3,
            0,
            vec![operand("acquired", "fd", 3, 7)],
        ),
        exit(vec![]),
    ];
    assert!(check(rows).unwrap_err().contains("disagrees"));
    // Missing exit record.
    assert!(check(vec![start()]).unwrap_err().contains("missing exit"));
    // Foreign process.
    let mut foreign = row("write", json!([1]), 0, 0, vec![operand("used", "fd", 1, 1)]);
    foreign["pid"] = json!(8);
    assert!(
        check(vec![start(), foreign, exit(vec![])])
            .unwrap_err()
            .contains("different process")
    );
    // Noncontiguous sequence.
    let mut text = render(vec![start(), exit(vec![])]);
    text = text.replace("\"sequence\":2", "\"sequence\":3");
    assert!(
        check_descriptor_observations(&text, POLICY)
            .unwrap_err()
            .contains("noncontiguous")
    );
    // Unknown call and unknown field.
    assert!(
        check(vec![
            start(),
            row("frobnicate", json!([]), 0, 0, vec![]),
            exit(vec![])
        ])
        .is_err()
    );
    let mut extra = row("write", json!([1]), 0, 0, vec![operand("used", "fd", 1, 1)]);
    extra["surprise"] = json!(1);
    assert!(check(vec![start(), extra, exit(vec![])]).is_err());
    // An injected failure cannot report success.
    let mut injected = row(
        "socket",
        json!([2, 1, 0]),
        3,
        0,
        vec![operand("acquired", "fd", 3, 1)],
    );
    injected["injected"] = json!(true);
    assert!(check(vec![start(), injected, exit(vec![])]).is_err());
    // Failure must return the sentinel and acquire nothing.
    let report = check(vec![
        start(),
        row("socket", json!([2, 1, 0]), -2, 0, vec![]),
        exit(vec![]),
    ])
    .unwrap();
    assert_eq!(violations(&report, "sentinel_validity"), 1);
}

#[test]
fn policy_text_drives_the_result() {
    let rows = vec![
        start(),
        row(
            "socket",
            json!([2, 1, 0]),
            3,
            0,
            vec![operand("acquired", "fd", 3, 1)],
        ),
        row(
            "close",
            json!([3]),
            0,
            0,
            vec![operand("released", "fd", 3, 1)],
        ),
        row(
            "close",
            json!([3]),
            -1,
            9,
            vec![operand("released", "fd", 3, 1)],
        ),
        exit(vec![]),
    ];
    let text = render(rows);
    let baseline = check_descriptor_observations(&text, POLICY).unwrap();
    assert_eq!(violations(&baseline, "release_at_most_once"), 1);
    // Weakening the clause removes the violation: Rust holds no copy of it.
    let weakened = POLICY.replace(
        "ensures normal not released_before;",
        "ensures normal released_before or 1;",
    );
    assert_ne!(weakened, POLICY);
    let report = check_descriptor_observations(&text, &weakened).unwrap();
    assert_eq!(violations(&report, "release_at_most_once"), 0);
    // A clause naming a fact the interpreter does not supply fails closed.
    let unknown = POLICY.replace(
        "ensures normal not released_before;",
        "ensures normal not unsupplied_fact;",
    );
    assert!(check_descriptor_observations(&text, &unknown).is_err());
    // A policy missing a property function fails closed.
    let missing = POLICY.replace("descriptor_release_at_most_once", "descriptor_renamed");
    assert!(check_descriptor_observations(&text, &missing).is_err());
}

/// Pinned real executions from the W43-DESCRIPTOR-NATIVE-CHECK measurement
/// (`docs/reports/w43-descriptor-native-check/records/`): the replay must keep
/// reproducing their recorded results under the current policy.
#[test]
fn pinned_fluent_bit_engine_run_reproduces_its_double_closes() {
    let records = include_str!("../fixtures/descriptor/engine_dummy_null.jsonl");
    let report = check_descriptor_observations(records, POLICY).unwrap();
    assert_eq!(report.records, 398);
    // flb_config_exit closes ch_event.fd and ch_manager[0] (one generation),
    // and flb_sched_destroy closes each timer fd before flb_sched_timer_destroy
    // closes it again: five second closes, each answered EBADF.
    assert_eq!(violations(&report, "release_at_most_once"), 5);
    for example in &report.properties["release_at_most_once"].examples {
        assert_eq!(example.call, "close");
        assert!(example.detail.contains("errno 9"), "{}", example.detail);
    }
    assert_eq!(total_violations(&report), 5);
    let leaked: Vec<_> = report
        .live_at_exit
        .iter()
        .filter(|live| !live.inherited)
        .collect();
    assert_eq!(leaked.len(), 1);
    assert_eq!(leaked[0].kind, Kind::Mapping);
}

#[test]
fn pinned_libc_probe_run_holds_every_property() {
    let records = include_str!("../fixtures/descriptor/probe_libc.jsonl");
    let report = check_descriptor_observations(records, POLICY).unwrap();
    assert_eq!(total_violations(&report), 0, "{:#?}", report.properties);
    assert!(report.properties["relation_holds"].checks >= 7);
    assert_eq!(report.properties["counted_reference"].checks, 1);
    assert!(report.properties["mapping_extent"].checks >= 3);
    assert!(report.live_at_exit.iter().all(|live| live.inherited));
}

#[test]
fn pinned_monkey_bind_failure_run_leaves_its_socket_live_at_exit() {
    let records = include_str!("../fixtures/descriptor/monkey_bind_in_use.jsonl");
    let report = check_descriptor_observations(records, POLICY).unwrap();
    assert_eq!(total_violations(&report), 0);
    let leaked: Vec<_> = report
        .live_at_exit
        .iter()
        .filter(|live| !live.inherited)
        .collect();
    assert_eq!(leaked.len(), 1);
    assert_eq!(
        (leaked[0].kind, leaked[0].origin_call.as_str()),
        (Kind::Fd, "socket")
    );
}
