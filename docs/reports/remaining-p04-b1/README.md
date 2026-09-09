# P04-B1 — the cleanup-outcome contracts declare their referents

Workstream W5 (Providers), follow-on **P04-NEXT-B1**. Bounded increment: the
two ChunkIO **cleanup-outcome** contracts now declare the vocabulary their
`CleanupProfile` binds — the field inventory with C types and reset values, the
syscall and helper names, the record pointer type and the ordered syscall
arguments — and the checker refuses a profile that is not exactly that
declaration.

This card **declares**. It does not change what any accepted input decides, and
it does not move the replay transport, which remains **P04-NEXT-B2**.

- Worktree cut from `main` at **`0f6d66c`** (wave-9 consolidation); branch
  `wave10-p04-b1`.
- Libevent submodule initialised at the pinned revision.
- No checker rule, diagnostic code or verdict changed; no raw unit is promoted.

---

## 1. The blocker being repaired

P04-NEXT-AB §6.3(a) measured the blocker and pinned it in code rather than
prose. Quoting its finding: the destructor family could transport its declared
owned-field inventory because `memory_backend.qcore` *declares* it —
`declared_owned_fields` extracts exactly that set from the contract's
`requires owned_or_null__…` clauses, and the core refuses a projection naming
any other set. The cleanup-outcome contracts declared nothing of the sort. A
pinned whole-identifier census
(`crates/qcore-verification/tests/call_use_dependents.rs`) asserted that
neither contract named `fd`, `map`, `alloc_size`, `map_truncated_warned`,
`path`, `close`, `munmap`, `unlink`, `__errno_location`, `cio_errno_print`,
`char`, `size_t` or `owned_or_null__`, with comments stripped so that
`filesystem_delete.qcore`'s header *prose* about the fd/map fields could not
pass for a declaration.

So the `CleanupProfile` was Rust-only declared data, and it is custody-bearing:
its `ResetValue`s assert what a successful syscall leaves behind (`fd = -1`,
`map = NULL`, `alloc_size = 0`). Putting that on a wire with nothing on the
contract side to cross-check it against is exactly the widening P04-NEXT's
closed-callee-table discipline exists to prevent.

That census test carried its own instruction: *if a future contract revision
declares this vocabulary, this test fails and the blocker must be re-evaluated
rather than inherited*. This card is that revision, and §6 records the flip.

---

## 2. The declared vocabulary

Each clause is one identifier, so it parses as an ordinary contract premise —
and, decisively, it is **evaluated** as one. `qcore-contract`'s predicate
interpreter reports `missing observation: NAME` for any identifier the checker
does not supply a value for, so a vocabulary clause cannot be added to a
contract without the reader recognising it. That is the fail-closed tie between
declaration and checker, not a convention.

```text
cleanup_vocabulary_v1                       the interpreter selector
cleanup_function__NAME                      the pinned C function
cleanup_root_type__TYPE                     the record pointer type
cleanup_syscall__NAME                       the external cleanup callee
cleanup_syscall_signature__RET[__ARG]*      its exact declared signature
cleanup_presence_field__NAME                the eligibility field
cleanup_field__INDEX__NAME__TYPE__RESET     one inventory entry, in order
cleanup_argument_field__INDEX__NAME         one syscall argument, in order
cleanup_mapping_field__NAME                 guarded-delete eligibility only
cleanup_errno_helper__NAME                  the errno accessor
cleanup_report_helper__NAME                 the diagnostic helper
```

`cleanup_vocabulary_v1` mirrors W1's `resource_contract_v1` marker discipline:
it selects the interpretation, and its absence is a refusal, never a fallback
to the caller's profile.

### 2.1 Spelling, exactly

A payload after a fixed prefix is the C identifier **verbatim**, so
`__errno_location` really does read `cleanup_errno_helper____errno_location`.
That is exact rather than pretty, and it is preferred to an escape the reader
would have to decode; it is pinned by a Lean theorem and by a Rust test.

C types travel through a closed scalar table plus a single-pointer `struct`
rule:

| C spelling | token |
|---|---|
| `int` | `int` |
| `char *` | `char_ptr` |
| `unsigned long` | `unsigned_long` |
| `size_t` | `size_t` |
| `void *` | `void_ptr` |
| `const char *` | `const_char_ptr` |
| `struct TAG *` | `struct_TAG_ptr` |

No token contains `__`, and none begins with `struct_`, so `__` stays an
unambiguous separator inside a field clause or a signature and the record rule
cannot collide with a scalar row. A record tag containing `__`, beginning with
a digit or carrying a non-identifier byte has **no** token, so the spelling
never becomes ambiguous by widening. Arrays, function pointers, double
indirection and typedef spellings are refused, not approximated: widening the
family is a deliberate table edit.

Reset values spell `null`, `integer_0` and `integer_m1` — negative magnitudes
take the `m` form so the token stays an identifier.

Field and argument names are single segments (nonempty, no `__`), which is what
makes the indexed clauses split back exactly. Indices are duplicate-free and
contiguous from zero; a gap or a repeat is a refusal.

### 2.2 What the three policies now declare

| Policy | function | syscall / signature | presence | fields (order, type, reset) | arguments | mapping |
|---|---|---|---|---|---|---|
| `native_close_outcomes` | `cio_file_native_close` | `close`, `int (int)` | `fd` | `fd:int:-1` | `fd` | — |
| `native_unmap_outcomes` | `cio_file_native_unmap` | `munmap`, `int (void *, size_t)` | `map` | `map:char *:NULL`, `alloc_size:unsigned long:0`, `map_truncated_warned:int:0` | `map`, `alloc_size` | — |
| `native_delete_outcomes` | `cio_file_native_delete` | `unlink`, `int (const char *)` | `fd` | `fd:int:-1`, `map:char *:NULL`, `path:char *:NULL` | `path` | `map` |

All three declare `__errno_location` and `cio_errno_print` as helpers.
`cleanup_mapping_field__` exists only on the delete policy: it is the
guarded-delete eligibility field, which is not a `CleanupProfile` member and is
bound separately, so the close/unmap policies must not declare one and the
delete policy must. Both directions are refused by test and by theorem.

### 2.3 The reader and the binding

`crates/qcore-verification/src/cleanup_vocabulary.rs` (new) reads a policy's
declaration back under the closed grammar, total or reject, and
`DeclaredCleanupVocabulary::bind` refuses unless the supplied `CleanupProfile`
**is** the declared shape, member for member. `check_cleanup_outcomes` and
`check_guarded_delete_outcomes` call it before any path is replayed, then
supply every declared clause as an observation with value `1` — checked facts,
not assumed premises, because the binding has already established that the
declared shape is the profile the replay then checks against the pinned source.

The `CleanupCertificate` and `GuardedDeleteCertificate` structs, their fields
and their field order are **unchanged**, so the serialized shape the Cycle 19
chunk-contract artifact carries could only move where the embedded contract
text itself moved (§5).

### 2.4 Exactly what the checker binds, no more

The declaration names the ten `CleanupProfile` members plus the guarded-delete
mapping field. Two things the checker binds are deliberately **not** declared,
and are recorded as open rather than quietly included:

- the errno helper's `int *(void)` signature and the report helper's
  `int (int, const char *, int)` signature are literals inside `Replay::run`,
  not profile data, so a declaration of them would have no transported
  counterpart (**P04-B1-HELPER-SIG**);
- `profile.policy` is bound by construction — it names the declaration being
  read — so declaring it would be self-referential.

---

## 3. What this establishes, and what it does not

**Establishes.** The profile is contract-derived rather than caller-invented:
there is now a machine-checkable contract-side referent for every field name,
C type, reset value, syscall name, helper name and argument position that a
transported `CleanupProfile` would carry. B2 therefore has something to
cross-check a projection against, which is what §6.3(a) said it lacked.

**Does not establish.** Anything about the OS. A declared `close`, `munmap` or
`unlink` is a name the pinned source must really call; whether a descriptor was
released, a mapping torn down or a file deleted is outside every claim here,
exactly as before. The contracts' `requires` premises (live exclusive record
provenance, valid syscall argument identity, returning external-call frames,
compiled ABI bindings) stay declared premises. No verdict changed and no raw
unit is promoted.

**Does not close B2.** P04-NEXT-AB recorded *two* independent blockers. (a) is
closed by this card. (b) is untouched: a producer would still have to
reimplement a symbolic replay evaluator — `field_writes`, `fields_preserved`
and `fields_reset` are functions of a symbolic field state evolved by a
nine-constructor value lattice over four (respectively five) sampled
`(entry, syscall outcome)` prestates — which by P04-NEXT's own rule is a
separate capability definition, not a widening of the third vocabulary. B2 is
still gated on that.

---

## 4. Evidence

### 4.1 The declaration is checkable against the pinned source

`crates/qcore-verification/tests/cleanup_vocabulary.rs` (5 tests) drives real
Clang over held-out C — a `struct sample_file` family unrelated to ChunkIO, in
both shapes the checker binds: a one-field descriptor release and a three-field
mapping release with two ordered syscall arguments.

**Positive controls.** Both shapes check across all four entry/outcome cases,
with the success case's `field_writes` pinned (`["handle"]` and
`["view_bytes", "view", "warned"]`). In every row the profile is **derived from
the edited declaration**, so what is measured is whether the declared referent
is bound by the source, not whether the caller matched the contract.

**13 source-binding refusals**, count pinned, each admitted by the binding of
§4.2 and refused by the replay:

| Row | What it declares |
|---|---|
| 1 | a field absent from the record |
| 2 | a present field at the wrong C type |
| 3 | a syscall the body never calls |
| 4 | the syscall at the wrong signature |
| 5 | an errno helper the body never calls |
| 6 | a diagnostic helper the body never calls |
| 7 | another function's name |
| 8 | a record type the source does not use |
| 9 | an eligibility field the body does not test |
| 10 | an inventory wider than the body resets |
| 11 | an inventory narrower than the body resets |
| 12 | the syscall arguments in the wrong order |
| 13 | a reset value the body does not write |

A fourteenth test drives an undeclared premise through the real route and
asserts the interpreter's `missing observation` refusal — the fail-closed tie
of §2, measured rather than argued.

### 4.2 A profile that is not the declaration is refused

**17 profile mutations**, count pinned, over the real `native_delete_outcomes`
declaration: the function, root type, syscall, signature, presence field; a
renamed, retyped, reset-changed, second-reset-changed, reordered, duplicated,
widened and narrowed inventory; a renamed and a widened argument list; and
either helper. Each is asserted to really differ before it is submitted. Plus:
the mapping field refused as absent, as a different field and as present on the
close policy; and a profile whose C type has no spelling in the closed grammar
(`int (*)(void)`) refused outright rather than approximated.

### 4.3 Declarations the closed grammar refuses whole

**20 contract-text mutations**, count pinned, each editing the real pinned
`filesystem_delete.qcore`: marker removed; a duplicated clause; two syscalls;
an unknown `cleanup_` clause; a field index gap, a duplicate index and a
non-canonical index; a field without an index; an unknown C type token; an
unknown reset token; a reset out of range; an unknown signature result type; an
empty payload; a missing root type, function, presence field and report helper;
no field inventory; no arguments; and a record tag carrying the separator. Two
further rows: a contract with no such declaration at all — every earlier
revision of these two files — is refused rather than defaulted, and a policy
the contract does not declare is refused by name.

### 4.4 A second, unrelated source set

`crates/qcore-verification/tests/fixtures/generic_source/cleanup.qcore` — the
held-out generic route that already exercised `check_cleanup_outcomes` outside
ChunkIO — now declares its own vocabulary
(`struct_sample_handle_ptr`, `sample_release`, `sample_errno`,
`sample_report`). It was **not** adapted to make a test pass: it failed closed
the moment the gate landed, which is the gate working, and the declaration it
now carries is checked against its own C by the same replay.

### 4.5 Lean

`proof/QCore/CleanupVocabulary.lean` and `proof/AuditCleanupVocabulary.lean`
add **37 audited theorems** in a new `QCore.CleanupVocabulary` namespace, with
**no new axiom** — every one depends on `propext`, `Classical.choice` and
`Quot.sound` only, the allowed closure. Two layers:

- *the spelling layer* (15): the ten prefixes are a prefix code (no prefix is a
  prefix of another) and every one ends with the separator; the scalar tokens
  and their C spellings are pairwise distinct, separator-free, and never look
  like a record token; reset and index tokens are separator-free; a name or
  record tag carrying the separator, a tag starting with a digit, an empty
  payload and an unspellable index each have **no** spelling; and the real
  clauses — `__errno_location` verbatim included — spell exactly as the pinned
  contracts write them.
- *the clause layer* (22): reading is single-valued; an unreadable declaration
  refuses **every** profile; binding admits exactly the declared shape; a
  missing or repeated marker refuses. Over an exhaustively enumerated 40-shape
  domain whose size and variety are pinned, spelling round-trips, is injective,
  and binds one profile per declaration, while dropping the marker, duplicating
  a field clause, opening an index gap and declaring a second eligibility field
  each refuse across the whole domain. On the real unmap shape, the argument
  order, each reset value, the inventory size, the mapping field, each field
  type and each named referent are individually load-bearing.

Two boundaries are recorded rather than modelled, in the module header and the
audit file: that `String::starts_with`/`split("__")` realise the segmentation
the spelling theorems constrain (the Rust tests exercise the real one, including
`__errno_location` and the `struct_cio__file_ptr` refusal), and — decisively —
that a declared name is *bound by the pinned C source*, which is not a property
of this relation at all but is decided by the replay and measured in §4.1. Two
model bounds are stated rather than hidden: an index and a reset magnitude are
single digits in the model (the real inventories have three fields and reset to
`-1`, `0` or `NULL`), refused rather than truncated.

---

## 5. Every moved digest, itemised

### 5.1 The 185-unit corpus

Re-measured before and after **from this worktree on both sides**, through the
canonical pinned route on the prepared pinned clone (Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit `/usr/bin/clang` 21.1.8),
with one provider package set (`build/cycle19/provider-packages-wave9`,
ChunkIO 0.7.0) used unchanged on both sides so package identity contributes no
delta:

```sh
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=<packages>/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone>/fluent-bit-clang-source \
  <clone>/monkey/monkey-build/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

Both runs exit 0 with zero gateway failures. Headline, unchanged:
**185 units — 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only.**

| Artifact | Before → after |
|---|---|
| `resource-diagnostics.json` (11 MB) | **byte-identical** |
| `monkey-source.json` | **byte-identical** |
| `resource-diagnostics.source-effects.json` | **byte-identical** |
| `resource-diagnostics.contracts.json` | **byte-identical** |
| `resource-diagnostics.chunk-contracts.json` | 18 leaves moved |

**Every moved leaf, named.** All 18 are `contract_source` and nothing else — no
leaf was added or removed, and no verdict, case, profile, diagnostic or
identity leaf moved:

| Count | Field | Carrying |
|---|---|---|
| 13 | `…contract_source` | `filesystem_outcomes.qcore` (2199 → 4639 bytes) |
| 5 | `…contract_source` | `filesystem_delete.qcore` (1039 → 2733 bytes) |

The complete path list is
[`artifacts/corpus-leaf-diff.txt`](artifacts/corpus-leaf-diff.txt); the two
hash sets are [`artifacts/before.sha256`](artifacts/before.sha256) and
[`artifacts/after.sha256`](artifacts/after.sha256), and the after-run log is
[`artifacts/corpus-after-run.log.gz`](artifacts/corpus-after-run.log.gz).

**This movement is the card, not a side effect.** `CleanupCertificate` embeds
the contract text it evaluated, so a contract that declares more carries more.
That is the documented behaviour of a contract-dependency digest: a changed
contract retires the identity that bound it, and a witness bound to the old
text is refused. Nothing else moved, which is the measurement that says the
declaration changed no decision.

**Harness validation.** The *before* run reproduces the archived wave-9
baseline (`docs/reports/wave9-corpus/artifacts/`) byte-identically on
`diagnostics.json`, `chunk-contracts.json` and `source.json`;
`contracts.json` and `source-effects.json` differ in the recorded
measurement-directory provenance class only, because this worktree is not the
directory the archive was measured from. That is location provenance, not
drift, and is settled by the before/after comparison above, which was run from
one directory.

**The archived baseline is not re-pinned by this card.** `chunk-contracts.json`
will move by these 18 leaves at the wave's corpus consolidation; the
re-measurement above is the itemised prediction for it.

### 5.2 The configured audits

The canonical CMetrics audit **could not be re-measured here**: the developer
Fluent Bit checkout has moved off the pin (`864c3d97…` where the profile pins
`66910c10…`) and the audit's own pin gate refuses to run — the same environment
condition C02 and G01 recorded, reported as a finding rather than worked
around.

Disjointness is established by measurement instead. A census of every
`include_str!` of the two edited contracts finds exactly two production sites,
both in the Cycle 19 compatibility adapter
(`compiler/src/verifier_plugins/chunkio/file_outcomes.rs` and `file_delete.rs`);
the other four are test files. `qcore-audit` depends on
gateway/contract/schemas/verification/host and **not** on `qcore-compiler`, and
`qcore-verification`'s production code embeds neither contract, so the
configured-audit routes cannot see these bytes. The chunkio-core, libevent-core
and CMetrics configured-audit tests in the workspace suite pass unchanged.

### 5.3 Contract-corpus pins

Three deliberate per-file clause-count updates, each with its reason recorded
beside the row in `compiler/tests/qcore_contract_corpus.rs`. No `ensures` clause
moved in any of them:

| File | Policy | Before | After | Added |
|---|---|---|---|---|
| `filesystem_outcomes.qcore` | `native_close_outcomes` | 16 | **26** | 10 |
| `filesystem_outcomes.qcore` | `native_unmap_outcomes` | 16 | **29** | 13 |
| `filesystem_delete.qcore` | `native_delete_outcomes` | 12 | **25** | 13 |

**The corpus stays 60 modules** — no new module was warranted, and none was
added: the declaration belongs on the policy whose profile it binds, where the
predicate interpreter evaluates it. The contract loader's rejected-declaration
count is unmoved at 145: that count is per declaration, and this card adds
clauses to three existing declarations without adding one.

---

## 6. The P04-NEXT-AB tripwire, flipped deliberately

`the_cleanup_outcome_contracts_declare_no_field_or_reset_vocabulary` asserted
the *absence* of the referent and instructed that a revision declaring it must
fail the test and force a re-evaluation. It is **inverted, not deleted**, and
renamed `the_cleanup_outcome_contracts_declare_their_field_and_reset_vocabulary`:
it now asserts that all three policies carry the selector and the profile
vocabulary, that the reader returns the shape the checker binds, and — still —
that the destructor contract keeps its own inventory and that the check is not
vacuous. Its docstring records that blocker (a) is closed and blocker (b) is
untouched. The comment-stripping helper is kept with its reasoning intact,
because the prose-versus-declaration distinction is exactly what changed.

---

## 7. Gate

Run in this worktree on Linux with Clang 21.1.8 available:

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1893 passed, 0 failed, 4 ignored** — against wave 9's 1888/4, exactly +5 (this card's five new tests) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | **49 audits passed** (48 → 49), no new axiom |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | **75 passed, 0 failed** |
| `python3 -B providers/package.py build` + `qcorec plugins verify` × 5 | all five built and verified (exit 0); ChunkIO stays 0.7.0 |
| `python3 -m unittest discover -s scripts/tests` | 124 passed, **1 failed** — see §7.2 |
| 185-unit corpus, before vs after, same directory | 18 leaves, all `contract_source` (§5.1) |

### 7.1 Load-flaky, observed and recorded

`vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
(`compiler/tests/cycle20_verifier_plugins.rs`) asserts a 500 ms wall bound and
failed while the Lean gate and the corpus audit were saturating the machine; it
was measured at ~2 s under that load, and it **passes in the clean full run
above**. This is the recorded P02/vp04 flake class. Its assertion is
deliberately **not** weakened. One provider run also
failed `real_packages_preserve_unknown_and_reject_mutations` with
`ExecutableFileBusy` when two provider suites overlapped; rerun alone it passes,
and the 75/0 figure above is from the clean rerun.

### 7.2 One Python failure the card cannot fix, by instruction

`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`
fails on `validation.lean_audits`: the committed
`docs/reviews/r0_3/acceptance_matrix.json` carries **48** and the freshly
derived value is **49**, because this card adds one audited Lean module. The
card's constraints forbid editing that file, so the number is left for the
integrator to reconcile — by regenerating the matrix (or supplying `--gate`)
at the wave's close, exactly as wave 9 did. **Integrator action item:**
`validation.lean_audits` 48 → 49. Nothing else in the matrix moves from this
card: the contract corpus stays 60 modules, CT coverage is untouched, and
`qcore-schemas` stays 8.0.0.

---

## 8. Open items and named boundaries

- **P04-NEXT-B2** — the symbolic cleanup-replay vocabulary (the fourth
  transported vocabulary) and the move of `file_outcomes` / `file_delete`
  through the process boundary. This card supplies the contract-side referent
  that §6.3(a) said was missing; §6.3(b) is unchanged and is what remains, and
  the dependent mechanism it needs (`PrerequisiteIdentity` /
  `revalidate_prerequisite`) already exists with no production consumer
  (P04-NEXT-AB-CONSUMER).
- **P04-B1-HELPER-SIG** — the errno and report helper *signatures* are checker
  literals, not profile data, and are therefore not declared. Declaring them
  would add a vocabulary item with no transported counterpart; recorded rather
  than included (§2.4).
- **P04-B1-CALLEE-TABLE** — the closed callee table P04-NEXT-AB's third
  vocabulary is taken under is still a caller assertion recorded on the import,
  not derived from a contract. This card declares the *outcome* family's
  referents; the attempt family's callee table is the same class of gap and
  stays open, as P04-NEXT-AB recorded.
- The declared C type grammar is closed to six scalar spellings plus
  `struct TAG *`. A second source set needing another spelling widens it by a
  deliberate table edit; nothing infers one.
- The Lean model bounds indices and reset magnitudes to one digit and models
  the string segmentation abstractly; both are stated in the module header and
  the audit file (§4.5).
- The in-process adapter for these two capabilities is retained (invariant 3);
  the canonical CMetrics re-measurement is blocked by the moved developer
  checkout (§5.2).
