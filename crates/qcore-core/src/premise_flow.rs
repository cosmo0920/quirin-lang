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

//! Pure three-valued logical flow. No evidence or source authority is admitted.
use crate::hypothesis::Disposition;
use Disposition::{Open, Refuted, Supported};
#[must_use]
pub fn and(left: Disposition, right: Disposition) -> Disposition {
    match (left, right) {
        (Refuted, _) | (_, Refuted) => Refuted,
        (Supported, Supported) => Supported,
        _ => Open,
    }
}
#[must_use]
pub fn or(left: Disposition, right: Disposition) -> Disposition {
    match (left, right) {
        (Supported, _) | (_, Supported) => Supported,
        (Refuted, Refuted) => Refuted,
        _ => Open,
    }
}
#[must_use]
pub fn except(left: Disposition, forbidden: Disposition) -> Disposition {
    match (left, forbidden) {
        (Refuted, _) | (_, Supported) => Refuted,
        (Supported, Refuted) => Supported,
        _ => Open,
    }
}
#[must_use]
pub fn terminal(left: Disposition, condition: Disposition) -> (Disposition, bool) {
    let status = and(left, condition);
    (status, status == Supported)
}
