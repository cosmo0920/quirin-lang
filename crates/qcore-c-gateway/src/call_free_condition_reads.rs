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

//! W48-BOTTOM-UP-BATCH4: keep a completely classified call-free condition
//! plan when a checked memory read needs its short-circuit graph. Leaf
//! admission, flow obligations and graph assembly remain their existing rules.

pub(crate) fn keep(typed: bool, elaborated: bool, checked_read: bool) -> bool {
    typed || elaborated || checked_read
}

#[cfg(test)]
mod tests {
    use super::keep;

    #[test]
    fn admission_matches_all_lean_vectors() {
        let proof = include_str!("../../../proof/QCore/CallFreeConditionReads.lean");
        assert!(proof.contains(
            "def vectors : List Bool := [false, true, true, true, true, true, true, true]"
        ));
        for bits in 0..8 {
            assert_eq!(keep(bits & 4 != 0, bits & 2 != 0, bits & 1 != 0), bits != 0);
        }
    }
}
