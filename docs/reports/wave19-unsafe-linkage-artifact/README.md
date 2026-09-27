# UNSAFE-LINKAGE-ARTIFACT — naming the build a waiver was waived from — 2026-09-10

> Runtime import maintenance: archived source inputs and runnable snippets now use `#import`; original results and hashes describe their recorded revision. See the [spelling attribution](../runtime-source-imports/spelling-attribution.json) for maintained source identities.

Card `UNSAFE-LINKAGE-ARTIFACT` of wave 19: the staged residual of card
`UNSAFE-LINKAGE` (wave 18), which landed unchecked linkage but could only say
*symbol X, unchecked, at ABI S* — never *from which build*.

**Base tree:** main `c7c0a3c`. Design documents:
[`docs/design/NATIVE_LIBRARY_LINKAGE_V1.md`](../../design/NATIVE_LIBRARY_LINKAGE_V1.md)
§4 (the manifest; adopted into the tree by wave 18) and
[`docs/design/UNSAFE_LINKAGE_V1.md`](../../design/UNSAFE_LINKAGE_V1.md) §4.2
(the contract-less entry — level **L1** of the linkage spectrum). Parent record:
[`docs/reports/wave18-unsafe-linkage/README.md`](../wave18-unsafe-linkage/README.md).

---

## Headline

**L1 landed.** A `source.native-library.v1` manifest now exists in the slice the
waiver needs, and a waived declaration a manifest names publishes

> `k_unchecked_add`, unchecked, at `(int32_t, int32_t) -> int32_t`, **from
> artifact `libqcore_unchecked_demo.a` of library `qcore_unchecked_demo 0.1.0` at
> `sha256:9f2c…c72e`, under stated ABI premises (data model lp64, no unwinding
> across the boundary, closed allocator custody, single-threaded caller)**

instead of the bare wave-18 sentence — and `qcorec check --native-library`
prints exactly that.

| | |
|---|---|
| What was added | a manifest format + total-or-reject loader (`qcore_core::native_library`), an explicit builder-side selection (`CompileOptions::native_libraries`, `qcorec --native-library`), and a second premise variant `ExternAssumption::ManifestUncheckedLinkage` |
| Admitted surface | **unmoved.** The closed wave-18 table decides first and unchanged; a manifest cannot buy a pointer, and a contract-less manifest entry declaring one rejects the whole manifest |
| Artifact identity | a **pin, not a probe** — well-formedness checked, nothing hashed, no filesystem read (§5) |
| Diagnostics | a new `NativeLibrary` pass owning **`Q039`–`Q042`**, the band `NATIVE_LIBRARY_LINKAGE_V1.md` §7 reserved, through the §L21 process |
| Default | unmoved in both directions: with no manifest selected, premises, certificates and emitted C are byte-identical to wave 18 (§3) |
| New reserved words | **none.** A manifest is not source; it adds no form to the source grammar |
| `qcore-schemas` | **untouched at 14.0.0** — no provider rebuild (§7) |
| Evidence | 42 new Rust tests (25 loader and core-model unit, 16 route, 1 CLI process) and **25 audited Lean theorems** in a new `QCore.NativeLibrary` namespace, no new axiom |

**Nothing is promoted.** A manifest-resident module is `Safe` **only under its
printed premises**, and the sharpened premise is strictly weaker than a declared
contract: it says *which code*, never what that code does. No raw Cycle 19 unit
moves, no CT mapping changes, the contract corpus is untouched (this card writes
no contract text at all), and full language conformance stays false.

---

## 1. What the card had to build, and what it deliberately did not

The wave-18 residual was blocked on one thing: *the manifest does not exist*.
Landing L1 first would have meant landing the whole of
`NATIVE_LIBRARY_LINKAGE_V1.md` stage (a) to carry one field. This card builds the
slice of stage (a) that a contract-less entry needs, and refuses the rest **by
name** rather than leaving it unimplemented and silent.

| §4 piece | This card |
|---|---|
| 4.1 symbol table, contract-less entries | **landed**, with may-not-bind-twice inside a manifest and across a selection |
| 4.1 contract slice binding, both directions | **not implemented** — a `contract fn` entry is `ManifestRefusal::ContractedEntry`, follow-on `NATIVE-LIBRARY-CONTRACT-SLICE` |
| 4.2 header agreement | **landed as a refusal** (not a withdrawal — §4), against the manifest's stated header declaration; reading a real `.h` is follow-on `NATIVE-LIBRARY-HEADER` |
| 4.3 artifact identity | **landed as a stated pin** (§5) |
| 4.4 ABI premises | **landed**: four mandatory closed-vocabulary fields, carried onto the certificate |
| 4.4 closed-allocator-custody structural check | vacuous by construction — a contract-less entry cannot acquire (`UNSAFE_LINKAGE_V1.md` §9.1 point 2), so there is nothing to check rather than a check that was skipped |
| §6 `std.buffer`, §12 stages (b)–(d) | untouched |

The refusals are what makes the partial implementation honest: the pieces this
card did not build are not absent, they are *declined*, each with its own
message.

## 2. The manifest format

One declarative file per library, `;`-terminated statements, `#` comments,
loaded **total or reject** on the resource-contract v1 loader's precedent. The
fixture in full
(`compiler/tests/fixtures/native_library/unchecked_demo.qcorelib`, comments
elided):

```
native_library manifest v1;
library qcore_unchecked_demo;
version 0.1.0;
artifact libqcore_unchecked_demo.a;
hash sha256:9f2c4a1e7b3d05860f14ca9e2d7b58311c06ae4f9d2b7c8e5a0341f6d9b8c72e;
data_model lp64;
unwinding abort;
allocator_custody closed;
thread_posture single_caller;

unchecked fn k_unchecked_add(int32_t, int32_t) -> int32_t;
unchecked fn k_unchecked_side_effect(uint32_t) -> void;
unchecked fn k_unchecked_measure(uint64_t) -> uint64_t;
```

Four decisions in it are worth stating rather than leaving to the grammar.

**The marker is the selector**, exactly as `requires resource_contract_v1` is
for the contract loader. A file that does not carry `native_library manifest v1;`
as its first statement is rejected rather than interpreted under a guess.

**Entry types are the C spellings, not the QCore ones.** The manifest states the
library's *exported header declaration*; the declaration states QCore types. So
`int32_t` here and `i32` there, and the gate compares the two ABI projections
rather than two strings. This is what makes §4.2's "header agreement" a
projection comparison in the small.

**A contract-less entry is explicitly marked, and only contract-less entries
exist.** Design §4.2 requires the marking so that a *dropped* contract can never
be read as a waiver. Here the requirement is met twice over: the entry must say
`unchecked`, and a `contract fn` entry rejects the whole manifest by name.

**A contract-less entry may not declare a pointer.** The three pointer spellings
are recognised and then refused
(`ManifestRefusal::PointerInUncheckedEntry`), because a contract-less entry has
no modes in which to state what a pointer may be used for. A library's pointer
symbols belong in a contract slice entry, which does not exist yet — so this
refusal is exactly the seam between the two, and it keeps the seam visible.

Every other rejection is its own named constructor: missing marker, unterminated
statement, unknown field, unknown field value, repeated field, missing field
(all eight are mandatory), malformed hash, malformed entry, unknown parameter or
result spelling, `unit` parameter, symbol bound twice, and a manifest binding
nothing. Fifteen constructors, no catch-all, one test row each.

## 3. The default does not move — pinned in both directions

The brief's sharpest requirement: *nothing about the manifest may make an
unmanifested program refuse or an assumption disappear.*

* `without_a_manifest_every_byte_of_the_wave_18_route_is_unchanged` — `None` and
  an explicitly empty selection produce the same premises and the same emitted C
  as wave 18, and the three bare `UncheckedLinkage` premises are present.
* `a_manifest_that_names_other_symbols_changes_nothing` — a loaded, valid
  manifest naming different symbols leaves the premise list identical to the
  no-manifest one.
* `a_manifest_naming_some_symbols_sharpens_exactly_those` — partial residence is
  admitted, not refused: the two premise kinds coexist in one certificate, each
  saying exactly what it knows.
* `the_emitted_c_and_the_checked_evidence_are_unchanged_by_the_manifest` — the
  C source, the call list, the effect row and the acquisitions are equal with and
  without a manifest. The **only** field that differs is the assumption list,
  which is the point.
* The whole wave-18 `unsafe_linkage.rs` suite (28 tests) passes unchanged.

And in Lean, `an_unmanifested_waiver_keeps_the_bare_premise` states the same
sentence over the model.

The premise is a **separate variant**, not an optional field on the wave-18 one.
That is a deliberate choice and it is what makes
`UNSAFE_LINKAGE_V1.md` §4.2's closing paragraph literally true: an L0 premise
stays a valid weaker statement rather than becoming a malformed L1 one, and a
reader tells the two apart by inspection
(`the_sharpened_premise_says_strictly_more_and_stays_distinguishable`,
`a_bare_premise_is_never_a_sharpened_one`).

## 4. One recorded deviation: refusal, not withdrawal

`NATIVE_LIBRARY_LINKAGE_V1.md` §4.2 specifies `LOADER-ABI`-style **per-unit
withdrawal** on header disagreement: the declaration is withdrawn for the unit
and its call sites fall through to the uncontracted refusal. This card
**refuses** instead (`Q041`), and the difference is not a shortcut.

Withdrawal works on the contracted route because there is a weaker admissible
reading below the withdrawn declaration, and that reading *refuses* (`Q023`).
On this route the only weaker reading is the **bare L0 waiver**, which is
accepted. Withdrawing to it would mean a manifest that contradicts a declaration
buys a certificate that says *less* about the artifact than the manifest was
selected to make it say — a contradiction rewarded with silence. So the
disagreement refuses outright, which is the monotone direction.

The property is pinned from the other side too, where it matters most:
`a_named_symbol_is_never_admitted_bare` (Lean) states that a symbol its manifest
names is never admitted with `residence = none`, so no execution path reaches the
weaker premise by contradiction. `a_disagreement_refuses_rather_than_withdrawing_to_the_weaker_premise`
(Rust) asserts there is no accepted-with-bare-premise outcome to find.

Recorded as a deviation from the adopted document, with the ground stated, per
that document's own §0 discipline.

## 5. The artifact identity is a pin, not a probe

The brief asked for a decision and a trust boundary. **v1 records the stated
identity as a premise and verifies nothing on disk.** Well-formedness is checked
(`sha256:` plus 64 lowercase hex digits, and no other algorithm — an identity a
reader cannot interpret is not a premise); the artifact is never opened, never
hashed, never searched for.

Two reasons, both load-bearing:

1. **A hash of a file on this host is not evidence about the link.** Whether the
   linker used the named artifact is the builder's act — `NATIVE_LIBRARY_LINKAGE_V1.md`
   §10 excludes verifying the link step, and this card does not widen that.
   Hashing a candidate file at check time would produce a stronger-*looking*
   statement that is not a stronger one, and a reader would be entitled to
   misread it as "the link was verified". This repository does not manufacture
   those.
2. **A premise must be rederivable from the program alone.** The driver
   publishes no certificate it did not independently re-derive
   (`recheck_extern_certificate`, `Q027`). A premise whose content depended on
   the checking host's filesystem could not be reproduced by a recheck
   elsewhere — so a stated pin is exactly what a certificate can honestly carry.

**The trust boundary, in the repository's premise language** (hard invariant 2,
constraint 28): the artifact identity and the four ABI premise fields are
**declared assumptions**, on the same footing as `observations_trusted`,
`DeclaredFootprintShape` and a declared C04 contract. Agreement discharges
nothing. What a certificate states is that the program was checked *on the
declaration that this symbol comes from this build* — which makes a substitution
at link time a **detectable** premise violation instead of an invisible one, and
that is the entire delta over wave 18.

A builder-side hash *observation* — comparing a real artifact against the pin at
build time and recording the comparison as an observation, never as a check — is
named as follow-on `UNSAFE-LINKAGE-ARTIFACT-PROBE`. It would be an observation
and could never be more.

## 6. Diagnostics

A new semantic pass `NativeLibrary` owning **`Q039`–`Q042`** — the band
`NATIVE_LIBRARY_LINKAGE_V1.md` §7 reserved after `UNSAFE-LINKAGE` took
`Q032`–`Q035` and `PIPE-OPERATOR` took `Q036`–`Q038` — through the §L21 process.
Spec §L21 gained the row, §L14.2 the normative section, and
`QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02d.2 the manifest grammar, all in this
change. **No existing code changed meaning**; `Q032`–`Q035` keep their exact
conditions and `Q033`'s text is unchanged.

| Code | Refusal | Pinned by |
|---|---|---|
| `Q039` | a selected manifest does not load (any of the fifteen named rejections) | 5 route rows + 25 loader unit tests |
| `Q040` | two selected manifests bind the same symbol — the binding must be a function | 1 route row + 1 unit test |
| `Q041` | a waived declaration and the manifest that names it disagree in arity, a parameter or the result | 3 route rows + 3 unit rows |
| `Q042` | a name the manifest declares contract-less is also reached through a declared contract | 1 route row |

The pass is separate from `UncheckedLinkage` on purpose, and the reason is the
same kind of reason §L21 requires elsewhere: `UncheckedLinkage` decides whether a
*signature* may cross a boundary nothing checks, and its decision is untouched
here; `NativeLibrary` decides whether the **manifest** a waiver is resident in
loads, binds and agrees. The trust object is the manifest, not the declaration.

`Q042` deserves its own note. It is the **mirror of `Q034`**. `Q034` refuses a
waiver for a name the contract environment declares — you may not waive a
contract you have. `Q042` refuses a *contract* for a name the selected manifest
declares contract-less — you may not assert modes and effects about a build whose
own manifest states none. One rule in two directions: where two declarations are
visible they are compared, and the refusing direction wins.

## 7. Schemas, providers, corpus: untouched by construction

* **`qcore-schemas` stays 14.0.0.** The manifest types live in `qcore-core`,
  which has no dependencies at all, and the format is hand-parsed rather than
  serde-derived. Constraint 18 is therefore not engaged: **no provider package
  was rebuilt and none needed to be.** This was a deliberate design constraint
  from the brief and it cost nothing — the manifest never crosses the plugin
  protocol.
* **No file under `crates/qcore-c-gateway/`, `crates/qcore-audit/`,
  `contract/qcore/` or `providers/` is touched.** The complete production diff is
  `qcore-core` (the new module, one `ExternCallee` field, one `ExternAssumption`
  variant, one constructor, one premise function), `qcore-frontend` (one
  `SemanticPass` variant and its code-map row), `qcore-verification` (one
  `CompileOptions` field, the manifest load, the `Q041`/`Q042` gates), `compiler`
  (one re-export) and `qcorec` (one flag, one printed line).
* `qcore-contract` is **not** touched, so the gateway's contract loader is
  untouched by construction.
* The contract corpus count is unchanged — this card writes no contract text —
  and no CT manifest row moves. The corpus baseline stays
  `docs/reports/wave17-corpus/`; this card is corpus-neutral by construction, on
  the same structural argument wave 18 recorded (§10 there), with one addition:
  this card changes **no parser rule at all**.

## 8. Lean

`proof/QCore/NativeLibrary.lean` (new) + `proof/AuditNativeLibrary.lean` (new,
registered), imported into `proof/QCore.lean`. **25 theorems audited, no new
axiom** (only `propext`, `Classical.choice`, `Quot.sound`).

The admission table, the callee record and the rendered signature are
**imported** from `QCore.UncheckedLinkage` — which imports its ABI vocabulary
from `QCore.ExternCall` — so there is one table in the model and not three that
could drift. That is also what makes the card's central claim statable rather
than merely asserted.

* **Totality survives the manifest** — the theorem the brief asked for:
  `manifest_assumption_totality` (every uncontracted resident still contributes a
  premise that names it, manifested or not),
  `there_is_no_assumption_free_path` (a waived callee's premise is never the
  contracted one, in either residence case),
  `the_manifest_never_removes_a_premise` (quantified over an **arbitrary**
  residence assignment, so it is not a statement about one manifest),
  `every_resident_contributes_exactly_one_premise`,
  `a_premise_names_a_resident_of_the_list`.
* **Sharpen, and only sharpen** — `an_unmanifested_waiver_keeps_the_bare_premise`,
  `a_manifested_waiver_names_its_artifact`,
  `a_bare_premise_is_never_a_sharpened_one`,
  `a_contracted_callee_names_no_artifact`,
  `a_contracted_callee_contributes_no_waiver`.
* **The surface does not move** —
  `a_refused_surface_stays_refused_under_every_manifest` (general),
  `a_manifest_cannot_buy_a_pointer_parameter` / `…_result`,
  `a_pointer_entry_is_not_an_admissible_contract_less_entry`,
  `a_pointer_result_entry_is_not_admissible`,
  `an_admitted_resident_was_admitted_by_the_closed_table` (so every property the
  wave-18 table establishes — custody-freedom, acquiring nothing, being marked —
  carries over unchanged), with `a_scalar_entry_is_admissible` for non-vacuity.
* **Refusal, not withdrawal** — `a_disagreeing_manifest_refuses`,
  `a_named_symbol_is_never_admitted_bare`,
  `agreement_is_declaration_equals_header`.
* **Determinism** — `the_binding_is_a_function`, `admission_is_deterministic`,
  `the_published_premise_is_a_function_of_the_admission`.
* **Non-vacuity** — `the_manifest_route_admits_and_names_the_artifact` and
  `an_unnamed_symbol_is_admitted_bare` are concrete admissions, so the refusals
  above refuse something reachable.

**Stated boundary of the Lean model.** As in `QCore.UncheckedLinkage`,
`AbiType` is the collapsed projection, so the modelled agreement gate compares
projections at that granularity while the Rust gate compares the full fifteen-way
table; the Rust side is pinned by `every_way_of_disagreeing_with_the_manifest_refuses`
and the route rows. The manifest's *concrete syntax* and its total-or-reject
loader are a Rust boundary and are tested there — what the Lean model is about is
what a loaded manifest **means**. Source extraction and the agreement of the Rust
code with the transcription remain premises, as for every model here.

## 9. Gate

| Check | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **3,027 passed, 1 failed, 8 ignored** across 181 test binaries — the one failure is the recorded constraint-16 load flake, re-verified green in isolation (§9.1) |
| `compiler/tests/native_library.rs` | 16 passed |
| `crates/qcore-core` unit `native_library` | 25 passed |
| `crates/qcorec` CLI premise printing | 1 passed |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `reevaluation_probe` | exit 0 |
| `reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **Passed 81 Lean audits** (80 at base + `AuditNativeLibrary`) |
| `python3 -m unittest discover -s scripts/tests` | 193 run, 1 expected failure (§9.2) |

`third_party/libevent` was initialised in this fresh worktree before the run
(`git submodule update --init --recursive`), as wave 18 also recorded.

### 9.1 The one workspace failure is a recorded load flake, verified in isolation

`qcore-verifier-host::confined_process_execution::a_confined_fixture_cannot_write_outside_the_admitted_paths`
failed once during the workspace run with exactly its recorded signature:

```
assertion `left == right` failed: Broken pipe (os error 32)
  left: Io
 right: Terminated
```

That is **P02-FLAKE row three** verbatim (`AGENTS.md` constraint 16: "the
confined-fixture write test (`Io(Broken pipe)` under load)"). The run took place
with four sibling agent worktrees and the integrator's own merged-tree gate
building on the same host — one-minute load average between 8 and 65 for the
duration, peaking at 65 — which is the documented condition. Re-run in isolation
on the same binary, **unweakened**:

```
$ cargo test -p qcore-verifier-host --test confined_process_execution -- --test-threads=1
test result: ok. 12 passed; 0 failed; 0 ignored
```

No assertion was touched. The other three constraint-16 rows (`RLIMIT_NPROC`,
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets`, `real_packages_…`) passed
in the workspace run itself and needed no isolation.

This card changes nothing that test touches: its production diff contains no
file under `crates/qcore-verifier-host/`.

### 9.2 One expected acceptance-matrix drift, left for the integrator

`scripts/tests` asserts the acceptance matrix's derived `lean_audits` row, which
must grow by one now that this card registers an audit file. Acceptance files are
integrator surface and are not this card's to edit; reported here rather than
worked around, exactly as wave 18 did for the same row.

## 10. The prior-art branch

`claude/qcore-c-abi-linking-c8c443` (tip `ecede86`) held the original
`source.native-library.v1` design. Wave 18 adopted that document into the tree
with four recorded adjustments, and **this card used the adopted in-tree copy
only**. Nothing further was taken from the branch: no code, no test, no
additional text. The branch is therefore fully superseded and the integrator may
delete it.

## 11. Named follow-ons

| Name | What is open |
|---|---|
| `NATIVE-LIBRARY-CONTRACT-SLICE` | the contracted half of §4.1: a manifest entry carrying a C04 contract slice member, with §4.1's both-directions totality and the pointer modes it unlocks. This is where a library's pointer symbols belong, and it is the prerequisite for levels L2→L3 of the spectrum. `ManifestRefusal::ContractedEntry` is the seam it fills. |
| `NATIVE-LIBRARY-HEADER` | reading the library's real `.h` instead of the manifest's transcription of it. The manifest states the exported declaration in the profile's canonical stdint spellings; a real header reader must be reproducible at certificate recheck without a typed Clang AST, which is the constraint that makes it its own card (`LOADER-ABI`'s precedent). Until then, that the manifest's transcription matches the header is a premise. |
| `NATIVE-LIBRARY-WITHDRAWAL` | §4.2's per-unit withdrawal semantics, which become the right shape once a contract slice exists (§4 above explains why they are the wrong shape now). |
| `UNSAFE-LINKAGE-ARTIFACT-PROBE` | a builder-side observation comparing a real artifact against the pin, recorded as an observation and never as a check (§5). |
| `NATIVE-LIBRARY-SOURCE-BINDING` | binding a manifest from the *source* file rather than resolving by symbol name across the selection — the `#import std.buffer;` shape of §1. Today a waiver resolves by name; a source-side binding would make *which library* visible to a reader of the source, as the waiver itself already is. Needs a registry name or a new form, so it is a card with a constraint-14 census. |
| `NATIVE-LIBRARY-MULTI-HASH` | hash algorithms beyond `sha256`. Rejected rather than carried opaquely today, deliberately. |
| `UNSAFE-LINKAGE-PTR`, `-ARGS`, `-CTX`, `-CLI`, `-EXTENT-PREMISE` | wave 18's follow-ons, all carried unchanged. `-CLI` is narrowed: `qcorec` now selects *manifests* (`--native-library`), but still selects no **contract environment**, which is what that follow-on is about. |

Variadics, function pointers, structs, floats and verifying the link step stay
out of scope for this route exactly as they are for the contracted one.
