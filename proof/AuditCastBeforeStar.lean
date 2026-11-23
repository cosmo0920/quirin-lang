import QCore.CastBeforeStar

/-! Card **CAST-BEFORE-STAR-READ-AS-BINARY**: a gateway token reader that
decides a single `*` before a name by the token before it treats a `)` that
may close a cast as allowing a dereference, through rule 74f's one shared
detector `ampersand_runs::may_close_a_cast` (gateway rule 74g, extending rule
74f, `QCore.CastBeforeAmpersand`, not restated here).

Two readers are audited.

**The refused hand-on.** The base missed the slot `*p` under a cast. The tip
hands on both readings' places, keeps every place the base handed on, and
hands on both for an undecided group.

**Premise 4.** The tip refuses wherever the base refused, and also refuses a
base dereferenced under a cast.

Each group's class (the Rust detector's output, bound to `STAR_VECTORS`) is
trust boundary 28's. It is pinned by the unit tests of `ampersand_runs.rs`,
`legacy_refused_operands.rs` and `aggregate_store.rs`, and by
`crates/qcore-c-gateway/tests/leak_pin_audit.rs` and
`crates/qcore-c-gateway/tests/alias_aggregate.rs`. -/

-- The refused hand-on.
#print axioms QCore.CastBeforeStar.old_reading_misses_the_slot
#print axioms QCore.CastBeforeStar.the_fix_only_adds_places
#print axioms QCore.CastBeforeStar.the_union_covers_both_readings
#print axioms QCore.CastBeforeStar.an_undecided_group_hands_on_both
#print axioms QCore.CastBeforeStar.a_call_is_binary
#print axioms QCore.CastBeforeStar.an_object_is_binary
#print axioms QCore.CastBeforeStar.star_vectors_hold

-- Premise 4.
#print axioms QCore.CastBeforeStar.premise_four_only_refuses_more
#print axioms QCore.CastBeforeStar.premise_four_refuses_a_cast
