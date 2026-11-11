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

//! W48-BOTTOM-UP-BATCH5: retain the existing fully checked scalar read.
//! No pointer destination, store frame or readback can lose its identity.
use crate::legacy_c::{AliasPeers, LegacyInstruction};

fn eligible(same: bool, scalar: bool, no_frame: bool, no_readback: bool) -> bool {
    same && scalar && no_frame && no_readback
}

pub(crate) fn retain(
    destination: &str,
    alias: Option<&AliasPeers>,
    candidate: Option<LegacyInstruction>,
) -> Option<LegacyInstruction> {
    let mut candidate = candidate?;
    let LegacyInstruction::IndirectScalarAccess {
        destination: target,
        frame,
        borrowed_destination,
        readback,
        alias_reads,
        typed_alternative: None,
        ..
    } = &mut candidate
    else {
        return None;
    };
    if !eligible(
        target.as_deref() == Some(destination),
        !*borrowed_destination,
        frame.is_none(),
        readback.is_none(),
    ) {
        return None;
    }
    if let Some(alias) = alias
        && !alias_reads.contains(alias)
    {
        alias_reads.push(alias.clone());
    }
    Some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scalar() -> LegacyInstruction {
        LegacyInstruction::IndirectScalarAccess {
            typed_alternative: None,
            bases: vec!["root".into()],
            alias_reads: Vec::new(),
            destination: Some("value".into()),
            frame: None,
            chains: Vec::new(),
            borrowed_destination: false,
            readback: None,
        }
    }

    #[test]
    fn original_alias_read_is_retained_exactly_once() {
        let alias = AliasPeers {
            place: "root".into(),
            peers: vec!["peer".into()],
        };
        let candidate = retain("value", Some(&alias), Some(scalar())).unwrap();
        let candidate = retain("value", Some(&alias), Some(candidate)).unwrap();
        let LegacyInstruction::IndirectScalarAccess {
            alias_reads,
            bases,
            destination,
            ..
        } = candidate
        else {
            unreachable!()
        };
        assert_eq!(alias_reads, vec![alias]);
        assert_eq!(bases, vec!["root"]);
        assert_eq!(destination.as_deref(), Some("value"));
    }

    #[test]
    fn destination_and_pointer_and_store_and_readback_guards_refuse() {
        assert!(retain("other", None, Some(scalar())).is_none());
        let mut borrowed = scalar();
        if let LegacyInstruction::IndirectScalarAccess {
            borrowed_destination,
            ..
        } = &mut borrowed
        {
            *borrowed_destination = true;
        }
        assert!(retain("value", None, Some(borrowed)).is_none());
        let mut stored = scalar();
        if let LegacyInstruction::IndirectScalarAccess { frame, .. } = &mut stored {
            *frame = Some(crate::legacy_c::RefusedWrites::Unbounded);
        }
        assert!(retain("value", None, Some(stored)).is_none());
        let mut read_back = scalar();
        if let LegacyInstruction::IndirectScalarAccess { readback, .. } = &mut read_back {
            *readback = Some("root->value".into());
        }
        assert!(retain("value", None, Some(read_back)).is_none());
    }

    #[test]
    fn all_admission_vectors_match_the_audited_model() {
        let mut actual = Vec::new();
        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    for d in [false, true] {
                        actual.push(eligible(a, b, c, d));
                    }
                }
            }
        }
        assert_eq!(
            actual,
            [
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, true
            ]
        );
        assert!(include_str!("../../../proof/QCore/ScalarReadIdentity.lean").contains(
            "def vectors : List Bool := [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, true]"));
    }
}
