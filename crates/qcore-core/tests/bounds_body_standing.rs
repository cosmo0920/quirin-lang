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
//! `qcore_core::bounds_body_standing` against `QCore.BoundsBodyStanding`'s
//! shared vectors (`coverageVectors`, `standingVectors`). Clause labels are the
//! Lean model's numbers spelled as strings.

use std::collections::{BTreeMap, BTreeSet};

use qcore_core::bounds_body_standing::{
    BODY, BodyCheck, ClauseKind, FRAME, Rollup, Standing, UnitOutcome, covers, rollup_class,
    standing, unit_outcome,
};

fn set(labels: &[u32]) -> BTreeSet<String> {
    labels.iter().map(u32::to_string).collect()
}

/// `QCore.BoundsBodyStanding.coverageVectors`: (kind, safe, refuted, frame
/// check, covered by).
#[test]
fn coverage_vectors_match_lean() {
    use ClauseKind::*;
    let vectors: [(ClauseKind, bool, bool, bool, Option<&str>); 12] = [
        (QuantCheck, true, false, false, Some(BODY)),
        (QuantCheck, false, false, true, None),
        (QuantCheck, true, true, true, None),
        (Precondition, true, false, false, Some(BODY)),
        (Precondition, false, false, false, None),
        (Frame, true, false, false, None),
        (Frame, false, false, true, Some(FRAME)),
        (Frame, true, true, true, None),
        (Relation, true, false, true, None),
        (ReturnExtent, true, false, true, None),
        (Preserves, true, false, true, None),
        (GlobalExtent, true, false, true, None),
    ];
    for (kind, safe, refuted, frame, expected) in vectors {
        assert_eq!(
            covers(kind, BodyCheck { safe, refuted }, frame),
            expected,
            "{kind:?} {safe} {refuted} {frame}"
        );
    }
    // Every kind the Lean model names is in the table.
    let named = vectors.iter().map(|v| v.0).collect::<BTreeSet<_>>();
    assert_eq!(named, ClauseKind::ALL.into_iter().collect());
}

/// `QCore.BoundsBodyStanding.standingVectors`: (held, clauses, units,
/// standing).
#[test]
fn standing_vectors_match_lean() {
    use UnitOutcome::{Failed, Held, Unvalidated, Validated};
    let vectors: Vec<(bool, BTreeSet<String>, Vec<UnitOutcome>, Standing)> = vec![
        (true, set(&[1]), vec![Validated(set(&[1]))], Standing::Held),
        (false, set(&[1]), vec![Held], Standing::Held),
        (false, set(&[1]), vec![], Standing::DeclaredAssumption),
        (
            false,
            set(&[]),
            vec![Unvalidated],
            Standing::DeclaredAssumption,
        ),
        (
            false,
            set(&[1, 2]),
            vec![Validated(set(&[1, 2]))],
            Standing::Validated,
        ),
        (
            false,
            set(&[1, 2]),
            vec![Validated(set(&[1, 2])), Validated(set(&[1, 2]))],
            Standing::Validated,
        ),
        (
            false,
            set(&[1, 2]),
            vec![Validated(set(&[1, 2])), Validated(set(&[1]))],
            Standing::PartlyValidated(set(&[2])),
        ),
        (
            false,
            set(&[1, 2]),
            vec![Validated(set(&[1])), Validated(set(&[2]))],
            Standing::DeclaredAssumption,
        ),
        (
            false,
            set(&[1, 2]),
            vec![Validated(set(&[1, 2])), Failed],
            Standing::DeclaredAssumption,
        ),
        (
            false,
            set(&[1, 2]),
            vec![Validated(set(&[1, 2])), Unvalidated],
            Standing::DeclaredAssumption,
        ),
        (
            false,
            set(&[1, 2]),
            vec![Validated(set(&[2]))],
            Standing::PartlyValidated(set(&[1])),
        ),
    ];
    assert_eq!(vectors.len(), 11, "standing_vectors_count");
    for (held, clauses, units, expected) in vectors {
        assert_eq!(
            standing(held, &clauses, &units),
            expected,
            "{held} {clauses:?} {units:?}"
        );
    }
}

#[test]
fn a_unit_outcome_follows_hold_then_refutation_then_coverage() {
    assert_eq!(unit_outcome(true, true, &set(&[1])), UnitOutcome::Held);
    assert_eq!(unit_outcome(false, true, &set(&[1])), UnitOutcome::Failed);
    assert_eq!(
        unit_outcome(false, false, &set(&[])),
        UnitOutcome::Unvalidated
    );
    assert_eq!(
        unit_outcome(false, false, &set(&[1])),
        UnitOutcome::Validated(set(&[1]))
    );
}

#[test]
fn the_rollup_needs_every_dependency_validated() {
    let standings = BTreeMap::from([
        ("v".to_owned(), Standing::Validated),
        ("p".to_owned(), Standing::PartlyValidated(set(&[1]))),
        ("d".to_owned(), Standing::DeclaredAssumption),
    ]);
    let class = |deps: &[&str]| {
        rollup_class(
            &deps.iter().map(|d| (*d).to_owned()).collect::<Vec<_>>(),
            &standings,
        )
    };
    assert_eq!(class(&[]), Rollup::NoDeclaration);
    assert_eq!(class(&["v"]), Rollup::OnlyValidated);
    assert_eq!(class(&["v", "p"]), Rollup::PartlyValidated);
    assert_eq!(class(&["v", "p", "d"]), Rollup::DeclaredAssumption);
    // A name with no standing is an assumption.
    assert_eq!(class(&["v", "unknown"]), Rollup::DeclaredAssumption);
}
