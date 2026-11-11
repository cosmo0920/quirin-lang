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

//! Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (gateway rules 14f–14g):
//! phase one's check of every declared body a unit defines is published as
//! evidence for the declaration, never as a row. The bodies are chunkio's
//! `cio_file_st.h` accessors as the pinned header defines them (byte-identical
//! at `66910c10` and `046accf6`) under the shipped modules, plus controls: a
//! refuting body, a held declaration, a body that copies its parameter, a
//! write through a read frame and a hand-off to a stronger frame.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use std::collections::BTreeMap;

use clang_gap::*;
use qcore_c_gateway::index_extent::{
    BoundsUnitReport, DeclarationBodyEvidence, analyze_bounds_c11,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::bounds_contract::{BoundsContracts, load_bounds_contract_sources};

/// The accessors as `lib/chunkio/include/chunkio/cio_file_st.h` defines them
/// (lines 69-126), with one main-source caller.
const HEADER: &str = "typedef unsigned short uint16_t;
typedef unsigned char uint8_t;
typedef unsigned int uint32_t;
typedef long ssize_t;
static inline char *cio_file_st_get_hash(char *map)
{
    return map + 2;
}
static inline uint16_t cio_file_st_get_meta_len(char *map)
{
    return (uint16_t) ((uint8_t) map[22] << 8) | (uint8_t) map[23];
}
static inline void cio_file_st_set_meta_len(char *map, uint16_t len)
{
    map[22] = (uint8_t) (len >> 8);
    map[23] = (uint8_t) (len & 0xFF);
}
static inline char *cio_file_st_get_meta(char *map)
{
    return map + 24;
}
static inline ssize_t cio_file_st_infer_content_len(char *map, size_t size)
{
    size_t content_length;

    content_length  = size;
    content_length -= 24;
    content_length -= cio_file_st_get_meta_len(map);

    return content_length;
}
static inline void cio_file_st_set_content_len(char *map, uint32_t len)
{
    uint8_t *content_length_buffer;

    content_length_buffer = (uint8_t *) &map[10];

    content_length_buffer[0] = (uint8_t) ((len & 0xFF000000) >> 24);
    content_length_buffer[1] = (uint8_t) ((len & 0x00FF0000) >> 16);
    content_length_buffer[2] = (uint8_t) ((len & 0x0000FF00) >>  8);
    content_length_buffer[3] = (uint8_t) ((len & 0x000000FF) >>  0);
}
struct cio_file { char *map; };
uint16_t through_a_member(struct cio_file *cf)
{
    char *hash = cio_file_st_get_hash(cf->map);
    (void) hash;
    return cio_file_st_get_meta_len(cf->map);
}
uint16_t full_header(void)
{
    char header[24];
    cio_file_st_set_meta_len(header, 7);
    return cio_file_st_get_meta_len(header);
}
";

fn module(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../providers/qcore-provider-chunkio/contracts/chunkio")
        .join(name);
    std::fs::read_to_string(&path).unwrap()
}

fn shipped() -> BoundsContracts {
    load_bounds_contract_sources(&BTreeMap::from([
        (
            "qcore-provider-chunkio:chunkio/meta_len_bounds.qv".to_owned(),
            module("meta_len_bounds.qv"),
        ),
        (
            "qcore-provider-chunkio:chunkio/file_st_bounds.qv".to_owned(),
            module("file_st_bounds.qv"),
        ),
    ]))
    .unwrap()
}

fn report(source: &str, functions: &[&str], bounds: &BoundsContracts) -> BoundsUnitReport {
    let text = format!("{DECLARATIONS}\n{source}\n");
    analyze_bounds_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        &functions
            .iter()
            .map(|f| (*f).to_owned())
            .collect::<Vec<_>>(),
        bounds,
    )
    .unwrap()
}

fn evidence(report: &BoundsUnitReport) -> BTreeMap<String, DeclarationBodyEvidence> {
    report
        .declaration_bodies
        .iter()
        .map(|e| (e.function.clone(), e.clone()))
        .collect()
}

#[test]
fn header_bodies_are_evidence_not_rows() {
    if !clang_available() {
        return;
    }
    let report = report(HEADER, &["through_a_member", "full_header"], &shipped());
    // Only the selected main-source functions have rows.
    let rows = report
        .functions
        .iter()
        .map(|r| r.function.as_str())
        .collect::<Vec<_>>();
    assert_eq!(rows, ["through_a_member", "full_header"]);
    let evidence = evidence(&report);
    assert_eq!(
        evidence.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "cio_file_st_get_hash",
            "cio_file_st_get_meta_len",
            "cio_file_st_infer_content_len",
            "cio_file_st_set_content_len",
            "cio_file_st_set_meta_len",
        ],
        "{evidence:#?}"
    );
    // set_content_len: its accesses through the byte view of `&map[10]` are
    // decided inside `[10, 14)` (rule 14g); the frame walk cannot follow the
    // view, so the write frame stays uncovered.
    let content = &evidence["cio_file_st_set_content_len"];
    assert_eq!(content.outcome, "validated", "{content:#?}");
    assert_eq!(content.body_verdict, "Safe", "{content:#?}");
    assert_eq!(
        content.covered,
        BTreeMap::from([(
            "quant checks map, Idx #i: 10 <= #i && #i < 14".to_owned(),
            "bounds-body"
        )]),
        "{content:#?}"
    );
    for name in [
        "cio_file_st_get_meta_len",
        "cio_file_st_set_meta_len",
        "cio_file_st_infer_content_len",
    ] {
        let e = &evidence[name];
        assert_eq!(e.outcome, "validated", "{e:#?}");
        assert_eq!(e.body_verdict, "Safe", "{e:#?}");
        assert_eq!(e.covered.len(), e.clauses.len(), "{e:#?}");
    }
    // get_hash: the frame is confirmed; the relation's length is a fact about
    // the caller's object, which no body establishes; the body's own
    // formation `map + 2` has no extent.
    let hash = &evidence["cio_file_st_get_hash"];
    assert_eq!(hash.outcome, "validated", "{hash:#?}");
    assert_eq!(
        hash.covered,
        BTreeMap::from([("bounds_frame_reads(map)".to_owned(), "bounds-frame")]),
        "{hash:#?}"
    );
    // Rows name the declarations they could have consulted.
    let member = report.row("through_a_member").unwrap();
    assert_eq!(
        member.contract_dependencies,
        ["cio_file_st_get_hash", "cio_file_st_get_meta_len"]
    );
}

#[test]
fn a_held_declaration_keeps_no_evidence() {
    if !clang_available() {
        return;
    }
    let mut contracts = shipped();
    contracts
        .hold_for_pin_mismatch("cio_file_st_get_meta_len", "source-changed", "changed")
        .unwrap();
    let evidence = evidence(&report(HEADER, &["full_header"], &contracts));
    let e = &evidence["cio_file_st_get_meta_len"];
    assert_eq!(e.outcome, "held", "{e:#?}");
    assert!(e.covered.is_empty());
}

const CONTROLS: &str = "module contract;
extern fn refutes(map: ptr<mut, u8>) -> u8
contract {
    requires bounds_contract_v1;
    quant checks map, Idx #i: 0 <= #i && #i < 2;
    ensures normal bounds_frame_reads(map);
};
extern fn copies(map: ptr<mut, u8>) -> u8
contract {
    requires bounds_contract_v1;
    quant checks map, Idx #i: 0 <= #i && #i < 2;
    ensures normal bounds_frame_reads(map);
};
extern fn writes_read(map: ptr<mut, u8>) -> unit
contract {
    requires bounds_contract_v1;
    quant checks map, Idx #i: 0 <= #i && #i < 2;
    ensures normal bounds_frame_reads(map);
};
extern fn hands_on(map: ptr<mut, u8>) -> unit
contract {
    requires bounds_contract_v1;
    quant checks map, Idx #i: 0 <= #i && #i < 2;
    ensures normal bounds_frame_reads(map);
};
extern fn writer(map: ptr<mut, u8>) -> unit
contract {
    requires bounds_contract_v1;
    quant checks map, Idx #i: 0 <= #i && #i < 2;
    ensures normal bounds_frame_writes(map);
};
";

const CONTROL_SOURCE: &str = "
static inline unsigned char refutes(char *map) { return (unsigned char) map[5]; }
static inline unsigned char copies(char *map) { char *q = map; return (unsigned char) q[1]; }
static inline void writes_read(char *map) { map[1] = 0; }
static inline void writer(char *map) { map[0] = 1; map[1] = 2; }
static inline void hands_on(char *map) { writer(map); }
";

#[test]
fn controls_refute_or_leave_clauses_uncovered() {
    if !clang_available() {
        return;
    }
    let contracts = load_bounds_contract_sources(&BTreeMap::from([(
        "fixture:controls.qv".to_owned(),
        CONTROLS.to_owned(),
    )]))
    .unwrap();
    let evidence = evidence(&report(CONTROL_SOURCE, &[], &contracts));
    // An access outside the check refutes: a failure, nothing covered.
    assert_eq!(evidence["refutes"].outcome, "failed", "{evidence:#?}");
    assert!(evidence["refutes"].covered.is_empty());
    // A copy: the frame is not confirmed (the walk cannot follow it).
    assert!(
        !evidence["copies"]
            .covered
            .contains_key("bounds_frame_reads(map)"),
        "{evidence:#?}"
    );
    // A write through a parameter declared read: refuted by rule 11e(4).
    assert_eq!(evidence["writes_read"].outcome, "failed", "{evidence:#?}");
    // A hand-off to a write position under a read frame: refuted.
    assert_eq!(evidence["hands_on"].outcome, "failed", "{evidence:#?}");
    // The write frame itself is confirmed.
    assert_eq!(
        evidence["writer"].covered.get("bounds_frame_writes(map)"),
        Some(&"bounds-frame"),
        "{evidence:#?}"
    );
}

const VIEWS: &str = "module contract;
extern fn view_outside(map: ptr<mut, u8>) -> unit
contract {
    requires bounds_contract_v1;
    quant checks map, Idx #i: 10 <= #i && #i < 14;
    ensures normal bounds_frame_writes(map);
};
extern fn wide_view(map: ptr<mut, u8>) -> unit
contract {
    requires bounds_contract_v1;
    quant checks map, Idx #i: 0 <= #i && #i < 8;
    ensures normal bounds_frame_writes(map);
};
";

const VIEW_SOURCE: &str = "typedef unsigned char uint8_t;
typedef unsigned int uint32_t;
static inline void view_outside(char *map)
{
    uint8_t *view = (uint8_t *) &map[10];
    view[4] = 0;
}
static inline void wide_view(char *map)
{
    uint32_t *view = (uint32_t *) &map[0];
    view[1] = 0;
}
unsigned char local_view(void)
{
    char buffer[4];
    unsigned char *view = (unsigned char *) &buffer[0];
    buffer[0] = 1;
    return view[0];
}
";

/// Rule 14g's byte view is narrow: an index through it outside the check
/// refutes like any other access; a wider view and a byte view of an object
/// that is no quant object keep their refusal.
#[test]
fn a_byte_view_reads_only_byte_types_of_quant_objects() {
    if !clang_available() {
        return;
    }
    let contracts = load_bounds_contract_sources(&BTreeMap::from([(
        "fixture:views.qv".to_owned(),
        VIEWS.to_owned(),
    )]))
    .unwrap();
    let report = report(VIEW_SOURCE, &["local_view"], &contracts);
    let evidence = evidence(&report);
    // view[4] is map[14], outside [10, 14): the body refutes its check.
    assert_eq!(evidence["view_outside"].outcome, "failed", "{evidence:#?}");
    // A four-byte view is not a byte view: no fact, nothing decided inside.
    assert_eq!(
        evidence["wide_view"].body_verdict, "Unknown",
        "{evidence:#?}"
    );
    assert!(evidence["wide_view"].covered.is_empty(), "{evidence:#?}");
    // A byte view of a local array (no quant object) keeps the refusal.
    let local = report.row("local_view").unwrap();
    assert_eq!(local.verdict, "Unknown", "{local:#?}");
    assert!(
        local
            .reasons
            .iter()
            .any(|r| r.contains("bounds-missing-extent")),
        "{local:#?}"
    );
}
