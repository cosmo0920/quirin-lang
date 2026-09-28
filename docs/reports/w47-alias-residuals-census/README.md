# W47-ALIAS-RESIDUALS-CENSUS

Plan P6 of the [unsupported-coverage queue](../../plans/unsupported-coverage/README.md):
the measurement the `unknown_alias` plan needs. Measurement only: no analyzer,
contract, gateway-rule or Lean change, and the corpus stays byte-identical.
Base `3ba72320` (local `main`, "Merge runtime imports and explicit contract
checks after complete gate"); `main` had not moved when the card started. The
prediction was frozen before any classification ran, in
[PREDICTION.md](PREDICTION.md) (commit `9fd3681c`).

## 1. The integrated wave-47 baseline (BEFORE)

`main` at `3ba72320` merged P3 (`561e1dbf`, with a conflict resolved in
`typed_access_path.rs`) and the runtime-import work after P3's AFTER was
measured at `9b2ac682`. This BEFORE is therefore also the wave's first
post-merge measurement. It is archived gzipped with a manifest (raw and
sorted-row SHA-256s) as the **integrated wave-47 baseline**:
[artifacts/baseline/corpus/](artifacts/baseline/corpus/) and
[artifacts/baseline/system/](artifacts/baseline/system/).

**Commands.** The compiler example `run_audit.py` drives was retired by
`b64be69e` ("Retire legacy pinned example after exact archive parity",
[audit-pinned-replay-cli](../audit-pinned-replay-cli/README.md)); its live
replacement is `quirin-audit pinned` through `scripts/run_pinned_audit.py`. The
corpus ran as `scripts/run_pinned_audit.py OUT 3ba72320 --inputs-root
build/cycle19 --jobs 5` and the system slice as
`w46-contract-gap-expansion/scripts/run_system.py OUT --jobs 5` at `046accf6`,
each with `QCORE_TYPED_PATH_PARITY` inside its output directory
(`private/tools/measure.sh` in this worktree). Both exit 0: corpus 00:26:29 to
00:34:41 UTC, system 00:34:41 to 00:35:48 UTC on 2026-09-28, on a host running
two other cards' arbiters.

**Identity against the P3 AFTER** ([baseline-identity/](artifacts/baseline-identity/)):

| Artifact | Corpus | System |
|---|---|---|
| diagnostics | byte-identical (`baseline_identity.py` exit 0, `verdict_bearing_identical`) | equal after masking digests (below) |
| chunk contracts | byte-identical | byte-identical |
| Monkey source | byte-identical | n/a |
| sorted typed-path parity rows and contexts | byte-identical | byte-identical |
| contracts sidecar | equal after rename and root alias, digests masked | same |
| source effects | equal with digests masked (3 rows' digests) | byte-identical |

Every non-identical artifact is attributed to one named commit on
`9b2ac682..3ba72320`, **EXT-02-CONTRACT-EXTENSIONS** (the `.qcore` to `.qv`
rename of every contract module; [report](../extension-contract-physical/README.md)).
`scripts/sidecar_attribution.py` shows it: after the checkout-root alias and
the `.qcore` to `.qv` spelling on both sides, all 54 corpus contract source
texts are equal. The one textual residue is each included module's recorded,
blanked `include` line, three spaces shorter per `.qv` include. Every other
difference is a digest over the renamed paths or texts: module, lock, pin,
task, obligation, build and configuration digests. Every structural field is
equal. The system slice's diagnostics differ only in each unit's
`compile_configuration` contract-manifest, contract-source and inventory
digests, and its invocation record only in `--jobs` and the output path.
Nothing is unattributed.

| Measure | P3 AFTER (`9b2ac682`) | BEFORE = baseline (`3ba72320`) |
|---|---:|---:|
| Corpus functions Safe / provisional Unsafe / Unknown | 507 / 9 / 4,318 | **507 / 9 / 4,318** |
| Corpus units Safe / Unsafe / Unknown / data-only | 6 / 8 / 167 / 4 | same |
| Corpus unsupported / unknown_alias / leak / summary_conflict / Pending | 36,262 / 7,440 / 34 / 1 / 0 | same |
| System unique functions Safe / Unsafe / Unknown (+3 not projected) | 33 / 2 / 199 | same |
| System configuration rows Safe / Unsafe / Unknown | 63 / 4 / 381 | same |
| System unsupported / unknown_alias / counted_user | 2,908 / 985 / 4 | same |

**Census build is inert.** The `indirect-census` build (`quirin-audit` with
`--features qcore-c-gateway/indirect-census`, `target/census`) ran both scopes
with the access-path unit stream (`QCORE_ACCESS_PATH_CENSUS`) and the parity
stream on (`scripts/run_census.py`, one unit at a time). Its diagnostics, chunk
contracts, Monkey source, contracts and source-effects sidecars are
byte-identical to the default build's BEFORE in both scopes. Its sorted parity
rows are byte-identical too.

## 2. Method

`scripts/alias_census.py` reuses W46-ACCESS-PATH-CENSUS's `access_census.py`
by import, not copy. Each analysed unit's exact text goes to Clang with the
gateway's typed-AST argument vector. The script walks the same constructs and
access paths, and joins document units to analyses the same way. The system
slice analyses each unit twice, so there a candidate must also contain every
leaf line inside its function's extent. All 180 function-bearing corpus units
and all 15 system units join, with 0 unmatched functions and 0 Clang errors.

An `unknown_alias` leaf carries only `unresolved resource identity `V`` and an
internal line. Its **class** is read from the typed AST of its own function:

- the spelling of `V`: a `place/peer` composite is C02's release alias, and
  `__qcore_literal__` is a literal actual;
- what `V` declares, and its Clang type;
- for a pointer object, every value the function assigns it (its *origins*:
  a callee and whether it is contracted, corpus-defined or external; a path
  read; a copy; an address; a conditional; arithmetic; a statement
  expression);
- whether a controlling expression null-tests `V` after the last origin
  before the leaf;
- whether an unsupported leaf of the same function lies between that test and
  the leaf (a poisoning refusal, rule 10a);
- every callee whose actuals mention `V`, which is the lexical never-bound
  closure's view;
- `V`'s use at the leaf line.

Control flow is not followed: "guarded" is lexical, so a label joining a null
and a non-null path counts as guarded.

The **discharge group** (`discharge_group`) gathers the leaves one card would
discharge together. Classes that need their own model come first: a
non-pointer name, a global, a peer release, a pointer-valued field store (P5)
or an address-taken pointer. The cause comes next: poisoned, or guarded with
no refusal between. The origin decides the rest. Pointer formals split by
closure reason: mentioned in an uncontracted callee's actuals; mentioned only
in contracted ones; copied (`T *x = formal`); or none of these.

`scripts/functions.py` gives the per-function figures. It counts the Unknown
functions blocked by `unknown_alias` alone, and by alias plus exactly one other
family (P1's vocabulary: IND, COND, RET, UNC, OTHER). It also computes P1's
call-graph closure, reusing W46 `predict.py`'s rule. With a group discharged,
a function freed of every leaf except uncontracted calls to callees that are
Safe or freed is taken Safe; this is the upper bound. The `same-unit` variant
is the existing summary rule. The `cross-unit` variant also accepts a Safe
callee defined in exactly one other unit, which no rule does today. Each gain
is reported beyond the nothing-discharged baseline, which frees 0 same-unit
functions and 3 cross-unit ones.

Pipeline: `scripts/pipeline.sh private/census private/before private/work`.
Every step exits 0, and `publish.py` refuses unless the class and group totals
equal the report's `unknown_alias` count.

## 3. Every leaf accounted for

**Corpus, 7,440 leaves, 0 unclassified** (classes sum to 7,440). By class:
formal with no assignment 2,797; corpus-callee result 961; contracted-callee
result 782; borrowed path read 622; scalar name 393; mixed origin 303;
enumeration constant or undeclared name 267; copy of a name 229;
address-taken pointer 202; address value 200; literal actual 154;
external-callee result 117; array object 102; result of several callee kinds
68; statement expression 59; arithmetic 59; pointer-valued field-store owner
40; unbound local 31; conditional 15; other origin 15; record 8; global
pointer 6; other type 4; constant 3; place spelling 2; peer release 1.

By cause: unguarded 5,271; poisoned by a later refusal 950 (first refusal
after the test: IND 497, UNC 276, COND 98, other 79); guarded with no refusal
between 282; not a pointer object 937. 3,497 leaves are the first of their
(function, name) pair; the rest repeat the same unresolved binding.

**System slice, 985 leaves, 0 unclassified.** By class: contracted result 227,
formal 218, address-taken 106, external result 90, statement expression 82,
borrowed path 67, mixed 42, address value 36, copy 34, corpus result 31,
literal 20, scalar 10, field-store owner 8, global 6, peer release 4,
constant 4. By cause: unguarded 485, poisoned 374, guarded-clean 86, not a
pointer 40. There are 319 first leaves.

Full per-leaf rows: [artifacts/census/corpus/leaves.jsonl.gz](artifacts/census/corpus/)
and [system](artifacts/census/system/). The tables are in
[artifacts/census/tables.json](artifacts/census/tables.json).

## 4. Class table (discharge groups)

Corpus, 4,318 Unknown functions, 1,922 with an alias leaf. The columns are:

- **Excl.:** functions where every alias leaf is in the group.
- **Alone:** functions blocked by the group alone.
- **+1:** functions blocked by the group plus exactly one other family.
- **Closure:** functions freed beyond the baseline, same-unit / cross-unit.
  The closure gain beyond round 0 is 0 in every row.

| Group | Leaves | First | Functions | Excl. | Alone | +1 (by family) | Closure |
|---|---:|---:|---:|---:|---:|---|---|
| formal-handed-uncontracted | 2,350 | 806 | 732 | 399 | 0 | 108 (IND 74, UNC 32, OTHER 1, COND 1) | 0 / 1 |
| non-pointer-name | 930 | 584 | 329 | 83 | **4** | 10 (UNC 4, IND 3, OTHER 2, COND 1) | 4 / 4 |
| poisoned-by-refusal | 900 | 292 | 365 | 130 | 0 | 39 (IND 33, UNC 5, OTHER 1) | 0 / 0 |
| derived-local-unguarded | 746 | 342 | 298 | 72 | 0 | 10 (IND 6, OTHER 4) | 0 / 0 |
| uncontracted-result-unguarded | 705 | 453 | 360 | 147 | 0 | 6 | 0 / 0 |
| borrowed-path-unguarded | 503 | 267 | 236 | 78 | 1 | 14 (IND 12, OTHER 2) | 1 / 1 |
| contracted-result-unguarded | 442 | 193 | 178 | 123 | 0 | 8 (IND 4, OTHER 4) | 0 / 0 |
| guarded-other | 262 | 130 | 167 | 32 | 0 | 9 | 0 / 0 |
| address-taken | 202 | 108 | 85 | 11 | 0 | 2 | 0 / 0 |
| formal-copied | 176 | 175 | 162 | 85 | 0 | 56 (IND 28, UNC 18, RET 7, OTHER 3) | 0 / 1 |
| formal-handed-contracted | 74 | 52 | 47 | 30 | 0 | 18 (IND 13, UNC 5) | 0 / 0 |
| statement-expression-unguarded | 54 | 38 | 35 | 20 | 1 | 13 | 1 / 1 |
| formal-not-handed | 49 | 37 | 35 | 11 | 0 | 3 | 0 / 0 |
| field-store-owner (P5) | 40 | 13 | 25 | 2 | 0 | 1 | 0 / 0 |
| global-pointer | 6 | 6 | 5 | 2 | 0 | 0 | 0 / 0 |
| peer-release | 1 | 1 | 1 | 0 | 0 | 0 | 0 / 0 |

**Alias is almost never the only blocker.** Only **8** Unknown corpus
functions are blocked by `unknown_alias` alone, whatever the classes:

- `non-pointer-name` alone: `cio_chunk_is_file`, `LZ4F_optimalBSID`,
  `net_plain_close`, `rd_kafka_global_cnt_get`;
- `borrowed-path-unguarded` alone: `flb_input_chunk_set_up_down`;
- `statement-expression-unguarded` alone: `cio_chunk_close_stream`;
- with a second alias class: `format_acl_error_message` and
  `LZ4F_getErrorName`.

Discharging every class frees 8 same-unit and 10 cross-unit functions, with
no closure gain. The bound is set by the other families: 352 functions are
blocked by alias plus exactly one other family, namely IND 232, UNC 76,
OTHER 24, RET 10 and COND 10.

**System slice** (199 Unknown unique functions, 104 with an alias leaf; joined
over configurations):

| Group | Leaves | First | Functions | Excl. | Alone | +1 |
|---|---:|---:|---:|---:|---:|---:|
| poisoned-by-refusal | 340 | 47 | 28 | 13 | 0 | 3 |
| formal-handed-uncontracted | 176 | 76 | 37 | 20 | 0 | 4 |
| address-taken | 106 | 28 | 12 | 4 | 0 | 2 |
| derived-local-unguarded | 96 | 27 | 13 | 2 | 0 | 0 |
| statement-expression-unguarded | 68 | 37 | 18 | 8 | 0 | 4 |
| borrowed-path-unguarded | 63 | 30 | 15 | 3 | 1 | 0 |
| guarded-other | 52 | 22 | 14 | 1 | 1 | 0 |
| non-pointer-name | 30 | 14 | 7 | 3 | 0 | 0 |
| uncontracted-result-unguarded | 14 | 8 | 5 | 0 | 0 | 0 |
| formal-handed-contracted | 13 | 9 | 5 | 2 | 1 | 1 |
| formal-copied | 9 | 9 | 6 | 4 | 2 | 2 |
| field-store-owner | 8 | 4 | 4 | 0 | 0 | 0 |
| global-pointer | 6 | 6 | 3 | 0 | 0 | 0 |
| peer-release | 4 | 2 | 1 | 0 | 0 | 0 |

In the system slice 9 functions are blocked by alias alone:

- `flb_engine_evl_set` and `flb_sched_ctx_set` (formal-copied);
- `flb_engine_evl_get` and `flb_sched_ctx_get` (a thread-local global copy);
- `flb_input_chunk_set_up_down`, `flb_task_queue_destroy`,
  `flb_output_task_queue_enqueue`, `flb_output_get_instance` and
  `sanitize_name_component`.

A further 20 are blocked by alias plus one family (IND 14, UNC 6).

## 5. Prediction against outcome

1. BEFORE reproduces the P3 AFTER: **held**. The corpus diagnostics are
   byte-identical, and every other difference is attributed to EXT-02.
   PREDICTION.md expected the same attribution for source effects ("path-derived
   digests"); for contracts it expected only the root alias, but the rename
   also mattered. That is a named commit, not an unattributed difference.
2. The system slice reproduces: **held**, with digests only.
3. The census build is inert: **held** in both scopes.
4. Every leaf is classified: **held**, and better than forecast, with 0
   unclassified against a forecast below 2%.
5. Borrowed path reads and call results make up more than half the leaves:
   **missed**. Together they are 2,550 of 7,440 (34%). The largest class is a
   formal no statement assigns, refused by the never-bound closure (2,797
   leaves); the forecast did not name it.
6. P5 field stores, peer releases, literals and non-object names are each
   below 10%: **held** for field stores (0.5%), peer release (1 leaf) and
   literals (2.1%). The group of all non-pointer names is 12.5%, but it
   joins four classes the forecast listed separately.
7. A few hundred functions alone, no class above about 150, closure below
   round 0: **held in direction, missed in size**. Alias alone blocks 8
   functions, not hundreds, and no class exceeds 4. The closure adds 0.
8. No verdict moves: **held**, since this is measurement only.

## 6. Proposed next cards (the P6 section of the plan)

Ranked by functions freed alone, then alias plus one family, then exclusive
functions. Because alias alone frees 8 functions, every card's function yield
depends on the IND and UNC work running beside it (P3 residuals and P4). The
card list is ranked but should not be read as a promise of Safe functions.

1. **W47-ALIAS-TYPED-SCALAR-NAMES** (High; next free gateway rule after
   10l/10m).
   - Scope: 930 leaves, 4 functions alone plus 2 with a second class, +1: 10,
     exclusive 83; system 30 leaves.
   - Rule: the lowering reads the Clang type of a copy's source, or of a
     decision's operand, from the typed AST P2 already matches. It binds
     nothing for a non-pointer object, an enumeration constant or a literal.
   - Premise to prove: a non-pointer value carries no resource identity,
     except an int at a descriptor-kinded contract position (C04.6), which
     keeps its identity.
   - Frozen-prediction sketch: the 4 alone functions become Safe if no other
     obligation fails. Of the 83 exclusive functions, those whose non-alias
     leaves are gone move; a leaf decrease of about 930 is the ceiling.
2. **W47-ALIAS-FORMAL-COPY** (High).
   - Scope: 176 leaves (175 first), +1: 56, exclusive 85; system 2 functions
     alone (`flb_engine_evl_set`, `flb_sched_ctx_set`).
   - Rule: W43-INDEX-TRAVERSAL's unbound-copy exemption, decided on Clang
     pointer types (including `void *`).
   - Premise to prove: the copy of an unbound pointer is unbound, and the
     formal's never-bound answer is still required.
   - Sketch: the two system setters become Safe, and the 176 leaves go.
     Corpus functions move only where the IND and UNC residuals also go.
3. **W47-ALIAS-CALLEE-BORROWED-CLOSURE** (High; shares the never-bound
   closure with W47-TYPED-ROOT-DECLARATION-READER).
   - Scope: 2,350 leaves (806 first), 732 functions, exclusive 399, +1: 108
     (IND 74, UNC 32); system 176 leaves.
   - Rule: the closure admits a mention at a position a contract or validated
     W4 summary declares `Borrowed`, and a member-address mention (`&(p)->f`,
     the rd_slice and rd_kafka_buf macros).
   - Premise to prove: the callee cannot bind, release or retain `p` through
     that position.
   - Sketch: leaves fall by up to 2,350. Functions move only together with P4
     (the same functions' UNC leaves) and the IND residuals; alone, 0.
4. **W47-ALIAS-NONNULL-ALLOCATOR-RESULTS** (High, provider).
   - Scope: 442 leaves (rd_calloc 222, rd_malloc 124, rd_strndup 34, ...),
     exclusive 123, +1: 8.
   - Rule: non-null result contracts for the wrappers whose own body
     guarantees non-null (librdkafka's asserting rd.h wrappers), checked
     against that body by W4 validation. They are reviewed and pinned at both
     sources in revision mode. A plain libc result stays nullable.
   - Sketch: the leaves go for the named wrappers. Functions move only where
     nothing else blocks them.
5. **Not new cards; sized in W47-ALIAS-FLOW-RESIDUALS.** These groups are
   discharged by other work or are the source's own obligations:
   - poisoned-by-refusal, 900 leaves: IND/COND refusals under
     W47-ACCESS-PATH-POSITION-RESIDUALS, UNC under P4;
   - uncontracted-result-unguarded, 705: P4 callee closure and contracts;
   - borrowed-path, derived-local and statement-expression unguarded,
     1,303 together: dereferences the C code does not null-test;
   - guarded-other, 262: releases and ambiguous joins;
   - address-taken, 202;
   - field-store-owner, 40: P5;
   - formal-handed-contracted and formal-not-handed, 123;
   - global-pointer and peer-release, 7.

## 7. Gate

[gate.json](gate.json), exits in [gate/](gate/) (`scripts/gate.sh`, each exit
in its own file, no pipe). The Rust part ran at `04a34176`. The later records
commit changes only `docs/`.

- submodule: exit 0.
- `run_test_binaries.py`: exit 0, **339 units, 352 summaries; 6,122 passed,
  0 failed, 11 ignored**
  (`private/test-binaries/20260928T011213Z-298718/summary.json`).
- `cargo clean --profile dev`, fmt, clippy, `reevaluation_probe` and
  `reevaluation_followup`: exit 0 each.
- `check_lean_audits.py`: exit 0, **216** audits (no Lean change).
- `clang --version`: 21.1.8, recorded here.
- Records part at `8b974790`: `unittest discover -s scripts/tests` exit 0
  (**365** OK); `build_docs_index.py --check` exit 0.

No `providers/`, `crates/` or `proof/` path is touched.

## 8. Follow-ups

- Retired: none (a measurement card).
- Registered ([followups.json](followups.json), `scripts/update_ledger.py`,
  then `build_matrix.py`), ledger 523 → 528:
  - W47-ALIAS-CALLEE-BORROWED-CLOSURE
  - W47-ALIAS-TYPED-SCALAR-NAMES
  - W47-ALIAS-FORMAL-COPY
  - W47-ALIAS-NONNULL-ALLOCATOR-RESULTS
  - W47-ALIAS-FLOW-RESIDUALS
- Amended: W47-ACCESS-PATH-POSITION-RESIDUALS (950 poisoned leaves by first
  refusal; 232 alias+IND functions) and W47-TYPED-ROOT-DECLARATION-READER (the
  closure's 2,350 formal leaves, registered separately).

## Files likely to conflict

`docs/STATUS.md`, `docs/plans/unsupported-coverage/README.md` (P6 row and
section), `docs/development/increments/wave-47.md`,
`docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
