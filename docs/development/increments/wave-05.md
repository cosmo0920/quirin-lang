# Wave 5 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave-5 delegated increment (2026-09-08): six bounded follow-on cards plus a CT
integration landed through delegated worktrees with per-card reports; this
paragraph supersedes nothing above and each card's record keeps its own
boundaries.
- Borrowed pointer-field actuals (C02-P): the admission had already landed
  inside O04's `borrowed_actual.rs`; no production line changed. The card
  adversarially re-verified it — fifteen refusal shapes keep their existing
  Unknown and diagnostics — and supplied the missing evidence: 36 held-out
  tests including the real `flb_input_chunk_create` -> `cio_chunk_open` shape
  and a contract-mode substitution the recheck refuses, plus 25 audited Lean
  theorems (`QCore.BorrowedActual`) modeling the `untracked_bases` closure.
  One boundary recorded, not repaired: two distinct declared names that may
  alias are unrelated by the name-based projection, pinned so the field-read
  and no-field-read forms move together. Evidence:
  `docs/reports/remaining-c02-p/README.md`.
- Neutral-frame recursion (C03-NFR): a third W4 recursive route descends a
  two-point per-member lattice to the greatest post-fixed point with all ten
  V1-V10 validation obligations checked; the design's circular-effectful
  counterexample is refused (the caller's leak is reported, not explained
  away) and a declaration-derived record is withheld (U7). Measured corpus
  reach is 0, correcting the design's ~9 estimate — all 12 reachable
  self-recursive functions refuse on other blockers — so the payoff gates on
  C02-P's admission. 34 audited Lean theorems (`QCore.NeutralFrameRecursion`);
  seven pins updated deliberately, each keeping its load-bearing negative
  half; the 185-unit audit re-measured byte-identical before/after at the pin.
  A pre-existing dead-uncontracted-call hazard is recorded, not repaired.
  Evidence: `docs/reports/remaining-c03-nfr/README.md`.
- Composition completeness (E03-B): the published composite is now a real
  merge (premise union, coverage union, per-obligation bounds, tightest task
  envelope, provenance sets) on a composite-only field; a lone component
  certificate is republished only when every component is that certificate.
  E03's QP012 unequal-premise-set refusal is deliberately superseded by the
  premise union (values still refuse); `derive_task` no longer folds the
  semantic profile into `build_digest`; observation obligations are keyed per
  provider and denominator; an unregistered provider refuses inside
  `composed` under existing QP002. `Composition` grows 15 -> 31 audited
  theorems with two pre-repair theorems retired by name;
  `qcore-verifier-host` and `qcore-verification` move to 0.2.0. Residuals
  recorded as E03-B-O. Evidence: `docs/reports/remaining-e03-b/README.md`.
- Guard-profile completions (T03-F): `close g unchanged;` derives the checked
  NoWriteOccurred witness for a decidable read-only region, dropping the
  declared invariant premise there; guard-scoped facts land as
  `let f = fact r;` / `assert f with g;` / `use f;` with cross-section use
  refused at the real Q616; `wait m;` gains a source form; a nested thread
  scope composes one child task in the mutex ledger only (a task created and
  joined strictly inside a guard stays refused by the unedited
  JoinWhileGuardHeld rule — recorded as T03-GUARD-JOIN, not lifted). The Lean
  event model gains the resource-destruction half: 19 new public theorems, no
  new axiom. Five new reserved words are recorded as a compatibility hazard
  and checked against every fixture. Evidence:
  `docs/reports/remaining-t03-f/README.md`.
- Manifest template binding (P01-F2): provider executables embed their
  manifest template; `verify`/`enable`, registry loads and the configured
  audit compare the installed manifest field-for-field against the template
  read from the executable's bytes, without executing it. All eight
  previously accepted digest-intact edits now refuse by field name; envelope
  absence/ambiguity/corruption each refuse distinctly; a template-less
  executable no longer verifies, so previously built packages must be rebuilt
  (all five shipped packages rebuilt and verified in the merged gate).
  28 audited Lean theorems (`QCore.PackageTemplateBinding`). Forged
  executables, TOCTOU and the unverified `from_package` loader stay open as
  P01-F3. Evidence: `docs/reports/remaining-p01-f2/README.md`.
- Bounded extern calls (`source.extern-call.v1`): a source function may call a
  declared extern function only behind `import std.extern;`, with a trusted
  C04 contract selected through `CompileOptions::extern_contracts` (inline
  source contracts stay refused), through a closed bridging table (scalars by
  value, borrowed Buffer, consumed own<Buffer>, nullable owned return) where
  every excluded C04 mode has a named refusal; the mandatory
  `succeeds BOOL else raise ERROR;` adapter lowers null returns onto the
  existing AllocError route, and its error arm is admitted only with no other
  live acquisition. Diagnostics Q022-Q027 under a new ExternCall pass with
  spec §L21 and the implemented grammars updated in the same change;
  Q020/Q021 untouched. Reference execution uses literal outcome controls; C
  emission prints ABI-table prototypes and checks declared nullness at
  runtime; two fixed ASan/UBSan/LSan fixtures link a real C callee, native
  matching reference line for line. 32 audited Lean theorems
  (`QCore.ExternCall`). Symbol resolution/linkage remains the builder's
  premise; the other call modes, sentinel adapters, extents, concurrent and
  error-region contexts stay named follow-ons (EXTERN-CALL-F). The design
  deviations, including the one new reserved word `succeeds`, are recorded.
  Evidence: `docs/reports/extern-call-v1/README.md` and
  `docs/design/EXTERN_CALL_SEMANTICS_V1.md`.
- Wave-5 CT integration: CT28, CT31, CT32 and CT48 map with stage-derived
  evidence and exact ordered cascades re-measured on fixture bytes; CT
  coverage moves 20/50 to 24/50. `cfg` stays unassessed for all four — the
  guard route has no CFG construction, and the harness refuses the overclaim.
  No native or proof evidence is added: sanitized-native stays 3 of 24 and
  proof 0 of 24, so the grown denominator is mapping, not conformance. CT33,
  CT34 and CT46 stay unmapped with unchanged reasons; the `build_matrix.py`
  divergence is recorded as thirteen cases in `generator_status`. Evidence:
  `docs/reports/wave5-ct-integration/README.md`.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units in the archived
artifacts, the contract corpus stays 60 modules and full language conformance
stays false. CT coverage advances to 24/50 mapped, by executed-stage evidence
only. The four pinned 185-unit artifacts were not re-pinned; CORPUS-REPIN
remains open and now records C03-NFR's byte-stable 43-provisional-Unsafe
re-measurement at the pin, to be reconciled with the archived 42 and the
explicit-Clang rerun's 43. `qcore-schemas` stays 5.0.0; `qcore-compiler` stays
0.2.0; `qcore-verifier-host` and `qcore-verification` are 0.2.0. Merged gate
at this checkpoint: 1624 workspace tests (1 ignored), 58 provider tests,
95 Python tests, 37 Lean audits, both reevaluation examples exit 0, fmt and
clippy clean, and all five provider packages rebuilt and verified under the
new embedded-template requirement. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).

<!-- migrated-section:end -->
