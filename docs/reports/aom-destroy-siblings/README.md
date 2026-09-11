# AOM-DESTROY-SIBLINGS — the two undeclared sibling destructors, decided differently — 2026-09-11

## Headline

**The two siblings card ADDRESS-OF-MEMBER-DESTROY-UNDECLARED left undeclared get
opposite answers, and reading each body on its own terms is what produced the
difference.** `flb_input_chunk_destroy_direct_routes` is declared — with a
**`nullable`** release where its already-declared neighbour carries `nonnull`,
because that body's third statement is `if (!routes) { return; }`.
`flb_input_chunk_destroy_corrupted` is **not** declared: its reading is written
out in full below, and its measured payoff is **zero**, because its single
in-corpus call site never reaches the call rule.

**The leak pin moves 272 -> 266, deliberately — a fourth move, the largest so
far, and the first to move by more than one.** All six removed leaves are in
`flb_input_chunk_get_direct_routes`, and they include `CONSTRUCTOR-RESIDUALS-LEAK-271`
(preprocessed line **19399**), which is now **decided — withdrawn**. The
withdrawal is **by refusal, not by discharge**: at all thirteen call sites the
projection cannot resolve the actual it is handed, so `apply_resource_action`
reports it and calls `mark_unresolved_effect`, and `report_leaks` skips a site
marked that way. That is the same mechanism wave 22 measured, and it means the
declaration's *discharging* arm is still unexercised anywhere in the pinned
corpus.

**Nothing is promoted anywhere, at any level.** Unit verdicts are unchanged at
**2 `Safe` / 42 provisional `Unsafe` / 137 `Unknown` / 4 data-only**; the
projected-function histogram is unchanged at **150 / 84 / 4,600** with **zero**
function status moves; `flb_input_chunk_get_direct_routes` keeps its seventh
leak leaf (L19544, the success exit) and therefore keeps its provisional
`Unsafe`; the unit keeps its provisional `Unsafe`; `flb_input_chunk_create`'s
function-level `Safe` is byte-identically where wave 20 left it (0 diagnostics).

Thirty-two leaves moved — 13 added, 19 removed — **all thirty-two in one unit
and two functions, at the thirteen call sites named in the prediction before the
run**. Corpus totals: `leak` 272 -> **266**, `unknown_alias` 4,408 -> **4,421**,
`unsupported` 49,794 -> **49,781**. Contract corpus **72 -> 73 modules**,
manifest-selected **17 -> 18**, v1 declarations **38 -> 39**. Both audit runs
exit 0 with zero gateway failures in all six scopes and **zero custody summary
conflicts** in all 136 reporting units. No Rust under `crates/*/src/` or
`compiler/src/` is touched — the BEFORE and AFTER runs used the **same binary**,
SHA-256 `746ab7603d22775c…` — so the admission relation is unchanged and **no
Lean module needed adding or amending** (95 audits, the same count and set as the
base tree).

---

## 1. The two bodies, read independently

### 1.1 `flb_input_chunk_destroy_direct_routes` — declared

[`contract/qcore/fluent_bit/input_chunk_destroy_direct_routes.qcore`](../../../contract/qcore/fluent_bit/input_chunk_destroy_direct_routes.qcore):

```
extern fn flb_input_chunk_destroy_direct_routes(routes: ptr<mut, u8>, route_count: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(routes, nullable);
    ensures normal resource_borrowed(route_count);
    ensures normal resource_return_none();
};
```

Read against the body at Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`,
`src/flb_input_chunk.c:2483-2504`; C declaration in force,
`include/fluent-bit/flb_input_chunk.h:162-163`. The module's own header carries
the full reading. The two facts the declaration turns on:

**Two exits, and `routes` is discharged on both.**

```c
if (!routes) { return; }                                        /* :2490-2492 */
for (index = 0; index < route_count; index++) { … }             /* :2494-2501 */
flb_free(routes);                                               /* :2503      */
```

The guarded exit is reached only when `routes` is null, and a null pointer
carries no caller obligation. On every other path control reaches
`flb_free(routes)` **unconditionally**: the only construct in between is one
`for` loop containing two `if` statements, with no `break`, no `return` and no
`goto` in it, and a non-positive `route_count` simply skips it. So the caller's
obligation is discharged in full on every path, which is what
`resource_released` says; over the whole v1 parameter vocabulary it is the only
mode with that caller-side effect.

**`nullable`, and this is where the sibling rule bites.** Card
ADDRESS-OF-MEMBER-DESTROY-UNDECLARED warned that declaring either sibling on the
strength of the other's name would be the unfaithful-declaration defect card P07
recorded. The warning was justified by the outcome: `flb_input_chunk_destroy` is
`nonnull` because its first statement dereferences `ic`, and copying that here
would assert a precondition this body demonstrably does not have — it guards its
own null argument and returns. `nullable` is the position `flb_free` holds, for
the same reason. The two contracts disagree on exactly the conjunct that decides
every call site.

`route_count` is an `int` by value — `resource_borrowed` is the only mode the
loader admits on a scalar and also the true one; it bounds the member-release
loop and selects nothing about the custody of `routes`. The result is `void`, so
`resource_return_none()`.

**Recorded as not claimed**, in the module and here: nothing about the member
releases (the loop frees `routes[index].label` only when it is non-null **and**
`routes[index].label_length > 0`, so a non-null label recorded with a zero
length would not be freed — a property this declaration asserts nothing about);
nothing about `route_count` agreeing with the allocated extent (this module
claims the release **happens**, never that it is **safe**); nothing about
retention (B03/R13).

### 1.2 `flb_input_chunk_destroy_corrupted` — read, and deliberately not declared

The body is `src/flb_input_chunk.c:2623-2701`; the C declaration is
`include/fluent-bit/flb_input_chunk.h:113-115`. It has **exactly one exit**
(`return 0;` at `:2700`; the only `continue`s are inside the accounting loop),
and on it:

```c
cio_chunk_close(ic->chunk, del);                                /* :2690 */
mk_list_del(&ic->_head);                                        /* :2691 */
if (ic->routes_mask != NULL) { flb_free(ic->routes_mask); … }   /* :2693 */
flb_free(ic);                                                   /* :2698 */
```

Its first statement is `mk_list_foreach(head, &ic->in->config->outputs)`
(`:2631`), which dereferences `ic->in->config` before any guard could see a null
`ic`. So the faithful contract, **written but not landed**, is:

```
extern fn flb_input_chunk_destroy_corrupted(ic: ptr<mut, u8>, tag_buf: ptr<const, u8>,
                                            tag_len: i32, del: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_released(ic, nonnull);
    ensures normal resource_borrowed(tag_buf);
    ensures normal resource_borrowed(tag_len);
    ensures normal resource_borrowed(del);
    ensures normal resource_return_none();
};
```

`nonnull`, unlike its other sibling and like `flb_input_chunk_destroy` — which is
the point: **the two answers are not correlated with each other, only with the
bodies.**

**It is not landed because its measured payoff is zero.** The whole pinned
185-unit corpus contains **no** `call to uncontracted function
\`flb_input_chunk_destroy_corrupted\`` leaf — checked on the frozen wave-23
archive and again on this card's own BEFORE run. The reason is exact and citable.
The function has one in-corpus call site, in `input_chunk_get`, and in the
analyzer's own `clang -E -P` view it is

```
 19823 | flb_input_chunk_destroy_corrupted(ic,
 19824 |                                   tag, tag_len,
 19825 |                                   in->config->storage_del_bad_chunks);
```

whose fourth actual is an indirect access, so the statement is refused at
**L19823** with `unsupported: indirect memory access is outside the resource
projection` and **never reaches the uncontracted-call rule**. A declaration
cannot move a statement that is refused before any contract is consulted.
Landing it would add a trusted assumption to the corpus and buy nothing.

This is the same disposition, under the same unchanged PROD-CONTRACTS rule, that
card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF gave `msgpack/packer.qcore` — written,
measured at payoff zero, declined, and landed later by
ABI-TYPEDEF-ADDRESS-OF-MEMBER once the blocker was repaired. Named
`AOM-DESTROY-CORRUPTED-UNPAID-PREMISE` (§9). Its own function row is
**`Unknown`, 30 diagnostics, BEFORE and AFTER**, unmoved.

### 1.3 Declaring a function this very unit defines

`flb_input_chunk_destroy_direct_routes` is defined in `src/flb_input_chunk.c`,
itself a corpus unit — the fourth such declaration, after PROD-CONTRACTS' and
NEIGHBOURS' helpers and wave 22's `flb_input_chunk_destroy`. A declared contract
always wins over a W4-inferred summary, a disagreement is a `SummaryConflict`
diagnostic and never a silent override, and declaring a defined function does not
stop the unit analysing that function's own body.

Measured, not assumed: `flb_input_chunk_destroy_direct_routes`'s own row is
**`Unknown`, 5 diagnostics, BEFORE and AFTER**. The declaration moved nothing
about it. Its body carries those five diagnostics, so W4 derives no summary for
it and constraint 8's duty gate is never reached — `summary_conflict` is **0** in
all 136 reporting units in both runs.

---

## 2. The prediction, and where it landed

Two documents, both committed before any AFTER measurement existed and neither
edited afterwards:

* [`prediction.md`](prediction.md), commit `38ec8e0c`, on the **base tree**,
  before the BEFORE run.
* [`prediction-addendum.md`](prediction-addendum.md), commit `b31d038b`, while
  the BEFORE run was in flight. It revised the **primary arm** from discharging
  to refusing, on evidence read out of `legacy_c.rs:6483-6486` — every refused
  statement lowers to `LegacyInstruction::Unknown`, which poisons every *pointer
  binding* while leaving `state.resources` alone — and left every other
  prediction unchanged.

**`prediction.md` was wrong about the mechanism and right about everything else;
the addendum was right about everything.** Both are reported here rather than
one of them.

| prediction | source | measured |
|---|---|---|
| declare `flb_input_chunk_destroy_direct_routes`, `nullable` release | `prediction.md` | **landed** |
| decline `flb_input_chunk_destroy_corrupted`, payoff zero | `prediction.md` | **declined**; 0 leaves corpus-wide, cause located at L19823 |
| 13 uncontracted-call leaves removed (12 + 1) | `prediction.md` | **exactly 13**, at the exact thirteen lines named |
| `leak` 272 -> **266** (−6), at 19384/19392/**19399**/19434/19489/19505 | `prediction.md` | **exactly those six** |
| L19544 leak **stays**; `flb_input_chunk_get_direct_routes` stays `Unsafe` | `prediction.md` | **stays**, `Unsafe` 98 -> 92 diagnostics, leak 7 -> 1 |
| the arm at the twelve sites | **`prediction.md`: discharging (A)** | **refusing (B)** — `prediction.md` was **wrong** here |
| the arm at the twelve sites | **addendum: refusing (B)** | **refusing (B)** |
| `unknown_alias` | `prediction.md`: 4,409 (+1) — **wrong** | — |
| `unknown_alias` | addendum: **4,421** (+13) | **4,421** |
| `unsupported` 49,794 -> **49,781** (−13) | both | **49,781** |
| leaves added / removed | addendum: **13 / 19** | **13 / 19** |
| `flb_input_chunk_map` 99 -> 99 (−1 `unsupported`, +1 `unknown_alias`) | addendum | **exactly that**, stays `Unknown` |
| RIPPLE-AGGREGATE-OWNER adds nothing | addendum | **0 added leaves** from that effect |
| no unit and no function status move; no newly surfaced `Unsafe` unit | both | **0 / 0 / 0** |
| `summary_conflict` 0 in all 136 reporting units | both | **0**, BEFORE and AFTER |
| LOADER-ABI type withdrawals: zero of any class | both | **zero**, in both 185-unit runs |
| contract corpus 72/17/38 -> **73/18/39**, rejected 145 | both | **exactly that** |
| no Lean module needs adding or amending | both | none; **95 audits**, unchanged |
| **recorded as NOT expected:** all seven leaves retiring, `Unsafe -> Unknown`, pin 265 | both | **did not happen** |

The one place the prediction was wrong is worth stating plainly rather than
glossing. `prediction.md` reasoned that `result` is bound directly from
`flb_calloc` with no joined provenance — true — and inferred that the release
would therefore **discharge**. It does not, because between the acquisition at
L19357 and the first call site at L19383 the identifier loop's refused statements
(L19365, L19371, L19379) poison every pointer binding in the state. The *site*
survives, which is why the leaves could still name `preprocessed line 19357`; the
*name* does not. The BEFORE artifact contained the evidence for this all along —
`flb_free(label_lengths)` at L19404 is a declared nullable release and refuses
for exactly this reason, while the same call at L19397 reports nothing because
`label_lengths` was rebound at L19388 with nothing refused in between — and the
addendum names that evidence.

---

## 3. The route, and the BEFORE run's oracle

```sh
git submodule update --init --recursive          # FIRST: the libevent pin gate fires at the END of a run
sh measure/prepare-pin.sh                        # archived as prepare-pin.sh
python3 providers/package.py build --output build/cycle19/provider-packages-siblings
sh measure/run-audit.sh <before|after> measure/bin/<before|after>
```

A **fresh** Clang pin, per the wave-20 corpus record's discarded-first-attempt
lesson: the long-lived developer database is not reused, and neither is wave 20's
`build/cycle19/wave20-pin` that the preceding card read. The pin's own production
database is copied into `<source>/build/compile_commands.json`, which the
preparation script does not create, and **both runs read the same bytes**
(SHA-256 `050439951c65867f2bd597f17e2c49518086f662e887de99427634101a0b1a8c`,
**1,415 entries**, `/usr/bin/clang` spelled — not `cc`). The analysed source is
byte-identical to the preceding card's: `src/flb_input_chunk.c` SHA-256
`a537fe347ec1be6c2af7971b55579284e0402f351334f3cfcf0c2c800fe0977e`, the digest
wave 23 recorded. Both runs: **exit 0, zero gateway failures, all six scopes
summing to 185 units.**

**This card changes no Rust, so the BEFORE and AFTER binaries are the same file**
— SHA-256 `746ab7603d22775c2163a35248ff515e35e584d633b1bc3ec17228ea8d0aae85`,
built once and executed from a worktree-private `measure/bin/`, never from
`target/`. The only input difference between the two runs is the contract
manifest and the module it selects. Memory was checked before each run (17–21 GB
available throughout, the audit's own RSS under 100 MB) and no run was discarded
or killed.

**The BEFORE run reproduces the frozen wave-23 baseline byte for byte**
([`byte_identity.py`](byte_identity.py),
[`logs/diff-summary-before.txt`](logs/diff-summary-before.txt)):

| artifact | frozen wave-23 archive | BEFORE run |
|---|---|---|
| `diagnostics.json` | `8dc339e4a8d9aeec…` | **identical** |
| `chunk-contracts.json` | `c4f438e4ed3dab3c…` | **identical** |

and `diff_artifacts.py --archived docs/reports/wave23-c-operations/artifacts`
itemises **zero** differences in the two sections that carry verdicts:

```
--- chunk-contracts: results 96/96 +0 -0 ~0;  backend_teardown 21/21 +0 -0 ~0
--- contracts:       sources 17/17, manifest 17/17, audit_selection 5/5;  keys {}
--- diagnostics:     units 185/185, +0 -0 ~0
--- source:          envelope key `compile_entry`; 4 wrapper rows changed; callbacks 3/3 +0 -0 ~0
--- source-effects:  3 result rows changed
```

with the histograms agreeing to the last number ([`headline.py`](headline.py)):
185 units · 2/42/137/4; 4,834 projected functions · 150/84/4,600; `leak` 272,
`unknown_alias` 4,408, `unsupported` 49,794; and six-for-six on the per-library
split.

The residues — the `source` envelope's `compile_entry` path literal, 4 wrapper
rows and 3 `source-effects` result rows (`cio_chunk_is_locked`,
`cio_chunk_unlock`, `flb_input_chunk_release_lock`) — are the checkout-root and
identity-digest classes every prior re-pin has recorded. They belong to the route
and not to this card, and this card can say so exactly: `source.json` and
`source-effects.json` are **byte-identical between the BEFORE and the AFTER run**
(`20f73714f1541867…` and `a626488f4bc5a896…` on both sides,
[`artifact-manifest.json`](artifact-manifest.json)).

One honest note on hygiene, in the same place wave 22 put its own. This card's
BEFORE run was taken on the working tree. Three new report scripts and the new,
**unselected** `.qcore` module were written to disk while the process ran; none
is an audit input, and the produced `resource-diagnostics.contracts.json` is
written at process start (timestamp 20:03:45) and records the 17-module manifest.
One genuine audit input, the *comment block* of the manifest-selected
`fluent_bit/input_chunk_destroy.qcore`, was edited at ~20:12 while the run was
still going; it was **reverted immediately** and re-applied only after the run
finished, so the BEFORE tree was pristine for every input the process could
re-read. The byte-level oracle above is the actual guarantee.

---

## 4. The candidate set, bounded before the AFTER run

`flb_input_chunk_destroy_direct_routes` is called from **exactly thirteen** sites
inside the one corpus unit that defines it, and from nowhere else in the 185
units. Upstream's other callers (`src/flb_task.c:735,767,779` and
`tests/internal/input_chunk_routes.c:575`) are outside the audited scope — the
`fluent-bit-chunk` scope holds exactly one unit. Read out of the BEFORE artifact
before the AFTER run:

| preproc lines | enclosing function | its status / diagnostics | its leaks |
|---|---|---|---|
| `L18929` | `flb_input_chunk_map` | `Unknown` / 99 | **none** |
| `L19383`, `L19391`, `L19398`, `L19405`, `L19433`, `L19452`, `L19460`, `L19488`, `L19494`, `L19504`, `L19516`, `L19523` | `flb_input_chunk_get_direct_routes` | `Unsafe` / 98 | 7 (six at post-call exits, one at the success exit) |

`flb_input_chunk_map` carries no leak at all, which is why
RIPPLE-UNKNOWN-CASCADE — `mark_unresolved_effect`'s `pointer.unknown` branch
marking *every* site in the state — had nothing to retire there, exactly as in
wave 22. The unit's other leak-carrying functions — `append_to_ring_buffer`
(3 leaves, `Unsafe`) and `input_chunk_collect_output_references` (1, `Unsafe`) —
call this function nowhere and are untouched, measured: both keep their exact
status, leaf count and diagnostic count. The unit's leaks go 11 -> 5, which is
exactly this function's 7 -> 1 and nothing else.

---

## 5. Corpus — every leaf attributed

[`census.py`](census.py), transcript
[`logs/census-and-diff-after.txt`](logs/census-and-diff-after.txt):

```
units: 185 -> 185;  added: [];  removed: []
unit status before/after: {'Safe': 2, 'Unknown': 137, 'Unsafe': 42, 'data_only': 4}
unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834;  function status moves: 0
leak: 272 -> 266 (-6);  unknown_alias: 4408 -> 4421 (+13);  unsupported: 49794 -> 49781 (-13)
units with diagnostic-count changes: 1
  fluent-bit-chunk/flb_input_chunk.c: {'leak': -6, 'unknown_alias': +13, 'unsupported': -13}
leaves added: 13; leaves removed: 19
```

Thirty-two leaves, in **one** unit and **two** functions. The description
inventory moves by exactly four rows, and they account for all thirty-two:

```
leak          / 'live obligation acquired at preprocessed line 19357'          : 7 -> 1   (-6)
unknown_alias / 'unresolved resource identity `result`'                        : 183 -> 195 (+12)
unknown_alias / 'unresolved resource identity `direct_routes`'                 : 1 -> 2   (+1)
unsupported   / 'call to uncontracted function `flb_input_chunk_destroy_direct_routes`' : 13 -> 0 (-13)
```

`call to uncontracted function \`flb_input_chunk_destroy_direct_routes\`` goes to
**zero**, which is the whole measured payoff of the declaration and is what
PROD-CONTRACTS' rule requires before a premise is landed. It is also exactly the
payoff `flb_input_chunk_destroy_corrupted` does **not** have.

**`flb_input_chunk_get_direct_routes` (`Unsafe` -> `Unsafe`, 98 -> 92; leak 7 -> 1)**

| leaf | attribution |
|---|---|
| twelve `unsupported: call to uncontracted function …` — **removed** | the declaration, working as intended: each statement is now a modelled release |
| twelve `unknown_alias: unresolved resource identity \`result\`` — **added** | `apply_resource_action`'s refusal. `result`'s *binding* is `unknown` at every one of the twelve sites, because each is downstream of refused statements (the identifier loop at L19365/L19371/L19379 for the first, the label and plugin loops for the rest) and `LegacyInstruction::Unknown` sets `pointer.unknown` on every name in the state. `nullable` removes the null conjunct but not this one |
| six `leak: live obligation acquired at preprocessed line 19357` at L19384, L19392, **L19399**, L19434, L19489, L19505 — **removed** | `mark_unresolved_effect` on the refused actual, and `report_leaks` skips any site in `unresolved_effect_sites` — §6 |
| L19544 `leak: … 19357` — **kept** | the success exit. `*routes = result` is refused (`store through \`*routes\` without an out-parameter contract`), no destroy call runs on that path, and the residual stands. **This is why the function keeps its provisional `Unsafe`** |

**`flb_input_chunk_map` (`Unknown` -> `Unknown`, 99 -> 99)**

| leaf | attribution |
|---|---|
| `L18929` uncontracted leaf — **removed**; `L18929` `unknown_alias: unresolved resource identity \`direct_routes\`` — **added** | `flb_input_chunk_destroy_direct_routes(direct_routes, direct_count);`. `direct_routes` is filled through `&direct_routes` by an uncontracted out-parameter and re-poisoned by the refusals at L18922/L18926, so the projection has no identity for it. **One `Unknown`-class refusal replaced by another at the same statement, reporting the reaching state instead of the syntactic shape** — the pattern card CONSTRUCTOR-RESIDUALS recorded for `mk_list_add` and wave 22 recorded at its own four sites. Net zero leaves, no verdict move |

**No other unit, no other function and no other leaf moved anywhere in the
185-unit corpus.** In particular `input_chunk_get` is **byte-identical**,
`Unknown` / 23 diagnostics, both `L19847` leaves intact: this card does not touch
`AOM-DESTROY-JOINED-PROVENANCE` (§9).

---

## 6. The leak pin moves 272 -> **266**, deliberately, by refusal

`fluent-bit-chunk/flb_input_chunk.c`, `flb_input_chunk_get_direct_routes`,
preprocessed lines **19384, 19392, 19399, 19434, 19489, 19505**. **This is a pin
movement and it is deliberate** (constraint 7). Both prediction documents named
272 -> 266 in advance, named which six leaves, named which seventh would survive,
and recorded the outcome they expected *not* to see.

The shape at every one of the six, in the analyzer's own view:

```
BEFORE                                                AFTER
L19383  Unknown: call to uncontracted function        L19383  Call flb_input_chunk_destroy_direct_routes(result, _)
        !! unsupported: call to uncontracted …                !! unknown_alias: unresolved resource identity `result`
L19384  return -2;                                    L19384  return -2;
        !! leak from L19357                                   (no leaf)
```

**What this does and does not say.** The leak was never an upstream defect claim
— invariant 2 governed it as it governs the 266 that stand — and it is not
withdrawn because the release was proved. It is withdrawn because, with the
callee declared, the projection can finally *say something* about the statement,
and what it says is `Unknown`: it cannot tell which identity the name `result`
denotes at that point, because the statements between the acquisition and the
release are outside the projection. That is a strictly more informative refusal
than "there is a callee here I cannot see", and retiring a residual claim is a
**withdrawal of a provisional claim**, never a promotion. Under invariant 1
nothing flipped toward `Safe`: no unit and no function changed status, the unit
stays provisional `Unsafe`, no raw unit is promoted, the Cycle 19 gates are
untouched.

**The discharging arm is what a faithful release contract exists for, and it did
not fire here either.** It would require `result`'s binding to survive from
L19357 to the call site, and the body's own refused statements prevent that. The
residual is therefore **not** about the declaration and **not** about
nullability: it is the pointer-binding poisoning that any refused statement
performs. Named `AOM-SIBLINGS-BINDING-POISON` (§9).

### 6.1 `CONSTRUCTOR-RESIDUALS-LEAK-271` is decided, and the withdrawal is itemised

Card CONSTRUCTOR-RESIDUALS §6.4 moved the pin 271 -> 272 with
`flb_input_chunk_get_direct_routes` L19399, and recorded the cost of deciding it:
*"Deciding it needs either a contract for `flb_input_chunk_destroy_direct_routes`
or an unresolved-effect rule for uncontracted callees that would withdraw the
six, which is a downgrade this card is not scoped to make."*

This card took the first route, and the cost is measured rather than assumed.
**Claim by claim**, the seven leaves of that family:

| leaf | wave-17 status | this card |
|---|---|---|
| L19384 | standing claim | **withdrawn** |
| L19392 | standing claim | **withdrawn** |
| **L19399** | the leaf wave 17 added — `CONSTRUCTOR-RESIDUALS-LEAK-271` | **withdrawn — the finding is DECIDED** |
| L19434 | standing claim | **withdrawn** |
| L19489 | standing claim | **withdrawn** |
| L19505 | standing claim | **withdrawn** |
| L19544 | standing claim | **stands** |

So the recorded cost was **one claim too pessimistic**: five of the six standing
claims are withdrawn, not six, because the sixth (L19544) is on the success path
where no destroy call runs and no unresolved effect is marked. Six leaves retire
in total — five old plus the wave-17 one. `flb_input_chunk_get_direct_routes`
keeps its provisional `Unsafe` on the surviving leaf, which is why this
withdrawal costs **no** function status move; had all seven gone, the function
would have moved `Unsafe -> Unknown` and the pin would read 265, and both
prediction documents recorded in advance that this was *not* expected.

The leak-pin history is now four deliberate moves: 271 -> 272 (wave 17,
accusing), 272 -> 273 (wave 20, accusing), 273 -> 272 (wave 22, withdrawing),
272 -> **266** (this card, withdrawing — the first move of more than one).

---

## 7. Against the frozen baseline, and constraint 22

`diff_artifacts.py --archived docs/reports/wave23-c-operations/artifacts`
([`summarise_diff.py`](summarise_diff.py),
[`artifacts/diff-wave23-to-after.json.gz`](artifacts/diff-wave23-to-after.json.gz)):

```
--- chunk-contracts: NO DIFFERENCES (results 96/96, backend_teardown 21/21, +0 -0 ~0)
--- contracts:       sources 17 -> 18 (+1, 1 changed, 0 removed); manifest modules 17 -> 18 (+1)
                     added:   qcore/fluent_bit/input_chunk_destroy_direct_routes.qcore
                     changed: qcore/fluent_bit/input_chunk_destroy.qcore  (comment block only)
                     audit_selection 5 -> 5
--- diagnostics:     {'leak': 272, 'unknown_alias': 4408, 'unsupported': 49794}
                  -> {'leak': 266, 'unknown_alias': 4421, 'unsupported': 49781};  units +0 -0 ~1
--- source:          envelope key `compile_entry`; 4 wrapper rows changed; callbacks 3/3 +0 -0 ~0
--- source-effects:  3 result rows changed
```

`chunk-contracts` is **byte-identical** to the frozen baseline's — same
uncompressed SHA-256 `c4f438e4ed3dab3c…` — and `source` and `source-effects`
carry only the classes §3 identified, byte-identical between this card's own two
runs. Exactly **one** unit's diagnostics changed.

The one `contracts` content change is the `input_chunk_destroy.qcore` **comment
block**: its `AOM-DESTROY-SIBLINGS-UNDECLARED` bullet said both siblings were
deliberately left undeclared, which this card makes false for one of them, so the
bullet now records how each was decided. The declaration text in that module is
byte-unchanged, its clause count is unchanged, and this is the same retirement
card CONSTRUCTOR-RESIDUALS performed on `logging.qcore`'s own "left undeclared"
entry for `flb_errno_print`.

Constraint 22, reading the **normalised and raw** module keys together
([`raw_contract_keys.py`](raw_contract_keys.py)):

```
archived raw keys: 17      measured raw keys: 18
normalised added   : ['…/qcore/fluent_bit/input_chunk_destroy_direct_routes.qcore']
normalised removed : []
normalised collision (before): False      (after): False
archived checkout roots: ['/home/cosmo/Documents/GitHub/qcore-c/compiler/../']
measured checkout roots: ['…/worktrees/agent-af4bc3dafe16187d1/compiler/../']
```

**No normalisation collision on either side**, so no two modules were merged into
one row and the section was not demoted to raw keys; under normalisation
`contracts.json` gains exactly one module, removes none and changes one; and the
raw keys differ additionally only by the checkout root.

**The baseline is not re-pinned here.** `docs/reports/wave23-c-operations/`
remains the corpus baseline exactly as it stands, and every prior archive beneath
it (constraint 21 — published beside, never edited). The leak pin and the two
`unknown_alias`/`unsupported` totals are the integrator's to consolidate (§10).

The `reason_code` sidecar tripwire (constraint 24) is unaffected: `unit
reason_code moves: 0`.

### 7.1 Promotions, reported separately

The wave-23 follow-on table requires unit promotions, function promotions and
withdrawn violation claims to be reported separately. They are:

* **Unit promotions: none.** 2 `Safe` / 42 provisional `Unsafe` / 137 `Unknown`
  / 4 data-only, before and after; zero unit status moves in either direction;
  zero newly surfaced provisional `Unsafe` units.
* **Function promotions: none.** 150 `Safe` / 84 provisional `Unsafe` / 4,600
  `Unknown`, before and after; **zero** function status moves of any kind across
  all 4,834 projected functions. Nothing became `Safe` anywhere.
* **Withdrawn violation claims: six**, all leak leaves in
  `flb_input_chunk_get_direct_routes` (L19384, L19392, L19399, L19434, L19489,
  L19505), each withdrawn by refusal and itemised in §5 and §6.1. No violation
  claim was added anywhere.

---

## 8. What was touched, and the pins moved deliberately

| file | what |
|---|---|
| `contract/qcore/fluent_bit/input_chunk_destroy_direct_routes.qcore` | **new.** One declaration, read against the pinned body, with its faithfulness argument, its divergence from its neighbour, and its non-claims |
| `contract/qcore/fluent_bit/input_chunk_destroy.qcore` | comment only: its `AOM-DESTROY-SIBLINGS-UNDECLARED` bullet now records how each sibling was decided |
| `contract/cycle19-resource-contracts.json` | manifest-selects it; 17 -> 18 modules |
| `compiler/tests/qcore_contract_corpus.rs` | corpus size 72 -> 73; one `EXPECTED_CONTRACTS` row `("fluent_bit/input_chunk_destroy_direct_routes.qcore", "flb_input_chunk_destroy_direct_routes", 4)` |
| `compiler/tests/contract_loader.rs` | three counts (`source_assumptions` 17 -> 18 twice, `modules.len()` 72 -> 73, `(selected, v1, rejected)` `(17,38,145)` -> `(18,39,145)`), and one new explicit row in the previous-modes comparison |
| `crates/qcore-c-gateway/tests/prod_contracts.rs`, `.../neighbours.rs` | `source_assumptions` 17 -> 18 each |
| `docs/reports/aom-destroy-siblings/` | this report, the two pre-registered prediction documents, the tooling and the evidence |

**No Rust under `crates/*/src/` or `compiler/src/` is touched.** The admission
relation, the checker, the loader, the importer and the W4 machinery are all
byte-unchanged — the BEFORE and AFTER audits ran the *same executable file* — so
constraints 4, 5, 5a, 6, 10, 11 and 12 are undisturbed by construction as well as
by measurement, and no Lean module needed adding or amending.

Invariant 5, said plainly: five pinned expectations moved, each deliberately and
each named in the commit message. The one that deserves its own sentence is
`contract_loader.rs`'s previous-modes comparison, which rebuilds the pre-existing
mode table and asserts every manifest declaration matches it.
`flb_input_chunk_destroy_direct_routes` is listed **explicitly beside
`flb_input_chunk_destroy`**, with its own mode vector
`[Released { nullable: true }, Borrowed] -> None` and a comment saying why
`nullable` and not the `nonnull` its neighbour carries. Putting the two rows
side by side is the point: the test now states, in the place a future card will
read it, that these two destructors are not interchangeable.

---

## 9. Lean

Nothing to add or amend, and this is a claim rather than an omission. This is a
**contract card**: the only executable artifacts it changes are one `.qcore`
module, one module's comments, a manifest entry and five test pins. The admission
relation, the refusal predicates, the leak-reporting rule and the
unresolved-effect mechanism are byte-unchanged Rust — provably so, since both
audits ran the same binary — so there is no semantic change for a Lean definition
to track (invariant 6). `scripts/check_lean_audits.py` passes **95 audits**, the
same count and the same set as the base tree
([`logs/lean-audits-summary.txt`](logs/lean-audits-summary.txt), full transcript [`logs/lean-audits.txt.gz`](logs/lean-audits.txt.gz)), placeholder-free with no
project axioms.

---

## 10. Gate

The full standing gate from `AGENTS.md`, run on the card's tip:

| step | result |
|---|---|
| `git submodule update --init --recursive` | run **first**, before either audit — the `libevent` pin gate fires at the *end* of a run |
| `cargo test --workspace` | **3,580 passed, 0 failed, 8 ignored**, one run, no partition and no retry ([`logs/workspace-tests.txt.gz`](logs/workspace-tests.txt.gz)) |
| `cargo fmt --all -- --check` | **clean** |
| `cargo clippy --all-targets -- -D warnings` | **clean**, zero warnings ([`logs/clippy.txt`](logs/clippy.txt)) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **95 Lean audits**, all passed ([`logs/lean-audits-summary.txt`](logs/lean-audits-summary.txt), full transcript [`logs/lean-audits.txt.gz`](logs/lean-audits.txt.gz)) |
| `python3 -m unittest discover -s scripts/tests` | **193 passed** ([`logs/python-tests.txt`](logs/python-tests.txt)) |
| corpus audits | **both exit 0**, zero gateway failures in all six scopes, 185 units, 136 reporting units with **0** custody summary conflicts |

Provider packages were **built** for the audit route (`providers/package.py build`,
`qcore-schemas` 14.0.0, exit 0) and are otherwise untouched: constraint 18's
rebuild-and-verify obligation is triggered by a manifest-template change or a
`qcore-schemas` major bump, and this card makes neither. No provider source,
manifest template or schema version is edited; the ledger's 122 provider tests
are carried from the preceding checkpoint, not rerun for this card.

**No constraint-16 row tripped**, in the workspace run or in either probe. Both
probes exited 0 on their first invocation, run after the AFTER audit and the
workspace tests had finished rather than concurrently with them — which is the
deliberate difference from wave 22, whose `reevaluation_followup` failed once on
the named `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` wall-clock row
while its audit was still running. Nothing about those rows is weakened or
changed; they simply were not put under load. The workspace run completed in one
pass with no partition, no rebuild and no repeated crate, so no deduplication
arithmetic is needed to state its number.

---

## 11. Named follow-ons

* **`AOM-DESTROY-CORRUPTED-UNPAID-PREMISE`** — `flb_input_chunk_destroy_corrupted`
  is read (§1.2), its faithful contract is written out, and it is deliberately
  **not** landed: measured payoff zero, because its one in-corpus call site
  (`input_chunk_get`, preprocessed L19823) is refused as an indirect memory
  access before the call rule is reached. The blocker is the fourth actual,
  `in->config->storage_del_bad_chunks`. When a card repairs indirect actuals in
  call arguments — the wave-23 table's "Typed scalar expression positions" and
  "Aggregate access" slices — this declaration becomes landable and its payoff
  must be **re-measured**, not assumed, exactly as `msgpack/packer.qcore` was.
* **`AOM-SIBLINGS-BINDING-POISON`** — the residual this card measured rather than
  inferred, and the reason its leak movement is a withdrawal rather than a
  discharge. Every refused statement sets `pointer.unknown` on **every** name in
  the state (`legacy_c.rs:6483-6486`), so an acquisition's binding does not
  survive any refused statement between it and its release, even when the *site*
  does and even when the name is never reassigned. In
  `flb_input_chunk_get_direct_routes` this is what refuses all twelve declared
  releases of `result`. Repairing it — restricting the poisoning to the names a
  refused statement could actually write — would move those twelve sites toward
  the discharging arm and is therefore a direction invariant 1 constrains: it
  must be argued, measured and published, never absorbed. Note that it also
  interacts with constraint 6's `C02-P-ALIAS-LEAK-ORDER`, since L19544's leaf
  survives on an intermediate iterate.
* **`AOM-DESTROY-JOINED-PROVENANCE`** carried forward, **untouched**. This card
  installs no model for the lookup/acquisition join, manufactures no entry
  custody, and does not revisit `input_chunk_get`; that function's row is
  byte-identical before and after, `Unknown` with 23 diagnostics and both
  `L19847` leaves intact. The two residuals are distinct: that one is about
  narrowing a *join* of two provenances; this card's is about a binding lost to
  an unrelated refused statement on a straight-line path.
* **`AOM-DESTROY-DIRECT-ROUTES-OUT-OF-SCOPE-CALLERS`** — the declaration is in
  force for the whole corpus, but `flb_input_chunk_destroy_direct_routes`'s other
  upstream callers (`src/flb_task.c:735`, `:767`, `:779`) are **outside** the
  185-unit scope, so this card measured no payoff there and claims none. If the
  audited scope grows to `flb_task.c`, the declaration applies unchanged and its
  effect there is unmeasured.
* **`AOM-DESTROY-DIRECT-ROUTES-MEMBERS-UNMODELED`** — the module claims the
  release of `routes` **happens**; it claims nothing about the member releases
  inside the loop, and in particular nothing about the asymmetric guard
  (`label` is freed only when non-null **and** `label_length > 0`, while
  `plugin_name` is freed on non-null alone). In the corpus's own producer the
  two fields are written together, but that is a property of the producer. Nor
  does it claim `route_count` matches the allocated extent.

---

## 12. For the integrator

Nothing in `AGENTS.md`, `DEVELOPMENT.md`, `README.md` or
`docs/reviews/r0_3/acceptance_matrix.json` is edited by this card, and no frozen
archive is touched. The facts to reconcile, all measured and all in this report:

1. **Leak pin 272 -> 266** (constraint 7). Attribution: six leaves in
   `flb_input_chunk_get_direct_routes` (L19384, L19392, L19399, L19434, L19489,
   L19505), each withdrawn by refusal at the preceding call site, §5 and §6.
   Constraint 7's history entry grows to **four** deliberate moves, and this is
   the first that moves by more than one.
2. **`CONSTRUCTOR-RESIDUALS-LEAK-271` closes — decided, withdrawn.** Its
   recorded cost ("would withdraw six standing claims") is measured at **five**:
   the sixth, L19544, stands. Constraint 7's text for the 271 -> 272 move should
   record the decision and the corrected cost.
3. **`AOM-DESTROY-SIBLINGS-UNDECLARED` closes**, decided in both halves: one
   declared, one declined at measured payoff zero.
4. **Function histogram unchanged at 150 / 84 / 4,600**; **zero** function status
   moves; **zero** unit status moves; no promotion of any kind.
5. **`unknown_alias` 4,408 -> 4,421; `unsupported` 49,794 -> 49,781.** Unit
   verdicts, the 185-unit denominator and the contract-clause pin are unchanged.
6. **Contract corpus 72 -> 73 modules**, manifest-selected 17 -> 18, v1
   declarations 38 -> 39, rejected unchanged at 145.
7. Baseline **not** re-pinned; `docs/reports/wave23-c-operations/` stands.
8. Five follow-ons open (§11); `AOM-DESTROY-JOINED-PROVENANCE` carries forward
   untouched.
