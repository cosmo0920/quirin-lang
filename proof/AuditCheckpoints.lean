import QCore.Checkpoints

/-! Card **W45-AUDIT-CHECKPOINTS**: per-group `.qo` audit checkpoints, resumed
under an identity guard and composed into one report.

Three directions are audited.

**Identity-guarded reuse.** Under the premise that a unit's row is determined
by the run identity and the unit's fingerprint (`Determined`), a checkpoint is
reused only when its identity equals the one recomputed now, and then its rows
are exactly the recomputed rows, whatever environment it was sealed from.

**Composition.** The whole run's publication (rows appended, tallies added,
sources united) is a fold that is a monoid homomorphism from lists, so folding
per group and then over the groups gives the whole run's publication for any
split into consecutive groups; with the independence premise (a group's rows are
its own units' rows), resumed-then-composed groups publish the whole run.

**Admission.** A composition is admitted only when its checkpoints cover the
plan's positions exactly and name one run, and one run's sealed checkpoints are
admitted and compose to the whole run.

The Rust boundary — that the fingerprint captures every analysis input (the
independence audit in `docs/reports/w45-audit-checkpoints/README.md`), that
every section's SHA-256 is checked, and that the streaming renderer prints the
folded publication byte for byte — is pinned by
`crates/qcore-audit/tests/checkpoint_cli.rs`, the unit tests of
`crates/qcore-audit/src/checkpoint.rs`, and the 185-unit corpus comparison in
the report. -/

-- Identity-guarded reuse.
#print axioms QCore.Checkpoints.rows_eq_of_fingerprints_eq
#print axioms QCore.Checkpoints.reused_eq_recomputed
#print axioms QCore.Checkpoints.resume_eq_recompute
#print axioms QCore.Checkpoints.reuse_takes_the_stored_rows

-- Composition is the whole run.
#print axioms QCore.Checkpoints.Publication.empty_join
#print axioms QCore.Checkpoints.Publication.join_empty
#print axioms QCore.Checkpoints.Publication.join_assoc
#print axioms QCore.Checkpoints.publish_append
#print axioms QCore.Checkpoints.compose_eq_whole
#print axioms QCore.Checkpoints.group_rows_flatten
#print axioms QCore.Checkpoints.resume_then_compose_eq_whole

-- Admission.
#print axioms QCore.Checkpoints.compose_admits_one_run
#print axioms QCore.Checkpoints.compose_covers_the_plan
#print axioms QCore.Checkpoints.one_run_composes_to_the_whole
