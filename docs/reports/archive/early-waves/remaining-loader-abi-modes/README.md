# LOADER-ABI-MODES — the declared custody *mode*, compared against the body — 2026-09-10

The named continuation of card **LOADER-ABI**
([`docs/reports/remaining-loader-abi/README.md`](../remaining-loader-abi/README.md)
§8), which closed the *type* half of the declared/source comparison and recorded
the rest as its own follow-on:

> this card compares **types**. It does not compare a declared *mode* against
> anything the source says — nothing does, and nothing can without body-level
> evidence. A faithful signature with an unfaithful mode is exactly as trusted
> as before.

This card is that gate, at the one place body-level evidence already exists: a
contracted function **defined in the analysed unit**, whose validated W4 summary
the pass already derives. A declared custody duty that summary **contradicts**
is withdrawn for that unit, at every call site, with a named keyed refusal.

**Base tree:** main `8ec8402` ("Consolidate wave 16: corpus re-pin, follow-ons,
ledger"), submodules initialised, `third_party/libevent` at `d82464a2`.

---

## Headline

**The fail-open is real, it is reproduced three ways, and it is closed.** The
evidence was already being computed and already being reported — W4 derives a
validated summary for every function defined in the unit, and
`derive_custody_summaries` already records when it disagrees with that
function's declaration. What was missing was the *decision*: the declaration
stayed in the environment and went on governing every call site, so the analyzer
reported the definition `Unknown` while certifying a caller checked against the
mode it had just measured the body to contradict.

| | |
|---|---|
| The fail-open | §1 reproduces three `Safe` callers — a double release, a hidden leak and a discarded acquisition — each beside a definition already reported `Unknown` with a `SummaryConflict`. |
| What the gate decides | a **duty** inequality, and nothing else (§3.1): `Retained` / `Transferred` / `Delivered` / `Restored` at a parameter, `Absent` / `Produced` at the result. Two general theorems prove the relation *is* the duty inequality; the whole 64+16 domain is decided on both sides and tied by one checksum. |
| What a contradiction does | withdraws the declaration's application for that unit — every call site refuses with `mode-parameter-contradiction` or `mode-result-contradiction`, monotone in the refusing direction, and the derived summary is **never** installed in its place (§3.3). |
| What it deliberately does not do | require validation before applying a contract (§3.2), or withdraw on a refinement *inside* one duty (§3.6). Both are proved, not just intended. |
| Census | §2 — the family's reach is a property of where the bodies live. Of the manifest's 31 declared names, 21 are comparable in some unit, and just **8 (unit, name) rows** are inside the audit's analysed function set — every one of them `no-summary`. |
| Sweep | **zero findings** (§4) — and §4.1 shows why that is structural rather than lucky: every derivable comparison in the corpus is a *nullability* refinement, in three independent production families (`rd_free`, `mk_mem_free`, `flb_free`; 113 rows). Withdrawing on inequality, which is what today's `SummaryConflict` tests, would have withdrawn a release contract in every unit including those headers. |
| Corpus | measured in **both** directions. All five artifacts are **byte-identical between BEFORE and AFTER**, so this card's contribution is measured at zero, and BEFORE reproduces the wave-16 baseline's `diagnostics.json` byte for byte. §6. |
| A fail-closed pin caught the implementation | card C03-NFR's, and it was repaired at the implementation rather than weakened. §3.5. |
| Evidence | 20 new tests in a held-out `m_*` family, 36 audited Lean theorems in a new `QCore.DeclaredModeAgreement` namespace (73 audits, no new axiom), and a normative sentence in contract spec §C04.1. |

**Nothing is promoted.** Every `Safe` in this card's own family is `Safe` for a
synthetic declared custody projection under premises the fixture names; the one
provisional `Unsafe` it produces is a projection allegation about a synthetic
fixture, never an upstream defect claim; no raw Cycle 19 unit moves.

---

## 1. The fail-open, reproduced three ways

The evidence W4 needs already existed, and the analyzer already knew about the
disagreement. `derive_custody_summaries` derives a *validated* summary for every
function defined in the unit, notices when it differs from that function's
declared contract, and `analyze_defined_function` reports a `SummaryConflict`,
makes the **defining** function `Unknown` and wipes its per-exit claims.

That is a report about the definition. It was never a decision about the
*interface*: the declared contract stayed in the environment and went on
governing every call site. So a caller in the same translation unit was checked
against a mode this very pass had already measured the body to contradict.

Three fixtures reproduce it, all measured on the base tree
(`crates/qcore-c-gateway/tests/loader_abi_modes.rs`, the held-out `m_*` family).
In every one the **caller is `Safe` with no diagnostic at all**, beside a
definition already reported `Unknown` with the conflict:

| | declared | body | what the caller does | measured before |
|---|---|---|---|---|
| **class 1** | `resource_borrowed(p)` | frees `p` | `m_sink(h); m_free(h);` — a **double release** | `caller` **`Safe`**, `[]` |
| **class 2** | `resource_released(p, nullable)` | never releases | `m_retire(h);` and stops — a **leak** | `caller` **`Safe`**, `[]` |
| **class 3** | `resource_return_none()` | returns a fresh acquisition | `m_make();` — a **discarded acquisition** | `caller` **`Safe`**, `[]` |

The recorded inferred/declared pairs are exactly the three shapes: class 1
`[Released{nullable:false}]` against `[Borrowed]`, class 2 `[Borrowed]` against
`[Released{nullable:true}]`, class 3 `result: Owned{nullable:true}` against
`result: None`.

**After this card** each caller refuses with a named key —
`mode-parameter-contradiction` for classes 1 and 2,
`mode-result-contradiction` for class 3:

```
call to `m_sink` whose declared contract was withdrawn against the summary
derived from its body in this translation unit (mode-parameter-contradiction)
```

Class 2's caller becomes a **provisional `Unsafe`**, because the withdrawal
exposes the leak the declared `Released` was accounting for. That is card
LOADER-ABI's recorded `LOADER-ABI-WITHDRAWAL-UNSAFE` direction on this axis; it
is asserted in the test rather than left to surprise a later reader, and — hard
invariant 2 — it is a statement about this synthetic fixture's projection, never
an upstream defect claim.

---

## 2. The census: how much of the declared corpus this family can reach

**Census first**, because it sizes the whole family — and because the answer is
a property of *where the bodies live*, not of whether the declarations are
faithful. A declared type can be compared wherever the unit **declares** the
callee, which is nearly everywhere. A declared mode can be compared only where
the unit **defines** it and W4 summarises the body.

**Method.** The production decision itself
(`cargo run -p qcore-c-gateway --example declared_mode_census`), over the
preprocessed form of **all 185** pinned units, against the production contract
manifest (twelve modules, 31 `resource_contract_v1` declarations). The pinned
corpus is fed by two compile databases and every one of the 185 units is covered
by an entry in one of them (verified, `artifacts/`); one pinned base name is not
unique (`mk_utils.c`, in both `mk_core` and `mk_server`), the same ambiguity card
LOADER-ABI recorded. Rows below are de-duplicated by (unit base name, contracted
name), so a unit compiled into two libraries is counted once.

### 2.1 The whole comparable population

| outcome | rows | what it means |
|---|---|---|
| `undefined` | **598** | the unit does not define the contracted name — nothing to compare, the declaration applies |
| `no-summary` | **484** | it does, and W4 derives no validated summary — nothing to compare, the declaration applies |
| `agreed` | **0** | — |
| `disagreed` | **113** | a summary was derived and differs *without* contradicting a duty (§3.6) |
| `contradicted` | **0** | — |

**Zero contradictions in the pinned corpus**, which is what makes §6's
byte-identical diagnostic surface the expected result rather than a lucky one.

Only **21** of the manifest's **31** declared names are ever comparable in any
pinned unit at all, and every one of
the 113 `disagreed` rows is one of exactly **three** names:

| name | rows | declared | derived |
|---|---|---|---|
| `rd_free` | 83 | `resource_released(ptr, nullable)` | `Released { nullable: false }` |
| `mk_mem_free` | 29 | `resource_released(ptr, nullable)` | `Released { nullable: false }` |
| `flb_free` | 1 | `resource_released(ptr, nullable)` | `Released { nullable: false }` |

All three are one-line `static inline` release wrappers in headers, and all
three are the *same* nullability refinement. §4.1 records why that is structural.

### 2.2 What the pinned audit can actually reach, which is smaller

The census applies **no** `selected_functions` filter; the pinned audit does —
card E01's main-source enumeration, which excludes definitions that arrive
through a header. The census is therefore an **upper bound** on the analyzer's
reach, and the two are intersected directly rather than assumed equal:

> **8 of the 1,195 census rows are inside the pinned audit's analysed function
> set, and every one of them reads `no-summary`.**

| unit | contracted names analysed there |
|---|---|
| `lib/chunkio/src/cio_chunk.c` | `cio_chunk_open`, `cio_chunk_close`, `cio_chunk_down`, `cio_chunk_up_force`, `cio_chunk_is_up` |
| `src/flb_input_chunk.c` | `flb_input_chunk_get_name`, `generate_chunk_name`, `input_chunk_write_header` |

So **not one** of the 113 `disagreed` rows is reachable by the production route:
they are all header inlines the enumeration excludes, and the analyzer never
derives a summary for them at all. That is the direct explanation of §6's
zero delta, and it is measured on both sides — the census from the comparison's
end, and the pinned report's own analysed-function inventory from the
analyzer's.

It is also a recorded consequence rather than a hidden one: **widening the
selection would make those 113 rows comparable**, and because they are
refinements rather than contradictions the gate would still withdraw nothing —
but a future selection change is exactly where this family's behaviour would
next move. Named `LOADER-ABI-MODES-SELECTION` (§8).

Archived: [`artifacts/mode-census-fluentbit.tsv.gz`](artifacts) and
[`artifacts/mode-census-monkey.tsv.gz`](artifacts).

### 2.3 Two method notes, recorded rather than glossed

The census preprocesses each unit with its **own full compile command**, while
the pinned compatibility route projects a historical option subset, so the two
texts are not byte-identical; and the census measures the *comparison*, not the
analysis, so it says nothing about verdicts beyond §6.

---

## 3. What the gate decides, and what it deliberately does not

### 3.1 The relation is the *duty* inequality

The comparison is taken in what a mode promises the **caller**, so that a
difference is a difference the caller can be wrong about — which side holds the
resource after the call.

| duty | parameter modes | what the caller must do |
|---|---|---|
| `Retained` | `Borrowed`, `Owned` | keep the duty across the call |
| `Transferred` | `Released`, `Consumed`, `Stored` | stop accounting for it at the call |
| `Delivered` | `Out` | expect fresh custody in the slot on return |
| `Restored` | `Removed` | expect direct custody back from an aggregate |

and on the result axis, `Absent` (`None`) against `Produced` (`Owned`,
`Reallocated`).

A contradiction is a duty inequality at some position, and nothing else. Two
general theorems say so rather than sampling it
(`a_single_parameter_contradicts_exactly_when_its_duty_differs`,
`a_result_contradicts_exactly_when_its_duty_differs`), and the whole finite
domain — 64 parameter pairs, 16 result pairs — is decided row by row on both
sides and reduced to one positional checksum that Rust and Lean compute
independently.

### 3.2 Only contradictions refuse — absence of evidence is not evidence

A declared mode this pass cannot check keeps the declared contract as the
trusted premise it already was:

* the unit does not define the function — the overwhelming majority (§2);
* it does, but W4 derives no validated summary (the body is outside the
  projection, or no single mode describes every exit);
* the derivation read a different parameter-vector length, which is a statement
  about the *signature* and is card LOADER-ABI's gate.

Requiring validation *before* applying a contract would withdraw the entire
declared-trust architecture — a corpus of library interfaces has its bodies in
other translation units — so this card does not, and
`a_body_that_yields_no_summary_leaves_the_declaration_in_force` and
`a_callee_defined_in_another_unit_is_never_compared` pin both halves.

### 3.3 Refusal only, never repair

A withdrawal rewrites call sites to a marked **uncontracted** callee, card
LOADER-ABI's technique, chosen for its reason: the withdrawal then holds
uniformly at every consumer of the token stream rather than at whichever
contract lookup remembered it.

It never installs the derived summary in the declaration's place, and that is
deliberate rather than incidental. An inferred summary can be *weaker* than the
declaration it convicts — dropping a non-null precondition, say — so swapping
one in would be a route towards `Safe`. Because an uncontracted call is
`Unknown` and poisons its function, the withdrawal is monotone in the refusing
direction; `a_derived_summary_is_never_applied` and `withdrawal_only_removes`
prove it and `a_withdrawal_never_moves_a_verdict_towards_safe` measures it.

### 3.4 The environment is left alone, unlike card LOADER-ABI

This gate does **not** remove the withdrawn name from the contract environment.
It is precisely the environment's declared contract that makes
`derive_custody_summaries` record the conflict, so removing it would *delete*
the `SummaryConflict` diagnostic and the `Unknown` it forces on the defining
function. Keeping it retains that evidence and adds the call-site refusal on
top: strictly more refusing than either half alone
(`the_defining_function_keeps_its_conflict_beside_the_withdrawal`).

### 3.5 A withdrawn function's own recursive call is exempt — measured, not assumed

**A fail-closed pin caught the first implementation and was not weakened.**
Card C03-NFR's `assumed_neutral_refuses_every_body_that_carries_an_effect`
asserts that a self-recursive body which acquires cannot hide behind a neutral
declared candidate, *by naming the `SummaryConflict`*. Rewriting the recursive
call inside the withdrawn function's own body changed the body the derivation
reads, so the very summary that convicted the declaration stopped being
derivable — and the conflict disappeared with it.

The repair exempts a withdrawn function's own body from the rewrite, and it
costs nothing the withdrawal exists to buy: the withdrawal stops *callers*
trusting an interface the body contradicts, and a function's own recursive call
is not a caller trusting it — it is the evidence. The defining function is
already `Unknown` on the conflict, with its per-exit claims wiped, so nothing
about it is trusted either way; every genuine caller, including a mutually
recursive one, is still rewritten.
`a_recursive_self_call_is_exempt_so_the_conflict_survives` asserts both halves
together.

### 3.6 Where the card deliberately stops

A refinement *inside* one duty is not a contradiction, in either direction:
nullability, a `Stored`/`Removed` owner index, a `Reallocated` replaced
parameter. Those change what the caller must additionally prove, never which
side holds the resource after the call. The existing `SummaryConflict` remains
their only report, and the census reads them `disagreed`. Recorded as
`LOADER-ABI-MODES-REFINEMENT`.

### 3.7 The fixed point, and why the set is carried

Withdrawing a callee changes the token stream, which changes the summaries
derived for the bodies that call it. The withdrawn set is therefore grown to a
fixed point: each round re-derives from the original stream with the whole
accumulated set applied, and the set only grows, so the loop settles after at
most one round per contracted name. When nothing is contradicted — the common
case, and the only case in the pinned corpus (§2) — the loop runs the single
derivation the caller needed anyway and nothing is re-derived.

Carrying is load-bearing, and one fixture shows why. `m_inner` is declared
`Borrowed` and frees; `m_outer` is declared `Released` and only calls
`m_inner`. The first round convicts **both** — `m_outer`'s summary reads
`Borrowed` because `m_inner`'s declaration says the call keeps the resource.
The second round convicts neither: `m_inner`'s call inside `m_outer` is now
uncontracted, so `m_outer` yields no summary at all. Recomputing each round
would acquit `m_outer` and oscillate; carrying keeps both withdrawn, which is
the refusing direction (`the_withdrawn_set_is_carried_rather_than_recomputed`,
and `growing_the_carried_set_only_removes` in Lean).

### 3.8 Where the gate runs

At the three entries that own an environment, the same three card LOADER-ABI
gates at and immediately after its type comparison — so a name the *type* gate
already withdrew has left the environment and is never mode-compared:

| entry | note |
|---|---|
| `analyze_legacy_c_translation_unit_selected_partial` | the production and certificate-recheck route; the gate hands back the derivation it settled on, so this route derives exactly once when nothing is contradicted |
| `recursive_component_census` | so card C03-REACH's census reads the components the analysis reads, over the stream the analysis reads |
| `analyze_legacy_c_tokens` | the single-function entry, which does not pass through the translation-unit pass. Measured reach: it has no production caller outside `legacy_c.rs` — it is the tests' entry — but it is gated for uniformity, because a declaration a body in the same stream contradicts would otherwise still govern this function's call sites |

Both sides of the comparison come from what a certificate already holds — the
preprocessed token stream and the environment's own contracts — so W4
derivation, W4 validation and `check_legacy_safety_certificate` reproduce the
decision. `derive_custody_summaries` now has exactly one caller, the gate, so no
route derives without the mode comparison.

### 3.9 The refusal vocabulary

| key | category (§L21.1) | when |
|---|---|---|
| `mode-parameter-contradiction` | `stale-evidence` | a declared parameter's custody duty is contradicted by the validated summary derived from the body here |
| `mode-result-contradiction` | `stale-evidence` | likewise for the result |

`StaleEvidence` because that is what it is: a declaration bound to a different
body than the one under check. Neither is an environment statement, so neither
can be read as saying the analysis could not run, and both keys are asserted
distinct from card LOADER-ABI's eight so a report can tell a signature
disagreement from a custody-duty contradiction
(`both_withdrawals_compose_and_stay_distinguishable`).

---

## 4. The sweep

**Zero findings, and the census in §2 is why — not luck.** Card LOADER-ABI's
sweep found eight unfaithful declarations because a *type* can be compared
wherever the unit declares the callee, which is nearly everywhere. A *mode* can
be compared only where the unit **defines** the callee and W4 summarises the
body, and §2 measures how rarely that is true.

### 4.1 The production corpus

Every declaration the pinned manifest loads — twelve modules, all 31
`resource_contract_v1` declarations, the same population card LOADER-ABI swept —
was put through the production decision itself over all 185 pinned units (§2).
**No declaration is contradicted anywhere**, so no contract module changed and
the corpus is byte-identical (§6).

What the sweep *did* surface is the boundary of §3.6, on real production source
rather than a fixture: every derivable comparison in the corpus lands on a
**nullability refinement**, in three independent families — `librdkafka`'s
`rd_free` (83 rows), Monkey's `mk_mem_free` (29) and Fluent Bit's `flb_free`
(1) — each declared `resource_released(ptr, nullable)` against a one-line inline
body that frees its argument.

That is structural rather than accidental, and the code says so:
`legacy_summary::parameter_mode` proposes `ResourceParameterMode::Released {
nullable: false }` for *every* released formal, so a faithful `nullable` release
declaration necessarily disagrees with its own derived summary. **Had this card
withdrawn on inequality — which is exactly what today's `SummaryConflict` tests
— it would have withdrawn a release contract in every unit that includes those
headers.** The duty abstraction of §3.1 is what makes the gate say nothing
there, and the byte-identical corpus of §6 is the measurement of that.

### 4.2 The test fixtures

By **execution**, which is card LOADER-ABI §4.2's method for the same reason:
most gateway fixtures declare their C inline in the test rather than in a file,
so the sweep is the workspace suite itself, and a contradicted fixture
declaration would move a pinned result. **2,766 workspace tests pass with zero
failures**, so no fixture declaration is mode-contradicted anywhere the suite
exercises it.

The three standalone fixture C sources were also censused directly, one contract
module at a time — several fixture modules declare the same names for different
cards, so loading them together is a duplicate-contract rejection rather than a
convenience problem. All **27** (module, source, name) rows read `undefined`:
none of those sources defines a contracted callee
([`artifacts/mode-census-fixtures.tsv`](artifacts)).

### 4.3 One fail-closed pin caught the implementation

Recorded in full at §3.5: card C03-NFR's
`assumed_neutral_refuses_every_body_that_carries_an_effect` failed against the
first implementation, because rewriting a withdrawn function's *own* recursive
call destroyed the derivation that convicted it and took the `SummaryConflict`
with it. The pin was repaired at the implementation, not weakened.

---

## 5. LOADER-ABI-UNDECLARED, sized

Card LOADER-ABI's fourth recorded boundary: a contracted name its reader finds
**no C declaration** for in a unit is not compared, and its declaration still
applies. That card measured it with a *mentions* filter over 183 units and found
12 rows, all artefacts of the filter. This card re-measures it over the whole
pinned corpus with a sharper filter and sizes the "require a declaration" gate.

**Method.** Card LOADER-ABI's own production decision
(`cargo run -p qcore-c-gateway --example declared_abi_census`) over all **185**
pinned units, from both compile databases, with every `undeclared` row split by
whether the preprocessed text contains a **call-position** occurrence of the
name (`(?<![A-Za-z0-9_])NAME\s*\(`). The call test is a regular expression, not
the analyzer's own call-position rule, and it is deliberately *over*-inclusive,
so the `called` number is an upper bound on what a gate would refuse.

| outcome | Fluent Bit leg (152 entries) | Monkey leg (29 units) | total |
|---|---|---|---|
| `agreed` | 1,043 | 171 | **1,214** |
| `undeclared-called` | 0 | 0 | **0** |
| `undeclared-mentioned` | 6 | 0 | **6** |
| withdrawn (any class) | 0 | 0 | **0** |

Archived as [`artifacts/undeclared-census.tsv.gz`](artifacts) and
[`artifacts/undeclared-census-monkey.tsv.gz`](artifacts).

**Sizing decision: do not implement, and the reason is not the number.** The
six `undeclared-mentioned` rows are the artefact class card LOADER-ABI already
identified — `malloc` inside `__attribute__((__malloc__))` or inside a longer
identifier — and **zero** contracted names are called without a declaration this
reader can read. So the gate would refuse nothing at all in the pinned corpus,
which makes it look free.

It is not free, and the census is what shows why: the number that matters is not
how many rows *are* undeclared today but what the reader's coverage would refuse
tomorrow. Every `agreed` row above is a declaration this token reader
successfully read; card LOADER-ABI's §3 records the forms it will not commit to
(K&R identifier lists, enumeration types, `long double`, function-pointer
parameters, three-level pointers, a declaration built by a macro it does not
follow), and each of those is already an `abi-unreadable-declaration`
**withdrawal** rather than an `undeclared` row — the fail-closed direction is
therefore *already* covered for every declaration the reader can see and reject.
What a require-a-declaration gate would add is refusal for callees whose
declaration the reader never sees at all, and that set is bounded by the
reader's coverage rather than by anything about the source. A corpus measuring
zero today says nothing about a corpus with one macro-built prototype.

Recorded, not repaired. `LOADER-ABI-UNDECLARED` stays open with this measurement
attached.

**Two method notes, recorded rather than glossed.** The census preprocesses each
unit with its **own full compile command**, while the pinned compatibility route
projects a historical option subset, so the two texts are not byte-identical;
and one pinned base name is not unique (`mk_utils.c`, in both `mk_core` and
`mk_server`) — the same ambiguity card LOADER-ABI recorded. Both are properties
of the measurement, not of the decision.

---

## 6. Corpus

### 6.1 Both directions measured, and the delta is exactly zero

Freshly prepared pinned clone (Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, forced-Libevent Monkey, explicit
`/usr/bin/clang`, `third_party/libevent` at `d82464a2`, all five provider
packages rebuilt for this run), the canonical pinned compatibility route, exit
0, zero gateway failures. Card CORPUS-REPIN's recorded deviation 2 is
reproduced: the prepared CMake build lives outside the clone, so its
`compile_commands.json` was copied verbatim to
`<clone>/build/compile_commands.json`, where the pinned route reads it.

```sh
python3 -B scripts/prepare_cycle19_clang_audit.py \
  --fluent-bit …/fluent-bit --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 \
  --clang clang --clangxx clang++
python3 providers/package.py build --output build/cycle19/provider-packages-modes
QCORE_CLANG=/usr/bin/clang QCORE_CHUNKIO_PACKAGE=<packages>/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone> <monkey>/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

**BEFORE** is the base tree's analyzer: this branch's three modified gateway
sources restored to `8ec8402` and `legacy_declared_modes.rs` removed, so the
measurement runs the unmodified pass. **AFTER** is this card's tree.

| artifact | BEFORE | AFTER | against the wave-16 baseline |
|---|---|---|---|
| `resource-diagnostics.json` (11,133,547 B) | `46d0f178…` | `46d0f178…` | **byte-identical** |
| `resource-diagnostics.chunk-contracts.json` (4,186,178 B) | `c4f438e4…` | `c4f438e4…` | **byte-identical** |
| `resource-diagnostics.contracts.json` (48,408 B) | `e5b2a807…` | `e5b2a807…` | differs — attributed below |
| `resource-diagnostics.source-effects.json` (118,505 B) | `4fcbfcad…` | `4fcbfcad…` | differs — attributed below |
| `monkey-source.json` (1,665,474 B) | `7da5d442…` | `7da5d442…` | differs — attributed below |

**All five artifacts are byte-identical between BEFORE and AFTER**, so this
card's contribution to the corpus is *measured* at zero rather than argued:
0 unit status moves, 0 diagnostic leaves added, 0 removed. Headline unchanged
at every checkpoint: **185 units, 2 Safe / 42 provisional Unsafe / 137 Unknown
/ 4 data-only**, leak unchanged at 271, `summary_conflict` unchanged at 0.

The two runs' console transcripts differ in **one line** — the output path each
was given — so even the audit's own per-unit narrative is identical
([`artifacts/corpus-before-run.log.gz`](artifacts),
[`artifacts/corpus-after-run.log.gz`](artifacts)).

The three that differ from the *baseline* differ **identically in BEFORE and
AFTER**, which is what makes them environment residue rather than anything this
card did. Both recorded residue classes are attributed leaf by leaf
(`artifacts/attribution.txt`):

| artifact | leaves vs baseline | after normalising path provenance | attribution |
|---|---|---|---|
| `contracts.json` | 24 | **0** | path-literal provenance: the retained-source map is keyed by absolute path, so all twelve modules appear under this worktree's path and are absent under the baseline's. **No contract text moved** — this card edits no contract module. |
| `source-effects.json` | 45 | 45 | identity digests only — `build_digest`, `configuration_digest`, `obligation_id`, `task_digest`, `witness.task_digest` (which bind the contract environment, whose source map carries those same absolute paths) plus the rebuilt `package_digest`. No result's verdict, coverage shape or bounds moved. |
| `source.json` | 17 | 17 | path-literal provenance: the `compile_entry` command and the `__FILE__` expansions inside the `mk_event_*` wrappers. |

The baseline is **not** re-pinned: no integrator consolidation is claimed here,
and this card moved nothing.

### 6.2 Why it is zero, measured rather than assumed

§2's census says why, and the direction is checked from the analyzer's own side
as well. Of the 31 declared names the pinned manifest loads, 21 are comparable in some
unit, and exactly **8 (unit, name) rows** are inside the audit's own analysed
function set — five `cio_*` names in `lib/chunkio/src/cio_chunk.c` and three in
`src/flb_input_chunk.c` — and the census reads every one of them `no-summary`. Nothing else is
comparable, so nothing can be withdrawn, and the byte-identical surface is the
expected result rather than a lucky one.

---

## 7. Validation

Run on the final tree (Linux, rustc 1.98.0, Ubuntu clang 21.1.8):

| gate | result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2,766 passed, 0 failed, 8 ignored** (baseline 2,746; +20 in the new `loader_abi_modes.rs` family) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **73 audits** pass (72 → 73) — the new `AuditDeclaredModeAgreement.lean` prints **36** theorems, all closing over `propext` or nothing at all; **no new axiom** |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | 115 passed, 0 failed |
| `python3 -m unittest discover -s scripts/tests` | 161 tests, **1 failure**, expected and named below |
| 185-unit pinned audit, BEFORE and AFTER | §6, exit 0, zero gateway failures, all five artifacts byte-identical between the two |

**The one Python failure is an integrator reconciliation, not a defect.**
`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`
compares the committed acceptance matrix's `validation.lean_audits` against the
number of `proof/Audit*.lean` files. Adding a Lean audit moves that count 72 →
73 while the committed matrix still reads 72. That file is an acceptance record
this card must not edit; the repository's own history shows the reconciliation
is an integrator commit after merge (`df90496`, `86ae37c`, `aea80fe` all read
"Reconcile derived … after … merge"). Recorded here so the integrator can
regenerate it rather than discovering it.

**Not run:** the Windows and macOS legs. Nothing in this card is
target-dependent — the comparison reads modes, not types, so card LOADER-ABI's
`LOADER-ABI-LP64` data-model caveat does not apply here — but the legs were not
executed and are not claimed.

---

## 8. Named follow-ons

| name | what |
|---|---|
| **`LOADER-ABI-MODES-CROSS-UNIT`** | a contracted callee **defined in another translation unit** is never compared, and its declaration applies exactly as before. This is the dominant class by an enormous margin (§2), and it is a property of where the bodies live, not of whether the declarations are faithful. Closing it needs a cross-unit summary derivation the gateway does not have: the analyzer's unit of work is one preprocessed translation unit, and the certificate binds exactly that. A whole-program route would also have to decide *which* definition of a name it read, which is card G01's compile-configuration ambiguity in a harder form. |
| **`LOADER-ABI-MODES-NO-SUMMARY`** | among the callees this unit *does* define, the dominant uncheckable class is a body W4 derives no validated summary for — **484** of the 597 defined rows, and **all 8** of the production-reachable ones. That is the fail-closed answer and the declaration keeps applying, but it means this gate's reach is exactly W4's reach: every future widening of `infer_validated_summary` widens this comparison for free, with no change here. |
| **`LOADER-ABI-MODES-SELECTION`** | the analyzer compares only within its `selected_functions` set, so a contracted callee defined through a *header* — every one of the corpus's 113 `disagreed` rows — is never derived and never compared, however unfaithful it might be. That is fail-closed (nothing is withdrawn on absent evidence) but it is a real bound on reach, and it is where this family's behaviour would next move if card E01's main-source enumeration ever widened. Measured in §2.2, not assumed. |
| **`LOADER-ABI-MODES-RELATIVE`** | a derived summary is derived *under the environment's other declared premises*, so a contradiction is a contradiction relative to them, not an absolute statement about the C. `a_contradiction_is_relative_to_the_environments_other_premises` measures the shape: a transitively unfaithful wrapper is **not** convicted, because the only interface that could convict it is itself withdrawn, and nothing here derives through a withdrawn interface. This is the same trust boundary every W4 summary already sits behind; it bounds what the gate *catches*, never what it wrongly refuses. |
| **`LOADER-ABI-MODES-REFINEMENT`** | nullability, a `Stored`/`Removed` owner index and a `Reallocated` replaced parameter are refinements inside one duty and do not withdraw. The existing `SummaryConflict` remains their only report. Deciding them needs an ordering on refinements — which side is stronger, and for which caller — that this card does not attempt. |
| **`LOADER-ABI-UNDECLARED`** (existing) | sized here in §5 rather than closed. |
| **`LOADER-ABI-WITHDRAWAL-UNSAFE`** (existing) | now reachable on this axis too, and measured: class 2's caller becomes a provisional `Unsafe` because the withdrawal exposes the leak the declared `Released` was accounting for. Not reachable in the pinned corpus (§6), and never an upstream defect claim. |

---

## 9. Increment paragraph (draft, for the integrator)

> Declared/derived custody-mode agreement (LOADER-ABI-MODES): a declared
> resource contract whose custody *duty* the validated W4 summary of the
> callee's own body in the analysed unit **contradicts** is now withdrawn for
> that unit — every call site refuses with `mode-parameter-contradiction` or
> `mode-result-contradiction` — closing the fail-open card LOADER-ABI recorded,
> where types were compared against the source and modes against nothing. The
> evidence was already computed and already reported: `derive_custody_summaries`
> records the disagreement and `SummaryConflict` makes the *defining* function
> `Unknown`, while the declaration went on governing every call site. Three
> `Safe` callers resting on that — a double release, a hidden leak and a
> discarded acquisition — are reproduced and closed. The relation is taken in
> what a mode promises the caller (`Retained`/`Transferred`/`Delivered`/
> `Restored`; `Absent`/`Produced`), proved to *be* the duty inequality rather
> than sampled, and deliberately stops short of refinements inside one duty:
> nullability, a `Stored`/`Removed` owner index and a `Reallocated` replaced
> parameter never withdraw. That boundary is not academic — the census measures
> that **every** derivable comparison in the pinned corpus is a nullability
> refinement, in three production families (`rd_free`, `mk_mem_free`,
> `flb_free`, 113 rows), so withdrawing on inequality, which is what today's
> `SummaryConflict` tests, would have withdrawn a release contract in every unit
> that includes those headers. Withdrawal is monotone in the refusing direction
> and never installs the derived summary in the declaration's place, because an
> inferred summary can be weaker than the declaration it convicts; a fail-closed
> pin from card C03-NFR caught the first implementation destroying its own
> evidence through a self-recursive rewrite and was repaired rather than
> weakened. The census sizes the family: 598 `undefined` / 484 `no-summary` /
> 113 `disagreed` / **0 contradicted** over all 185 pinned units, of which just
> **8 rows are inside the audit's own analysed function set and all 8 read
> `no-summary`** — so the 185-unit corpus was measured in **both** directions
> and all five artifacts are **byte-identical between BEFORE and AFTER**:
> 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only unchanged, leak
> unchanged at 271, and the wave-16 baseline's `diagnostics.json` reproduced
> byte for byte. `LOADER-ABI-UNDECLARED` is sized rather than closed
> (1,214 `agreed`, **0** contracted names called without a readable declaration,
> 6 mention-only artefacts) with the gate recorded as bounded by the reader's
> coverage rather than by the corpus. 20 new tests in a held-out `m_*` family,
> 36 audited Lean theorems in a new `QCore.DeclaredModeAgreement` namespace
> (73 audits, no new axiom), and contract spec §C04.1 gains the mode-agreement
> requirement. Nothing is promoted. Follow-ons: `LOADER-ABI-MODES-CROSS-UNIT`,
> `LOADER-ABI-MODES-NO-SUMMARY`, `LOADER-ABI-MODES-SELECTION`,
> `LOADER-ABI-MODES-RELATIVE`, `LOADER-ABI-MODES-REFINEMENT`. Evidence:
> `docs/reports/remaining-loader-abi-modes/README.md`.

**One integrator note:** adding a Lean audit moves the acceptance matrix's
derived `validation.lean_audits` from 72 to 73, which this card must not edit;
`scripts/tests/test_build_matrix_sections.py` fails on that single row until the
usual post-merge reconciliation commit (§7).
