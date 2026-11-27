# CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE

Discharges `CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE`, registered by
[CG25-IN-PLACE-REGEX](../../gateway-and-clang/cg25-in-place-regex/README.md) (r64). USER DECISION
2026-11-24: "launch them all". Base `3ca85c7a6` (main, r65). Gateway
[rule 74n](../../../../agents/gateway.md#constraint-74n), amending
[rule 74i](../../../../agents/gateway.md#constraint-74i) (3).

## 1. The clause

```
ensures normal resource_kind_failure_use(p, KIND);
```

It states that the in-place use at `p` is **defined on an object whose
initialiser may have failed**, provided no finaliser has run since. It is a
predicate the loader reads into the contract IR, not a premise statement, so
QF001 and its successors do not arise.

**Placement** (contract spec §C04.6.1; grammar G05; `contract_in_place.rs`
`check_failure_uses`). `p` must be a `resource_borrowed` parameter of type
`ptr<const, u8>` or `ptr<mut, u8>` that also carries `resource_kind(p, KIND)`,
and `KIND` must be that position's own in-place kind. The module is rejected
whole for each of these:

* an initialiser position;
* a finaliser position;
* a by-value `thread_key` `u32` position;
* a position without the kind, or with another kind;
* a kind that is not in place;
* `heap`;
* a duplicate clause.

**IR.** `ResourceFunctionContract::failure_uses: BTreeSet<usize>` and
`is_failure_use`. The set is left out of the serialized form and the `Debug`
text when empty, so every other contract stays byte-identical.
`custody_projection` drops it, because no derived summary can state it. A
declaration that carries it is therefore never checked against a body.

**Reading** (`legacy_in_place_custody.rs`, `failure_use_admitted`, called
from rule 74i's `report_uncertified_key_uses`). Rule 74i may not certify a use
that carries the clause. Such a use is still admitted when the settled
binding of the object, on every reaching path:

* is an identity and is not `unknown`;
* names at most one site, which is live and released on no reaching path;
* if it names no site, is definitely null. Only rule 11v's nonzero edge
  leaves that state, because it removes the site.

The positions follow from that reading:

| Position | Reading |
|---|---|
| Before the status test | Admitted: the site is live and possibly failed. |
| On the failure edge | Admitted. |
| A join of the two | Admitted. |
| Before any initialiser | Refused: a path with no binding joins to `unknown`. |
| After `regfree` | Not admitted: keeps rule 74i's reading, the `use_after_close` candidate. |
| After a `regfree` on the failure edge | Not `Safe`: that `regfree` already refuses (`UnknownAlias`). |
| A use without the clause, `regexec` included | Exactly rule 74i's reading. |

**Decision recorded for the user.** One reading would admit only the
failure edge and still refuse `regerror` before the status test. It is
narrower than the brief asked for. POSIX supports the brief: before the
test, the object is either compiled or failed, and `regerror` is defined on
both. A zero `errcode` leaves only the message unspecified. Both readings
restore `rd_regex_match`, which calls `regerror` only on the failure edge.
This card implements the brief's reading.

## 2. POSIX grounding

The Open Group Base Specifications Issue 8 (IEEE Std 1003.1-2024),
`regcomp()` page, which also covers `regerror`, `regexec` and `regfree`:

* **regerror:** it generates a string for `errcode`, "which the application
  shall ensure is the last non-zero value returned by regcomp() or regexec()
  with the given value of preg. If errcode is not such a value, the content of
  the generated string is unspecified." A failed `regcomp(&re, …)` returns
  exactly that value for that `preg`. The call is therefore defined on the
  object the failed `regcomp` left, even though "the content of preg is
  undefined" after the failure.
* **regexec:** "If the preg argument to regexec() or regfree() is not a
  compiled regular expression returned by regcomp(), the result is
  undefined." So `regexec` does not carry the clause.
* **regfree:** "A preg is no longer treated as a compiled regular expression
  after it is given to regfree()." So the clause admits nothing after the
  finaliser.

The Linux `regex(3)` page agrees: "If preg isn't a null pointer, errcode must
be the latest error returned from an operation on preg." The citation is
also in `regex.qv` beside the clause.

## 3. Changes

* **Contract IR.** `crates/qcore-schemas/src/contract.rs` adds the
  `failure_uses` field and `is_failure_use`; `custody_projection` clears the
  field.
* **Loader.**
  * `crates/qcore-contract/src/contract_loader.rs` reads the clause beside
    `resource_kind` and assembles it into the contract.
  * `crates/qcore-contract/src/contract_in_place.rs` adds `FAILURE_USE` and
    `check_failure_uses`.
* **Gateway.** `crates/qcore-c-gateway/src/legacy_in_place_custody.rs` adds
  `failure_use_admitted`, consulted only at a position where
  `is_failure_use` holds.
* **Provider.** `providers/quirin-provider-libc/contracts/libc/regex.qv`
  adds `resource_kind_failure_use(preg, regex)` on `regerror`, with the POSIX
  citation. Only `regerror` carries it.
* **Specification.** `docs/QCORE_CONTRACT_SPEC_r0_3.md` §C04.6.1 gains the
  paragraph "A use defined on a failed object".
  `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` G05 gains the production and the
  static conditions.
* **Lean.** `proof/QCore/InPlaceFailureUse.lean` and
  `proof/AuditInPlaceFailureUse.lean` are new; `proof/QCore.lean` was
  regenerated.
* **Rules.** `docs/agents/rules/74n.md` is new. `docs/agents/rules/74i.md`
  (3) now records the discharge, and its "Do not" now exempts rule 74n's
  clause. `docs/agents/gateway.md` and `docs/agents/README.md` were
  regenerated.
* **Tests.**
  * `crates/qcore-c-gateway/tests/in_place_regex_failure_use.rs` (new).
  * `crates/qcore-contract/tests/in_place_failure_use.rs` (new).
  * `crates/qcore-c-gateway/tests/in_place_regex_certified.rs`: the r64
    pin `a_regerror_on_the_failure_edge_is_refused` is retired. It pinned
    the recorded over-refusal; its two shapes are now pinned `Safe` in the
    new file.
  * `compiler/tests/contract_loader.rs`: the previous-mode table
    (`production_contracts_preserve_the_previous_resource_modes`) gives
    `regerror` `failure_uses {1}`. This is a deliberate pin update. The first
    gate run caught it (`private/gate-2/rust-suite.log`: 1 failed, the
    `regerror` mode).

## 4. Lock diff

Pinned provider locks are load-bearing (AGENTS.md invariant 5). Both lock
changes are in the implementation commit with `regex.qv`.

* **`providers/quirin-provider-libc/contract-inventory.lock`**, from
  `providers/contracts.py sync`:
  * `libc/regex.qv`: `clause_count` 29 -> 30 and `sha256` `f224448b…` ->
    `5b0483c9…`.
  * `regerror`: `clauses` 8 -> 9, and its `resource_mode` gains
    `"failure_uses": [1]`.
  * The file's declaration count and every other declaration are unchanged.
  * Corpus totals, recomputed from `providers/*/contract-inventory.lock`:
    154 modules, 1,002 declarations, 8,800 -> **8,801** clauses. STATUS is
    edited to match.
* **`providers/quirin-provider-libc/contracts.lock`**, the `quirin-audit pin`
  lock:
  * The first `sync` exited 1 with "stale lock: `regerror`'s contract text
    changed since it was pinned" (`private/card-logs/contracts-sync.log`).
  * `regerror` was re-pinned with `quirin-audit pin --source-root
    private/fb-046accf6/fluent-bit-clang-source --crate quirin-provider-libc
    --function regerror` (`private/card-logs/pin-regerror.log`, exit 0). The
    entry stays `flexible`; `contract_sha256` `2e144f76…` -> `fe06553a…`;
    `pinned_at` 2026-10-25T18:10:00Z -> 2026-11-24T00:30:00Z.
  * This card is the review of that text change. No other entry moved, and
    the lock stays tied to `046accf6`.
* **Second `sync`.** It exited 0 (`private/card-logs/contracts-sync-2.log`).
  `contract-load-path.json` is unchanged, because `regex.qv` is not a
  published module.
* **No other pins.** No provider pins a premise or declaration census that
  this touches. The clause is not a premise, and the declaration count is
  unchanged.

## 5. Pins

`crates/qcore-c-gateway/tests/in_place_regex_failure_use.rs` runs through the
production `regex.qv`. At base it ran with the base sources and the new test
files (`private/card-logs/pins-at-base.log`: 4 passed, 2 failed). At the tip
it ran 6 passed (`private/card-logs/pins-tip-1.log`).

| Test | Shapes | Base | Tip |
|---|---|---|---|
| `a_regerror_on_the_failure_edge_is_safe` (change) | `regerror` on the nonzero edge; `rd_regex_match` as rdregex.c spells it (r64's string) | Unknown (74i) | **Safe**, no diagnostic |
| `a_regerror_before_the_status_test_is_safe` (change) | `regerror` before `if (r)`; after a join of the zero and nonzero edges | Unknown (74i) | **Safe** |
| `a_regexec_on_the_failure_edge_is_still_refused` | probe R3, probe R4, `regexec` after an admitted `regerror` on the failure edge | Unknown (74i) | Unknown (74i) |
| `a_regerror_before_the_initialiser_is_refused` | `regerror` before `regcomp`; a path with no `regcomp` joined | Unknown (74i) | Unknown (74i) |
| `a_regerror_after_regfree_is_not_admitted` | `regerror` after `regfree`; after a join with a freeing path | non-Safe, `use_after_close` | the same |
| `a_regerror_after_a_failure_edge_regfree_is_not_safe` | `regfree` then `regerror` on the nonzero edge | Unknown | Unknown (`UnknownAlias` at the `regfree`) |

Both change pins fail at base with exactly rule 74i's refusal text; the
controls pass at base and at the tip. The `rd_regex_match` string was pinned
`Unknown` by r64's `a_regerror_on_the_failure_edge_is_refused`, which passed
at base.

`crates/qcore-contract/tests/in_place_failure_use.rs` has three tests:

* the production `regerror` has `failure_uses == {1}`, and `regexec`,
  `regcomp` and `regfree` have none, also absent from their JSON and
  `Debug`;
* the clause loads on both pointer types;
* the eight rejected placements, and an unknown parameter, each reject with
  a failure-use (or heap-kind) message.

At base the file does not compile, because the field does not exist
(`private/card-logs/loader-pins-at-base.log`). At the tip all 3 pass. The
other eight r64 pins in `in_place_regex_certified.rs` pass unchanged at the
tip.

**Lean**, `QCore.InPlaceFailureUse`. `transferF` is rule 74i's `transferC`
with one extra instruction, `failureUse`. That instruction is admitted where
`admitted` holds, and is otherwise rule 74i's use.

* `admitted_iff` characterises the admitted states: bound to a site that is
  live or gone, never released. `certified_is_admitted` shows the clause
  only widens rule 74i.
* Positions:
  * `admitted_before_the_status_test` and `admitted_on_the_failure_edge`;
  * `refused_before_the_initialiser`;
  * `not_admitted_after_the_finaliser`, which is the `use_after_close`
    candidate;
  * `a_use_without_the_clause_is_rule_74i` and
    `a_plain_use_on_the_failure_edge_is_refused`.
* Each step is rule 74i's or an admitted failure use
  (`each_step_is_74i_or_an_admitted_failure_use`).
* `a_program_without_the_clause_is_rule_74i` and
  **`a_new_safe_verdict_rests_on_an_admitted_failure_use`**.
* **`a_safe_verdict_used_only_admitted_objects`**.
* Witnesses:
  * `rd_regex_match`'s failure path is refused under rule 74i and `Safe`
    now; its success path stays `Safe`; `regerror` before the test is
    `Safe`;
  * `regexec` on the failure edge, `regerror` before `regcomp`, after
    `regfree` (a candidate) and after a failure-edge `regfree` are not
    `Safe`.

`#print axioms` lists only `propext` (and `Quot.sound` where inherited); there
are no placeholders and no project axioms.

## 6. The Unknown -> Safe argument (invariant 1)

Restoring `rd_regex_match` to `Safe` flips an `Unknown` to `Safe`. Invariant 1
forbids a refactor from doing that. This is not a refactor: a new declared
obligation discharges the refused use.

1. Before this card, `rd_regex_match` had one leaf: the rule 74i refusal at
   `regerror(r, &re, …)` on the nonzero edge. The refusal stood because no
   declared fact said that the use is defined on a failed object. Its other
   uses were already certified: `regexec` after the zero edge, and `regfree`.
2. The clause is that fact. It is a trusted assumption under trust boundary
   28, like every libc contract, and it is grounded in POSIX's `regerror`
   text (§2). It is stated only on `regerror`'s `preg`.
3. Rule 74n admits the use only where the clause's meaning holds: the object
   is bound to its latest initialiser's outcome, and no finaliser has run on
   any reaching path. Every other state still refuses, or keeps its candidate.
4. The Lean model proves the boundary. A program without the clause is
   unchanged (`a_program_without_the_clause_is_rule_74i`). A verdict can
   become `Safe` only through an admitted failure use
   (`a_new_safe_verdict_rests_on_an_admitted_failure_use`,
   `a_safe_verdict_used_only_admitted_objects`).
5. No `Unknown` moves without the clause. Only `regerror` declares it, and
   `rd_regex_match` is the only tracked `regex` object on any measured row
   (r64's census). The measurement in §7 confirms the one move.

## 7. Measurement against the prediction

The arbiter is opt-in `current --monkey-linked` over Fluent Bit 046accf6.
Each side ran from its own `git archive` tree with its own release binary
([runs](evidence/runs.md)):

| | Safe / Pending / Unknown / Unsafe | moved rows |
|---|---|---|
| base `3ca85c7a6` | 1,171 / 2 / 3,623 / 8 | |
| tip `16ad6f982` | 1,172 / 2 / 3,622 / 8 | `rdregex.c:rd_regex_match` Unknown -> Safe |

**The measurement matches the [prediction](PREDICTION.md) exactly.**

* One row moved, the predicted one. Its one leaf was removed: the rule 74i
  refusal at `rdregex.c:127`, the `regerror`.
* No other row changed status or leaves, and no new `Unsafe` appeared.
* The access-gate rows and leaves are unchanged: 2,118, 215 and 8,992.
* The base figures equal CG25-IN-PLACE-REGEX's tip, so r65 moved nothing
  here.

**Pinned routes.** `quirin-audit measure --declared` cannot run on this
host: it has no `build/cycle19` inputs and no baselines. It was not run, and
no result is claimed. The prediction block expects `rd_regex_match`
Unknown -> Safe on the corpus, and no move on system or slice. The
integrator measures with base `rev:3ca85c7a6` against the prediction.

## 8. Files likely to conflict

* **Generated:** `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`. Rerun `gen_rule_docs.py` and `gen_lean_roots.py`.
* **`crates/qcore-schemas/src/contract.rs`:** the `ResourceFunctionContract`
  struct, `new`, `Debug` and `custody_projection`. Any card that adds a
  contract field touches the same lines.
* **`crates/qcore-contract/src/contract_loader.rs`:** the kind-clause match
  and the contract assembly near `contract.link_members`.
* **`crates/qcore-c-gateway/src/legacy_in_place_custody.rs`:**
  FRAME-HOLDER-SHADOWED-NAME also reads `Parser::automatic_object` in this
  file. This card edits only `report_uncertified_key_uses` and adds
  `failure_use_admitted`.
* **Provider locks:** `providers/quirin-provider-libc/contract-inventory.lock`
  and `contracts.lock`. On conflict, rerun `providers/contracts.py sync` and
  re-pin `regerror`.
* **`docs/agents/rules/74i.md`** (3).
* **`docs/STATUS.md`:** the contract-corpus row (8,800 -> 8,801 clauses),
  through `status_edits`. A parallel card that changes clause counts needs a
  recount at landing.

## 9. Follow-ups

None registered. `thread_key` has no defined use on a failed key (POSIX
leaves a key value not obtained from `pthread_key_create` undefined), and the
clause refuses a by-value position, so no other in-place kind needs it. No
other libc callee was examined, which is out of scope.

## 10. Gate

`sh private/toolchain/env.sh python3 scripts/gate_scope.py --base 3ca85c7a6
--out private/gate-3` ran at `cbd597bfe` (the code is that of `ad79c44f7`;
`cbd597bfe` adds only this directory) and exited 0. The record is
[gate.json](gate.json).

* **Scope:** full, because the diff reaches `qcore-c-gateway`.
* **Lean:** every audit was checked, because `proof/QCore.lean` changed.
* **Provider gate:** selected, because `providers/` changed.

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy | 0, 0, 0 | |
| Rust suite | 0 | 686 units, 731 summaries; 10,017 passed, 0 failed, 24 ignored, 148 filtered |
| reevaluation probe, follow-up | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 497 audits |
| Python suite | 0 | 807 tests, 0 skipped |
| provider package build, verify-all, verify (libevent) | 0, 0, 0 | |
| provider tests | 0 | 49 summaries; 174 passed, 0 failed, 1 ignored |
| provider fmt, clippy | 0, 0 | |

Logs are in `private/gate-3/`, with per-unit Rust logs under `rust-suite/`.

**Earlier attempts, not evidence:**

* `private/gate-1` failed clippy on an unused test constant and was
  stopped. The fix is `ba5746092`.
* `private/gate-2` failed the Rust suite in one unit,
  `qcore-compiler::test::contract_loader`: the `regerror` previous-mode pin,
  fixed in `ad79c44f7`. In that run, Lean (497) and Python (807) passed. It
  was stopped during the provider tests.

**Other checks:**

* `python3 scripts/apply_amendments.py
  docs/reports/regex-regerror-failure-edge/integration-amendments.json
  --check` exited 0.
* `providers/contracts.py sync` exited 1 on the first run, because of the
  stale pin lock. After the re-pin it exited 0 (§4).
