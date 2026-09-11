# ADDRESS-OF-MEMBER-DESTROY-UNDECLARED — `flb_input_chunk_destroy` declared, and the leak-273 leaf decided — 2026-09-11

## Headline

**The leak pin moves 273 -> 272, deliberately, and the leaf card
ABI-TYPEDEF-ADDRESS-OF-MEMBER opened one wave ago is decided — as a
*withdrawal*, not a discharge.** `fluent-bit-chunk/flb_input_chunk.c`,
`input_chunk_get`, preprocessed line **19849**,
`leak: live obligation acquired at preprocessed line 19835`, is **removed**, and
with it that function's provisional `Unsafe`: `input_chunk_get` moves
**`Unsafe` -> `Unknown`**, the corpus's only status move of any kind. The
projected-function histogram goes **150 / 85 / 4,599 -> 150 / 84 / 4,600**, unit
verdicts are **unchanged at 2 `Safe` / 42 provisional `Unsafe` / 137 `Unknown` /
4 data-only**, and **nothing is promoted anywhere** — no unit or function became
`Safe`, and `flb_input_chunk_create`'s function-level `Safe` is byte-identically
where wave 20 left it (0 diagnostics).

The pin moved because the declared release's **actual is refused**, not because
it was admitted. At all five call sites in the corpus the projection cannot
resolve the pointer it is handed, so `apply_resource_action` reports the refusal
and calls `mark_unresolved_effect`, and `report_leaks` skips a site marked that
way. The leak claim rested on `flb_input_chunk_destroy` being invisible; with the
callee declared, the statement is modelled and the *reaching state* is what
refuses — which is a strictly more honest `Unknown` than the leak it replaces,
and is **not** a discharge and **not** an upstream-defect withdrawal of anything
else. `CONSTRUCTOR-RESIDUALS-LEAK-271` is untouched: `flb_input_chunk_get_direct_routes`
keeps all seven of its leaves and its `Unsafe`, on a different callee this card
deliberately does not declare.

Twelve leaves moved — six added, six removed — **all twelve in one unit, at the
five call sites named in the prediction before the run**. Corpus totals:
`leak` 273 -> **272**, `unknown_alias` 4,403 -> **4,408**, `unsupported`
49,825 -> **49,821**. Contract corpus **71 -> 72 modules**, manifest-selected
**16 -> 17**, v1 declarations **37 -> 38**. Both audit runs exit 0 with zero
gateway failures and **zero custody summary conflicts** in all 136 reporting
units. No Rust under `crates/` or `compiler/src/` is touched, so the admission
relation is unchanged and **no Lean module needed adding or amending**
(90 audits, same as the base tree).

---

## 1. The declaration, and the argument that it is faithful

[`contract/qcore/fluent_bit/input_chunk_destroy.qcore`](../../../contract/qcore/fluent_bit/input_chunk_destroy.qcore):

```
extern fn flb_input_chunk_destroy(ic: ptr<mut, u8>, del: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_released(ic, nonnull);
    ensures normal resource_borrowed(del);
    ensures normal resource_return_none();
};
```

Read against the body at Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`,
`src/flb_input_chunk.c:2704-2807`. The module's own header carries the full
reading; the two facts the declaration turns on are these.

**One exit, and the release is on it.** There is no `return` anywhere between
`:2704` and the `return 0;` at `:2807` — every conditional in the body is a
statement-level guard, never an early exit. So these run on **every** path:

```c
cio_chunk_close(ic->chunk, del);                                  /* :2796 */
mk_list_del(&ic->_head);                                          /* :2797 */
if (ic->routes_mask != NULL) { flb_free(ic->routes_mask); ... }   /* :2799 */
flb_free(ic);                                                     /* :2804 */
```

Those are exactly the three acquisitions card ABI-TYPEDEF-ADDRESS-OF-MEMBER
measured `flb_input_chunk_create` handing to its caller through the returned
`ic` — the `cio_chunk_open` result, the `flb_calloc` result and the routes-mask
allocation — released in the same order, and nothing the caller still owns is
left over. The caller's obligation is discharged in full and unconditionally,
which is what `resource_released` says. Over the whole v1 parameter vocabulary
it is the **only** mode with that caller-side effect: `resource_borrowed` would
leave a discharged obligation standing and `resource_stored` would consume it
under an owner that does not exist, and an unfaithful declaration is worse than
`Unknown`.

**`nonnull`, because the first statement dereferences.** The body opens with
`if (flb_input_chunk_is_up(ic) == FLB_FALSE)` (`:2712`), and
`flb_input_chunk_is_up` (`:3128`) immediately evaluates `ic->chunk`. A null `ic`
is undefined behaviour before any guard could see it. Declaring `nullable` would
be unfaithful in the **permissive** direction — it would discharge a caller's
obligation at a site where the projection cannot prove the pointer non-null,
which is the opposite of fail-closed. `flb_free` carries `nullable` because
`free(NULL)` is defined; this is not that. §6 shows this choice is load-bearing:
it is exactly the conjunct that refuses `input_chunk_get`'s actual.

`del` is an `int` by value — the only mode the loader admits on a scalar and
also the true one, the same position `chunkio/resource_open_close.qcore` gives
`cio_chunk_close`'s own `delete`. The result is `int`, is the literal `0` on the
one exit and carries no custody.

Recorded as **not claimed**, in the module and here: nothing about the
size-accounting loop, nothing about whether `flb_hash_table_del_ptr`'s "TRY"
found the entry or whether some other table still holds the pointer about to be
freed (the unmodeled retention this corpus records as B03/R13 — **this module
claims the release HAPPENS, never that it is SAFE**), nothing about
`cio_chunk_close` failing (the body ignores its result and frees anyway),
nothing about `ic->trace` (the pin is compiled `-DFLB_HAVE_CHUNK_TRACE`, so
`flb_chunk_trace_destroy(ic->trace)` *is* in the analysed body; it releases a
member, not the caller's `ic`). And `flb_input_chunk_destroy_corrupted` (`:2623`)
and `flb_input_chunk_destroy_direct_routes` (`:2483`) are **deliberately left
undeclared** — different bodies, different exits, and the second is the callee
`CONSTRUCTOR-RESIDUALS-LEAK-271` rests on. Named `AOM-DESTROY-SIBLINGS-UNDECLARED`.

### 1.1 Declaring a function this very unit defines

`flb_input_chunk_destroy` is defined in `src/flb_input_chunk.c`, itself a corpus
unit. That is the arrangement cards PROD-CONTRACTS and NEIGHBOURS already
established for `flb_input_chunk_get_name`, `generate_chunk_name` and
`input_chunk_write_header`: a declared contract always wins over a W4-inferred
summary, a disagreement is a `SummaryConflict` diagnostic and never a silent
override, and declaring a defined function does not stop the unit analysing that
function's own body — it only fixes what its **callers** may assume.

Measured, not assumed: `flb_input_chunk_destroy`'s own row is
**`Unknown`, 39 diagnostics, BEFORE and AFTER**
([`logs/probe-flb_input_chunk_destroy-after.txt`](logs/probe-flb_input_chunk_destroy-after.txt)).
The declaration moved nothing about it.

---

## 2. The prediction, and where it landed

[`prediction.md`](prediction.md) was written and **committed on the base tree**
(`6b6996d6`, parent `ec5535a9`) after the BEFORE run was launched and before any
AFTER measurement existed. It was right on every checkable point, including the
one it flagged as uncertain.

| prediction | measured |
|---|---|
| leak pin moves **273 -> 272** under *both* arms, because `report_leaks` skips `unresolved_effect_sites` | **273 -> 272** |
| **primary (B): the REFUSING arm**, not the discharging one — the actual will not resolve at `L19847` | **(B).** `L19847` reports `unresolved resource identity \`ic\`` *and* `nullable resource \`ic\` used by a non-null contract`, then retires the residual |
| at `L19847`: uncontracted leaf removed; `unknown_alias` added; `nullable resource` added; `L19849` leak removed | **all four, exactly** |
| `input_chunk_get` `Unsafe` -> `Unknown`, a withdrawal, never `Safe` | **`Unsafe` -> `Unknown`**, 23 diagnostics before and after |
| `flb_input_chunk_destroy`'s own status `Unknown`, unchanged | **`Unknown` 39 -> `Unknown` 39** |
| unit `fluent-bit-chunk/flb_input_chunk.c` provisional `Unsafe`, unchanged | **unchanged** |
| function histogram 150 / 85 / 4,599 -> **150 / 84 / 4,600** | **exactly that** |
| **five** call sites, all in this one unit, none elsewhere in the corpus | **exactly five**, §4 |
| `unsupported` down at least 5; `unknown_alias` up to +5; **no** `double_release`, `release_after_escape` or `custody_conflict` | `unsupported` **-4** (five removed, one `nullable` leaf added), `unknown_alias` **+5**, and none of the three appeared |
| LOADER-ABI type agreement: **zero withdrawals of any class** | zero, in both 185-unit runs |
| LOADER-ABI-MODES: W4 derives no summary for this body, so the duty gate is not reached; `summary_conflict` **0** | **0** in all 136 reporting units, BEFORE and AFTER |
| **no** Lean module needs adding or amending | none; 90 audits, unchanged |

The one place the prediction under-committed is worth naming rather than
glossing: **RIPPLE-UNKNOWN-CASCADE** predicted that at a wholly-unresolved
actual, `mark_unresolved_effect`'s `pointer.unknown` branch marks *every* site
in the state and could therefore retire leaks other than the target one. It
retired **none**, and the reason is checkable in the frozen wave-20 archive: of
the four functions carrying a call site, only `input_chunk_get` carries a leak
at all (§4). That bound was read off the frozen archive *after* the prediction
was committed and *before* the AFTER run, and it is stated here as a refinement
of the prediction, not as part of it. **RIPPLE-W4** produced nothing either: no
function anywhere in the corpus gained or lost a finding through a
newly-derivable summary.

---

## 3. The route, and the BEFORE run's oracle

```sh
git submodule update --init --recursive          # FIRST: the libevent pin gate fires at the END of a run
python3 -B scripts/prepare_cycle19_clang_audit.py \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 \
  --output <worktree>/build/cycle19/clang-pin --clang /usr/bin/clang --clangxx /usr/bin/clang++
python3 -B providers/package.py build --output build/cycle19/provider-packages-destroy
sh measure/run-audit.sh <before|after> <binary>
```

A **fresh** Clang pin, per the wave-20 corpus record's discarded-first-attempt
lesson: the compile-database identity is load-bearing, so the long-lived
developer `fluent-bit-clang-production` directory is not reused. The pin's own
production database is copied into `<source>/build/compile_commands.json`,
which the preparation script does not create, and **both runs read the same
bytes** (SHA-256 `3e20ad90a9e35049908f3e44cc58889d838a1694ba8ba59c104bd88677aeef4c`,
1,415 entries, `/usr/bin/clang` spelled — not `cc`). Both runs: **exit 0, zero
gateway failures, all six scopes summing to 185 units.** Each binary was built
once and executed directly from a worktree-private `measure/bin/`, never from
`target/`; memory was checked before each run (~20 GB available throughout, the
audit's own peak far below it) and no run was discarded or killed.

**The BEFORE run reproduces the frozen wave-20 baseline exactly.** The tree was
clean at `ec5535a9` when it started, and
`diff_artifacts.py --archived docs/reports/wave20-corpus/artifacts`
([`logs/diff-summary-before.txt`](logs/diff-summary-before.txt),
[`artifacts/diff-wave20-to-before.json.gz`](artifacts/diff-wave20-to-before.json.gz))
itemises **zero** differences in the two sections that carry verdicts:

```
--- chunk-contracts: results 96/96 +0 -0 ~0;  backend_teardown 21/21 +0 -0 ~0
--- contracts:       sources 16/16, manifest 16/16, audit_selection 5/5;  keys {}
--- diagnostics:     units 185/185, +0 -0 ~0
--- source:          envelope key `compile_entry`; 4 wrapper rows changed
--- source-effects:  3 result rows changed
```

and the histograms agree to the last number
([`headline.py`](headline.py)):

| measure | wave-20 frozen archive | BEFORE run |
|---|---|---|
| units | 185 · 2 `Safe` / 42 `Unsafe` / 137 `Unknown` / 4 data-only | **identical** |
| projected functions | 4,834 · 150 / 85 / 4,599 | **identical** |
| `leak` / `unknown_alias` / `unsupported` | 273 / 4,403 / 49,825 | **identical** |
| per library (6 of 6) | cmetrics `0/16/15`, ctraces `1/2/13`+1, librdkafka `0/10/81`+3, chunkio `0/2/11`, fluent-bit-chunk `0/1/0`, monkey-libevent `1/11/17` | **six for six** |

The residues — the `source` envelope's `compile_entry` path literal, 4 wrapper
rows and 3 `source-effects` result rows — are the checkout-root and
identity-digest classes every prior re-pin has recorded, and they are identical
in the BEFORE and AFTER diffs, so they belong to the route and not to this card.

One honest note on hygiene: this card's BEFORE run was taken on the working tree
rather than on a detached base checkout, so `prediction.md` (a new markdown file
under `docs/reports/`) and the unselected `.qcore` module were written to disk
while the process ran. Neither is an audit input — the produced
`resource-diagnostics.contracts.json` is written at process start, timestamped
before either file existed, and the manifest it records is the 16-module one.
The oracle above is the actual guarantee, and it is a byte-level one.

---

## 4. The candidate set, bounded against the frozen archive

`flb_input_chunk_destroy` is called from **exactly five** sites inside the one
corpus unit that defines it, and from nowhere else in the 185 units. The other
upstream callers (`flb_task.c`, `flb_storage.c`, `flb_input_log.c` and the
`tests/` callers) are outside the audited scope — the `fluent-bit-chunk` scope
holds exactly one unit. Read out of
`docs/reports/wave20-corpus/artifacts/diagnostics.json.gz` before the AFTER run:

| preproc line | source | enclosing function | its status / diagnostics | its leaks |
|---|---|---|---|---|
| `L18149` | `:718` | `flb_input_chunk_release_space` | `Unknown` / 51 | **none** |
| `L19847` | `:2907` | `input_chunk_get` | `Unsafe` / 23 | `L19849` — the target |
| `L20014` | `:3184` | `memrb_input_chunk_release_space` | `Unknown` / 15 | **none** |
| `L20180`, `L20199` | `:3454`, `:3485` | `input_chunk_append_raw` | `Unknown` / 67 | **none** |

Three of the four carry no leak at all, which is why RIPPLE-UNKNOWN-CASCADE had
nothing to retire there. The unit's other leak-carrying functions —
`flb_input_chunk_get_direct_routes` (7 leaves, `Unsafe`),
`input_chunk_collect_output_references` (1, `Unsafe`) and `append_to_ring_buffer`
(3, `Unsafe`) — call this function nowhere and are untouched, measured: all
three keep their exact status and diagnostic count.

---

## 5. Corpus — every leaf attributed

[`census.py`](census.py), transcript
[`logs/census-before-after.txt`](logs/census-before-after.txt):

```
units: 185 -> 185;  added: [];  removed: []
unit status before/after: {'Safe': 2, 'Unknown': 137, 'Unsafe': 42, 'data_only': 4}
unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834;  function status moves: 1
  FUNCTION (fluent-bit-chunk/flb_input_chunk.c, input_chunk_get): Unsafe -> Unknown
leak: 273 -> 272 (-1);  unknown_alias: 4403 -> 4408 (+5);  unsupported: 49825 -> 49821 (-4)
units with diagnostic-count changes: 1
leaves added: 6; leaves removed: 6
```

Twelve leaves, in **one** unit and **four** functions. Every one:

**`input_chunk_get` (`L19847`, `L19849`; `Unsafe` -> `Unknown`)** — §6.

**`flb_input_chunk_release_space` (`L18149`; `Unknown` -> `Unknown`, 51 -> 51)**

| leaf | attribution |
|---|---|
| `unsupported: call to uncontracted function \`flb_input_chunk_destroy\`` — **removed**, and `unknown_alias: unresolved resource identity \`old_input_chunk\`` — **added** | `flb_input_chunk_destroy(old_input_chunk, FLB_TRUE);` (`:718`). The statement is now modelled; the actual is a member-walked chunk from a `mk_list_foreach` over another instance's chunk list, which this body never binds to a resource identity, so `apply_resource_action` refuses it. **One `Unknown`-class refusal replaced by another at the same statement, reporting the reaching state instead of the syntactic shape** — the pattern card CONSTRUCTOR-RESIDUALS recorded for `mk_list_add` ("row 7's line reports the reaching state instead"). Net zero leaves, no verdict move |

**`memrb_input_chunk_release_space` (`L20014`; `Unknown` -> `Unknown`, 15 -> 15)**

| leaf | attribution |
|---|---|
| uncontracted leaf — **removed**, `unknown_alias: unresolved resource identity \`ic\`` — **added** | `flb_input_chunk_destroy(ic, FLB_TRUE);` (`:3184`), same shape and same cause: `ic` is a list-walked chunk this body never binds |

**`input_chunk_append_raw` (`L20180`, `L20199`; `Unknown` -> `Unknown`, 67 -> 67)**

| leaf | attribution |
|---|---|
| two uncontracted leaves — **removed** (2), two `unknown_alias: unresolved resource identity \`ic\`` — **added** (2) | `flb_input_chunk_destroy(ic, FLB_TRUE);` at `:3454` (the zero-content arm) and `:3485` (the non-routable arm). `ic` here comes from `input_chunk_get`, whose result this body does not bind to a tracked identity; both refuse identically |

**No other unit, no other function and no other leaf moved anywhere in the
185-unit corpus.** The description inventory moves by exactly the four rows the
twelve leaves account for:

```
leak       / 'live obligation acquired at preprocessed line 19835' : 9 -> 8  (-1)
unknown_alias / 'unresolved resource identity `ic`'               : 24 -> 28 (+4)
unknown_alias / 'unresolved resource identity `old_input_chunk`'  : 0 -> 1   (+1)
unsupported / 'call to uncontracted function `flb_input_chunk_destroy`' : 5 -> 0 (-5)
unsupported / 'nullable resource `ic` used by a non-null contract' : 2 -> 3  (+1)
```

`call to uncontracted function \`flb_input_chunk_destroy\`` goes to **zero**,
which is the whole measured payoff of the declaration and is what PROD-CONTRACTS'
rule requires before a premise is landed.

---

## 6. The leak pin moves 273 -> **272**, deliberately, by refusal

`fluent-bit-chunk/flb_input_chunk.c`, `input_chunk_get`, preprocessed line
**19849**. **This is a pin movement and it is deliberate** (constraint 7);
`prediction.md` declined in advance to promise the pin would hold, predicted it
would move to 272 either way, and named which arm it expected.

[`logs/probe-input-chunk-get-after.txt`](logs/probe-input-chunk-get-after.txt),
against wave 20's archived probe of the same function:

```
BEFORE                                              AFTER
L19847 n3  Unknown: call to uncontracted function   L19847 n3  Call flb_input_chunk_destroy(ic, _) -> _
       !! unsupported: call to uncontracted ...            !! unresolved resource identity `ic`
                                                           !! unsupported: nullable resource `ic` used by a non-null contract
L19849 n2  NoOp   | return ((void*)0);              L19849 n2  NoOp   | return ((void*)0);
       !! leak from L19835                                  (no leaf)

exits:  L19849 node 2 custody Unknown                exits:  L19849 node 2 custody Unknown
                     sites ["site0@L19835 Live"]                          sites ["site0@L19835 Mixed"]
```

| leaf | attribution |
|---|---|
| `L19847` `unsupported: call to uncontracted function \`flb_input_chunk_destroy\`` — **removed** | the declaration, working as intended: the statement is now a modelled `Call flb_input_chunk_destroy(ic, _) -> _` |
| `L19847` `unknown_alias: unresolved resource identity \`ic\`` — **added** | `apply_resource_action`'s refusal. `ic` at this point is the **join** of two provenances — the `flb_input_chunk_create` acquisition at `L19835`, narrowed non-null by `if (!ic) return NULL;`, and the `flb_hash_table_get` lookup path, which reaches the same program point without ever passing that guard and whose `ic` the projection never resolves. The join keeps `may_be_null` set, and `!nullable && may_be_null` is an unresolved actual |
| `L19847` `unsupported: nullable resource \`ic\` used by a non-null contract` — **added** | the same join, seen through the other conjunct: `site0` is in `nullable_sites` because `flb_input_chunk_create`'s declared result is nullable and this body's guard does not narrow it on the joined path. **This is the `nonnull` in the declaration doing its job**, and it is the reason the leaf set is one larger here than at the other four sites |
| `L19849` `leak: live obligation acquired at preprocessed line 19835` — **removed** | `apply_resource_action` calls `mark_unresolved_effect` on the refused actual, and `report_leaks` skips any site in `unresolved_effect_sites` — the same mechanism constraint 10 records for the publication refusals. The exit's residual goes `Live` -> `Mixed` and the claim is **withdrawn**, not discharged |

**What this does and does not say.** The leak was never an upstream defect claim
— invariant 2 governed it as it governs the 272 that stand — and it is not
withdrawn because the release was proved. It is withdrawn because, with the
callee declared, the projection can finally *say something* about the statement,
and what it says is `Unknown`: it cannot tell which of two joined provenances the
pointer carries, and the declaration it now consults requires non-nullness it
cannot establish. That is a strictly more informative refusal than "there is a
callee here I cannot see", and moving a function out of provisional `Unsafe` into
`Unknown` is a **withdrawal of a provisional claim**, never a promotion. Under
invariant 1 nothing flipped toward `Safe`: the unit stays provisional `Unsafe`,
no raw unit is promoted, the Cycle 19 gates are untouched.

**The discharging arm is what a faithful release contract exists for, and it did
not fire here.** It would have required `ic` to resolve to the single narrowed
site at `L19847`, and `input_chunk_get`'s own control flow prevents that. The
residual is therefore **not** about the declaration: it is
`input_chunk_get`'s two-provenance join, and repairing it is a checker question
about narrowing across the `if (!ic) { ic = create(...); }` join, not a contract
one. Named `AOM-DESTROY-JOINED-PROVENANCE` (§10).

**Relation to `CONSTRUCTOR-RESIDUALS-LEAK-271`.** That finding is a different
leaf in a different function (`flb_input_chunk_get_direct_routes`, `L19399`)
resting on a different uncontracted callee
(`flb_input_chunk_destroy_direct_routes`). Both were the same **shape** — a
release the projection cannot see because its callee is undeclared. This card
decides one of the two and **withdraws nothing** about the other: that function
keeps all seven of its leaves, its 99 diagnostics and its `Unsafe`, measured.
The leak-pin history is now three deliberate moves: 271 -> 272 (wave 17,
accusing), 272 -> 273 (wave 20, accusing), 273 -> **272** (this card,
withdrawing).

---

## 7. Against the frozen baseline, and constraint 22

`diff_artifacts.py --archived docs/reports/wave20-corpus/artifacts`
([`logs/diff-summary-after.txt`](logs/diff-summary-after.txt),
[`artifacts/diff-wave20-to-after.json.gz`](artifacts/diff-wave20-to-after.json.gz)):

```
--- chunk-contracts: NO DIFFERENCES (results 96/96, backend_teardown 21/21, +0 -0 ~0)
--- contracts:       sources 16 -> 17 (+1, 0 changed, 0 removed); manifest modules 16 -> 17 (+1)
                     added: qcore/fluent_bit/input_chunk_destroy.qcore
                     audit_selection 5 -> 5; keys {}
--- diagnostics:     {'leak': 273, 'unknown_alias': 4403, 'unsupported': 49825}
                  -> {'leak': 272, 'unknown_alias': 4408, 'unsupported': 49821};  units +0 -0 ~1
--- source:          envelope key `compile_entry`; 4 wrapper rows changed
--- source-effects:  3 result rows changed
```

`chunk-contracts` is **byte-identical** to the frozen baseline's — same
uncompressed SHA-256 `c4f438e4ed3dab3c…` — and `source` and `source-effects`
carry only the path-literal and identity-digest classes §3 already identified,
unchanged between the BEFORE and AFTER diffs. Exactly **one** unit's diagnostics
changed.

Constraint 22, reading the **normalised and raw** module keys together
([`raw_contract_keys.py`](raw_contract_keys.py),
[`logs/raw-contract-keys.txt`](logs/raw-contract-keys.txt)):

```
archived raw keys: 16      measured raw keys: 17
normalised added   : ['…/qcore/fluent_bit/input_chunk_destroy.qcore']
normalised removed : []
normalised collision (before): False      (after): False
archived checkout roots: ['/home/cosmo/Documents/GitHub/qcore-c/compiler/../']
measured checkout roots: ['…/worktrees/agent-a448cb40452261259/compiler/../']
```

**No normalisation collision on either side**, so no two modules were merged into
one row and the section was not demoted to raw keys; under normalisation
`contracts.json` gains exactly one module, removes none and changes none; and the
raw keys differ additionally only by the checkout root.

**The baseline is not re-pinned here.** `docs/reports/wave20-corpus/` remains the
corpus baseline exactly as it stands, and `docs/reports/wave17-corpus/` beneath it
(constraint 21 — published beside, never edited). The leak pin, the
function-status histogram and the two `unknown_alias`/`unsupported` totals are the
integrator's to consolidate (§11).

The `reason_code` sidecar tripwire (constraint 24) is unaffected: `unit
reason_code moves: 0`.

---

## 8. What was touched, and the pins moved deliberately

| file | what |
|---|---|
| `contract/qcore/fluent_bit/input_chunk_destroy.qcore` | **new.** One declaration, read against the pinned body, with its faithfulness argument and its non-claims |
| `contract/cycle19-resource-contracts.json` | manifest-selects it; 16 -> 17 modules |
| `compiler/tests/qcore_contract_corpus.rs` | corpus size 71 -> 72; one `EXPECTED_CONTRACTS` row `("fluent_bit/input_chunk_destroy.qcore", "flb_input_chunk_destroy", 4)` |
| `compiler/tests/contract_loader.rs` | three counts (`source_assumptions` 16 -> 17 twice, `modules.len()` 71 -> 72, `(selected, v1, rejected)` `(16,37,145)` -> `(17,38,145)`), and one new explicit row in the previous-modes comparison |
| `crates/qcore-c-gateway/tests/prod_contracts.rs`, `.../neighbours.rs` | `source_assumptions` 16 -> 17 each |
| `docs/reports/wave22-aom-destroy/` | this report, the pre-registered prediction, the tooling and the evidence |

**No Rust under `crates/*/src/` or `compiler/src/` is touched.** The admission
relation, the checker, the loader, the importer and the W4 machinery are all
byte-unchanged, which is why constraints 4, 5, 6, 10, 11 and 12 are undisturbed
by construction as well as by measurement, and why no Lean module needed adding
or amending.

Invariant 5, said plainly: five pinned expectations moved, each deliberately and
each named in the commit message. The one that deserves its own sentence is
`contract_loader.rs`'s previous-modes comparison. That test rebuilds the
pre-existing mode table and asserts every manifest declaration matches it, and
its existing rows are a list of all-borrowed declarations added inside a loop.
`flb_input_chunk_destroy` is the family's **first production declaration that is
not all-borrowed**, so it is listed explicitly beside `rd_realloc` with its own
mode vector `[Released { nullable: false }, Borrowed] -> None` and a comment
saying why `nonnull` and not the `nullable` that `flb_free` carries. It is stated
rather than exempted, which is the point of that test.

---

## 9. Lean

Nothing to add or amend, and this is a claim rather than an omission. This is a
**contract card**: the only executable artifacts it changes are one `.qcore`
module, its manifest entry and five test pins. The admission relation, the
refusal predicates, the leak-reporting rule and the unresolved-effect mechanism
are all byte-unchanged Rust, so there is no semantic change for a Lean definition
to track (invariant 6). `scripts/check_lean_audits.py` passes **90 audits**, the
same count and the same set as the base tree
([`logs/lean-audits.txt`](logs/lean-audits.txt)), placeholder-free with no
project axioms.

---

## 10. Gate

The full standing gate from `AGENTS.md`, run on the card's tip:

| step | result |
|---|---|
| `git submodule update --init --recursive` | run **first**, before either audit — the `libevent` pin gate fires at the *end* of a run |
| `cargo test --workspace` | **3,343 passed, 0 failed, 8 ignored** ([`logs/workspace-tests.txt.gz`](logs/workspace-tests.txt.gz)) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** — see §10.1 |
| `python3 scripts/check_lean_audits.py` | **90 Lean audits**, all passed ([`logs/lean-audits.txt`](logs/lean-audits.txt)) |
| `python3 -m unittest discover -s scripts/tests` | **193 passed** ([`logs/python-tests.txt`](logs/python-tests.txt)) |
| corpus audits | **both exit 0**, zero gateway failures, six scopes, 185 units |

Provider packages were **built** for the audit route (`providers/package.py build`,
`qcore-schemas` 14.0.0, exit 0) and are otherwise untouched: constraint 18's
rebuild-and-verify obligation is triggered by a manifest-template change or a
`qcore-schemas` major bump, and this card makes neither. No provider source,
manifest template or schema version is edited.

### 10.1 One load flake, in isolation (constraint 16)

`reevaluation_followup` **failed once** while the AFTER corpus audit was running
concurrently: `VP07_frame_limit_disagreement`, `process_result: "Err(Timeout)"`.
That is constraint 16's named
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets` row — the wall-clock one
that card `remaining-p03-f` measured failing at load 50 and passing at load 37 on
the same binary. It was re-run **in isolation** after the audit and the workspace
tests finished (load average 12, 23 GB available) and **passes, exit 0, every
observation `pass: true`**
([`logs/followup-probe-isolated.json`](logs/followup-probe-isolated.json)). The
row is not weakened and nothing about it is changed; it is recorded because a
nonzero probe exit is a finding to attribute, not a build failure to hide. No
other constraint-16 row tripped, in either the workspace run or the probes.

---

## 11. Named follow-ons

* **`AOM-DESTROY-JOINED-PROVENANCE`** — the residual this card measured rather
  than inferred. At `input_chunk_get` `L19847` the declared release is refused
  because `ic` is the join of a narrowed `flb_input_chunk_create` acquisition and
  an unresolved `flb_hash_table_get` lookup, so neither `may_be_null` nor the
  `nullable_sites` membership survives narrowing across the
  `if (!ic) { ic = create(...); ... }` join. This is a **checker** question about
  narrowing at a join, not a contract one, and it is the only thing standing
  between this leaf and the discharging arm. Note carefully that repairing it
  would move `input_chunk_get` from `Unknown` toward a modelled release, which is
  a direction invariant 1 constrains: it must be argued, measured and published,
  never absorbed.
* **`AOM-DESTROY-SIBLINGS-UNDECLARED`** — `flb_input_chunk_destroy_corrupted`
  (`:2623`) and `flb_input_chunk_destroy_direct_routes` (`:2483`) are deliberately
  left undeclared. The second is `CONSTRUCTOR-RESIDUALS-LEAK-271`'s callee and
  carries the seven-leaf family in `flb_input_chunk_get_direct_routes`; declaring
  either on the strength of its neighbour's name would be the
  unfaithful-declaration defect card P07 recorded. Deciding those leaves needs
  each body read on its own terms.
* **`AOM-DESTROY-OUT-OF-SCOPE-CALLERS`** — the declaration is in force for the
  whole corpus, but `flb_input_chunk_destroy`'s other upstream callers
  (`flb_task.c:929`, `flb_storage.c:634`, `flb_input_log.c:1431`) are **outside**
  the 185-unit scope, so this card measured no payoff there and claims none. If
  the audited scope grows to those units, the declaration applies unchanged and
  its effect there is unmeasured.
* **`AOM-DESTROY-RETENTION-UNMODELED`** carried forward, not new: the module
  claims the release HAPPENS, never that it is SAFE. Whether
  `flb_hash_table_del_ptr`'s "TRY" removed every reference before `flb_free(ic)`
  is the B03/R13 unmodeled retention and is untouched here.

---

## 12. For the integrator

Nothing in `AGENTS.md`, `DEVELOPMENT.md`, `README.md` or
`docs/reviews/r0_3/acceptance_matrix.json` is edited by this card, and no frozen
archive is touched. The facts to reconcile, all measured and all in this report:

1. **Leak pin 273 -> 272** (constraint 7). Attribution: `input_chunk_get`
   `L19849`, withdrawn by refusal at `L19847`, §6. Constraint 7's history entry
   grows to three deliberate moves.
2. **Function histogram 150 / 85 / 4,599 -> 150 / 84 / 4,600**; one function
   status move, `input_chunk_get` `Unsafe` -> `Unknown`.
3. **`unknown_alias` 4,403 -> 4,408; `unsupported` 49,825 -> 49,821.**
   Unit verdicts, the 185-unit denominator and the contract-clause pin are
   unchanged.
4. **Contract corpus 71 -> 72 modules**, manifest-selected 16 -> 17, v1
   declarations 37 -> 38, rejected unchanged at 145.
5. Baseline **not** re-pinned; `docs/reports/wave20-corpus/` stands.
6. `ADDRESS-OF-MEMBER-DESTROY-UNDECLARED` **closes**; four follow-ons open (§10).
