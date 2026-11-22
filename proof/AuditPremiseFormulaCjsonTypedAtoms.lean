import QCore.PremiseFormula.CjsonTypedAtoms

/-! Card **SP6-SYMBOLIC-HYPOTHESIS-ATOMS** (C04.18.7, C04.18.9 SP6 block): the
nine `detach_*` hypotheses of `cJSON_DetachItemViaPointer` as typed atoms,
the typed rendering of the declaration's premise formula over registered
terms.

Each atom holds, in the rendering of the model an entry environment induces,
exactly when its `QCore.CjsonRepresentationPremises` definition does; the
premise formula holds exactly when the nine typed atoms do; a string is not
a declared hypothesis atom, and the atom decision vectors agree with the
Rust table.

The Rust boundary is pinned by
`crates/qcore-contract/tests/cjson_typed_atoms.rs`
(`the_nine_atoms_are_the_lean_definitions`,
`the_atom_decision_vectors_are_the_lean_table`). -/

-- What each atom states of any model (its typed rendering).
#print axioms QCore.CjsonTypedAtoms.chain_atom_reads
#print axioms QCore.CjsonTypedAtoms.distinct_atom_reads
#print axioms QCore.CjsonTypedAtoms.empty_atom_reads
#print axioms QCore.CjsonTypedAtoms.first_prev_atom_reads
#print axioms QCore.CjsonTypedAtoms.last_next_atom_reads
#print axioms QCore.CjsonTypedAtoms.link_targets_atom_reads
#print axioms QCore.CjsonTypedAtoms.no_writer_atom_reads
#print axioms QCore.CjsonTypedAtoms.parent_outside_atom_reads
#print axioms QCore.CjsonTypedAtoms.predecessor_atom_reads

-- Faithfulness: each atom is its CjsonRepresentationPremises definition.
#print axioms QCore.CjsonTypedAtoms.chain_atom_iff
#print axioms QCore.CjsonTypedAtoms.distinct_atom_iff
#print axioms QCore.CjsonTypedAtoms.empty_atom_iff
#print axioms QCore.CjsonTypedAtoms.first_prev_atom_iff
#print axioms QCore.CjsonTypedAtoms.last_next_atom_iff
#print axioms QCore.CjsonTypedAtoms.link_targets_atom_iff
#print axioms QCore.CjsonTypedAtoms.no_writer_atom_iff
#print axioms QCore.CjsonTypedAtoms.parent_outside_atom_iff
#print axioms QCore.CjsonTypedAtoms.predecessor_atom_iff
#print axioms QCore.CjsonTypedAtoms.typed_atoms_are_the_definitions

-- The premise formula is the nine typed atoms; the atoms assume something.
#print axioms QCore.CjsonTypedAtoms.detach_formula_is_the_nine_typed_atoms
#print axioms QCore.CjsonTypedAtoms.parent_outside_atom_can_fail

-- A string is not a declared hypothesis atom (QF002).
#print axioms QCore.CjsonTypedAtoms.string_is_not_a_hypothesis
#print axioms QCore.CjsonTypedAtoms.declared_accepted_iff_typed
#print axioms QCore.CjsonTypedAtoms.hypothesis_vectors_agree
