import QCore.DialectTargetIdentity

/-! Card **CG30-DIALECTS-FLAGS-TYPES** (gateway rule 11y,
`crates/qcore-c-gateway/src/c_frontend_identity.rs`): distinct dialects and
targets are distinct evidence identities and a cached verdict is answered only
under its own key; only the LP64 premise is admitted; a withdrawn definition is
`Unknown`; an extension refusal is never `Safe`; one specifier multiset has one
type identity. Premise: SHA-256 collision freedom for the Rust digest. Not a
proof about the Rust readers (trust boundary 28). -/

#print axioms QCore.DialectTargetIdentity.dialectByte_injective
#print axioms QCore.DialectTargetIdentity.distinct_dialect_distinct_bytes
#print axioms QCore.DialectTargetIdentity.baseline_bytes_unchanged
#print axioms QCore.DialectTargetIdentity.c2y_extends_c23
#print axioms QCore.DialectTargetIdentity.lookup_minted_under_key
#print axioms QCore.DialectTargetIdentity.no_reuse_across_dialects
#print axioms QCore.DialectTargetIdentity.no_reuse_across_targets
#print axioms QCore.DialectTargetIdentity.admit_iff
#print axioms QCore.DialectTargetIdentity.ilp32_abi_refused
#print axioms QCore.DialectTargetIdentity.llp64_target_refused
#print axioms QCore.DialectTargetIdentity.withdrawn_is_unknown
#print axioms QCore.DialectTargetIdentity.refusal_never_safe
#print axioms QCore.DialectTargetIdentity.refusal_keeps_unsafe
#print axioms QCore.DialectTargetIdentity.refusal_is_monotone
#print axioms QCore.DialectTargetIdentity.perm_same_typeId
#print axioms QCore.DialectTargetIdentity.bool_is_Bool
#print axioms QCore.DialectTargetIdentity.long_unsigned_int_vector
#print axioms QCore.DialectTargetIdentity.distinct_counts_distinct
