import QCore.PointerQualifierReader

/-! Card **W43-LIBC-RESULT-ALIAS**, discharging
`W43-LIBC-POINTER-QUALIFIER-READER`: the declared-ABI reader drops the
qualifiers after a parameter declarator's last `*`, which qualify the parameter
object and are dropped for type compatibility (C11 6.7.6.3p15).

Four directions are audited. **Conservativity**: every parameter the earlier
reader projected is projected identically, and nothing that was read became
unreadable. **The rule as an equation**: any run of trailing qualifiers reads as
its absence. **Exactly one position**: a qualifier between two `*` still
refuses, a declarator with no `*` is unchanged, and the step is idempotent.
**Non-vacuity**: glibc's `const char *__restrict`, measured unreadable, now
projects to `ptr<const, u8>` and keeps its pointee's constness.

What this is not: a model of the token reader. That the word list is the
declarator C declares is trust boundary 28's extraction premise, and result
declarators are deliberately not widened. -/

-- Conservativity.
#print axioms QCore.PointerQualifierReader.the_widening_only_adds
#print axioms QCore.PointerQualifierReader.the_widening_loses_no_reading

-- The rule, as an equation, and where it applies.
#print axioms QCore.PointerQualifierReader.the_widening_reads_the_unqualified_type
#print axioms QCore.PointerQualifierReader.a_declarator_without_a_star_is_unchanged
#print axioms QCore.PointerQualifierReader.drop_is_idempotent
#print axioms QCore.PointerQualifierReader.a_qualifier_between_two_stars_still_refuses

-- Non-vacuity, with the measured glibc witness.
#print axioms QCore.PointerQualifierReader.the_glibc_format_parameter_was_unreadable
#print axioms QCore.PointerQualifierReader.the_glibc_format_parameter_is_now_read
#print axioms QCore.PointerQualifierReader.the_widening_is_strict
#print axioms QCore.PointerQualifierReader.the_pointee_constness_is_kept
#print axioms QCore.PointerQualifierReader.a_const_pointer_to_mutable_bytes_is_mutable
#print axioms QCore.PointerQualifierReader.a_restrict_between_two_stars_refuses
