# CG10-SMALL-BUNDLE: three follow-ups of the CG10 reader cards (rules 74, 74a, 74b)

USER DECISION 2026-11-21: launch this bundle. It takes three
`named_followups` entries: CG10-UNCAST-RETURN-UNCONTRACTED-ATTRIBUTION,
CG10-RESULT-HOLDER-RETAINED-BY and CG10-W4-FUNCTION-POINTER-DECLARATOR-FORMALS.
The base is `8e7e8d509` (integrate/r48). The rules are gateway
[rule 74](../../agents/gateway.md#constraint-74),
[74a](../../agents/gateway.md#constraint-74a) and
[74b](../../agents/gateway.md#constraint-74b). Rules 12k, 70 and 71b each
gain one sentence pointing at them. The Lean modules are
`QCore.UncastReturnAttribution` (9 audited theorems),
`QCore.ResultHolderRefusal` (5) and `QCore.FunctionPointerFormal` (12).
[PREDICTION.md](PREDICTION.md) was committed (`9971508b3`) before the
implementation (`47c3b38fd`, the measured binary).

## 1. Readings and the refusal

* **Rule 74** (`legacy_uncontracted_access.rs::uncast_uncontracted_reading`).
  Rule 12k reads a whole-call return with its spelled actuals, and a cast is
  an actual neither reader recognises. Where that reading declines and every
  view of the return is one rule 71a removes, rule 12k is asked of rule 71a's
  uncast call. While it is asked, rule 71a's `Transparent` guard is held, so
  W44's typed reader matches each uncast actual to the view's operand. This
  is a re-attribution only: the writes and the verdict are kept. Lean:
  verdict and status unchanged, the answer is the gate's refusal or the
  statement's own callee, a kept view or a non-whole call leaves rule 12k's
  spelled answer, a contracted or held callee keeps the gate, and the same
  identities are handed (over `QCore.UncontractedAccessAttribution` and
  `QCore.ReturnCastView`).
* **Rule 74a, a recorded refusal.** Deriving `resource_retained_by(q,
  result)` needs two things that no derived summary states. In the body, it
  needs a per-identity retention record on the tracked result: rule 10v
  admits a store into a tracked owner only through O04-B08's field link,
  which reads the pointer as owned. At every caller, rule 13e ends the record
  at the result's release. For a declared contract, the destructor's not
  releasing `q` is a reviewed assumption (constraint 4). For a derived
  summary the destructor is any body, and its release of the member that
  keeps `q` would end the record and release the caller's object unseen.
  `QCore.ResultHolderRefusal` builds this case over
  `QCore.MemberStoreRetention`'s concrete memory. The end-of-record reading
  accepts a caller whose run faults (a double release) under a destructor
  that releases the member. Read as what it does, the same destructor makes
  the checker refuse. A destructor with no member effect does not fault. The
  two destructors are indistinguishable to the abstraction, so the reading
  needs a per-destructor premise (follow-up
  CG10-RESULT-HOLDER-DESTRUCTOR-MEMBER-EFFECT). Nothing in the code changes.
* **Rule 74b** (`function_pointer_formal.rs`, one fallback in
  `legacy_tokens.rs::function_parameters`). Where the old reader refuses a
  list holding `(`, the list is split at its top-level commas. Every group
  holding `(` must be exactly `SPEC+ ( * NAME ) ( BALANCED )`, and the body
  must not call through `NAME` (`NAME (` or `( * NAME ) (`). The list is then
  read with `NAME` at its position, a function-pointer formal, and every
  other group is read by the old reader. A called formal is refused because
  an invocation of a caller-chosen function may release what it is handed
  (§C04.19). It is also refused because consumers resolve a call by its
  spelling, so a formal named `q_free` would be read as `q_free`. Other
  parenthesised shapes, variadic lists and repeated names stay refused. The
  recursion route's own type reader is unchanged. Lean: positions kept, a
  called formal or another declarator refuses, a plain list reads as before,
  and a function pointer adds no obligation. The decision table is bound to
  the Rust by `rule_74b_lean_vectors_match_the_rust_table`.

## 2. Census and consumers (item c)

[scripts/fnptr_formals.py](scripts/fnptr_formals.py) reads the source text of
every unit of the base report, so it is a lower bound: a definition a macro
spells is missed. The result is
[census/fnptr-formals-base.jsonl](census/fnptr-formals-base.jsonl), with
**102 definitions** that have a parenthesised parameter group:

* **64 read** by rule 74b (61 Unknown, 3 Safe: `rd_kafka_set_logger`,
  `rd_kafka_buf_set_maker`, `rd_kafka_conf_set_oauthbearer_token_refresh_cb`);
* **30 with a called formal**, which stay refused (`rd_kafka_q_serve`-style
  invokers, `qsort` drivers that call `cmp`);
* **8 other declarators**, which stay refused (`int (cmp)(…)` three times,
  `STACK_OF(X509) *` three times, `void (*log_cb)` and `void (*cb)` with no
  parameter list).

The round-4 tip report gave the same counts. On the callers' side,
[census/callers-base.txt](census/callers-base.txt) finds **12 Unknown rows**
whose only blockers are uncontracted calls to the 64. That is an upper bound:
each callee must also validate a summary.

The reader is shared by every consumer below. For each one, the table records
what it sees for a function-pointer formal. Every consumer reads a formal by
its position, and none can give a function pointer custody.

| consumer (file) | what it sees |
|---|---|
| W4 derivation and validation (`legacy_summary.rs` ×3, `legacy_unwritten_class.rs`, `legacy_declared_entry_check.rs`, `legacy_checked_spec.rs`, `legacy_c.rs` entry custody) | `EntryCustody::build` zips names with contract modes by index (kept). Inference finds no release, consumption or out-slot write of the name, so it is `Borrowed`. At a call, a function designator, null or function-pointer actual carries no identity. |
| body formal set (`legacy_c.rs:3988`: fixed and protected formals) | The name joins the set of caller objects. A function-pointer formal is never released or written through, so it changes no admission. Rules 10v and 70 class its value by Clang type (function pointer): no retention, only an overwritten path. |
| selection and lookup frames (`legacy_selection.rs` ×2, `legacy_lookup.rs`) | These need exactly two object formals with fixed shapes, so they decline as before. `legacy_lookup.rs:514`'s same-spelling guard now also sees a function-pointer formal, which withholds more. |
| recursion (`legacy_recursion.rs`) | `parameter_types` uses the unchanged `parse_parameter_declarators`, which refuses a list holding `(`, so the route declines as before. |
| neutral callees (`legacy_neutral.rs`) | The component gets `N(m)` with one `Borrowed` per position. The body is validated as for any other formal. |
| self-link frames (`legacy_self_link_init.rs` ×2), member release (`legacy_member_release.rs`), detach (`legacy_detach.rs`) | These need one or two specific object formals, so they decline. |
| member effect (`legacy_member_effect.rs`), counted users (`legacy_counted_users.rs`) | These read `F->member` operations on formals. A function pointer has no members, so nothing is added. |
| out-alias normalisation (`legacy_out_alias.rs`), exact formal returns (`legacy_exact_formal_return.rs` ×2) | `normalize` splits the raw list on every comma and requires the counts to agree, so an inner comma declines it. Exact formal returns read declared pointer formals only. |
| derived results, relational forwarding (`legacy_derived_summary.rs`, `legacy_relational_forwarding.rs` ×2) | They index by position. The name is `Borrowed`. |
| cross-unit replay (`legacy_cross_unit.rs`) | It compares ordered removals by formal name. A function pointer is never removed. |

## 3. Measurement

This is the opt-in `quirin-audit current --monkey-linked` over Fluent Bit
046accf6, through [scripts/opt-in-current.sh](scripts/opt-in-current.sh)
(CG10-READER-GAPS' script). The prepared inputs are used directly; their
prefix was fixed today. Both sides are release `indirect-census` builds
([scripts/build.sh](scripts/build.sh)). Each side tree is `git archive
8e7e8d509`, unpacked twice with byte-identical bytes. The binary is the only
difference between the runs. A first tip attempt on the base's tree was
stopped during its checker-install step while the base ran
(`private/sb/run-tip-stopped-shared-tree`). It is not evidence, and the tip
was re-run on its own tree. Logs are in the worktree's `private/sb/`.

| run | revision | binary | started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `8e7e8d509` | `91f0c2d6…` | 00:50:11 / 01:50:06 | 0 | 1,159 / 2 / 3,636 / 7 | 2,135 | 209 | 9,027 |
| **tip** | `47c3b38fd` | `f0e04c84…` | 01:04:13 / 02:03:47 | 0 | **1,159 / 2 / 3,636 / 7** | **2,132** | **207** | **9,026** |

The figures come from [evidence/totals-base.json](evidence/totals-base.json)
and [evidence/totals-tip.json](evidence/totals-tip.json). Row comparisons are
in [evidence/changed-rows.txt](evidence/changed-rows.txt) and
[evidence/compare-base-tip.json](evidence/compare-base-tip.json). Named rows
are in [evidence/named-rows.jsonl](evidence/named-rows.jsonl)
([scripts/evidence.sh](scripts/evidence.sh)). Both consoles report 536 Safe
rows on a declared assumption, 641 declarations consulted, 0 refuted
contracts, and 150 checker bindings, all `Pending`.

* **No status move.** There is no Safe -> non-Safe move, no new Unsafe,
  Pending is unchanged, and no leak leaf is added.
* **Six rows change leaves**, all of them still `Unknown`:
  * rule 74: `pb_dec_bool` -> ``call to uncontracted function
    `pb_decode_bool` ``, `pb_enc_fixed_length_bytes` -> `pb_encode_string`, and
    `pb_dec_fixed_length_bytes` -> `pb_read` (not predicted, same shape);
  * rule 74b, with the formal set now known:
    * `rd_kafka_q_cb_event_enable`: two access leaves become rule 38i's
      member-release and member-store names;
    * `rd_list_sort`: `unresolved resource identity rl` added on line 3107;
    * `rd_map_init`: four access leaves added on lines 3082–3085, the
      function-pointer stores after the refused bucket store on line 3081.

  The added leaves are more refusals in rows that were already Unknown. Their
  exact route was not diagnosed further. Rule 10v's old-value premise over
  the formal root, unknown after line 3081's refused store, fits the
  `rd_map_init` lines.

### Per-row evidence

| row | entry | base -> tip | remaining blocker |
|---|---|---|---|
| `pb_dec_bool` | UNCAST | Unknown, **re-attributed** (access -> ``uncontracted `pb_decode_bool` ``) | `pb_decode_bool` unsummarised |
| `pb_enc_fixed_length_bytes` | UNCAST | Unknown, **re-attributed** (-> `pb_encode_string`) | `pb_encode_string` unsummarised |
| `print_string` | UNCAST | Unknown, unchanged | Rule 71a itself declines the view: `(unsigned char *)item->valuestring` with `item` a `const cJSON *` (a `const` pointer to a record typedef, with a non-const view) is refused even for a contracted callee, so rule 74 has no uncast call. Each part alone is read. This is pinned in `tests/small_bundle.rs` (follow-up CG10-UNCAST-VIEW-CONST-TYPEDEF-BASE). |
| `PartitionMemberInfo_new`, `rd_kafka_op_new_cb` (sole) | RESULT-HOLDER | Unknown, unchanged | the recorded refusal (rule 74a) |
| `rd_kafka_interceptor_method_new`, `rd_kafka_member_assigned_partitions_pair_new` (mentioning) | RESULT-HOLDER | Unknown, unchanged | rule 74a, plus an owned result stored into a member (`rd_strdup`, `rd_list_new`) |
| `rd_kafka_buf_set_maker` | W4-FN-PTR | Safe, unchanged | Its summary is now published (its formal list is read). Its one caller site in the census keeps another blocker. |

**Against the prediction.** Rule 74: 2 of the 3 named rows re-attributed, plus
one row that was not named. `print_string` is withdrawn (the rule 71a residual
above). Access-alone fell 209 -> 207 (predicted -3) and access leaves fell by
1 net (the predicted -3 to -10 was offset by rule 74b's +4). Rule 74a: no
move, as predicted. Rule 74b: 0 status moves (predicted 0–4) and 3 rows
change leaves (predicted 0–60). None of the 12 upper-bound callers moved:
each callee keeps another blocker. No Safe -> non-Safe move and no new
Unsafe, as predicted.

**Pinned routes.** `quirin-audit measure --declared` cannot run on this host
because there are no `build/cycle19` inputs or baselines. The prediction for
the integrator is unchanged from PREDICTION.md:

```measure-prediction
route corpus
none
route system
none
route slice
none
```

Expect leaf re-attributions on the nanopb rows and leaf changes on
librdkafka rows with function-pointer formals wherever a route analyses them.
No status move is expected.

## 4. Fixtures

`crates/qcore-c-gateway/tests/small_bundle.rs` has 8 tests:

* Rule 74 names `un_peek` and `un_bool` for character and scalar views. A
  union view, a return inside an expression, and the scalar-conversion shape
  keep their refusals. A contracted callee is still rule 71a's `Safe`. One
  test pins `print_string`'s residual, with three controls.
* Rule 74a: the result-holder store stays Unknown with no summary. Every
  caller sees an uncontracted callee, including the hazard shape (a
  destructor releasing the member).
* Rule 74b: the `rd_kafka_buf_set_maker` shape is `Safe` with null or
  function-designator actuals. The caller's own retained object remains
  `retention-undecided`. A neutral comparator formal and a releasing body
  are summarised, and a double release after the releasing call is not
  `Safe`. A call through the formal in either spelling, a formal named
  `q_free`, `int (cmp)(…)` and a variadic list keep the uncontracted callee.

Unit tests (`tests/unit/function_pointer_formal.rs`, 6):

* the decision vectors and their Lean binding;
* the exact declarator shape (14 spellings);
* both call spellings;
* positions kept;
* refused lists.

**One pin flipped deliberately:**
`reader_gaps::a_function_pointer_declarator_formal_leaves_its_function_unsummarised`
is now `Safe`, with a called-formal control that keeps the uncontracted
callee.

## 5. Gate

Records in [gate/](gate/); logs in the worktree's `private/gate-*`.

* **gate-1** (`96190c611`, the code, rules and census; scoped card gate
  `sh private/toolchain/env.sh python3 scripts/gate_scope.py --base 8e7e8d509
  --out private/gate-1`, mode full) exited 1. Every step passed except the
  Python suite:

  | step | result |
  |---|---|
  | submodules | 1 submodule |
  | fmt | pass |
  | clippy | pass |
  | `run_test_binaries.py` | 672 units, 9,848 passed, 0 failed, 21 ignored, 148 filtered |
  | probes | 0 / 0 |
  | `check_lean_audits.py` | 477 audits |
  | Python | 779 tests, **1 failure** |

  The one failure is
  `test_build_matrix_sections…test_each_derived_dimension_matches_the_section_it_counts`:
  `dimensions.named_followups` is 1,041 against 1,044 entries. **It fails
  identically on the base archive** (`private/sb/base-matrix-test.log`), so it
  is integrate/r48's matrix drift. This card does not touch the matrix. It is
  left to the integrator, whose amendment application regenerates the
  dimensions.
* **Gate read census.** `gate_read_census.py record --packages
  qcore-c-gateway` and `reduce` both exited 0. 263 units were traced and 1
  was not ok under the tracer (`detach_lookup_membership`, as in earlier
  rounds; it passes in the gate). The one new unit,
  `qcore-c-gateway::test::small_bundle`, was added to
  `docs/reports/gate-scope-precise-selection/read-census.json`.

## 6. Follow-ups ([integration-amendments.json](integration-amendments.json))

* **Retired:**
  * CG10-UNCAST-RETURN-UNCONTRACTED-ATTRIBUTION (rule 74; `print_string`'s
    residual is re-registered as its own shape);
  * CG10-RESULT-HOLDER-RETAINED-BY (rule 74a's recorded refusal);
  * CG10-W4-FUNCTION-POINTER-DECLARATOR-FORMALS (rule 74b).
* **Amended:** CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (access-alone 209 -> 207).
* **Registered:**
  * CG10-UNCAST-VIEW-CONST-TYPEDEF-BASE (rule 71a declines a non-const view
    of a member read through a `const` pointer to a record typedef);
  * CG10-RESULT-HOLDER-DESTRUCTOR-MEMBER-EFFECT (the per-destructor premise a
    result-holder reading needs);
  * CG10-CALLED-FUNCTION-POINTER-FORMALS (30 census functions that call
    through a function-pointer formal stay unsummarised; reading them is the
    callback shape, §C04.19).

## 7. Files likely to conflict

* `crates/qcore-c-gateway/src/legacy_c.rs` (one `mod` line);
* `legacy_tokens.rs` (`function_parameters`'s fallback and a helper);
* `legacy_uncontracted_access.rs` (one method and one call);
* `tests/reader_gaps.rs` (one pin);
* `docs/agents/rules/12k.md`, `70.md` and `71b.md` (one sentence each);
* the generated `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`;
* `docs/reports/gate-scope-precise-selection/read-census.json` (one unit).

Nothing in `pointer_store.rs`, `legacy_pointer_store.rs`,
`member_store_retention.rs` or the field-link model is touched. No provider,
contract or lock is touched.
