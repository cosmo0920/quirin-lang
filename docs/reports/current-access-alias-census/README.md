# CURRENT-ACCESS-ALIAS-CENSUS: the IND, ALIAS, COND and RET leaves of the user's `current` run, by shape

Card CURRENT-ACCESS-ALIAS-CENSUS (round 26 D), 2026-11-07, base `7b726587b`
(origin/main). It continues
[unsupported-coverage](../../plans/unsupported-coverage/README.md) P1
([W46-ACCESS-PATH-CENSUS](../w46-access-path-census/README.md)) and P6
([W47-ALIAS-RESIDUALS-CENSUS](../w47-alias-residuals-census/README.md)) on the
route they did not measure: the user's `quirin-audit current` run.

This is a **records-only** card. It changes no analyzer, contract or Lean code and
no verdict; its own [PREDICTION.md](PREDICTION.md) predicts no row moves.

## 1. Method

- **Run.** The census build (`cargo build --release -p qcore-audit --features
  qcore-c-gateway/indirect-census`, 2m15s) ran the user's `current` argv (186
  configurations of the production compile database at Fluent Bit `046accf6`, the
  Monkey database and source report, `--jobs 8`) with the access-path unit stream
  and the typed-path parity stream on
  ([scripts/run_census.py](scripts/run_census.py)). A census stream analyses one
  unit at a time: 01:49:44 wall clock (08:59:16 to 10:49:00 UTC), through
  `gate_slot.py audit`, whose receipt notes a peak resident set of 7.9 GB against
  the 7 GiB admission budget
  ([artifacts/census-run/census.slot.json](artifacts/census-run/census.slot.json)).
  Output stayed under the worktree's `private/`; nothing under `build/` was written.
- **Reuse, no new scanner.** [W46's `access_census.py`](../w46-access-path-census/scripts/access_census.py)
  gives each analysed text to Clang with the gateway's own argument vector and walks
  constructs and typed paths. [W47's `alias_census.py`](../w47-alias-residuals-census/scripts/alias_census.py)
  classifies every `unknown_alias` leaf and matches configurations to analyses
  (several analyses can carry one file). [scripts/shape_census.py](scripts/shape_census.py)
  joins the IND/COND/RET leaves through that same matching, [scripts/rank.py](scripts/rank.py)
  ranks and runs the closure, [scripts/reasons.py](scripts/reasons.py) reads the
  parity stream's refusal reasons, and [scripts/cooccur.py](scripts/cooccur.py) lists
  each shape's next blocker.
- **Tags.** Every leaf carries the smallest things one card could remove: for
  IND/COND/RET the typed path's refusal causes (`union`, `pointer-valued-store`,
  `aggregate-value`, `call`, `indirect-call`, `cast`, ...), else the coarse shape of an
  expressible path (`expr:<statement|cond|return>:<call-arg|direct>`), else `none`
  (no typed access); for `unknown_alias` the W47 discharge group. A function row is
  *freed* by a set of tags when every one of its access and alias leaves is removed
  and it has no other blocker; the figures are upper bounds (a freed function is taken
  Safe, a discharged obligation and nothing more).
- **Row counts.** A row is one function in one unit configuration, as the user's report
  and STATUS count it: 4,912 rows, 3,832 Unknown (3,782 of 4,848 distinct
  `file, function` pairs). Distinct counts are given where a list is frozen.

### Base versus the user's run

The census run reproduces the shape of the user's report on the new base. The
independent recount of `scripts`-free `fam.py` (family by description) on this run
gives IND alone 229, ALIAS alone 65, COND alone 55, IND+ALIAS 495, +COND+RET 765,
+UNC-RUN 2,118, matching the join below (229, 65, 56, 495, 753 after the unjoined
groups are held back). The user's earlier report gave 265 / 65 / 55 / 560 / 837 / 2,264:
the base removed 36 IND-only functions and 65 of the IND+ALIAS ones (the
landed rounds). Report counts of this run: 26,263 unsupported, 11,177
`unknown_alias`, 40 leak, 4 summary conflict, 1 double release; 186 configurations
(10 Safe, 8 Unsafe, 164 Unknown, 4 data-only), 1,070 Safe / 10 Unsafe / 3,832
Unknown function rows.

**Inertness.** The census build's report is not byte-compared with a default build
(that second 1h50 run was not made); the instrument is the one W47 proved inert
(`indirect-census` is `cfg`-compiled and its streams env-gated; no gateway source
changed in this card), and the independent recount above agrees with the join. This
is recorded as the unmeasured part, not as evidence of inertness.

**Joins.** 180 of 181 function-bearing configurations join to an analysis (183
analyses recorded for 186 configurations). The remaining one is `lib/cmetrics/src/cmt_map.c`:
its report lines reach 5,877 but the only recorded analysis text has 5,664 lines, so a
second analysis path of that unit is not hooked (10 functions, 33 alias leaves, 39
IND/COND groups). 160 IND/COND/RET leaf groups (of 13,209) bind to no construct
(`rdhttp.c` conditions 59, `cmt_map.c` 39, a dozen scattered): they carry the tag
`unjoined` and are never counted as removable. Registered as
`CURRENT-CENSUS-UNIT-STREAM-GAP`.

## 2. The leaves

IND/COND/RET leaf groups joined: IND 9,490, COND 3,517, RET 202. Alias leaves 11,177
(all classified except 33 in the unjoined unit).

| Family | Leaves by tag (top) |
|---|---|
| IND | call-actual access in statements 5,116 (`call-arg:formal` 2,165, `call-arg:local` 1,833, mixed roots the rest); `pointer-valued-store` 2,102; `union` 779; direct statement access 985; `aggregate-value` 222; `indirect-call` 158; `statement-expression` 117; `call` 115 |
| COND | call-actual access 1,492; `none` (no typed access) 952; direct access 528; `pointer-valued-store` 259; `union` 173; `aggregate-value` 26; `static-local` 21 |
| RET | `pointer-valued-store` 67; direct access 58; `none` 29; `union` 15; `indirect-call` 12; call-actual access 11 |
| ALIAS | `formal-handed-uncontracted` 3,803; `poisoned-by-refusal` 1,281; `non-pointer-name` 1,072; `derived-local-unguarded` 1,037; `borrowed-path-unguarded` 875; `uncontracted-result-unguarded` 859; `guarded-other` 674; `contracted-result-unguarded` 587; `address-taken` 284; `formal-copied` 273; `formal-handed-contracted` 150; `statement-expression-unguarded` 98 |

Alias classes (new since W47: `formal-custody` 4,562): call-result-corpus 1,378,
borrowed-path-read 1,151, call-result-contracted 1,125, mixed-origin 470, scalar-name
398, non-object-name 338, derived-address 324, address-taken 284, copy-of-name 283.
Causes: unguarded 7,980, poisoned 1,346, guarded-clean 714, not a pointer 1,137.

**Why expressible access is still refused** (parity stream, [artifacts/reasons.json](artifacts/reasons.json)):
for the 5,116 statement call-actual leaves the reasons at the leaf's line are "no
value form of this card" 3,853, `address-of` 966, "no unique Clang node for the
statement" 858, "an arm's expression an older route refuses" 818 and `CallRefused`
632 (callees `rd_slice_abs_offset` 558, `rd_slice_read` 541, `flb_log_suppress_check`
32); roots are formal 3,139 / local 2,837 / global 102, hops are one or more in 4,707
of them. The reasons are keyed by function name and line and overlap; they describe the
refusal, they are not verdicts.

## 3. Shape ranking

Rows freed **alone** (every other leaf of the function is already gone, so only the
named tag set removes the function from the blocked set) among the 3,832 Unknown rows.
"Touched" counts Unknown rows carrying the tag. Full table:
[artifacts/rank-coarse.json](artifacts/rank-coarse.json) (`tag_stats`).

| # | Shape (tag) | Family leaves | Leaves | Rows touched | Freed alone |
|---:|---|---|---:|---:|---:|
| 1 | `expr:statement:call-arg` | IND | 5,116 | 1,415 | **69** |
| 2 | `pointer-valued-store` | IND 2,102, COND 259, RET 67 | 2,428 | 929 | **53** |
| 3 | `expr:return:call-arg` | IND 99, RET 11 | 110 | 102 | **48** |
| 4 | `none` (no typed access) | COND 952, IND 77, RET 29 | 1,058 | 604 | **40** |
| 5 | `expr:cond:call-arg` | COND | 1,492 | 605 | 16 |
| 5 | `alias:guarded-other` | ALIAS | 674 | 390 | 16 |
| 7 | `expr:return:direct` | RET | 58 | 52 | 14 |
| 8 | `alias:formal-copied` | ALIAS | 273 | 241 | 13 |
| 9 | `alias:borrowed-path-unguarded` | ALIAS | 875 | 354 | 9 |
| 9 | `aggregate-value` | IND, COND, RET | 252 | 169 | 9 |
| 11 | `union` | IND 779, COND 173, RET 15 | 967 | 323 | 8 |
| 11 | `call` (call in the path) | IND, COND, RET | 137 | 98 | 8 |
| 13 | `alias:non-pointer-name` | ALIAS | 1,072 | 347 | 7 |
| 14 | `expr:statement:direct` | IND | 985 | 530 | 6 |
| 14 | `indirect-call` | IND, COND, RET | 185 | 142 | 6 |
| 16 | `alias:formal-handed-contracted` | ALIAS | 150 | 96 | 3 |
| 17 | `alias:derived-local-unguarded`, `statement-expression-unguarded`, `static-local`, `pointer-arithmetic` | | | | 2 each |
| | `alias:formal-handed-uncontracted` (3,803 leaves, 886 rows) | ALIAS | 3,803 | 886 | 0 |

**Greedy cover** (marginal rows freed, ties by rows one tag away):
`expr:statement:call-arg` +69 (69), `pointer-valued-store` +60 (129), `none` +49 (178),
`expr:return:call-arg` +50 (228), `expr:cond:call-arg` +28 (256), `union` +28 (284),
`alias:formal-handed-uncontracted` +31 (315), `alias:formal-copied` +32 (347),
`alias:derived-local-unguarded` +29 (376), `expr:statement:direct` +35 (411). All 26
shapes reach 753 rows.

**Bundles** (proposed cards; rows freed alone / marginal when every other bundle has
already gone / same-unit closure rows / cross-unit closure rows):

| Bundle | Tags | Leaves | Alone | Marginal | Closure same-unit | Closure cross-unit | Units |
|---|---|---:|---:|---:|---:|---:|---:|
| K1 call-actual access | statement, return, cond `call-arg` | 6,718 | 138 | 371 | 149 | 160 | 0 |
| K3 pointer-valued stores | `pointer-valued-store` | 2,428 | 53 | 167 | 68 | 81 | 0 |
| K5 local-origin aliases | ten alias groups | 6,823 | 43 | 232 | 49 | 62 | 0 |
| K7 residual causes | union, aggregate, call, indirect call, cast, ... | 1,849 | 40 | 184 | 48 | 57 | 0 |
| K2 no typed access | `none` | 1,058 | 40 | 87 | 47 | 56 | 0 |
| K6 direct-position access | statement, return, cond `direct` | 1,571 | 21 | 142 | 27 | 35 | 0 |
| K4 formal-custody aliases | four alias groups | 4,289 | 16 | 163 | 23 | 59 | 0 |

Pairs: K1+K5 222, K1+K4 206, K1+K3 202, K1+K7 189, K1+K2 185. All seven bundles
together free **753 rows (751 distinct functions)**; by family, IND alone frees 229, ALIAS
65, COND 56, RET 46, IND+ALIAS 495.

**Units whose whole blocker set a shape covers.** No single bundle covers any unit.
All four families gone cover **5** of 164 Unknown configurations (IND+ALIAS: 1), and the
call-graph closure adds none (every other unit keeps a SYNTAX, CALLBACK or NULLABLE
function). Access and alias work moves functions, not units.

## 4. Call-graph closure (criterion 2)

The closure freed-set rule is W46 `predict.py`'s: a function whose only remaining
items are calls to run-defined callees that are Safe or already freed is freed in the
next round. Same-unit is the existing summary rule; cross-unit is built by nothing today.

| Removed | Direct | Same-unit (rounds) | Cross-unit (rounds) |
|---|---:|---|---|
| nothing | 0 | 6 (6) | 13 (13) |
| K1 + K3 + K2 (greedy top 3) | 178 | 199 (184, 12, 2, 1) | 222 (197, 21, 3, 1) |
| greedy top 6 | 284 | 312 (290, 19, 2, 1) | 335 (303, 28, 3, 1) |
| IND + ALIAS | 494 | 563 (501, 46, 14, 2) | 650 (553, 70, 19, 5, 2, 1) |
| IND + ALIAS + COND + RET | 753 | 882 (760, 94, 23, 4, 1) | 1,031 (818, 157, 39, 12, 3, 2) |

**The call-graph is the binding constraint.** Of the Unknown rows each bundle touches,
62 to 74% also carry a call to a function defined in the run that is itself not Safe
(K1 1,096 of 1,651; K3 572 of 929; K4 896 of 1,209; K5 1,143 of 1,646), and 5 to 8% a
SYNTAX leaf. Removing UNC-RUN alone frees 398 rows (unchanged from the user's report);
removing every access and alias shape and every UNC-RUN leaf frees 2,118. The access
and alias cards therefore buy the closure its leaves; their yield in Safe functions
is bounded by the bottom-up callee work (`W48-BOTTOM-UP-RESIDUAL-PLAN`).

## 5. Proposed implementation cards (all card-high; amend the owner, no new ids)

Every shape below is already owned by a follow-up; the ledger amends the owner with
this run's numbers instead of registering a duplicate. The four ids a sibling triage
proposed (`CHAINED-ASSIGNMENT-INNER-DESTINATION`, `LOOP-REACQUISITION-LEAK-ACCUSATION`,
`BUILTIN-EXPECT-NULL-TEST`, `REFUSED-STORE-OF-CALL-RESULT-ESCAPE`) are not duplicated;
K3 names the first and last as related. Expected Safe applies the W46 P3 yield (83 of
220 freed, 38%) with a 25 to 50% band to the distinct functions freed alone, then adds
the same-unit cascade; the per-card drafts hold the frozen lists
([predictions/](predictions/README.md)).

| Order | Card (owner id to launch) | Rules | Freed alone (distinct) | Expected Safe (band) | Cascade, same-unit |
|---:|---|---|---:|---:|---:|
| 1 | K1 `W44-CALL-ARGUMENT-UNTYPED-ACTUAL` (+ `W44-CALL-ARGUMENT-NON-STATEMENT`, `W44-CALL-ARGUMENT-UNCONTRACTED`) | 10k, 10n, 10u | 135 | 51 (34-68) | +11 |
| 2 | K3 `CG11-UNRESOLVED-OWNER-POINTER-STORES` (+ `CG11-UNREAD-POINTER-PLACES`) | 10v | 53 | 20 (13-26) | +15 |
| 3 | K5 `W47-ALIAS-FLOW-RESIDUALS` (+ `W47-ALIAS-TYPED-SCALAR-NAMES`, `W47-ALIAS-NONNULL-ALLOCATOR-RESULTS`) | 5i, 5l, 10a, provider | 43 | 16 (11-22) | +6 |
| 4 | K7 `CG13-UNION-LEAVES-OTHER-CAUSES` and the other residual-cause owners | 10w, 11b, 11p, 14 | 40 | 15 (10-20) | +8 |
| 5 | K2 `W46-LEXICAL-GATE-NON-ACCESS` (+ `W44-CONDITION-CALL-REFUSED-SHAPES`) | 10k, 10n | 39 | 15 (10-20) | +7 |
| 6 | K6 `W47-ACCESS-PATH-POSITION-RESIDUALS` (+ `CG10-BOUND-ROOT-*`) | 10k, 10n | 21 | 8 (5-10) | +6 |
| 7 | K4 `W47-ALIAS-CALLEE-BORROWED-CLOSURE` (+ `W47-ALIAS-FORMAL-COPY`) | 5i, 5l, P4 | 16 | 6 (4-8) | +7 |

All seven together: 751 distinct functions freed, about 285 Safe at the central yield,
about 330 with the same-unit cascade (rows 882). Rule ids for any new rule: 40 is
reserved for LINK-STANDING; none of the seven needs a new id (each extends the named
rule), and if one does, the next free id on `main` and `integrate/r5` is 41. No Safe
is predicted without a discharged obligation: every freed function that fails a
non-null or live obligation stays Unknown.

K4's top group (`formal-handed-uncontracted`, 3,803 leaves, 886 rows, **0 alone**) is
the largest alias group and frees nothing without the callee-closure cards, which is
why K4 is last; its cross-unit closure (59 rows) shows the pairing.

## 6. Records

- [PREDICTION.md](PREDICTION.md): this card predicts no row moves.
- [predictions/](predictions/README.md): seven drafts with frozen candidate lists.
- [integration-amendments.json](integration-amendments.json): registers and retires
  this card, registers `CURRENT-CENSUS-UNIT-STREAM-GAP`, amends the owner ids.
- [artifacts/](artifacts/): run record and slot receipts, rank, reasons, co-occurrence,
  joined leaf rows and unmatched lists.

Reproduce (all steps through `gate_slot.py` where heavy): build the census binary,
`scripts/run_census.py OUT 7b726587b target/census/release/quirin-audit`,
`alias_census.py OUT/... --parity OUT/parity.jsonl`, `shape_census.py`, `rank.py
SHAPE OUT.json coarse`, `reasons.py`, `cooccur.py`, `make_drafts.py`.
