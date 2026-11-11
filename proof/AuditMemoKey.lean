-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

import QCore.MemoKey

/-! Card **W45-GATEWAY-MEMO-KEY**: the per-thread memo of
`member_releasing_functions` (`crates/qcore-c-gateway/src/legacy_member_release.rs`)
is served only on key equality.

Audited: **a hit has an equal key**; **a forced collision is a miss**, and an
entry with another key gives the uncached answer whatever it holds; under the
memo's own invariant every call is the uncached function and keeps the
invariant, so **any sequence of calls is observationally the uncached
function**, for every digest; and **the replaced digest-only rule serves a
collision** (a witness). Card **W45-MEMO-MEMORY** removed the memo: **removing
it changes no answer**, from the empty memo or any state it filled.

What this is not: a proof about the Rust (trust boundary 28). That the Rust key
was the whole input the uncached function reads was a review premise, pinned by
the memo's unit tests; since the removal the Rust has no key and no memo. -/

#print axioms QCore.MemoKey.a_hit_has_an_equal_key
#print axioms QCore.MemoKey.a_colliding_entry_is_a_miss
#print axioms QCore.MemoKey.a_mismatched_entry_gives_the_uncached_answer
#print axioms QCore.MemoKey.call_eq_uncached
#print axioms QCore.MemoKey.call_keeps_filled
#print axioms QCore.MemoKey.run_eq_map
#print axioms QCore.MemoKey.empty_is_filled
#print axioms QCore.MemoKey.the_digest_alone_serves_a_collision
#print axioms QCore.MemoKey.removing_the_memo_changes_no_answer
