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

//! Pure conjunction rule. A provenance label supplies no truth or authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    Open,
    Supported,
    Refuted,
}
/// Reporting composition only: no caller may use this as certificate admission.
#[must_use]
pub fn conjunction(atoms: &[(bool, Disposition)]) -> Disposition {
    if atoms.is_empty() || atoms.iter().any(|(same, _)| !same) {
        Disposition::Open
    } else if atoms
        .iter()
        .any(|(_, status)| *status == Disposition::Refuted)
    {
        Disposition::Refuted
    } else if atoms
        .iter()
        .all(|(_, status)| *status == Disposition::Supported)
    {
        Disposition::Supported
    } else {
        Disposition::Open
    }
}
