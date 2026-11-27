# CALLBACK-CANDIDATE-LITERAL-PREMISE: a premise at a callback candidate

Base main `062974679` (r33). Named follow-up
`CALLBACK-CANDIDATE-LITERAL-PREMISE` (recorded by
OPENSSL-BIO-READER-INVOCATIONS); user decision 2026-11-19.
[PREDICTION](PREDICTION.md) was committed before any implementation
(`7c5959b9c`). The implementation, tests and Lean are `581e8f55a`.

## 1. The gap

A literal-argument premise (C04.5) restricts a declaration to the call
sites whose named actual is the named literal. Route G enforces it by
renaming an unmet call to an undeclared callee
(`legacy_literal_premise::restrict_to_declared_literals`). A callback
candidate is not a call the rewrite sees. It is a function designator handed
to an invoker, and the invoker chooses the actuals. The callback route
classified a candidate by its contract alone, on four paths:

* an invocation (synchronous, table, member, slot) of a function-map
  candidate, through `callback_contracts::classify` in `classified_in`;
* a withheld candidate whose callback clauses are all member invocations,
  through `candidate_composition` (rule 13u amended), which a slot
  invocation beside a premise qualifies for since r33;
* a registration (rule 13w), which stores the candidate's contract for the
  registry's later invocation and element custody;
* a constructed member registration (rule 12o), which stores it the same
  way.

**Measured before the change.** The card's fixture
(`crates/qcore-c-gateway/tests/callback_literal_premise.rs`, the `q_*`
callees) was run at base with the new tests: every premise row read `Safe`
(`each_premise`, `each_slot_premise`, `list_premise`, `op_premise`), each
applying a premise-carrying identity or releasing candidate whatever the
invoker hands it ([base reading](evidence/candidate-rows-base.txt)).

## 2. Rule 57 amended in place

A candidate whose declaration carries a literal-argument premise, and every
candidate under a poisoned premise set, has no contract in force wherever it
is a candidate.

* **The premise set.** The parser now keeps the set route G read for the
  body (`Parser::literal_premises`, a function fact for the probe state), so
  the callback route reads exactly what the rewrite enforced, from the same
  declared texts. `LiteralPremises::withdraws_candidate` is true for a
  declared premise or a poisoned set.
* **Invocations and composition.** `Parser::candidate_withdrawn` classifies
  such a candidate `Missing` first in `classified_in`, before its contract,
  its composition, its hand-off (rule 12m) or its member paths (rule 30)
  are read. Every invocation kind and a composed candidate's own
  invocations go through `classified_in`, so each refuses it
  (`callback-uncontracted`, naming it).
* **Registrations.** `Parser::candidate_contract` stores no contract for it
  at a registration and a constructed member registration, so a registry
  holding it is not releasing and an invocation of the member refuses it.
* **No actual check.** The premise is not checked against the invocation's
  actuals: a `_` or a supplied object never satisfies it, and a parameter
  actual is the caller's, which the declaration does not bound.

This only refuses candidates that were applied before; it never applies one
that was not. A premise-free candidate is classified exactly as before. The
route-only change leaves every provider contract and lock untouched.

The changes are in contract specification C04.5 ("At a callback
candidate"), [rule 57](../../../../agents/rules/57.md) (amended in place, no new
rule) and the module comment of `callback_contracts.rs`.

## 3. The survey

Every literal-argument premise in the shipped provider crates, read from
each crate's `contracts/` (no other contract set ships one: libcurl,
libevent and the remaining crates declare none):

| Crate | Declaration | Premise | Before, as a candidate |
|---|---|---|---|
| Fluent Bit | `flb_hash_table_add` | `val_size = 0` | identity (5 actuals): **applicable** |
| Fluent Bit | `flb_task_users_dec` | `release_check = 0` | effectful (a counted-user drop) |
| cfl | `cfl_kvlist_insert_bytes` | `referenced = 0` | identity (5): **applicable** |
| cfl | `cfl_array_append_bytes` | `referenced = 0` | identity (4): **applicable** |
| libc | `fcntl` | `cmd = 2` | identity (2): **applicable** |
| libc | `prctl` | `option = 15` | identity (1): **applicable** |
| libc | `pthread_join` | `thread_return = 0` | identity (2): **applicable** |
| libc | `pthread_key_create` | `destructor = 0` | effectful (an out and a kind) |
| librdkafka | `rd_kafka_topic_partition_destroy0` | `do_free = 1` | releases at an `element` position, takes at `supplied_owned`: **applicable** as a releasing or taking candidate |
| OpenSSL | `PEM_read_bio_X509`, `PEM_read_bio_PrivateKey` | `x = 0` | withheld, not composable: uncontracted |
| OpenSSL | `d2i_PKCS12_bio` | `p12 = 0` | withheld, composable; its owned result makes it effectful |

The in-tree fixtures (`hash_contracts_generic.qv`, the `contract_inherit`
fixtures, the inline sets of the premise tests) and the frozen copy under
`docs/reports/quant-contract-hypotheses/plugin/before/` are not a measured
route.

**Did a measured row rely on it?** Seven declarations could have been
applied as a candidate. Fluent Bit `046accf6` (`git grep` over `*.c`, `*.h`)
names none of the non-OpenSSL nine as a designator outside a direct call,
except a cast `fcntl` in SQLite's `aSyscall` table (a cast actual, which
rule 11p opens) and `REG_NATIVE_FUNC(pthread_join, …)` in WAMR, whose macro
names the `_wrapper` function. The opt-in run below confirms no row moved.
The pinned routes could not be run on this host (§5).

## 4. Tests and Lean

In existing targets, so the read census needs no new unit:

| Arm | Test | Before | After |
|---|---|---|---|
| a function-map candidate (`q_put`, `flb_hash_table_add`'s shape) | `a_premise_carrying_function_map_candidate_has_no_contract_in_force` | `Safe` | `callback-uncontracted` naming `q_put`; `q_put_any` (no premise) `Safe`; a direct call within the premise `Safe` |
| a composed candidate (`q_put_slot`, a slot invocation beside the premise) | `a_premise_carrying_composed_candidate_has_no_contract_in_force` | `Safe` | `callback-uncontracted` naming it, never composed; `q_put_slot_any` `Safe` |
| a registered releasing candidate (`q_drop`) | `a_premise_carrying_registered_candidate_has_no_contract_in_force` | `Safe` | refused (`callback-`); `q_drop_any` `Safe` |
| a constructed member registration (`q_op_new(q_put)`) | `a_premise_carrying_constructed_candidate_has_no_contract_in_force` | `Safe` | `callback-uncontracted` naming `q_put`; `q_put_any` `Safe` |
| a premise, a premise-free declaration, a poisoned set | `a_premise_or_a_poisoned_set_withdraws_a_callback_candidate` (unit, `legacy_literal_premise.rs`) | | withdrawn, not withdrawn, every name withdrawn |

The registration and constructed guards are not covered by the
`classified_in` guard (both read the stored contract). Each was reverted
alone once and its test then read the premise row `Safe`
(`private/logs/registry-unguarded.log`,
`private/logs/constructed-unguarded.log`; measured once, harness not
shipped). The constructed-path test runs without run facts (§7, the second
follow-up). The poisoned arm is reachable only in the unit test: a poisoned set also
renames every contracted call, so the invoker is undeclared before any
candidate is read. All of
`callback_literal_premise` (11), `openssl_undeclared_handshake_callbacks`
(17), `handoff_candidate_own_callbacks` (8), `cg22_callback_registry` (7)
and `op_constructor_declarations` (10) pass (`private/logs/gtest-tip-3.log`).

**Lean.** `QCore.CallbackCandidatePremise` (new, audited by
`proof/AuditCallbackCandidatePremise.lean`, 18 theorems) beside
`QCore.CallbackLiteralPremise`:

* a premise-carrying or poisoned candidate is `missing`; a premise-free one
  is classified as before; the guard only withdraws;
* an applied invocation or a releasing registry has no premise-carrying
  candidate;
* the guarded decision applies or releases only where the old one did, and
  equals it on a premise-free set;
* the `cand_*` vectors: `flb_hash_table_add`'s identity shape,
  `rd_kafka_topic_partition_destroy0`'s releasing shape and
  `d2i_PKCS12_bio`'s composed shape, each applied before and refused now.

Axioms are within `propext` and `Quot.sound` (`private/logs/lean-1.log`).

## 5. Measurement

**Pinned routes: not run on this host.** The pinned corpus, system and
slice routes need the prepared inputs under `build/cycle19` (and the
published baselines), which this host does not have, so neither `quirin-audit
pinned` nor `quirin-audit measure --declared` (the parity check a route change
calls for) could run. The prediction's `measure-prediction` block (none on
all three) is for the integrator. The pinned modules do carry the Fluent Bit,
cfl and libc premises, so a move there, if any, would be `Safe` or `Pending`
to `Unknown`, the fix working, and must be reported row by row.

**The opt-in `current` run (this card's arbiter).** `quirin-audit current
--monkey-linked` over Fluent Bit `046accf6` with
OPENSSL-SSL-CTX-CALLBACK-SLOTS'
[script](../../contracts-and-premises/openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh)
(`SOURCE_DATE_EPOCH=1767225600`, `--jobs 12`). Each side is a `git archive`
side tree with its own release `quirin-audit` and its own `package.py
checkers` step (`checkers.exit` 0 on both). Inputs were prepared by the base
binary's `prepare --fluent-bit ../fluent-bit --revision 046accf6 --clang
/usr/lib/llvm-21/bin/clang` into `private/fb-046accf6` (exit 0,
`private/logs/prepare.log`). The two runs overlapped in time.

| Run | Tree | Files / configurations | Functions Safe / Pending / Unknown / Unsafe | Bindings | Wall |
|---|---|---|---|---|---|
| base (`private/opt-in/base-1`) | `062974679` | 180 / 181: 9 Safe, 0 Pending, 5 Unsafe, 163 Unknown, 4 data-only | 1098 / 2 / 3699 / 5 | 113, all `Pending` | 58 min |
| tip (`private/opt-in/tip-1`) | `581e8f55a` | same | 1098 / 2 / 3699 / 5 | 113, all `Pending` | 58 min |

Both exit 0 with no unmatched row
([runs](evidence/opt-in/),
[comparison](evidence/opt-in/comparison-base-tip.json) by
OPENSSL-PROVIDER-REMAINING-CALLEES' `evidence/compare.py`). **As predicted,
no row moved and no row changed content**: no `Safe` to non-`Safe` move, no
new `Unsafe`, no row gaining a `callback-uncontracted` reading. The base
equals r33's tip figures. So no row of this route relied on a
premise-carrying candidate.

## 6. Gate

Command: `sh private/toolchain/env.sh python3 scripts/gate_scope.py --base
062974679 --out private/gate-1` (Clang 21.1.8, Lean 4.35.0), the scoped card
gate (full mode: the diff reaches `qcore-c-gateway`; no provider gate, no
provider file changed). Record: [`gate/gate-1.json`](gate/gate-1.json), logs
`private/gate-1/` and `private/logs/gate-1.log`. Tree `581e8f55a`; the
report, evidence and amendments were uncommitted (docs only). Overall exit 0,
8 steps passed.

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` | 0, 0, 0 | 1 submodule (`third_party/libevent`) |
| Rust suite (scoped) | 0 | 662 units, 707 summaries; 9707 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 453 audits (new `AuditCallbackCandidatePremise`) |
| Python suite | 0 | 779 tests |

No known flake fired. No new test target was added, so the read census was
not re-recorded. The committed matrix's `lean_audits` is 452 at base; this
card produces 453, which the integrator's amendment application and matrix
regeneration catch up (the card gate passes the card base, which excuses the
stale-matrix check).

## 7. Follow-ups

[`integration-amendments.json`](integration-amendments.json) (`python3
scripts/apply_amendments.py … --check` exits 0):

* retires `CALLBACK-CANDIDATE-LITERAL-PREMISE`;
* registers **CALLBACK-CANDIDATE-OBLIGATION-PREMISE**: route G also refuses
  every call of a callee with a loop-owner, thread-synchronization or
  lifecycle obligation, and those are still classified by their contract
  as candidates (`flb_start`, `flb_stop`, `flb_destroy`, the four
  `flb_input_collector_*`, the five `pthread_cond_*`; `pthread_join` is
  withdrawn here by its literal premise). Out of this card's scope
  (literal-argument premises); the discharge extends
  `withdraws_candidate`;
* registers **CONSTRUCTOR-PARAMETER-REGISTRATION-RUN-STORE**, observed in
  this card's fixture only: `declared_stores` makes no run store of a
  constructor's registration of its callback parameter into its result's
  member, so under the run's facts an effectful candidate registered
  through `q_op_new` read `Safe` (it is refused without run facts, where
  rule 12o reads the registration; `private/logs/constructed-run-probe.log`).
  The one shipped instance, `rd_kafka_op_new_cb`, stores the callback in its
  own body, which every measured run reads. The card's constructed-path test
  therefore runs without run facts.

**Files likely to conflict:** `legacy_callback_route.rs` (`classified_in`,
the registration arm), `legacy_constructed_route.rs`, `legacy_c.rs` (the
`Parser` struct and its initializer) with `legacy_probe_state.rs` and
`tests/unit/legacy_probe_state.rs` (any card adding a parser field),
`legacy_literal_premise.rs`, `docs/agents/rules/57.md` and the generated
`docs/agents/gateway.md` and `docs/agents/README.md`, contract specification
C04.5, `proof/QCore.lean` (generated),
`crates/qcore-c-gateway/tests/callback_literal_premise.rs`.
