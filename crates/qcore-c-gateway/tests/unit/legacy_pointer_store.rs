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

//! Card CG11-POINTER-VALUED-STORES (rule 10v): the flow helpers.

use super::*;

#[test]
fn a_place_rooted_at_the_root_is_not_the_root() {
    assert!(rooted_at("s->p", "s"));
    assert!(rooted_at("s.in.p", "s"));
    assert!(rooted_at("s[i]", "s"));
    assert!(rooted_at("*s", "s"));
    assert!(rooted_at("(*s).p", "s"));
    assert!(!rooted_at("*sp", "s"));
    assert!(!rooted_at("s", "s"));
    assert!(!rooted_at("sp->p", "s"));
    assert!(!rooted_at("t->p", "s"));
}
