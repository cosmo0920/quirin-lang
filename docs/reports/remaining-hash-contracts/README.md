# HASH-CONTRACTS — declared contract text for the four `flb_hash_table_add` sites — 2026-09-09

The named continuation of card **O04-B09B10**
([`docs/reports/remaining-o04-b09b10/README.md`](../remaining-o04-b09b10/README.md)
§1 rows 5-8 and §4), whose declaration of `flb_hash_table_add` carried the
recorded boundary **`O04-B09B10-VALUE-DEPENDENT`**.

**Base tree:** main `bf210eb` ("Consolidate wave 12: triangulated corpus re-pin,
follow-ons, ledger"), submodules initialised, `third_party/libevent` at
`d82464a2`. Every measurement below was taken on that tree with only this
card's own diff applied; the three cards running in parallel in
`crates/qcore-c-gateway` (O04-TRACE, REACH-GATE, BORROW-LIVENESS) are **not** in
this worktree and none of their changes is reflected here.

## Headline

The size-0 mode is landed as a **production** contract module, and the branch it
was read from is **enforced**, not assumed: a new declared *literal-argument
premise* restricts the declaration to call sites whose `val_size` actual is
exactly the literal `0`, and every other call is refused as if the function were
undeclared.

**The contract corpus moves 60 → 61 modules and the resource-v1 declaration
count 19 → 20. Both pins are updated deliberately** (§6), together with the
manifest's selected-module count 5 → 6.

**Nothing is promoted.** The 185-unit headline is **2 Safe / 42 provisional
Unsafe / 137 Unknown / 4 data-only**, unchanged, with **zero unit and zero
function verdict movement**, **no newly surfaced provisional `Unsafe` unit**, and
leak diagnostics unchanged at **271**. Exactly **one** diagnostic leaf in the
whole corpus moves, and it moves in the refusing direction (§5).

**One measured correction to the prior card.** O04-B09B10 recorded the
declaration as unfaithful at a positive size because
`flb_hash_table_entry_free` frees `entry->val` when `entry->val_size > 0`.
Reading the body (§1) shows the pointer freed there is the **table's own
allocation**, never the caller's, so *as to custody* `resource_borrowed` is
faithful on **both** arms. What is genuinely arm-dependent is **retention**, and
that is not a custody mode at all (§2.3). The premise is landed anyway, on the
stronger and honest ground that the declaration's whole faithfulness argument
was read off one arm (§3).

---

## 1. The pinned upstream body, read

Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, `src/flb_hash_table.c`.
`flb_hash_table_add` (line 401) reaches the value through
`entry_set_value(entry, val, val_size)` on **both** paths — the replacement path
(line 437) and the fresh-entry path (line 475). `entry_set_value` (line 357) has
exactly two arms:

| Arm | Condition | `entry->val` | `entry->val_size` | Freed by `entry_free`? |
|---|---|---|---|---|
| copy | `val_size > 0` | `flb_malloc(val_size + 1)`, then `memcpy` from `val` | `val_size` | **yes** — the callee's own allocation |
| reference | otherwise | `val` itself, no allocation, no copy | **`-1`** | **no** |

`flb_hash_table_entry_free` (line 85) frees `entry->val` under exactly
`entry->val && entry->val_size > 0`. A referenced value records `-1`, so **the
table never frees a pointer it was handed at size 0, on any teardown path.** The
upstream comment says it plainly: "just do a reference".

Three further facts read at the same time and recorded rather than modelled:

* the replacement path's `if (entry->val_size > 0) flb_free(entry->val)` frees
  the *previous* entry's value, and by the same rule that is a previous copy,
  never a previously referenced caller pointer;
* `flb_hash_table_add` declares `ssize_t val_size` while `entry_set_value`
  takes `size_t`, so a **negative** argument converts to a very large unsigned
  length and takes the *copy* arm. This card refuses every such call site
  (§3), so nothing here rests on it;
* nothing about the key, the eviction modes, the counters or the table's
  locking is claimed by anything below.

## 2. Which C04 mode is faithful

### 2.1 `resource_stored` is refused on two independent grounds

§C04.2 reads `resource_stored(p, owner, n)` as custody "indexed beneath the
designated owner", whose destruction or transfer cascades to the child.

1. **It is false of the body.** This table's teardown does not free a referenced
   value. Declaring it stored would move the caller's obligation to an owner
   that never discharges it — i.e. it would **delete the caller's later cleanup
   obligation** for a resource nothing else releases. That is the direction hard
   invariant 1 forbids, and the direction the card brief named as fatal.
2. **It is unreachable.** §C04.2 requires `owner` to name a parameter declared
   `Owned`. At every measured call site the table is reached as a member read of
   a borrowed instance, so no caller supplies live custody of `ht`.

Ground 2 is not argued, it is **executed**: `the_stored_mode_is_rejected_for_this_callee`
substitutes `resource_stored(val, ht, nonnull)` into the production module text
and asserts the loader refuses it by name.

### 2.2 `resource_borrowed` is the faithful mode, on both arms

The projection's per-parameter claim is custody. On the reference arm the table
frees nothing and demands nothing; on the copy arm it touches only storage it
allocated itself. So the caller keeps exactly the obligation it had before the
call, on either arm, and `resource_borrowed` — "no custody precondition, no
transferred obligation" — states exactly that.

Over the whole v1 parameter vocabulary this is the *only* mode with that
caller-side effect: `owned` demands live custody, `consumed` transfers,
`released` discharges, `out` creates custody at a destination, `stored`
transfers under an owner and `removed` detaches from one. That exhaustive
statement is proved in Lean (§7).

### 2.3 What the declaration does **not** say — the real arm-dependence

At `val_size == 0` the table **retains** the caller's pointer for as long as the
entry lives. Retention is a reachability fact, not a custody fact: this
projection does not claim the retained pointer stays valid, does not model the
entry's lifetime, and does not decide whether a later release of the value
leaves the table holding a dangling entry. That publication is O00 blockers
**B03/R13** and stays unmodeled. Both halves are proved separately in Lean —
the admitted arm retains, and retention does not change the caller-side effect —
precisely so neither can be read as the other.

This is the measured correction to `O04-B09B10-VALUE-DEPENDENT`: the
arm-dependence is real, but it lives in retention, not in the freed pointer.

## 3. Encoding the size-0 boundary

### 3.1 The vocabulary cannot condition on a literal — measured

`docs/QCORE_CONTRACT_SPEC_r0_3.md` §C04.2 lists seven parameter modes, none
value-dependent; §C04.4 records that "general success-dependent parameter modes
… are not in v1"; §C04 rule 3 admits only `ensures normal` predicate calls with
"exact identifier/comma forms". `qcore_schemas::contract::ResourceParameterMode`
has no field that could carry an argument value, and `ResourceContracts` carries
only `functions` and `source_assumptions`. **A first-class clause would be a
published-schema change**, which this card is forbidden to make.

### 3.2 The declared literal-argument premise

The premise is therefore a machine-parsed line of the contract module's own
text:

```text
// qcore-premise: literal_argument(flb_hash_table_add, 4, 0)
```

`crates/qcore-c-gateway/src/legacy_literal_premise.rs` reads every such line out
of `ResourceContracts::source_assumptions`, which is the exact declared text the
loader already retains, `check_source_assumptions` already reloads, and every
safety certificate already carries. W4 derivation, W4 validation and certificate
rederivation each hold that same text, so all three reproduce the restriction
identically — which is why the premise is carried in the module text rather than
in the manifest or an out-of-band table.

**Grammar, total-or-reject.** `literal_argument(NAME, INDEX, LITERAL)`, exactly:
`NAME` a C identifier the environment declares, `INDEX` inside that function's
declared parameter list, `LITERAL` a decimal integer with an optional sign and no
suffix. Anything else — a missing field, an unclosed form, a hex or suffixed
literal, an unknown function, an out-of-range index, two premises on one
parameter — **poisons** the whole premise set, and a poisoned set refuses
*every* contracted call in the analysis.

**Enforcement.** `build_function_cfg` rewrites the callee token of every call
that does not satisfy its declared premise to `qcore_premise_unmet__NAME`, an
identifier nothing declares. That is deliberately a *token projection* rather
than a check threaded through the contract lookups: the restriction then holds
uniformly at every reader of the stream — the out-alias normalizer, the typed
access facts, C02-P's untracked bases, the aggregate-store bases, the
member-address and bound-read admissions and the resource CFG — with no lookup
site able to forget it. It is the same technique
`legacy_c::recursion::contextual_cfg` uses for its checked scalar actuals. A
declarator position is never rewritten, so a unit that *defines* the callee keeps
its definition analyzable (asserted by
`the_callees_own_definition_is_never_rewritten`).

The rewritten call is refused before the lexical access gate, so the statement
reports the unmet premise rather than the access the withdrawn declaration would
have admitted. Both are the same `Unknown`; only the attribution differs.

**Direction.** The premise can only ever *withdraw* a declaration. `applied`
returns the declared contract unchanged or nothing — never a different contract
— which is proved in Lean, along with the fact that adding premises can never
re-admit what a smaller set withdrew.

### 3.3 The premise is not vacuous on the real corpus

Fifteen `flb_hash_table_add` call sites exist in the pinned Fluent Bit tree.
**Five** pass the literal `0`; **ten** do not:

| Call site | `val_size` actual | Premise |
|---|---|---|
| `src/flb_input_chunk.c:2606` `ht_log_chunks` | `0` | satisfied |
| `src/flb_input_chunk.c:2610` `ht_metric_chunks` | `0` | satisfied |
| `src/flb_input_chunk.c:2614` `ht_trace_chunks` | `0` | satisfied |
| `src/flb_input_chunk.c:2617` `ht_profile_chunks` | `0` | satisfied |
| `src/flb_oauth2_jwt.c:910` | `0` | satisfied |
| `src/flb_input_chunk.c:216` | `sizeof(value)` | **withdrawn** |
| `src/flb_callback.c:68` | `sizeof(struct flb_callback_entry *)` | **withdrawn** |
| `src/flb_upstream_ha.c:312` | `vlen` | **withdrawn** |
| `src/flb_env.c:186`, `:283` | `vlen` | **withdrawn** |
| `src/flb_http_common.c:308`, `:1171` | `value_length` | **withdrawn** |
| `src/http_server/flb_http_server_http1.c:464` | `(ssize_t) value_length` | **withdrawn** |
| `src/http_server/flb_http_server_http2.c:142`, `:561` | `(ssize_t) value_length`, `value_length` | **withdrawn** |

The census is pinned in Lean (`exactly_five_call_sites_satisfy_the_premise`,
`exactly_ten_call_sites_are_withdrawn`). Only two of the fifteen sites lie in a
pinned 185-unit translation unit, which is why the corpus effect is small (§5).

## 4. Rows 5-8, before and after

Measured on this worktree's base tree by
`crates/qcore-c-gateway/tests/hash_contracts.rs::print_the_measured_publication_ledger`
over the byte-verbatim upstream regions, under O04-B09B10's four non-hash
premises with and without this card's production declaration. Full transcript:
[`logs/publication-ledger.txt`](logs/publication-ledger.txt).

| Row | Statement (preprocessed line) | Before | After |
|---|---|---|---|
| 5 | `flb_hash_table_add(in->ht_log_chunks, tag, tag_len, ic, 0);` (264) | `indirect memory access …` | **decided** |
| 6 | `flb_hash_table_add(in->ht_metric_chunks, …, 0);` (268) | `indirect memory access …` | **decided** |
| 7 | `flb_hash_table_add(in->ht_trace_chunks, …, 0);` (272) | `indirect memory access …` | **decided** |
| 8 | `flb_hash_table_add(in->ht_profile_chunks, …, 0);` (275) | `indirect memory access …` | **decided** |
| 0 | `flb_trace(…)` (255) | uncontracted callee | unchanged |
| 9 | `return ic;` (277) | nullable-resource residue | unchanged |

Rows 5-8 decide "through the existing read family" exactly as O04-B09B10
predicted: C02-P's member-read admission was already implemented, and the only
thing missing was declared contract text. The function's status stays
`Unknown` — rows 0 and 9 are owned by two named follow-ons neither of which is
this card. `one_publication_moved_off_the_literal_is_refused` moves one
publication to `, 1)` and asserts exactly that one is refused, so the four
admissions are shown to rest on the premise.

## 5. Corpus impact — one leaf, attributed

### 5.1 Provenance

Canonical pinned route, on the shared prepared pinned clone (Fluent Bit
`66910c10…`, explicit `/usr/bin/clang` 21.1.8), following
[`docs/reports/corpus-repin/provenance.json`](../corpus-repin/provenance.json):

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-hash
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=<worktree>/build/cycle19/provider-packages-hash/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone>/fluent-bit-clang-source \
  <clone>/monkey/monkey-build/compile_commands.json \
  <reports>/monkey-source.json <reports>/resource-diagnostics.json
```

Run twice from this worktree with the same provider packages, before and after
the card's whole diff. Both exit 0 with zero gateway failures. Transcripts:
[`logs/corpus-before.log.gz`](logs/corpus-before.log.gz),
[`logs/corpus-after.log.gz`](logs/corpus-after.log.gz).

**The BEFORE run against the archived wave-12 baseline**
([`docs/reports/wave12-corpus/artifacts/`](../wave12-corpus/artifacts/)):

| Artifact | before vs archived |
|---|---|
| `diagnostics.json` | **byte-identical** (`a807b471…`) |
| `source.json` | **byte-identical** (`53d55b7e…`) |
| `chunk-contracts.json` | **byte-identical** (`c4f438e4…`) |
| `contracts.json` | differs — **path-literal provenance residue only** |
| `source-effects.json` | differs — **package-identity residue only** |

Both residue classes were recorded by CORPUS-REPIN and every consolidation
since, and both are identical between this card's before and after runs, so they
do not touch the isolation.

### 5.2 Before → after

Full attribution: [`logs/corpus-attribution.txt`](logs/corpus-attribution.txt).

| Measure | before | after |
|---|---|---|
| units | 2 Safe / 42 Unsafe / 137 Unknown / 4 data-only | **identical** |
| unit verdict moves | — | **0** |
| unit reason / reason_code moves | — | **0** |
| function status moves | — | **0** |
| `leak` / `unknown_alias` / `unsupported` totals | 271 / 4410 / 49960 | **identical** |
| units with changed diagnostics | — | **1** (`flb_input_chunk.c`) |

The whole corpus-wide diagnostic delta is two lines:

```
     0 ->      1  call to `flb_hash_table_add` outside its declared literal-argument premise
 18875 ->  18874  indirect memory access is outside the resource projection
```

and the single changed leaf is

```
-  update_logs_tag_records_metrics L17794 indirect memory access …
+  update_logs_tag_records_metrics L17794 call to `flb_hash_table_add` outside its declared literal-argument premise
```

which is `src/flb_input_chunk.c:216`, the `sizeof(value)` call site — the
premise firing on a real positive-size publication. The move is **fail-closed
and attribution-only**: the statement was `Unknown` before and is `Unknown`
after; what changed is that it now says why.

`contracts.json` changes in exactly three lines: the embedded manifest text and
one new `source_assumptions` entry for the added module. Nothing else moves.

### 5.3 Why rows 5-8 do **not** move in the production corpus

Measured, not assumed. The four publications keep `indirect memory access …` in
the pinned corpus because their base `in` is refused there for an unrelated
reason: `flb_input_chunk_create` calls
`flb_routes_mask_set_by_tag(ic->routes_mask, tag, tag_len, in)`, and that callee
is **uncontracted in production** (it is one of the O04 fixture's declared
premises), so C02-P's `seed_call_arguments` adds `in` to `risky` and the member
read is refused before any contract is consulted. The production function
retains seven uncontracted callees the fixture declares — `generate_chunk_name`,
`cio_chunk_is_up`, `cio_chunk_up_force`, `input_chunk_write_header`,
`flb_time_now`, `flb_log_print`, `cio_chunk_down` — so its blocker set is
strictly a superset of the fixture's. Landing those as production declarations
is a separate decision this card deliberately does not take.

## 6. Pin updates — stated loudly

| Pin | before | after | Why |
|---|---|---|---|
| `compiler/tests/qcore_contract_corpus.rs:389` corpus size | **60** | **61** | the new module |
| `compiler/tests/qcore_contract_corpus.rs` `EXPECTED_CONTRACTS` | 145 rows | **146** rows | one `extern fn`, 7 clauses, at the path-sorted position |
| `compiler/tests/contract_loader.rs` corpus module count | **60** | **61** | same file |
| `compiler/tests/contract_loader.rs` `(selected, v1, rejected)` | `(5, 19, 145)` | **`(6, 20, 145)`** | the module is entirely v1, so the rejected count does not move |
| `compiler/tests/contract_loader.rs` `source_assumptions().len()` ×2 | **5** | **6** | the manifest selects six modules |
| `contract/cycle19-resource-contracts.json` | 5 modules | **6 modules** | the declaration must be in force to matter |

`production_contracts_preserve_the_previous_resource_modes` keeps its
load-bearing half — every *previous* declaration is still compared against the
historical table — and the one new declaration is listed **explicitly** in that
table with a comment saying it is an addition, rather than exempted from the
comparison.

## 7. Lean

New module `QCore.LiteralArgumentPremise` and audit
`AuditLiteralArgumentPremise.lean`: **40 theorems**, no new axiom, no
placeholder. `python3 scripts/check_lean_audits.py` passes **58** audits
(57 before).

* **Conservativity** — an applied contract is the declared one unchanged; an
  undeclared callee is never applied; an environment with no premise is the
  unrestricted one exactly.
* **Every withdrawal ground** — poison, an unmet premise, a mismatched actual
  and a **missing** actual; only the exact literal satisfies.
* **Monotonicity** — extra premises preserve every withdrawal and can never
  re-admit.
* **Non-vacuity** — the literal call is applied, the positive and short calls
  are withdrawn, poison withdraws the literal call, and without the premise the
  positive call is applied.
* **The measured census** — fifteen sites, five satisfying, ten withdrawn.
* **The upstream arms** — the reference arm records `-1` and is never freed; the
  copy arm stores the callee's own allocation and is freed; and the sentence the
  whole declaration rests on: `the_callers_pointer_is_never_freed_by_the_table`.
* **Which mode is faithful** — `borrowed_is_the_only_mode_with_the_measured_effect`
  over the whole v1 vocabulary, plus the four per-mode corollaries, plus
  `no_stored_reading_of_the_publication_is_well_formed`.
* **Retention** — real on the admitted arm, absent on the withdrawn arm, and
  proved not to change the caller-side effect.

What is *not* proved: that the upstream arms are as transcribed (they were read
by hand from the pinned file, cited by line), that a declared `borrowed` mode
really borrows, and C's object model. Those are premises supplied from outside,
as in every card in this family.

## 8. Tests

23 new asserting tests plus one reporting aid.
`crates/qcore-c-gateway/src/legacy_literal_premise.rs` carries 7 unit tests over
the directive grammar (including a seven-spelling malformation matrix), and
`crates/qcore-c-gateway/tests/hash_contracts.rs` carries 16 tests and the aid:

* the held-out generic `p_*` family: admission at the literal; **the published
  value's own obligation survives the publication**; **closing the table
  discharges nothing**; a double release is still caught; a positive literal and
  six non-literal spellings are refused; without the directive the same
  positive-size call is admitted (the control that isolates the premise); a
  malformed directive refuses all five contracted callees; the certificate
  recheck refuses a call moved outside the premise; a definition is never
  rewritten;
* the real source: the four publications decide; exactly one moved off the
  literal is refused; neither real function is promoted; the four non-hash
  premises are character-for-character O04-B09B10's; the rewrite prefix collides
  with no input.

## 9. Gate

On the merged card at this worktree:

| Gate | Result |
|---|---|
| `cargo test --workspace` | **2341 passed, 0 failed, 6 ignored** |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `reevaluation_probe` / `reevaluation_followup` | exit 0 / exit 0 |
| `python3 scripts/check_lean_audits.py` | **58 audits** pass |
| `providers/` `cargo test` | **87 passed, 0 failed** |
| `python3 -m unittest discover -s scripts/tests` | 125 run, **1 failure** — see below |
| pinned 185-unit audit, before and after | exit 0, zero gateway failures |

**One expected, integrator-owned failure.**
`test_build_matrix_sections.BuildMatrixSections.test_the_derived_facts_are_recomputed_rather_than_carried`
compares the committed `docs/reviews/r0_3/acceptance_matrix.json` against the
generator's derived facts and reports `['9.0.0', 57] != ['9.0.0', 58]`: the
matrix records 57 Lean audits and the tree now has 58. This card is forbidden to
edit `acceptance_matrix.json`, and the file is regenerated at integration; the
tripwire is firing exactly as designed. **The integrator must regenerate the
matrix.** No other Python test moved.

`crates/qcore-verifier-host/tests/confined_process_execution.rs::the_evidence_records_the_achieved_confinement_and_its_boundary`
failed once under full-workspace parallelism with `Broken pipe` and passes in
isolation; it is the recorded P03 load-flaky case and its assertion was not
touched.

## 10. Open boundaries

* **`HASH-CONTRACTS-CLAUSE`** — the premise is carried in a comment because the
  v1 clause grammar is closed and a first-class clause needs a
  `qcore_schemas::contract` variant. A schemas increment should promote it to
  real syntax, at which point the loader rejects a malformed premise instead of
  poisoning the analysis. Recorded as a deliberate deviation, not an oversight.
* **`HASH-CONTRACTS-PAIRING`** — the loader rejects a premise naming a function
  the environment does not declare, but nothing can *require* a premise for a
  declaration that needs one. Pairing a declaration with its restriction remains
  the module author's obligation.
* **`HASH-CONTRACTS-RETENTION`** — the size-0 publication retains the caller's
  pointer, and no implemented route models the entry's lifetime. This is O00
  **B03/R13**; nothing here narrows it.
* **`HASH-CONTRACTS-CALL-POSITION`** — the declarator test that protects a
  callee's own definition is a token-level heuristic (the previous token is an
  identifier other than `return`, or `*`). A call whose previous token is an
  unusual keyword would not be rewritten. No such shape occurs in the corpus,
  and the whole test is asserted only in the *refusing* direction elsewhere.
* **Rows 0 and 9** of the O04-B09B10 census are unchanged and remain owned by
  that card's named follow-ons (a variadic tail form and nested-call modeling).
* The four publications' remaining production blocker (§5.3) is the seven
  fixture-only callee premises; landing them in production is a separate
  decision.
* The corpus baseline is **not** re-pinned here. `artifacts/` is this card's
  after-measurement, archived as its diff record; the wave-12 baseline is
  untouched and remains current until an integrator consolidation moves it.
