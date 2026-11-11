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

//! Card CG18-FOOTPRINT-CONNECTION: unit tests of the gateway's connection to
//! the core footprint model.

use super::*;
use qcore_core::disjoint_footprint::FootprintDiagnostic;

/// Clang 21's `-fdump-record-layouts-complete` of
/// `struct I { void *p; int x; }; struct S { int a; int b; unsigned bf : 3;
/// unsigned bg : 2; char c; unsigned bh : 1; void *p; struct I in; int arr[4];
/// void (*cb)(void); };` on x86-64.
const DUMP: &str = "
*** Dumping AST Record Layout
         0 | struct I
         0 |   void * p
         8 |   int x
           | [sizeof=16, align=8]

*** Dumping AST Record Layout
         0 | struct S
         0 |   int a
         4 |   int b
     8:0-2 |   unsigned int bf
     8:3-4 |   unsigned int bg
         9 |   char c
    10:0-0 |   unsigned int bh
        16 |   void * p
        24 |   struct I in
        24 |     void * p
        32 |     int x
        40 |   int[4] arr
        56 |   void (*)(void) cb
           | [sizeof=64, align=8]
";

#[test]
fn the_layout_reader_keeps_direct_members_only() {
    let layout = parse_member_layout(DUMP, "S").expect("S");
    assert_eq!(layout.size, 64);
    let names = layout
        .members
        .iter()
        .map(|(name, byte, bits)| (name.as_str(), *byte, *bits))
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            ("a", 0, None),
            ("b", 4, None),
            ("bf", 8, Some((0, 2))),
            ("bg", 8, Some((3, 4))),
            ("c", 9, None),
            ("bh", 10, Some((0, 0))),
            ("p", 16, None),
            ("in", 24, None),
            ("arr", 40, None),
            ("cb", 56, None),
        ]
    );
    assert_eq!(parse_member_layout(DUMP, "I").expect("I").members.len(), 2);
    assert_eq!(parse_member_layout(DUMP, "T"), None);
    // Two dumps of one name are ambiguous.
    assert_eq!(parse_member_layout(&format!("{DUMP}{DUMP}"), "S"), None);
}

#[test]
fn the_core_model_decides_member_pairs() {
    let layout = parse_member_layout(DUMP, "S").expect("S");
    for (left, right) in [
        ("a", "b"),
        ("b", "bf"),
        ("bg", "c"),
        ("c", "bh"),
        ("bf", "bh"),
        ("in", "arr"),
        ("arr", "cb"),
    ] {
        assert!(
            core_members_disjoint(&layout, left, right, false),
            "{left} {right}"
        );
    }
    // One member, one storage unit, an unknown member: refused.
    assert!(!core_members_disjoint(&layout, "a", "a", false));
    assert!(!core_members_disjoint(&layout, "bf", "bg", true));
    assert!(!core_members_disjoint(&layout, "a", "zz", false));
    // A flexible array member has no bytes: the core contract is refused.
    let flexible = MemberLayout {
        size: 8,
        members: vec![("n".into(), 0, None), ("data".into(), 8, None)],
    };
    assert!(!core_members_disjoint(&flexible, "n", "data", false));
}

/// Rule 11a's generations in the core model: two generations of one site are
/// two allocations, never assumed disjoint; the summary has no identity; a
/// reallocation retires every handle of the current generation.
#[test]
fn rule_11a_generations_agree_with_the_core_model() {
    use crate::loop_generations::{older, previous};
    let mut state = Replay::default();
    replay(&mut state, SiteEvent::Acquire(3));
    replay(&mut state, SiteEvent::Acquire(3));
    assert_eq!(core_identity(&state, 3), CoreIdentity::Allocation(2));
    assert_eq!(
        core_identity(&state, previous(3)),
        CoreIdentity::Allocation(1)
    );
    assert_eq!(core_identity(&state, older(3)), CoreIdentity::Summary);
    assert_eq!(core_identity(&state, 4), CoreIdentity::Absent);
    let slice = |handle, alloc, generation| FootprintEvent::DeriveSlice {
        handle,
        alloc,
        generation,
        elem_offset: 0,
        elem_count: 8,
        declared_bytes: None,
    };
    let mut program = state.program.clone();
    program.events.push(slice(10, 2, 1));
    program.events.push(slice(11, 1, 1));
    program.events.push(FootprintEvent::AssertDisjoint {
        left: 10,
        right: 11,
    });
    assert_eq!(
        check_disjoint_footprint_program(&program),
        Err(vec![
            FootprintDiagnostic::CrossAllocationDisjointnessUnsupported {
                event: 4,
                left: 2,
                right: 1,
            }
        ])
    );
    // A third acquisition: the handle of the former current generation still
    // resolves (it is now the previous one), and the oldest joins the summary.
    let mut state_three = state.clone();
    replay(&mut state_three, SiteEvent::Acquire(3));
    assert_eq!(
        core_identity(&state_three, previous(3)),
        CoreIdentity::Allocation(2)
    );
    // A reallocation of the current generation: its earlier handle is stale.
    let mut program = state.program.clone();
    program.events.push(slice(10, 2, 1));
    let mut realloc = state.clone();
    realloc.program = FootprintProgram::default();
    replay(&mut realloc, SiteEvent::Reallocate(3));
    assert_eq!(current_generation(&realloc, 3), Some((2, 2)));
    program.events.extend(realloc.program.events);
    program.events.push(slice(12, 2, 2));
    program.events.push(FootprintEvent::AssertDisjoint {
        left: 10,
        right: 12,
    });
    let refused = check_disjoint_footprint_program(&program).expect_err("stale handle");
    assert!(
        matches!(
            refused.as_slice(),
            [FootprintDiagnostic::StaleGeneration {
                alloc: 2,
                named: 1,
                ..
            }]
        ),
        "{refused:?}"
    );
}

/// The mechanical binding of `generationVectors` in
/// `proof/QCore/DisjointFootprint.lean`: after two acquisitions of one site,
/// which generations have a core identity.
#[test]
fn cg18_lean_generation_vectors_match_the_replay() {
    use crate::loop_generations::{older, previous};
    let lean = include_str!("../../../../proof/QCore/DisjointFootprint.lean");
    let table = lean
        .split("def generationVectors : List (SiteGeneration × Bool) := [")
        .nth(1)
        .expect("generationVectors");
    let table = &table[..table.find("\n]").expect("end of vectors")];
    let rows = table
        .lines()
        .filter(|line| line.trim_start().starts_with("(."))
        .map(|line| {
            let inner = line.trim().trim_end_matches(',');
            let inner = &inner[2..inner.len() - 1];
            let (generation, has) = inner.split_once(',').expect("pair");
            (generation.trim().to_owned(), has.trim() == "true")
        })
        .collect::<Vec<_>>();
    let mut state = Replay::default();
    replay(&mut state, SiteEvent::Acquire(3));
    replay(&mut state, SiteEvent::Acquire(3));
    let rust = [
        ("current", 3),
        ("previous", previous(3)),
        ("older", older(3)),
    ]
    .map(|(name, identity)| {
        (
            name.to_owned(),
            matches!(core_identity(&state, identity), CoreIdentity::Allocation(_)),
        )
    });
    assert_eq!(rows, rust.to_vec());
}

#[test]
fn the_premise_table_names_the_core_premises() {
    use qcore_core::disjoint_footprint::FootprintAssumption;
    let names = BOUNDARY_PREMISES
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>();
    for premise in [
        FootprintAssumption::DeclaredRecordLayout,
        FootprintAssumption::DistinctAllocationsDisjoint,
        FootprintAssumption::EffectiveTypeDeclared,
        FootprintAssumption::ByteRepresentation,
    ] {
        assert!(names.contains(&premise.premise_name()), "{premise:?}");
    }
    assert!(names.contains(&"P-RESTRICT"));
}
