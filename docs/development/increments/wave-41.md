# Wave 41 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`CT-PROOF-LEG-3` (2026-09-19).** The fifth executed `proof` row, and the
measurement that the cheap route to a sixth is spent. The card first **re-ran
the survey rather than inheriting it**: every one of the 47 mapped cases' own
fixtures compiled through `compile_files`, every task-route module executed by
`execute_task_module` and handed to `AdmittedCausalOrder::derive`, on the
default script and on five failure scripts, with the raw output shipped
(`docs/reports/ct-proof-leg-3/probes/survey.tsv`). Eleven mapped cases derive on
the default script; four were already proved; CT12's and CT14's runs are **event
for event CT01's**; CT13, CT15, CT16, CT17 and CT20 derive but their default runs
do not exhibit their scenarios, whose defining arms are refused as
`UnattributableEvent` (creation failure) or carry an `Error` outcome the Lean
alphabet has no constructor for (allocation failure, and CT16's default run).
`STABILITY-REEVAL-4`'s claim that **CT50 is the only remaining F02-shaped
candidate** therefore holds, verified rather than inherited — and this card spent
it, so the set is now empty (`CT-PROOF-F02-CANDIDATES-EXHAUSTED`, registered;
`CT-PROOF-FOURTH-CASE-UNTRACKED` retired). CT50's leg is
`proof/QCore/Ct50LifetimeIdentity.lean` with `proof/AuditCt50LifetimeIdentity.lean`
(Lean audits **141 → 142**) and the host-unconditional
`compiler/tests/ct50_proof_leg.rs`; proof **4 of 47 → 5 of 47**, which is
evidence about one case's artifact and never conformance (constraint 23). Two
things about it are worth the record. Its **closure is the degenerate case and
is proved, not assumed**: the run has one actor, so a relation demanding
agreement on every actor's projection over the same multiset admits nothing but
the run itself (`an_admitted_rendering_is_the_reference_run`), which the Rust
leg re-measures on the artifact by enumerating `admitted_linearizations` and
requiring the singleton. And its **scope is the capability half of the
scenario**: CT50 is named after a numeric *address* reuse that an alphabet
carrying generations cannot see, so what is proved is disjoint generation
lifetimes and that no fact of the run uses the old generation, while address
identity stays native observation (`CT50-PROOF-ADDRESS-REUSE-UNPROVED`,
registered). Beside the leg, `compiler/tests/proof_leg_digests.rs` closes
`PROOF-LEG-DIGESTS-UNCROSSCHECKED` (retired): every `theorem ct<N>…digest` in
`proof/QCore/` is compared with every corresponding `compiler/tests/` constant,
total or reject in both directions, and the sweep found **six** Rust pins where
the residual's own inventory named five — CT01's is written twice. Its own
premise is registered (`PROOF-LEG-DIGEST-CROSSCHECK-TEXTUAL`). **The non-vacuity
harness found two defects in this card's own claims and both were repaired
rather than reworded**: the read-by-a-second-actor mutant fails *two* clauses so
it isolates neither (replaced by a result-claimed-by-a-second-actor mutant that
does, with the old one kept and named as non-isolating), and the payload clause
is *implied* by the read-window chain together with the read count — now the Lean
theorem `the_payload_clause_is_implied_by_the_read_window_and_the_read_count`
rather than a silent weak spot, and its mutation row is expected green with that
ground. Twenty mutations, every restore sha256-equal, baseline green before and
after (`mutations.log`). Three carried entries disposed: `LEASE-BOUNDARY-TRANSCRIPTION-PREMISE`'s
discharge text repaired to name the mechanism that now exists (the entry stays
open; this card deliberately did not apply it to another card's Lean module),
and `CT-PROOF-CONTROL-ARM-TIER` and `CT-PROOF-CREATION-FAILURE-ALPHABET`
restated as measured boundaries with wave-41 measurements — the second sharpened,
because the cases it lists need **two** alphabet extensions and the entry named
one. No `Q` code, no reserved word, no lexer change, no checker or emitter
change; `git diff --stat` over `crates/` and `compiler/src/` is empty.
`named_followups` **257 → 258**.
