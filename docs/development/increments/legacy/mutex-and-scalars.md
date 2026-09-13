# Historical guidance: mutex invariants and scalar fields

[Development table of contents](../../../../DEVELOPMENT.md) · [History index](../README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
#### Checked mutex invariants (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 1068-1101 — fuller variant.*

Checked mutex invariants (2026-09-07): T02 implements `CheckedMutexInvariantV1`
as a new `crates/qcore-core/src/mutex_invariant.rs`, beside an unedited
`check_concurrent` whose body, 26 pinned tests and `Lifetime` Lean model are
byte-identical; `TrustedExplicitTrace` retains `GuardFootprintAuthority` and
`SynchronizedCustodyEdges`. Mutex generations are liveness-ledgered under V01's
rules: a refused destroy leaves the generation live and a repeat destroy reports
a dead generation, not an ownership complaint. `CreateMutex` fixes its declared
footprints and consumes the creator's `FULL_ACCESS` share over each, so a
protected footprint has exactly one protector and no fallback authority path;
pointwise share conservation over exact interval atoms is verified before the
first event and after every event, refused events included. Each `Lock` mints a
fresh never-reused guard; reads need that guard, writes need an open invariant,
and unlock or typed-error exit requires a closed invariant with no live
guard-derived borrow, dropping every guard-scoped fact. Custody transfer carries
a checker-verified `SyncWitness` compared against derived happens-before, in the
`weak_memory` style; no `bool` occupies an authority position. Dynamic shapes,
condition variables, try/timed locks, rwlocks, nested protecting mutexes, lock
ranks and recursive acquisition are represented refusals, not omissions.
Diagnostics occupy the free `Q610`-`Q623` range with `Q600` retained for carried
-over structural refusals. Certificates name exact generations, admitted
invariant transitions and derived edges; an independent recheck rederives every
field and refuses either trusted premise, including through the plain-string
route. 48 new tests bring the workspace to 898 passing; 46 provider tests,
43 Python tests, both probes and the standing gates pass, and
`check_lean_audits.py` passes 21 audits after a new `QCore.Concurrency.MutexInvariant`
namespace lands all 18 designed theorems plus 13 supporting ones and 29 decidable
scenarios, with no new axiom. Evidence:
`docs/reports/remaining-t02/README.md`. This is model-level only: no source
lowering, no CT case mapped (11/50 unchanged), no raw unit promoted, and the four
pinned 185-unit artifacts are unchanged by construction rather than re-measured,
the corpus being absent from the worktree. `DeclaredFootprintShape` and
`InvariantPredicateDeclared` are new declared premises; source-to-trace
refinement, C layout, deadlock freedom, source lowering (T03), general recursive
shape preservation and `research_closure` profile recording remain open.

*Moved verbatim from `CLAUDE.md` lines 825-851 — variant with the T02 module/entry-point framing.*

Checked mutex invariants (2026-09-07): T02 implements `CheckedMutexInvariantV1`
in a new `crates/qcore-core/src/mutex_invariant.rs` with its own entry point,
`check_mutex_invariant_program`. `check_concurrent`'s body, its 26 pinned tests
and `proof/QCore/Concurrency.lean`'s `Lifetime` namespace are byte-identical;
`TrustedExplicitTrace` still issues `GuardFootprintAuthority` and
`SynchronizedCustodyEdges`, and the two premises are discharged only inside the
new profile, over supplied traces. Generation-indexed mutexes follow V01's
destruction rules; `CreateMutex` fixes its declared footprints and parks the
creator's `FULL_ACCESS` share, so protected footprints have one protector and no
fallback authority. Pointwise conservation over exact interval atoms is checked
at every step. Fresh guards key all protected access, writes require an open
invariant, and unlock/error exit require a closed one with no live borrow,
dropping every guard-scoped fact. `SyncWitness` edges are derived and compared,
never believed. New diagnostics occupy `Q610`-`Q623`; carried-over structural
refusals keep `Q600`. Certificates name exact generations and admitted invariant
transitions, and an independent recheck rederives them and refuses either
trusted premise through both the typed set and the plain-string route. 48 new
tests (898 workspace total), 46 provider tests, 43 Python tests, both probes and
all standing gates pass; a new `QCore.Concurrency.MutexInvariant` Lean namespace
and `AuditMutexInvariant.lean` bring `check_lean_audits.py` to 21 audits with all
18 designed theorems proved, no new axiom and no placeholder. Two obligations are
narrowed and named open: general `step_preserves_declared_shape` and the
pointwise-atom refinement of the abstract conservation total. Evidence:
`docs/reports/remaining-t02/README.md`. Source lowering (T03), CT mapping
(11/50 unchanged), lock-order analysis, condition variables, rwlocks, try/timed
locks and any raw promotion remain out of scope and open; the four pinned
185-unit artifacts are unchanged by construction, not re-measured.

#### Typed scalar-field access classification (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 1103-1144 — byte-identical in CLAUDE.md 853-894.*

Typed scalar-field access classification (2026-09-07): W4 replaces the lexical
indirect-access gate with a typed premise for one conjunctive whitelist. A
statement is a resource-projection no-op only when it carries no call, no `&`,
`*`, `[` or `]`, every access is a direct `base -> field` or `base . field` on a
declared single-star record pointer or record value, the record is a `struct`
defined exactly once whose every member declaration was read, the field's
declared type is a non-pointer, non-array arithmetic or enumeration type without
`volatile` or `_Atomic`, the access does not continue, every other identifier is
an enumeration constant or a declared scalar variable, and the base is a name
this function never binds to a resource identity. That last premise was added
after an implementation of C00's recommendation turned a read through a released
pointer into `Safe`; the existing fail-closed pin caught it and was not weakened.
Unions, bitfields, nested anonymous aggregates, function-pointer members,
pointer and array fields, chains, subscripts, casts, address-of escapes and
call-bearing statements all keep their existing `Unknown` and diagnostics, and a
declaration the reader cannot read refuses every access in its function. The
premise is read from declarations in the same preprocessed token stream rather
than a separate Clang AST, so W4 derivation, W4 validation and certificate
rederivation repeat the classification from the certificate's own content; the
deviation from C00's recommendation and its cost are recorded. Branch, loop and
return conditions, and C02-P borrowed pointer-field reads, stay out of scope.
A held-out generic fixture family checks two admitted owned lifetimes, three
newly surfaced violations (leak, double release, release after escape), a
caller whose classified callee stops releasing, twelve ambiguous accesses that
each keep the existing diagnostic, and four declaration substitutions the
certificate recheck refuses with their measured reason. The canonical CMetrics
audit changes exactly one function: `cmt_counter_allow_reset` moves Unknown to
Safe in the resource projection and its unsupported diagnostic disappears
(36 to 35). `cmt_counter_create` is unchanged because its `calloc`-bound base is
refused, and the six source-contract Unknown functions remain Unknown on their
other blockers, as C00 predicted. The raw unit remains provisional Unsafe with
one checked function contract and six Unknown; no unit is promoted. The pinned
185-unit corpus cannot be re-audited here — the developer checkout has moved off
`66910c10` and the pin gate refuses — so the four archived artifacts are left
untouched and their re-measurement and re-pinning is a named follow-up, where
any newly surfaced provisional Unsafe unit must be recorded. Thirty-one Lean
theorems cover the neutral instruction and the whitelist, including its
non-vacuity and the released-base refusal; the reader's type extraction and C
effective types remain explicit premises. 937 workspace tests (36 new), 47
provider tests, 43 Python tests, 20 Lean audits and all standing gates pass.
CT coverage remains 11/50 mapped and the contract corpus 59 modules. Evidence:
`docs/reports/remaining-c02/README.md`.

<!-- migrated-section:end -->
