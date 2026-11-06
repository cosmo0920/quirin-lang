import QCore.StagedCheckpoints

/-! Card **CORPUS-GROUP-CHECKPOINTS**: per-group checkpoints of the staged
pinned corpus route, reused only under an identity that includes the stage's
whole-run facts.

**A stage's reuse is recomputation.** Under the premise that a unit's row is
determined by the run identity, the stage's facts (the refutation table, the
imported homes) and the unit's fingerprint, a checkpoint whose identity equals
the one recomputed now yields exactly the recomputed rows; one sealed under
other facts is recomputed.

**The staged run is the whole run.** Each later stage's facts are computed
from the previous stage's rows of every group and accepted or refused on its
own rows; with every group of every stage reused or recomputed, every facts
and acceptance decision and the final rows equal the whole run's, and the
composed publication is the whole run's.

The Rust boundary — that the identity captures every input of a unit's
analysis, that every section's SHA-256 is checked, and that the shared fold
prints the rows byte for byte — is pinned by the tests of
`crates/qcore-audit/src/pinned_route/checkpoints/tests.rs` and the corpus runs
in `docs/reports/corpus-group-checkpoints/README.md`. -/

-- A stage's reuse.
#print axioms QCore.StagedCheckpoints.rows_eq_of_fingerprints_eq
#print axioms QCore.StagedCheckpoints.reused_eq_recomputed
#print axioms QCore.StagedCheckpoints.other_facts_recompute
#print axioms QCore.StagedCheckpoints.stageResumed_eq_stage

-- The staged run.
#print axioms QCore.StagedCheckpoints.resumed_eq_whole
#print axioms QCore.StagedCheckpoints.resumed_publication_eq_whole
