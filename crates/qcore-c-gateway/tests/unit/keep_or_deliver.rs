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

//! Unit pins for card OUT-SLOT-CONDITIONAL-DELIVERY (rule 85): the
//! keep-or-deliver vectors against [`decide`] over `out_mode`, and the
//! readings a summary installs.

use std::collections::BTreeMap;

use qcore_schemas::contract::{ResourceFunctionContract, ResourceReturnMode};
use qcore_schemas::keep_or_deliver::KeepOrDeliverSummary;

use super::*;
use crate::legacy_c::{ExitObservation, ReturnedValue};

fn slot(spelling: &str) -> OutSlotValue {
    let site = |disposition, nullable| OutSlotValue::Site {
        site: 7,
        nullable,
        disposition,
        kind: ResourceKind::Heap,
    };
    match spelling {
        "null" => OutSlotValue::Null,
        "ambiguous" => OutSlotValue::Ambiguous,
        "site-live" => site(SiteDisposition::Live, false),
        "site-live-nullable" => site(SiteDisposition::Live, true),
        other => panic!("unknown exit spelling {other}"),
    }
}

fn exit(value: OutSlotValue) -> ExitObservation {
    ExitObservation {
        parameters: vec![None],
        returned: ReturnedValue::None,
        out_slots: vec![Some(value)],
        caller_containment: BTreeMap::new(),
        pending_replacements: BTreeMap::new(),
        derived_base_touched: false,
        selected: None,
    }
}

#[test]
fn keep_or_deliver_vectors_hold() {
    for (null_run, held, expected) in KEEP_OR_DELIVER_VECTORS {
        let observations = null_run
            .iter()
            .map(|spelling| exit(slot(spelling)))
            .collect::<Vec<_>>();
        let read = decide(crate::legacy_summary::out_mode(0, &observations), held)
            .map(|nullable| if nullable { "out-nullable" } else { "out" });
        assert_eq!(read, *expected, "{null_run:?} {held:?}");
    }
}

/// Card OUT-SLOT-MEMBER-THEN-FRESH: the slot-mode vectors against [`mode`]
/// over `out_mode` (`QCore.MemberMoveOut.slotModeVectors`).
#[test]
fn slot_mode_vectors_hold() {
    for (null_run, held, expected) in SLOT_MODE_VECTORS {
        let observations = null_run
            .iter()
            .map(|spelling| exit(slot(spelling)))
            .collect::<Vec<_>>();
        let read = mode(crate::legacy_summary::out_mode(0, &observations), held).map(
            |(null_only, nullable)| match (null_only, nullable) {
                (false, false) => "keep-out",
                (false, true) => "keep-out-nullable",
                (true, false) => "null-only-out",
                (true, true) => "null-only-out-nullable",
            },
        );
        assert_eq!(read, *expected, "{null_run:?} {held:?}");
    }
}

/// A null-only summary installs no kept reading: a slot not known null is
/// refused, never read as the caller's object.
#[test]
fn a_null_only_summary_has_no_kept_reading() {
    let summary = KeepOrDeliverSummary {
        slot: 0,
        delivered: ResourceFunctionContract::new(
            [ResourceParameterMode::Out { nullable: true }],
            ResourceReturnMode::None,
        ),
        null_only: true,
    };
    assert!(summary.kept().is_none());
}

/// Only an `Out` reading of the null run is a delivery: a slot the null run
/// reads as anything else publishes nothing, whatever the owned run held.
#[test]
fn only_an_out_null_run_is_a_delivery() {
    for mode in [
        ResourceParameterMode::Borrowed,
        ResourceParameterMode::Owned { nullable: false },
        ResourceParameterMode::Released { nullable: false },
    ] {
        assert_eq!(decide(Some(mode), &[true]), None);
    }
    assert_eq!(decide(None, &[true]), None);
}

/// The two readings the parser installs: the delivered contract as
/// validated, and the kept one with the slot read `Owned` on the object.
#[test]
fn the_kept_reading_is_the_delivered_one_with_the_slot_owned() {
    let summary = KeepOrDeliverSummary {
        slot: 0,
        delivered: ResourceFunctionContract::new(
            [
                ResourceParameterMode::Out { nullable: true },
                ResourceParameterMode::Borrowed,
            ],
            ResourceReturnMode::None,
        ),
        null_only: false,
    };
    let kept = summary.kept().expect("an out slot");
    assert_eq!(
        kept.parameters,
        [
            ResourceParameterMode::Owned { nullable: false },
            ResourceParameterMode::Borrowed
        ]
    );
}
