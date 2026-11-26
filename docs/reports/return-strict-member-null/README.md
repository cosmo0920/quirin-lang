# RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT: a bare member read through an untracked root keeps family S's base

Card RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT, launched on USER DECISION
2026-11-24 ("launch both cards"). The base is `497c859fc`, the tip of
RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST (rule 74t, approved as r74). That
card sits on rule 74r (r72) and rule 74m (r68), on main `3ca85c7a6`.
[PREDICTION.md](PREDICTION.md) was committed (`48cb75417`) before the
implementation (`8434d92a3`). The card adds gateway rule
[74v](../../agents/gateway.md#constraint-74v) and `QCore.ReturnStrictMemberNull`
with its audit.

## 1. Reachability

**The gap is reachable, but through a route other than the one
registered.** The fixture declares
`struct s { long n; long a[2]; struct s *next; … }` and the `k_*` surface.
The end-to-end probe ran at the base (`private/rsm/reach/census-base.log`)
and at the tip (`census-tip.log`). It is
`private/rsm/reach/zz_probe_strict.rs` and is not committed.

| body | base | tip |
|---|---|---|
| `long w(void) { const struct s *p = 0; return p->n; }` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `struct s *w(void) { const struct s *p = 0; return p->next; }` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `void *w(void) { const struct s *p = 0; return (void *)p->next; }` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `… return p->n + 1;`, `long x = p->n; return x;` | `Unknown`, `unknown_alias:p` | same |
| every other row of the census in §2 | as in §2 | same (only these three rows differ) |

The finding card named `supported_expression`'s `Returned` rule or its
`Decides` widening. A debug run (`private/rsm/reach/dbg.log`) shows that
both refuse `p -> n` here. The strict and F2–F4 rules need
`!FunctionTypes::may_bind(base)`, and `p = 0` writes `p`. The admitting route
is **card B01's opaque member read** in `return_expression_instruction`
(`legacy_return_routes.rs`), which runs before family S
(`private/rsm/reach/dbg2.log`). It classifies the synthetic
`__qcore_returned_value = p -> n`. The classifier answers C02-P's opaque
field read, `OpaqueValue { destination, alias_read: None }`
(`legacy_classify.rs`, `borrowed_actual::is_opaque_field_read`), whose base
premise is `UntrackedBases::never_bound`. That premise admits `p = 0`, which
is a call-free, `&`-free assignment. B01 takes the `OpaqueValue` when the
expression has exactly one member selection. The node binds an unspelled
place and carries **no obligation** for the root `->` dereferences, so the
flow check never saw the null `p`. Family S lowers `p->n + 1` to
`IndirectScalarAccess { bases: ["p"] }`, and the declaration route does the
same for `long x = p->n;`. Both refuse the null binding.

## 2. Census of the routes before family S

The routes before family S, in order, are:

1. rule 36's exact-formal return;
2. `cast_return`;
3. the strict `Returned` rule (the lexical predicate, then F1–F4);
4. its `Decides` widening;
5. the W4 whole call;
6. B01's call-in-scalar rule and B01's opaque member read;
7. W43's refused-whole-call lowering.

| route | can it dereference an untracked root without the obligation? |
|---|---|
| rule 36 exact formal | No: the bare formal, no access. |
| `cast_return` | No: it strips a carrier-preserving cast and hands the operand to the same routes, or it refuses. That is how `(void *)p->next` reached B01. |
| lexical `supported_neutral_expression` | No: it refuses `->`, `[`, `]`, `.` (non-numeric) and every `*` that may be unary. |
| F2 / F3 / F4 (`Returned` and `Decides`) | No new admission: `*`, `[` and `&` refuse, and a `->` base must be `!may_bind`. A written root fails that, and an unwritten one (a formal or an uninitialised local) has no binding, so family S's obligation discharges for it anyway. Probes: `long w(const struct s *p) { return p->n; }` and `const struct s *p; return p->n;` are `Safe` at both sides, as `p->n + 1` is. |
| W4 whole call, B01 call-in-scalar | Their arguments are classified as a statement's are. A borrowed `p->n` actual over a never-bound root carries no base obligation, which is the same gap in statement position: `return k_read(p->n, 0, 0);` and `k_read(p->n, 0, 0); return 0;` are `Safe` at both sides. This is out of scope and registered (§7). |
| **B01 opaque member read** | **Yes:** `R -> F` (one selection) and the casts `cast_return` strips. **This card fixes it.** |
| W43 refused whole call | No: it re-attributes a refusal and admits nothing. |

The brief's spellings at the base, all with a null untracked `p`:

* `return p->n;` is `Safe` (B01).
* `return p[0];`, `return *p;`, `return ((T *)p)->n;`, `return (p)->n;`,
  `return p->a.b;` (two selections), `return p->q->n;`, `return (p->n);` and
  `return (long)p->n;` are `Unknown` (`unknown_alias:p`). Family S already
  decides them.
* `return &p->n;` keeps the return refusal
  (`unsupported:unmodeled call, aggregate or indirect access in return expression`),
  and it keeps that at the tip.

## 3. Change

* **`legacy_return_routes.rs`** is the only source file changed.
  `legacy_c.rs` is not touched.
  * At the B01 opaque-read admission, the answer goes through a new method,
    `opaque_member_read_return`, which uses a new free function,
    `member_read_base`:
    * `ROOT - > FIELD` (two names) is `Root(ROOT)`. It lowers to
      `IndirectScalarAccess { bases: [ROOT], alias_reads: read_alias(ROOT), destination: None, frame: None, chains: [] }`.
      That is family S's node for the spelling: `scalar_read` gives the same
      base and `->` root (`the_root_is_family_s_base`).
    * No `>`, `[`, `]`, `*` or `&` is `Value`, a `.` read of a record value.
      The `OpaqueValue` is kept.
    * Anything else is `Declined`. B01's admission is dropped, and W43,
      family S, rule 10k and rule 11h decide the expression. B01 admits no
      such spelling today.
  * The exit stays a `NoOp` (`returned_evaluation_escapes` is `false`), so
    nothing new escapes. The module header gains a 74v paragraph, and one
    `mod` line is added at the end.
* **Why not route these returns to family S?** Family S declines a pointer
  result (`scalar_result`). `return p->next;` would then fall to the return
  refusal, and the unbound formal's `struct s *w(struct s *p) { return p->next; }`
  would lose its `Safe`. Reusing family S's *node* keeps that row and gives
  the scalar case exactly family S's verdict.
* **Direction.** B01 admitted every expression it read. A `Root` read is
  admitted only when the base obligation discharges. A `Declined` read goes
  to routes that, at the base, saw nothing B01 admitted. So no reading admits
  more than at the base (`the_fix_only_refuses_more`).
* **Readings kept.** An unbound formal, an uninitialised local, a copy of a
  formal (`p = f`) and a member read of one (`p = f->next`) stay `Safe`, and
  so does a root a null test guards (`if (!p) return 0;`). These are the
  bindings the flow check settles, exactly as for `p->n + 1`. A tracked root
  is never B01's (premise 4) and keeps `leak` or
  `UseAfterRelease` + `unknown_alias:p`. `v.n` on a record value stays
  `Safe`.

## 4. Pins

* **End to end** (`tests/return_strict_member_null.rs`; each test checks
  every row before failing):
  * `a_bare_member_read_through_a_null_untracked_root_is_refused` has 5 rows:
    a scalar member through a const root, a mutable root and a root
    initialised with `(void *)0`; a pointer member; and a pointer member
    under `(void *)`. All are `Unknown`, `unknown_alias:p`.
  * `the_bare_read_gets_family_s_verdict` covers 6 root bindings. Each
    compares `p->n` with `p->n + 1`: null (refused), and an unbound formal,
    an uninitialised local, a copy of a formal, a member of a formal and a
    guarded root (all `Safe`). It also covers a pointer member of an
    unbound formal (`Safe`).
  * `the_base_readings_are_kept` covers the 9 spellings family S already
    refuses, the declaration route, `&p->n`, `v.n`, and tracked roots
    (live, released, and read before release).
* **Unit** (`tests/unit/legacy_return_member_read.rs`, a new module of
  `legacy_return_routes`):
  * `rule_74v_read_vectors_hold` reads `readVectors` (12 rows) from
    `proof/QCore/ReturnStrictMemberNull.lean`;
  * `a_member_read_through_a_root_dereferences_the_root`,
    `a_record_value_member_dereferences_nothing` and
    `every_other_spelling_declines` (11 spellings) check the reading;
  * `the_root_is_family_s_base` checks it against `scalar_read`.
* **Amended pin.** `tests/null_premise_custody.rs`
  (`no_premise_where_nothing_is_read_through_the_alias`) held
  `struct S *n = a->next; return n->a;` at **no** null premise. That was
  B01's obligation-free node, the gap. `return n->a + 1;` and
  `int v = n->a; return v;` already name `n` under rule 14n
  (`private/rsm/reach/npc-tip.log`). The pin now expects `[(5, "n")]` for
  all three spellings.

**Checked at the base.** `legacy_return_routes.rs` was replaced byte for
byte with the base's copy. The new test file and the amended pin were run
against it, and the tip bytes were restored afterwards (`cmp` clean, then
`touch`). The logs are in `private/rsm/base-fixtures/`.

* `e2e.log`: `a_bare_member_read_…_is_refused` fails, and its 5 rows all
  measure `Safe []`. `the_bare_read_gets_family_s_verdict` fails on exactly
  one row, the null-local bare read (`Safe []`).
  `the_base_readings_are_kept` passes.
* `npc.log`: the amended pin fails with
  `Run { status: Safe, descriptions: [], premises: [] }`.

The unit module and its Lean vectors test new code, which does not exist at
the base.

**Pins:** no contract, lock, submodule or CT pin is touched.

## 5. Lean

`QCore.ReturnStrictMemberNull` and `proof/AuditReturnStrictMemberNull.lean`
have no placeholders and no project axioms; every theorem depends on
`propext` at most. The gate's audit run counted 500 audits.

* Admission is modelled over B01's flag, the token classes, an abstract
  discharge predicate `ok` and the later routes' verdict.
* `the_fix_only_refuses_more` and `outside_b01_unchanged` give the
  direction.
* `a_value_read_is_unchanged`, `a_root_read_needs_its_base`,
  `a_declined_read_defers`, `the_bare_read_gets_family_s_verdict` and
  `a_discharged_root_keeps_its_admission` cover the readings.
* `the_root_is_the_dereferenced_name` and
  `a_value_read_dereferences_nothing` cover what the reading names.
* `old_misses_the_base` states the gap, and `read_vectors_hold` checks the
  vectors.

Token classes are trust boundary 28's, bound by `readVectors`.

## 6. Measurement against the prediction

Opt-in `current --monkey-linked` over Fluent Bit 046accf6, each side from
its own `git archive` tree and release binary
([evidence/runs.md](evidence/runs.md)):

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `497c859fc` | 1,171 / 2 / 3,623 / 8 | 2,118 / 215 / 8,992 |
| **tip `8434d92a3`** | **1,171 / 2 / 3,623 / 8** | 2,118 / 215 / 8,992 |

**No row changes status, and no row changes its diagnostics.** There is no
`Safe -> Unknown` move and no `Unknown -> Safe` move. Keyed by unit index,
**two rows change**
([rows-by-unit-base-tip.jsonl](evidence/rows-by-unit-base-tip.jsonl)). Both
are `src/flb_input_chunk.c::flb_input_chunk_get_name`, a file the route
reports in two units:

```c
ch = (struct cio_chunk *) ic->chunk;
return ch->name;
```

Each stays `Safe`. Each now lists the rule-14n premise
`null-deref:flb_input_chunk_get_name:20522:ch`, which raises the console's
"carrying a null premise" count from 42 to 44. `ch` is a possibly-null
pointer read from a formal's member. At the base, B01's `OpaqueValue`
dereferenced it under no stated premise. At the tip, the read is family S's
node, and its `Safe` rests on the named premise, as a declaration
`… = ch->name;` already does. This is the card's change working
as intended. It is not a precision loss.

**Against the prediction.** The prediction was 0 to 5 `Safe -> Unknown`
moves among 8 census candidates, and the measurement found **0**, at the
low end of that range.

* `flb_input_chunk_get_name` was a candidate. It kept `Safe` as predicted,
  but gained a premise, which the prediction did not anticipate.
* `get_context_arena` and `rd_kafka_topic_partition_partition` bind the
  root from a cast of a formal. That is a settled untracked binding, so no
  premise is needed and the rows are unchanged.
* The five call-result and `container_of` candidates
  (`rd_kafka_message_timestamp`, `…_broker_id`, `…_status`,
  `mk_rconf_meta_get`) did not move. Their roots are assigned from a call-shaped
  expression (`rd_kafka_message2msg (…)`, `mk_list_entry`'s
  `container_of` expansion). `UntrackedBases` counts a name assigned from
  anything other than a call-free expression or an admitted field read as
  risky, so B01's opaque read does not admit them, and their rows are
  unchanged. The census over-counted them because it does not apply
  premise 4.
* The 135 formal-rooted hits discharge, as predicted.

No row was flagged as precision loss, so no restoring follow-up is needed.

**Pinned routes.** These were not run. The inputs they need are still
absent on this host. Rule 74r's attempt was refused for a missing
`private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-production/compile_commands.json`,
and `build/cycle19` does not exist either. No result is fabricated. A move
there would be `Safe -> Unknown` at a bare `return R -> F;` over a never-bound
root the flow check cannot settle.

## 7. Gate

Every gate run below was `sh private/toolchain/env.sh python3
scripts/gate_scope.py --base 497c859fc --out <dir>` (Clang 21, Lean 4.35).

**First run** (`private/rsm/gate-1/`, head `8434d92a3`) exited 1. Every step
passed except the Rust suite: 688 units, 10,032 passed, **1 failed**, 24
ignored. The failure was `null_premise_custody::no_premise_where_nothing_is_read_through_the_alias`.
That pin held the gap at no premise, and §4 amends it in `7d47d0651`.

**Second run** (`private/rsm/gate-2/`, head `7d47d0651`) exited 0, with all 8
steps passed. While it ran, this report and its evidence were uncommitted
docs files, and later commits change only those. The record is
[gate/gate-2.json](gate/gate-2.json).

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 688 units, 733 summaries; 10,033 passed, 0 failed, 24 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 500 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

No provider file changed, so the provider gate was not selected.

## 8. Ledger, conflicts and follow-ups

[integration-amendments.json](integration-amendments.json) is in schema
`quirin-amendments/2` and is not applied. It retires
RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT and registers two follow-ups:

* **RETURN-CALL-ACTUAL-NULL-UNTRACKED-ROOT.** A borrowed call actual
  `k_read(p->n, 0, 0)` through a null untracked `p` is `Safe` both in a
  statement and in a return. C02-P's borrowed actual has the same
  premise-4 gap.
* **UNTRACKED-BASE-NULL-JOIN-DISCHARGE.** `const struct s *p = 0; if (f) p = f;`
  followed by `return p->n + 1;` (family S) or by `return p->n;` is `Safe`.
  The flow check appears to read the join of a null binding and an unbound
  formal as discharged.

`status_edits` is empty, because STATUS states no fact this card changes.
The id this card retires is registered by rule 74t's amendments, which are
applied at r74, so `--check` against this tree alone
is refused. I extracted a `git archive` of this card's head into
`private/rsm/amend-root`, with this report directory copied in. There I
applied r68's, r72's and r74's amendments in order
(`return-family-deref-resolved-cast`, `return-family-untracked-deref-cast`,
`return-family-untracked-member-cast`), and all three exited 0
(`private/rsm/amend-apply-*.log`). Against that root, `--check` exits 0
(`private/rsm/amend-check.log`).

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/legacy_return_routes.rs`: the B01
  opaque-read admission (three lines), a new method at the end of the
  `impl`, and a free enum, function and `mod` line at the end of the file.
  The sibling RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC edits
  `legacy_return_family.rs`, which this card does not touch. `legacy_c.rs`
  is not touched, so it cannot conflict with r66 or r67.
* `crates/qcore-c-gateway/tests/null_premise_custody.rs`: one pin.
* The generated `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`. On a conflict, rerun `gen_rule_docs.py` and
  `gen_lean_roots.py`.

The 74t end-to-end test's comment says `return p->n;` alone is "the strict
`Returned` rule's". It is B01's (§1). The comment is left as written in that
card's file.

New files: `docs/agents/rules/74v.md`, `tests/return_strict_member_null.rs`,
`tests/unit/legacy_return_member_read.rs`,
`proof/QCore/ReturnStrictMemberNull.lean`,
`proof/AuditReturnStrictMemberNull.lean`, and this report with its scripts
and evidence.
