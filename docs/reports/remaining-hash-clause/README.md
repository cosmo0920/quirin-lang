# HASH-CLAUSE — the literal-argument premise becomes a real v1 clause — 2026-09-09

Closes the named follow-on **`HASH-CONTRACTS-CLAUSE`** recorded in
[`docs/reports/remaining-hash-contracts/README.md`](../remaining-hash-contracts/README.md)
§10: "the premise is carried in a comment because the v1 clause grammar is
closed and a first-class clause needs a `qcore_schemas::contract` variant. A
schemas increment should promote it to real syntax, at which point the loader
rejects a malformed premise instead of poisoning the analysis."

**Base tree:** main `009bde0` ("Consolidate wave 13"), submodules initialised,
`third_party/libevent` at `d82464a2`. Card PROD-CONTRACTS ran in parallel and
also touches `contract/qcore/` and the corpus pin tables; every count below is
stated **relative to base 61 modules**.

---

## Headline

The size-0 boundary is now a **clause of the declaration**, validated by the
loader total-or-reject:

```qcore
extern fn flb_hash_table_add(ht: ptr<mut, u8>, key: ptr<const, u8>, key_len: i32, val: ptr<mut, u8>, val_size: isize) -> i32
contract {
    requires resource_contract_v1;
    requires resource_literal_argument(val_size, 0);
    …
};
```

**The schemas change the prior card predicted is not required.** `qcore-schemas`
stays **10.0.0**, no wire format changes, and no obligation identifier or
archived task digest moves *because of this card* — the last of those is
measured against a base-tree control, not argued (§5.2). The premise
follows O04-TRACE's `resource_variadic_data_tail` precedent: it is rederived
from the environment's own retained trusted texts rather than carried on
`ResourceFunctionContract`, so the contract map — which is what the evidence
identities bind — is byte-for-byte what it was.

**The comment carrier is retired, loudly.** A contract module whose text still
contains the `qcore-premise` introducer is rejected by name, in the loader and
again by a corpus-wide sweep of all 61 modules. A leftover directive cannot rot.

**Parity is measured, not argued.** The refusal set at every call site is
identical under the clause to what the directive produced, taken with both
carriers live over eleven call-site shapes and the real upstream body, and
pinned so the claim survives the directive's removal (§4).

**Nothing is promoted.** The 185-unit headline is **2 Safe / 42 provisional
Unsafe / 137 Unknown / 4 data-only**, unchanged. `diagnostics.json` is
**byte-identical** to the wave-13 baseline; leak diagnostics stay at **271**.
This card's entire corpus effect is **one changed leaf**, triangulated against a
base-tree control (§5).

**One deliberate pin update:** `fluent_bit/hash_table_publication.qcore` moves
**7 → 8 clauses**. The corpus stays at **61 modules** and the resource-v1
declaration count stays at **20** (§6).

---

## 1. The clause, and why this spelling

The card brief proposed `requires literal_argument(INDEX, VALUE);` "or the
spelling the grammar's conventions dictate — study the existing clause forms".
Two deviations were taken from the proposal, both to match G05:

| Proposal | Landed | Why |
|---|---|---|
| `literal_argument` | **`resource_literal_argument`** | Every predicate in the v1 vocabulary is `resource_*`-prefixed. The unprefixed spelling is pinned **refused** so the two cannot be confused. |
| `INDEX` | **parameter *name*** | Every other predicate names parameters by name (`resource_borrowed(p)`, `resource_stored(p, owner, n)`). A name is checked against the declaration's own parameter list by the loader's existing `index(name)` lookup, so a typo cannot silently be a valid index. The brief's "index out of range" mutation row becomes "names a parameter this declaration does not declare", which is the same fail-closed check on a strictly more checkable input. |

`requires` rather than `ensures normal` is deliberate and is the third
convention point: this is a precondition on *applicability*, not a
postcondition, and `requires` was already the marker's envelope. §C04 rule 2 now
reads "exactly one `requires resource_contract_v1;` marker. The only other
admitted precondition is the literal-argument premise of C04.5"; every other
`requires` spelling still rejects.

**Grammar** (G05, updated in the same change):

```ebnf
resource_clause = "requires", "resource_contract_v1", ";"
                | "requires", literal_argument_premise, ";"
                | "ensures", "normal", resource_predicate, ";" ;
literal_argument_premise =
    "resource_literal_argument", "(", IDENT, ",", integer_literal, ")" ;
integer_literal = ["-"], DIGITS ;
```

The literal is recorded as the **C token texts** the spelling produces — `0` is
`["0"]`, `-1` is `["-", "1"]` — so the consumer compares token vectors against
the actual argument and never re-parses a number. Alternate bases and suffixes
lex as an *extra* token (`0x0` → `Integer("0")`, `Word("x0")`) and so fail the
structural match rather than being approximated; that is checked, not assumed
(§3 rows 5-6).

**Semantics** (§C04.5, new): the whole declaration applies only at call sites
whose actual at the named parameter is exactly that literal. A consumer must
treat the callee as *undeclared* everywhere else, including at a call that does
not supply an actual at that parameter at all. The clause may only withdraw call
sites; it may not widen one and may not alter any mode.

### 1.1 Why no schema change was needed

The prior card recorded that a first-class clause "needs a
`qcore_schemas::contract` variant — a published-schema change", which it was
forbidden to make. That is measurably not so, because O04-TRACE had already
established the pattern for a whole-declaration fact that is not a mode:
`resource_variadic_data_tail` is validated by the loader but **not** carried on
`ResourceFunctionContract`; it is rederived from `source_assumptions` by
`variadic_data_tails`. The premise takes the same route
(`literal_argument_premises` / `declared_literal_argument_premises`).

The consequence is the interesting one, and it is measured in §5.2: because the
contract *map* does not move, the obligation identifiers, task digests and
build digests that bind it do not move either.

### 1.2 The one fail-open direction, and how it is closed

For the variadic tail, "no declared sources ⇒ no tails" is fail-closed: fewer
tails means fewer admissions. For a premise it is the **opposite** — an empty
premise set applies every declaration at every call site. So the two consumers
differ deliberately:

* the **loader** returns an honest empty map for an environment with no retained
  sources, and an **error** for a source set it cannot read; and
* the **gateway** treats that error as a **poison**, refusing every contracted
  call in the analysis, and only skips the rederivation when no retained text
  mentions either the clause or the retired introducer.

Both directions are proved in Lean (§7) and pinned in Rust
(`a_clause_source_that_no_longer_reloads_poisons_the_set`,
`a_malformed_premise_refuses_every_contracted_call`).

## 2. The migration, and the retirement

The card ran as two commits so that both mechanisms were live and comparable
before either was removed.

| Commit | State |
|---|---|
| `a0d78c0` | The clause lands. **Both carriers live**; the gateway reads the union (sound because either can only withdraw, and two premises on one parameter poison rather than compete). Both carriers' premise sets and refusal sets measured equal — §4. |
| `7549fd9` | The production module and the generic fixture move to the clause; the directive parser is **deleted**; the loader **rejects** the introducer; the measured parity vector is pinned. |

The retirement is refusal, never silence, in three places:

1. **The loader** rejects any contract module source containing `qcore-premise:`,
   naming the replacement clause in the message. The check is deliberately
   broader than the shape the old parser recognised — any occurrence, any comment
   shape — so a directive cannot survive by being written differently.
2. **The gateway** poisons a *retained text* still holding the introducer. A
   module with the directive cannot be loaded, but an environment assembled by
   other means could still carry one, and a carrier nothing parses must not
   silently leave a declaration unrestricted.
3. **A corpus-wide test** sweeps all 61 modules. Only the six the manifest
   selects ever reach the resource loader; this catches a directive rotting
   unread in one of the other 55.

## 3. Mutation table — every row refuses

| # | Mutation | Where it refuses | Test |
|---|---|---|---|
| 1 | `resource_literal_argument(size)` — literal missing | loader, module rejected | `a_malformed_clause_rejects_the_whole_module` |
| 2 | `(size, 0, 0)` — extra field | loader, module rejected | ″ |
| 3 | `(absent, 0)` — **parameter the declaration does not declare** (the brief's index-out-of-range row) | loader, module rejected | ″ |
| 4 | `(size, zero)` — non-literal value | loader, module rejected | ″ |
| 5 | `(size, 0x0)` — alternate base | loader, module rejected | ″ |
| 6 | `(size, 0u)` — suffixed literal | loader, module rejected | ″ |
| 7 | `resource_literal_arguments(size, 0)` — misspelled predicate | loader, module rejected | ″ |
| 8 | `literal_argument(size, 0)` — the brief's unprefixed spelling | loader, module rejected | ″ |
| 9 | `resource_literal_argument(size, 0) and true` — Boolean combination | loader, module rejected | ″ |
| 10 | Two clauses on one parameter | loader, module rejected | `two_clauses_on_one_parameter_reject_the_module` |
| 11 | Retired directive present, no clause | loader, module rejected **by name** | `the_retired_comment_directive_rejects_the_module` |
| 12 | **Clause-vs-directive conflict**: retired directive beside a live clause | loader, module rejected | `the_retired_directive_is_refused_beside_a_live_clause` |
| 13 | **Substitution at rederivation**: retained text replaced with one the loader cannot read | gateway, premise set **poisoned** | `a_clause_source_that_no_longer_reloads_poisons_the_set` |
| 14 | Retained text still holding the retired directive | gateway, premise set **poisoned** | `a_retained_text_still_holding_the_directive_poisons_the_set` |
| 15 | A poisoned set reaching the analysis | **all five** contracted callees refused, no certificate | `a_malformed_premise_refuses_every_contracted_call` |
| 16 | An admitted call moved outside the premise in the certificate's own source | certificate recheck refuses | `a_certificate_rests_on_the_declared_premise` (retained) |
| 17 | `resource_stored(val, ht, nonnull)` substituted into the production text | loader, refusal names the declaration | `the_stored_mode_is_rejected_for_this_callee` (retained) |

Row 12 is the conflict case the brief asked for. During the transition
(commit `a0d78c0`) the same shape **poisoned** rather than rejecting, and that
was tested there; in the landed state the loader refuses it earlier, which is
strictly louder.

Rows 1-12 are a real strengthening over the directive: the directive was parsed
*after* the module loaded, so a typo poisoned the analysis; a malformed clause
now means the environment never exists.

## 4. Parity — measured with both carriers live, then pinned

`crates/qcore-c-gateway/tests/hash_contracts.rs` runs the held-out generic
publication at eleven call-site shapes and records whether the declaration was
withdrawn. Transcript, taken at commit `a0d78c0` with **both** carriers present:
[`logs/carrier-parity.txt`](logs/carrier-parity.txt).

```
val_size actual             directive   clause  pinned
0                             applied  applied applied
8                           withdrawn withdrawn withdrawn
1                           withdrawn withdrawn withdrawn
-1                          withdrawn withdrawn withdrawn
n                           withdrawn withdrawn withdrawn
sizeof(struct p_value)      withdrawn withdrawn withdrawn
(long) n                    withdrawn withdrawn withdrawn
0 + 0                       withdrawn withdrawn withdrawn
-0                          withdrawn withdrawn withdrawn
0L                          withdrawn withdrawn withdrawn
(0)                         withdrawn withdrawn withdrawn
```

Eleven of eleven agree. The vector is pinned as `PINNED_REFUSALS`, so the claim
stays checkable now that the directive column cannot be recomputed.

On the **real** upstream body the complete ordered diagnostic vector for
`flb_input_chunk_create` was compared under both carriers at `a0d78c0` and is
now pinned as `PINNED_REAL_DIAGNOSTICS`:

```
call to uncontracted function `flb_trace`
nullable resource `ic` used by a non-null contract
```

Two entries, neither a premise refusal — the four publications decide, exactly as
HASH-CONTRACTS §4 measured. These two residues are rows 0 and 9 of O04-B09B10's
census and remain owned by that card's named follow-ons.

**Why premise-set equality is enough.** The refusal set is computed by
`restrict_to_declared_literals`, which this card does not touch and which reads
nothing but the premise set. That the refusal set is a function of the premise
environment alone is proved in Lean
(`equal_carriers_decide_every_call_site_alike`), so measuring premise-set
equality — which the unit test
`the_clause_and_the_directive_declare_the_same_premise_set` did at `a0d78c0` —
establishes refusal-set parity at *every* call site, not only the measured ones.

**The one corpus premise leaf keeps its exact spelling.** HASH-CONTRACTS'
single moved leaf was
`update_logs_tag_records_metrics L17794 call to `flb_hash_table_add` outside its
declared literal-argument premise`. `diagnostics.json` is byte-identical to the
wave-13 baseline (§5), so that leaf is unchanged character for character.

## 5. Corpus — one changed leaf, triangulated

### 5.1 Provenance

Canonical pinned route on the shared prepared pinned clone (Fluent Bit
`66910c10…`, explicit `/usr/bin/clang` 21.1.8), provider packages rebuilt at
this tree, exit 0, **zero gateway failures**. Log:
[`logs/corpus-run.log.gz`](logs/corpus-run.log.gz).

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-hashclause
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=<worktree>/build/cycle19/provider-packages-hashclause/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone>/fluent-bit-clang-source \
  <clone>/monkey/monkey-build/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

Three measurements, not two. The archived wave-13 baseline was produced from the
main checkout with that wave's provider packages; this card runs from a worktree
with freshly built packages, so an **archive → after** diff alone would conflate
this card's effect with two recorded environment residue classes. A **control**
run of the *base tree* `009bde0` in this same worktree, with these same provider
packages, separates them exactly.

### 5.2 The three-way result

| Artifact | archive → control | control → **after** |
|---|---|---|
| `diagnostics.json` | byte-identical | **byte-identical** |
| `source.json` | byte-identical | **byte-identical** |
| `chunk-contracts.json` | byte-identical | **byte-identical** |
| `contracts.json` | differs — 12 path-literal leaves | differs — **1 leaf** |
| `source-effects.json` | differs — 32 identity-digest leaves | **byte-identical** |

**This card's entire corpus effect is one changed leaf**: the
`hash_table_publication.qcore` module text retained in `contracts.json`
(5235 → 5225 bytes), which is the migration itself. Nothing else in any artifact
moves.

The two archive → control classes are the ones every consolidation since
CORPUS-REPIN has recorded, and neither is this card's:

* `contracts.json` — six absolute-path map keys, worktree location provenance;
  the six module *texts* are identical after path normalisation except the one
  this card edits.
* `source-effects.json` — 32 leaves on 3 of 185 results, all identity digests
  (`package_digest`, `build_digest`, `configuration_digest`, `task_digest`) and
  the `resource_contracts:<hash>` obligation identifiers keyed by them. No
  verdict, status, reason or diagnostic is among them (established by an
  exhaustive leaf walk, not by sampling).

**The measured schema finding.** `source-effects.json` is byte-identical between
control and after. The clause therefore moves **no obligation identifier and no
task digest** — because the premise is rederived from the retained text rather
than folded into `ResourceFunctionContract`, the contract map the evidence
identities bind is unchanged. Had the premise been added as a schema field, all
three of those identities would have moved and every archived witness would have
been retired. That is the concrete payoff of following the variadic-tail
precedent, and it is measured rather than predicted.

### 5.3 Headline

| Measure | wave-13 baseline | after |
|---|---|---|
| units | 185 | 185 |
| verdicts | 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only | **identical** |
| unit verdict moves | — | **0** |
| function status moves | — | **0** |
| `leak` / `unknown_alias` / `unsupported` | 271 / 4410 / 49956 | **identical** |
| gateway failures | 0 | **0** |

Per scope: cmetrics 31 (0/16/15/0), ctraces 17 (1/2/13/1 data-only), librdkafka
94 (0/10/81/3 data-only), chunkio 13 (0/2/11/0), fluent-bit-chunk 1 (0/1/0/0),
monkey-libevent 29 (1/11/17/0).

**The baseline is not re-pinned here.** `docs/reports/wave13-corpus/artifacts/`
remains current; the only artifact this card would move is `contracts.json`, by
its own module text, and that is an integrator consolidation decision — not least
because PROD-CONTRACTS is editing the same corpus in parallel.

## 6. Pin updates — stated loudly

| Pin | before | after | Why |
|---|---|---|---|
| `compiler/tests/qcore_contract_corpus.rs` `EXPECTED_CONTRACTS`, `fluent_bit/hash_table_publication.qcore` | **7** clauses | **8** clauses | the premise becomes a clause of the declaration |
| corpus size (`qcore_contract_corpus.rs`, `contract_loader.rs`) | 61 | **61** | unchanged |
| `contract_loader.rs` `(selected, v1, rejected)` | `(6, 20, 145)` | **unchanged** | a clause is not a declaration |
| `source_assumptions().len()` ×2 | 6 | **unchanged** | |
| `qcore-schemas` | 10.0.0 | **10.0.0** | no schema change; see §1.1 |

The load-bearing negative halves of every touched pin are kept: the corpus test
still asserts every other file's clause counts, and
`production_contracts_preserve_the_previous_resource_modes` still compares every
declaration's modes against the historical table — which is what makes "the modes
did not change" a checked statement rather than a claim.

One **new** pin: the corpus-wide sweep asserting no module contains the retired
introducer.

## 7. Lean

`QCore.LiteralArgumentPremise` grows from 40 to **50** theorems in
`AuditLiteralArgumentPremise.lean`; the audit count stays at **62** (an existing
audit file was extended, not added). No new axiom, no placeholder.

The ten new theorems model the **carrier**:

* **Parity, in general form.** `equal_carriers_decide_every_call_site_alike` and
  `equal_carriers_withdraw_alike`: two carriers stating the same premises decide
  every call site identically. This is what makes the Rust side's measurement of
  *premise-set* equality sufficient for *refusal-set* parity — the measurement
  is over eleven shapes, the theorem is over all of them.
* **The instance.** `the_migrated_carriers_state_the_same_premises`,
  `the_migration_preserves_every_refusal`,
  `the_clause_reading_is_the_declared_environment`.
* **Non-vacuity.** `the_carriers_can_disagree` — a text whose carriers differ
  decides differently, so the parity theorem is not trivially true.
* **The retirement, both directions.**
  `an_ignored_directive_readmits_the_withdrawn_call` exhibits the hazard: a
  reader that silently ignores a leftover directive applies the declaration at
  exactly the call site the premise withdrew.
  `refusing_the_directive_text_withdraws_every_call` proves refusing closes it.
* **Why poison is not empty.**
  `the_fail_open_reading_applies_what_poison_refuses` and
  `the_fail_open_reading_even_applies_at_an_unmet_premise`: on the same
  unreadable text the empty-set reading applies the declaration where the
  poisoned reading refuses, including at a call that never reaches the declared
  parameter.

What is **not** proved is unchanged from the prior card: that the upstream arms
are as transcribed, that a declared `borrowed` mode really borrows, and C's
object model. Newly, the Lean carrier model abstracts the *parsing* of either
carrier — it models what a carrier states, not that the loader's parser extracts
it correctly. That correspondence is Rust-tested (§3), not proved.

## 8. Tests

26 asserting tests plus two ignored reporting aids in the two files this card
owns:

* `crates/qcore-c-gateway/src/legacy_literal_premise.rs` — 10 unit tests over the
  clause grammar, the nine-spelling malformation matrix, the duplicate-parameter
  rejection, negative literals, the two retirement directions, the rederivation
  substitution and the no-sources case.
* `crates/qcore-c-gateway/tests/hash_contracts.rs` — 18 integration tests
  (2 ignored aids), all of HASH-CONTRACTS' originals retained and re-pointed at
  the clause, plus the two parity pins.

## 9. Gate

Run on the merged card at this worktree.

| Gate | Result |
|---|---|
| `cargo test --workspace` | **2461 passed, 0 failed, 8 ignored** |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `reevaluation_probe` / `reevaluation_followup` | exit 0 / exit 0 |
| `python3 scripts/check_lean_audits.py` | **62 audits** pass |
| `providers/` `cargo test` | **94 passed, 0 failed** |
| all five provider packages built and verified | pass |
| `python3 -m unittest discover -s scripts/tests` | **125 run, 0 failures** |
| pinned 185-unit audit | exit 0, zero gateway failures |

One recorded load-flaky observation:
`cycle20_verifier_plugins::vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
failed once under full-workspace parallelism and passes in isolation. It is the
recorded vp04 flake; its assertion was not touched.

## 10. Open boundaries

* **`HASH-CLAUSE-TYPE`** — the clause accepts a premise on *any* parameter,
  including a pointer. Only a scalar premise is meaningful in practice, but the
  directive imposed no such restriction either, and adding one would be a new
  refusal with no measured need. Recorded, not repaired.
* **`HASH-CLAUSE-LITERAL`** — the literal vocabulary is a decimal integer with an
  optional minus. Character constants, `enum` constants, macro-expanded spellings
  and anything a call site writes as `0L`, `(0)` or `0 + 0` are all **refused**,
  which is the fail-closed direction but also means a real call site that
  passes a differently spelled zero is not covered. Measured, §4.
* **`HASH-CONTRACTS-PAIRING`** — unchanged and still open: the loader rejects a
  premise naming a parameter the declaration does not have, but nothing can
  *require* a premise for a declaration that needs one. Pairing a declaration
  with its restriction remains the module author's obligation.
* **`HASH-CONTRACTS-RETENTION`** and **`HASH-CONTRACTS-CALL-POSITION`** — both
  unchanged; this card touches neither the custody reading nor the token-level
  declarator test.
* **Observed, not repaired:** §C05's sentence "The reviewed
  `contract/cycle19-resource-contracts.json` selects five modules and nineteen
  resource-v1 declarations" has been stale since HASH-CONTRACTS moved it to six
  and twenty. It is left alone deliberately: PROD-CONTRACTS may move the same
  counts again in this wave, and a pin fight over one prose sentence is not worth
  it. The integrator should correct it once the wave's module count settles.
* The corpus baseline is **not** re-pinned here (§5.3).
