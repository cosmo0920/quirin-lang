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

//! Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS (gateway rule 12r): the shipped
//! chunkio bounds module (`providers/qcore-provider-chunkio/contracts/chunkio/
//! meta_len_bounds.qv`) decides the two header accessors' bodies as the pinned
//! sources define them (`lib/chunkio/include/chunkio/cio_file_st.h:76-86`,
//! byte-identical at `66910c10` and `046accf6`), every caller must cover
//! indices 22 and 23, and a pin hold (an audit route's `contract-pin-mismatch`)
//! holds them like a review marker: Pending at most, never accusing.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use std::collections::BTreeMap;

use clang_gap::*;
use qcore_c_gateway::index_extent::{BoundsFunctionRow, analyze_bounds_c11, row_verdict};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::bounds_contract::{BoundsContracts, load_bounds_contract_sources};
use qcore_core::index_extent::BoundsVerdict;

use BoundsVerdict::{Pending, Safe, Unknown, Unsafe};

/// The two accessors as the pinned header defines them, and callers.
const SOURCE: &str = "typedef unsigned short uint16_t;
typedef unsigned char uint8_t;
static inline uint16_t cio_file_st_get_meta_len(char *map)
{
    return (uint16_t) ((uint8_t) map[22] << 8) | (uint8_t) map[23];
}
static inline void cio_file_st_set_meta_len(char *map, uint16_t len)
{
    map[22] = (uint8_t) (len >> 8);
    map[23] = (uint8_t) (len & 0xFF);
}
struct cio_file { char *map; };
uint16_t full_header(void)
{
    char header[24];
    cio_file_st_set_meta_len(header, 7);
    return cio_file_st_get_meta_len(header);
}
uint16_t one_short(void)
{
    char header[23];
    return cio_file_st_get_meta_len(header);
}
uint16_t through_a_member(struct cio_file *cf)
{
    return cio_file_st_get_meta_len(cf->map);
}
";

const FUNCTIONS: [&str; 5] = [
    "cio_file_st_get_meta_len",
    "cio_file_st_set_meta_len",
    "full_header",
    "one_short",
    "through_a_member",
];

fn shipped() -> BoundsContracts {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../providers/qcore-provider-chunkio/contracts/chunkio/meta_len_bounds.qv");
    let text = std::fs::read_to_string(&path).unwrap();
    load_bounds_contract_sources(&BTreeMap::from([(
        "qcore-provider-chunkio:chunkio/meta_len_bounds.qv".to_owned(),
        text,
    )]))
    .unwrap()
}

fn rows(bounds: &BoundsContracts) -> BTreeMap<String, BoundsFunctionRow> {
    let text = format!("{DECLARATIONS}\n{SOURCE}\n");
    analyze_bounds_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        &FUNCTIONS.map(str::to_owned),
        bounds,
    )
    .unwrap()
    .functions
    .into_iter()
    .map(|row| (row.function.clone(), row))
    .collect()
}

#[test]
fn the_shipped_module_states_the_exact_indices_and_closed_frames() {
    let contracts = shipped();
    assert_eq!(contracts.functions.len(), 2);
    for (name, frame) in [
        ("cio_file_st_get_meta_len", "bounds_frame_reads(map)"),
        ("cio_file_st_set_meta_len", "bounds_frame_writes(map)"),
    ] {
        let contract = &contracts.functions[name];
        assert!(
            !contract.held(),
            "written after reading the source; no marker"
        );
        let texts = contract
            .clauses
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>();
        assert!(
            texts.contains(&"quant checks map, Idx #i: 22 <= #i && #i < 24"),
            "{texts:?}"
        );
        assert!(texts.contains(&frame), "{texts:?}");
        assert!(contract.frame().is_some());
    }
}

#[test]
fn in_force_the_bodies_are_safe_and_every_caller_covers_indices_22_and_23() {
    if !clang_available() {
        return;
    }
    let rows = rows(&shipped());
    let verdict = |f: &str| row_verdict(&rows[f]);
    assert_eq!(
        verdict("cio_file_st_get_meta_len"),
        Safe,
        "{:#?}",
        rows["cio_file_st_get_meta_len"]
    );
    assert_eq!(
        verdict("cio_file_st_set_meta_len"),
        Safe,
        "{:#?}",
        rows["cio_file_st_set_meta_len"]
    );
    assert_eq!(verdict("full_header"), Safe, "{:#?}", rows["full_header"]);
    // A 23-byte header misses index 23 on a definitely reached call (a
    // fixture violation, provisional by rule 11s).
    assert_eq!(verdict("one_short"), Unsafe, "{:#?}", rows["one_short"]);
    // The corpus shape: a member with no extent (CG16-BOUNDS-POINTER-EXTENTS).
    assert_eq!(
        verdict("through_a_member"),
        Unknown,
        "{:#?}",
        rows["through_a_member"]
    );
}

#[test]
fn a_pin_hold_makes_them_pending_and_never_accuses() {
    if !clang_available() {
        return;
    }
    let mut contracts = shipped();
    for name in ["cio_file_st_get_meta_len", "cio_file_st_set_meta_len"] {
        contracts
            .hold_for_pin_mismatch(name, "source-changed", "the audited header differs")
            .unwrap();
    }
    let rows = rows(&contracts);
    let verdict = |f: &str| row_verdict(&rows[f]);
    assert_eq!(
        verdict("cio_file_st_get_meta_len"),
        Pending,
        "{:#?}",
        rows["cio_file_st_get_meta_len"]
    );
    assert_eq!(verdict("cio_file_st_set_meta_len"), Pending);
    assert_eq!(
        verdict("full_header"),
        Pending,
        "{:#?}",
        rows["full_header"]
    );
    assert_eq!(verdict("one_short"), Unknown, "{:#?}", rows["one_short"]);
    assert_eq!(verdict("through_a_member"), Unknown);
    assert!(
        rows["cio_file_st_get_meta_len"]
            .reasons
            .iter()
            .any(|r| r.starts_with(
                "contract-pin-mismatch: cio_file_st_get_meta_len: contract — pin source-changed"
            )),
        "{:#?}",
        rows["cio_file_st_get_meta_len"]
    );
}
