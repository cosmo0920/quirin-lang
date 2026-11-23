import QCore.CastBeforeAmpersand

/-! Card **CAST-BEFORE-AMPERSAND-READ-AS-BINARY**: a gateway token reader that
decides a single `&` by the token before it reads `) &` as unary when the
group may close a cast, through the one shared detector
`ampersand_runs::may_close_a_cast` (gateway rule 74f, extending rule 74e,
`QCore.AmpersandRunParity`, not restated here).

Three directions are audited.

**The fix.** The base refused hand-on missed the address under a cast; the tip
reads unary wherever the base did and adds exactly the cast closes.

**The resolution.** An undecided group is a cast; an argument list, an
expression group and a known object are binary.

**The hand-on.** A unary `&` hands on every prefix of the chain, a superset of
the binary reading's place.

Each group's class (the Rust detector's output, bound to `CAST_VECTORS`) is
trust boundary 28's, pinned by the unit tests of `ampersand_runs.rs`,
`legacy_refused_operands.rs` and `legacy_call_instantiation.rs` and by
`crates/qcore-c-gateway/tests/leak_pin_audit.rs`. -/

-- The fix.
#print axioms QCore.CastBeforeAmpersand.old_reading_misses_a_cast
#print axioms QCore.CastBeforeAmpersand.the_fix_only_adds_addresses
#print axioms QCore.CastBeforeAmpersand.the_fix_adds_only_cast_closes

-- The resolution.
#print axioms QCore.CastBeforeAmpersand.an_undecided_group_is_a_cast
#print axioms QCore.CastBeforeAmpersand.a_call_is_binary
#print axioms QCore.CastBeforeAmpersand.a_value_group_is_binary
#print axioms QCore.CastBeforeAmpersand.cast_vectors_hold

-- The hand-on.
#print axioms QCore.CastBeforeAmpersand.the_address_hands_on_a_superset
#print axioms QCore.CastBeforeAmpersand.a_cast_hands_on_the_root
