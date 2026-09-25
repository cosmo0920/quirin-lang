import QCore.GnuBuiltinReader

/-! Card **W44-LIBC-RESIDUALS**: two words the declared-ABI reader stopped at.
`__extension__`, a GNU keyword carrying no type, is now a discarded specifier,
so glibc's header-inline `__bswap_64` reads as `u64 (u64)`; and the compiler's
`__builtin_va_list`, which `__gnuc_va_list` names, projects to the loader's
parameter-only `va_list` spelling, so glibc's `vsnprintf` and `vprintf` agree.

Five directions are audited. **Conservativity**: both widenings only add, and
a prefix without `__extension__` or a chain that does not reach the builtin
reads exactly as before. **Exactness**: the builtin admits `va_list` and
nothing else, no earlier reading admits `va_list`, and a result or a pointer
still refuses. **Shadowing**: a unit that `typedef`s the builtin name is read
through its own typedef. **Custody**: `va_list` is not a `ptr` spelling, so
`Borrowed` is the only mode on it, and it is not a result spelling.
**Non-vacuity**: the measured `__bswap_64` prefix and `vsnprintf` signature,
unreadable before, are read now.

What this is not: a model of the token reader. That the words are the ones the
token stream carries is trust boundary 28's extraction premise. -/

-- `__extension__`: conservativity and the rule as an equation.
#print axioms QCore.GnuBuiltinReader.the_core_now_ignores_extension
#print axioms QCore.GnuBuiltinReader.extension_is_invisible
#print axioms QCore.GnuBuiltinReader.the_two_cores_agree_without_extension
#print axioms QCore.GnuBuiltinReader.a_prefix_without_extension_is_unchanged
#print axioms QCore.GnuBuiltinReader.the_core_before_refuses_extension
#print axioms QCore.GnuBuiltinReader.a_prefix_with_extension_was_unreadable
#print axioms QCore.GnuBuiltinReader.the_extension_widening_only_adds
#print axioms QCore.GnuBuiltinReader.the_bswap64_result_was_unreadable_and_is_now_read

-- `__builtin_va_list`: conservativity, exactness and where it still refuses.
#print axioms QCore.GnuBuiltinReader.the_va_list_widening_only_adds
#print axioms QCore.GnuBuiltinReader.the_va_list_widening_is_strict
#print axioms QCore.GnuBuiltinReader.only_va_list_is_admitted_for_the_builtin
#print axioms QCore.GnuBuiltinReader.the_builtin_is_not_its_x86_64_representation
#print axioms QCore.GnuBuiltinReader.a_va_list_result_refuses
#print axioms QCore.GnuBuiltinReader.a_pointer_to_a_va_list_refuses
#print axioms QCore.GnuBuiltinReader.a_pointer_projection_never_admits_va_list
#print axioms QCore.GnuBuiltinReader.no_earlier_reading_admits_va_list

-- Resolution and shadowing.
#print axioms QCore.GnuBuiltinReader.a_chain_to_the_builtin_was_unresolved
#print axioms QCore.GnuBuiltinReader.resolution_is_conservative
#print axioms QCore.GnuBuiltinReader.a_unit_typedef_of_the_builtin_name_is_followed
#print axioms QCore.GnuBuiltinReader.the_glibc_chain_reaches_the_builtin
#print axioms QCore.GnuBuiltinReader.a_typedef_pointer_to_the_builtin_stays_unresolved

-- Custody and the loader's vocabularies.
#print axioms QCore.GnuBuiltinReader.a_va_list_is_not_a_pointer_spelling
#print axioms QCore.GnuBuiltinReader.borrowed_is_the_only_mode_on_a_va_list
#print axioms QCore.GnuBuiltinReader.an_owned_result_of_a_va_list_is_rejected
#print axioms QCore.GnuBuiltinReader.va_list_is_a_parameter_spelling_only
#print axioms QCore.GnuBuiltinReader.the_result_vocabulary_lost_only_va_list

-- The comparison, with the measured `vsnprintf`.
#print axioms QCore.GnuBuiltinReader.vsnprintf_agrees_now_and_was_withdrawn_as_unreadable
#print axioms QCore.GnuBuiltinReader.a_pointer_spelling_of_the_va_list_disagrees
