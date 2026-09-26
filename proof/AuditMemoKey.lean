import QCore.MemoKey

/-! Card **W45-GATEWAY-MEMO-KEY**: the per-thread memo of
`member_releasing_functions` (`crates/qcore-c-gateway/src/legacy_member_release.rs`)
is served only on key equality.

Audited: **a hit has an equal key**; **a forced collision is a miss**, and an
entry with another key gives the uncached answer whatever it holds; under the
memo's own invariant every call is the uncached function and keeps the
invariant, so **any sequence of calls is observationally the uncached
function**, for every digest; and **the replaced digest-only rule serves a
collision** (a witness).

What this is not: a proof about the Rust (trust boundary 28). That the Rust key
is the whole input the uncached function reads is a review premise, pinned by
the module's unit tests. -/

#print axioms QCore.MemoKey.a_hit_has_an_equal_key
#print axioms QCore.MemoKey.a_colliding_entry_is_a_miss
#print axioms QCore.MemoKey.a_mismatched_entry_gives_the_uncached_answer
#print axioms QCore.MemoKey.call_eq_uncached
#print axioms QCore.MemoKey.call_keeps_filled
#print axioms QCore.MemoKey.run_eq_map
#print axioms QCore.MemoKey.empty_is_filled
#print axioms QCore.MemoKey.the_digest_alone_serves_a_collision
