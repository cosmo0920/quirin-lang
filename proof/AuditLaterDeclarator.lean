import QCore.LaterDeclarator

/-! Card **LATER-DECLARATOR-INITIALISER-UNREAD**: a declaration of several
init-declarators reads as its declarations written one per statement, or keeps
its old reading (gateway rule 74k).

Four directions are audited.

**C's meaning.** The declaration is the ordered sequence of its one-per-
statement declarations, keeping every declarator under the same specifiers.

**The reading.** The lowered block runs as the sequence of the pieces'
lowerings, so a reading faithful for one declarator alone is faithful for the
declaration.

**The splitter.** Top-level comma splitting gives the declarators back.

**Fail-closed.** The decision table is bound to the Rust by shared vectors;
one unread declarator, or a declaration with no pointer declarator, keeps the
whole declaration on its old reading.

Faithful states in the Rust, and the correspondence with it, are trust
boundary 28's, pinned by
`crates/qcore-c-gateway/tests/later_declarator_initialiser.rs`. -/

-- C's meaning.
#print axioms QCore.LaterDeclarator.the_declaration_is_its_split
#print axioms QCore.LaterDeclarator.the_split_keeps_every_declarator
#print axioms QCore.LaterDeclarator.each_piece_is_one_declarator

-- The reading.
#print axioms QCore.LaterDeclarator.the_block_reads_as_the_sequence
#print axioms QCore.LaterDeclarator.reading_the_declaration_is_reading_its_split
#print axioms QCore.LaterDeclarator.no_event_changes_nothing

-- The splitter.
#print axioms QCore.LaterDeclarator.split_after_a_group
#print axioms QCore.LaterDeclarator.splitting_the_joined_declarators_gives_them_back

-- Fail-closed.
#print axioms QCore.LaterDeclarator.vectors_match
#print axioms QCore.LaterDeclarator.a_non_automatic_object_is_refused
#print axioms QCore.LaterDeclarator.an_uninitialised_declarator_must_evaluate_nothing
#print axioms QCore.LaterDeclarator.a_cast_that_is_not_null_is_refused
#print axioms QCore.LaterDeclarator.a_scalar_needs_a_numeric_literal
#print axioms QCore.LaterDeclarator.no_event_only_without_an_initialiser
#print axioms QCore.LaterDeclarator.a_declaration_without_a_pointer_keeps_the_old_reading
#print axioms QCore.LaterDeclarator.one_unread_declarator_keeps_the_old_reading
#print axioms QCore.LaterDeclarator.every_admitted_declarator_is_decided
