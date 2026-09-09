# CTWIN-PUBLISH — publication into an object that outlives the call

Date: 2026-09-10. Base: main `8069a99` (wave-15 consolidation).
Branch: `claude/wave-16-hatch-opening-43bc0d` (isolated worktree). Wave 16,
card `CTWIN-PUBLISH`, kind gateway/soundness, scope L2.

Card `CT-CTWIN` recorded, as the open finding **`CT-CTWIN-PUBLISH`**
([`docs/reports/wave9-ctwin/README.md`](../wave9-ctwin/README.md) §1.1, matrix
entry `CT-CTWIN-O`), that *a store of an owned pointer into a file-scope object
is invisible to the resource projection — publish-then-free is `Safe` with no
diagnostic*. This card reproduces that, repairs it fail-closed at the general
gateway site, and measures the consequence on the pinned 185-unit corpus.

**Headline.** The hazard is reproduced and closed. The repair is a **refusal**,
not an escape — because the escape the brief proposed is fail-closed in only one
direction, and the other direction is a *promotion* (§3). Corpus reach is
**zero**: all five artifacts are byte-identical before and after, and a
toggled-rule probe (§6.3) shows why. **Nothing is promoted**: raw Cycle 19 units
stay 2 Safe / 42 provisional Unsafe / 137 Unknown plus four data-only, the
contract corpus stays 67 modules, CT coverage stays 32/50, and the wave-15
baseline is **not** re-pinned because it did not move.

---

## 1. The hazard, reproduced

Measured at the branch base `8069a99`, through the real gateway entry points
(`validate_unrestricted_c11`, then `analyze_unrestricted_c11`) over card C02-P's
generic synthetic `b_*` custody API:

```c
struct b_handle *g_published;
void w(int n) { struct b_handle *h; h = b_open(0, 0, n);
                if (!h) { return; } g_published = h; b_close(h); }
```

| Shape | Base `8069a99` | After this card |
|---|---|---|
| publish, then release | **`Safe`, no diagnostics** | `Unknown`, `[publication:g_published, unknown_alias:h]` |
| publish, no release | `Unsafe`, `[leak]` | `Unknown`, `[publication:g_published]` |
| release, **then** publish | **`Safe`, no diagnostics** | `Unknown`, `[publication:g_published]` |
| acquire directly into the object | `Unsafe`, `[leak]` | `Unknown`, `[publication:g_published]` |
| `static` **local** as the destination | **`Safe`, no diagnostics** | `Unknown`, `[publication:cache, unknown_alias:h]` |
| `static` file-scope object | **`Safe`, no diagnostics** | `Unknown`, `[publication:g_published, unknown_alias:h]` |

Four `Safe`-with-no-diagnostic answers, one of them (row 3) publishing a pointer
the projection *itself* records `RELEASED`.

**The mechanism, and one correction to the brief.** The brief describes the
store as "a no-op". It is not: the importer lowered `g_published = h;` to an
ordinary `LegacyInstruction::Alias`, so the file-scope object became **one more
local name**. Custody therefore stayed inside the function, the release
discharged it, and the dangling published reference had no representation at
all. The leak in row 2 is the same fact seen from the other side — the alias
kept the identity live, so the exit reported it.

## 2. Where the invisibility lives, measured

**It is general gateway behaviour, not C-twin-route-specific.** The reproduction
above runs through `analyze_unrestricted_c11` over a synthetic contract set with
no CT harness, no `legacy_c` route and no CT fixture involved; the same shapes
answer identically inside the configured audit route. The repair is therefore at
the general site:

* [`crates/qcore-c-gateway/src/publication.rs`](../../../crates/qcore-c-gateway/src/publication.rs)
  (new) — **which destinations outlive the call**, read from the same
  preprocessed token stream every other typed premise is read from, so W4
  derivation, W4 validation and certificate rederivation repeat the
  classification from the certificate's own content;
* the `CTWIN-PUBLISH` block of `legacy_c::transfer` — **when naming one is a
  refusal**, checked flow-sensitively before any effect of the instruction
  applies;
* `LegacyCfg` gains a `publications` field carrying the destination set, and
  `legacy_recursion::combine` unions the two contexts' sets rather than assuming
  they agree.

No diagnostic code was added or changed: a refusal is an existing `Unsupported`,
which `classify_diagnostics` publishes as the `Unknown` class.

## 3. Why refusal, and not an escape — the measured deviation from the brief

The brief directed that the store "must be an **ESCAPE** (fail-closed)". That is
fail-closed in **one direction only**, and the other direction is a promotion:

| Body | Escape model | Implemented model |
|---|---|---|
| publish, then release | `ReleaseAfterEscape` → provisional `Unsafe` | `Unknown` |
| publish, **no** release | escape discharges the residual → **`Safe`** | `Unknown` |

Row 2 is the defect. Measured at the base, that body is `Unsafe` with a `Leak`;
under an escape model it would become **`Safe`**, which is exactly the
`Unknown`→`Safe` movement the card forbids and hard invariant 1 forbids. And it
would assert an accounting no contract states: unlike a return or a declared
consuming position, a store into a static-storage object hands custody to *no
named party*, so there is no interprocedural obligation for the escape to
discharge.

The implemented rule withdraws **both** directions instead, which is also what
`CT40`'s own expectation asks for in words — *"Not certified; unsupported
lifetime/publication protocol"*. Two consequences, both deliberate:

* **The `Leak` in row 2 is withdrawn.** The published identity is marked as
  carrying an unresolved effect (`mark_unresolved_effect`, the mechanism the
  projection already uses for an unresolved callee effect), so `report_leaks`
  withholds it. This is the `w4-nullable-removal` rule — *a residual leak that
  depends on an unresolved effect stays `Unknown`* — applied to a residual that
  depends on an unmodeled publication protocol. It is a **downgrade** of a
  provisional `Unsafe`, never a promotion, and it removes an overclaim: the
  identity may well be released through the published object.
* **A publication is never a validated violation.** `Unsafe` is reserved for a
  validated reachable violation (invariant 2). `g = p; free(p);` is not one: the
  dangling reference only becomes a violation when someone *reads* the published
  object, which one translation unit cannot establish.

Both directions are proved in Lean (`the_escape_model_promotes_on_some_input`,
`a_publication_is_never_a_validated_violation`).

## 4. The rule

### 4.1 The destination half — which objects outlive the call

`publication::destinations(unit, body, formals)` is the union of:

* the body's **`static`-duration locals** — a declaration run in the body whose
  specifiers carry `static` or `_Thread_local`. A `static` local has block scope
  but static storage duration, so it outlives the call exactly as a file-scope
  object does; this was **not** in the brief and was added by measurement (row 5
  of §1 was `Safe` at the base);
* the **file-scope objects** the unit declares, minus every name this body
  shadows — as a formal parameter or as a declaration of its own.

Both readers are deliberately narrow and **positive**: a name enters only when
the reader recognised a declarator for it. Recorded consequences:

| Shape | Recognised | Note |
|---|---|---|
| `struct h *g;`, `static struct h *g;`, `extern int g;` | yes | |
| `struct h *g_a, *g_b, g_c;` | all three | |
| `struct h *g[4];`, `int g[2][3];` | yes, by name | trailing array declarators are skipped |
| `int g = 1;`, `int g[] = {1,2};` | yes | the initializer is dropped first, and a brace initializer does not desynchronise the run scanner |
| `struct r { … } g;`, `enum e { … } g;` | yes | |
| `struct h;`, `struct r { … };`, `enum e { … };`, `typedef …;` | no object | a tag is not an object |
| `void f(struct h *h);` (prototype) | nothing, **including `h`** | a `(`-carrying declarator group is skipped whole |
| `int f(void) { … }` (definition) | nothing, and the scan stays synchronised | |
| `void (*g_hook)(int);` | **no** — recorded as `CTWIN-PUBLISH-SCAN` | a `(`-carrying declarator |

`CTWIN-PUBLISH-SCAN` is the one fail-open direction, stated rather than hidden
and audited in Lean as `an_unrecognised_name_is_not_a_destination`: a
declaration shape the reader cannot read contributes nothing, so that name keeps
its **pre-existing** modelling. It never invents a refusal; a better scanner is
monotonically more refusing (`a_wider_scan_only_adds_destinations`).

The body reader over-approximates in the direction that *reduces* refusal: a
statement it mistakes for a declaration (`a * b;`) only adds a shadowing name. An
assignment is never mistaken for a declaration — `is_declaration_candidate`
refuses `g = h`, which is what keeps the rule from disabling itself.

### 4.2 The value half — only a tracked identity this body holds

Naming such a destination is **not** the refusal. `transfer` refuses only when
the stored value carries an identity this body holds:

| Instruction | Refused when | Argument |
|---|---|---|
| `Alias { destination, source }` | `source` denotes at least one site **not** in `caller_sites` | the publication of an identity this body acquired |
| `Acquire` | always | the acquisition is published at the moment it is created; refused before the site exists |
| `ContractCall` with this destination | the contract's result is not `None` | same, through a contracted call |
| `Clear` (`g = NULL;`) | never | the null constant carries no identity |
| `OpaqueValue` | never | a value this function never tracks |

Per-direction arguments for the admissions, since the default is refusal:

* **A borrowed pointer stays admitted** (`CTWIN-PUBLISH-BORROWED`). `g = p;`
  where `p`'s identity is the *caller's* entry custody was `Safe` at the base and
  stays `Safe`. Publishing a borrowed pointer is a **lifetime** hazard, not an
  acquire/release accounting error, and this projection's documented property is
  one translation unit's acquire/release accounting. Refusing it would newly
  refuse bodies that are sound in the property actually decided. Named as an open
  follow-on rather than silently covered.
* **A null store stays admitted** because `Clear` stores the null pointer
  constant: no identity leaves the body, and refusing it would refuse the
  ordinary "publish then retract" idiom for no gain.
* **A scalar store into a file-scope scalar stays admitted** for the same
  reason, and it is measured (`g_count = n;` is `Safe` before and after).
* **A call whose contract yields no custody stays admitted**
  (`g_state = b_probe(h);`): the destination binds no identity.
* **An unbound source stays admitted**: nothing the projection tracks is
  published, and the pre-existing modelling (including its own
  `UnknownAlias`) is preserved exactly.

Two properties worth stating because they are *not* exceptions:

* **The rule reads no per-site status.** Publishing an identity the projection
  records `RELEASED` is refused exactly as publishing a live one is (§1 row 3),
  proved as `the_rule_reads_no_status`.
* **No destination shape was widened.** A store into a member of a file-scope
  aggregate (`g_reg.slot = h`), through a file-scope pointer (`g_reg->slot = h`)
  and into a file-scope array element (`g_slots[0] = h`) were **already** refused
  by the lexical destination gate, and each keeps its own pre-existing
  diagnostic — `indirect memory access is outside the resource projection` — and
  gains none of this card's. That is asserted, not assumed
  (`indirect_file_scope_destinations_keep_their_pre_existing_refusal`).

## 5. `CT-CTWIN-SYNC`: the `volatile` invariance, decided

Card `CT-CTWIN` measured that adding or removing a `volatile` qualifier changes
*neither the verdict nor one diagnostic*, and left the question open as
`CT-CTWIN-SYNC`: is that invariance correct, or a gap?

**It is correct, and it is now an asserted decision rather than an omission.**
C11 `volatile` is an access-ordering qualifier for the abstract machine, not a
synchronization primitive: §5.1.2.4 gives happens-before to atomics, mutexes and
thread creation/join, and to nothing else. A `volatile` completion flag
establishes no order between a producer's store and a consumer's release, so a
projection that derives **no** synchronization fact **must** answer identically
with and without it. Answering differently would be the defect: it would mean a
verdict of this projection depended on a fact it never derived.

`a_volatile_qualifier_moves_no_resource_projection_verdict` pins that in both
directions over three shapes — a publication body, an admitted local-acquisition
body, and a `volatile`-qualified published object — comparing the **whole**
verdict: the unit result, the withdrawal reason verbatim, and the complete
diagnostic vector. The model half is
`the_destination_decision_reads_only_the_four_name_sets`: the destination
decision is a function of four *name* sets, in which no qualifier appears, so no
qualifier change can move it.

`CT-CTWIN-ENTRY`/M20 is already asserted by the wave-9 runner and was not
touched. `CT-CTWIN-DESCRIPTOR` is out of this card's scope and is unchanged.

## 6. Corpus measurement

### 6.1 Provenance

Freshly prepared pinned clone in this worktree (Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit Clang
`Ubuntu clang version 21.1.8`, `/usr/bin/clang`), Libevent submodule
`d82464a277d0f42703702c4dfd9af6af38595a83`, forced-Libevent Monkey at the pin,
provider packages rebuilt into `build/cycle19/provider-packages-ctwin`, and the
canonical pinned compatibility route:

```sh
python3 -B scripts/prepare_cycle19_clang_audit.py \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 --clang clang --clangxx clang++
python3 -B providers/package.py build --output build/cycle19/provider-packages-ctwin
cp <build>/compile_commands.json <clone>/build/compile_commands.json

QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=<packages>/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone> <monkey>/compile_commands.json \
  <reports>/monkey-source.json <reports>/resource-diagnostics.json
```

Both runs exit 0 with **zero gateway failures**.

### 6.2 BEFORE reproduces the wave-15 baseline; AFTER does not move

**BEFORE** (base tree, the card's own source changes reverted):

| Artifact | Against `docs/reports/wave15-corpus/artifacts/` |
|---|---|
| `resource-diagnostics.json` | **byte-identical** (`1da0555bd5e3…`) |
| `resource-diagnostics.chunk-contracts.json` | **byte-identical** (`c4f438e4ed3d…`) |
| `resource-diagnostics.contracts.json` | identical after normalising the worktree path prefix |
| `monkey-source.json` | identical apart from `char[147]` → `char[189]` in 7 leaves |
| `resource-diagnostics.source-effects.json` | identical apart from package/build/configuration digests |

Both residuals are the two classes wave-9 and wave-15 already record, and both
are attributed exactly: the `monkey-source` delta is
**path-literal provenance** — `189 − 147 = 42`, and
`/.claude/worktrees/agent-a6fcda7e331521e94` is exactly 42 bytes, so the array
extent is this worktree's own `__FILE__` — and the `source-effects` delta is
**package identity**, the provider packages rebuilt at a different absolute
path. Headline counts reproduce exactly: **2 Safe / 42 provisional Unsafe /
137 Unknown / 4 data-only**, per library
`0/16/15`, `1/2/13` (+1 data-only), `0/10/81` (+3), `0/2/11`, `0/1/0`, `1/11/17`.

**AFTER** (the committed tree):

> **All five artifacts are byte-identical to the BEFORE run.** Zero unit verdict
> moves, zero function status moves, zero diagnostic leaf moves. Headline
> unchanged at 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only.

There is therefore **no `Unknown`→`Safe` movement to defend, and nothing to
re-pin**: the wave-15 baseline is left exactly as it stands. The comparison is
archived as [`artifacts/`](artifacts/).

### 6.3 Why reach is zero — the toggled-rule probe

A null result is only informative if the rule could have fired. One further
corpus run measures that directly, in the pattern card BORROW-RESIDUALS used: the
**tracked-value conjunct alone** was disabled — the `Alias` arm made to fire on
*any* store into a recognised static-storage destination — and the corpus
re-measured.

The probe is **not** committed — it was applied to a clean tree, measured, and
reverted, and the tree was verified clean afterwards.

**Result: 6 units move, all in librdkafka, and all at real static-storage
destinations.**

| Unit | Destination named by the probe | Leaves −/+ |
|---|---|---|
| `librdkafka/cJSON.c` | `global_error` | 1 / 1 |
| `librdkafka/lz4frame.c` | `codeError` | 2 / 1 |
| `librdkafka/rdkafka_sasl_oauthbearer.c` | `jose_header_encoded` (+ others) | 13 / 13 |
| `librdkafka/rdkafka_sasl_oauthbearer_oidc.c` | `scope` | 4 / 4 |
| `librdkafka/rdkafka_transport.c` | `rd_kafka_curr_transport` | 5 / 5 |
| `librdkafka/rdports.c` | `rd_qsort_r_arg` | 1 / 1 |

So the destination scan is **not vacuous on the pinned corpus**: it recognises
real file-scope objects and `static` locals, and real bodies really do store
into them at sites the projection visits. Reach is zero because of the **value**
conjunct — none of those stores publishes an identity the projection holds at
that point. That is the honest reading of the null result: the rule is live and
the corpus does not contain the hazard in this shape.

Two further observations from the probe, recorded because they bound what the
probe shows: even with the conjunct removed the **headline does not move**
(2 / 42 / 137 / 4, five `Unknown` → `Unknown` reason-count changes and zero
function status moves), and the movement is not purely additive — 26 leaves
removed against 25 added — because the refusal poisons the path earlier than the
diagnostics it displaces. Neither is a reason to widen the rule: a store of an
*untracked* value publishes nothing, and refusing it would be refusal without a
property.

## 7. CT candidate rows — recorded, not mapped

`docs/reevaluation/ct_manifest.json` is **untouched** by this card and CT
coverage stays **32/50**. The measurements the next CT lane needs are recorded
here instead.

### CT40 — "Atomic pointer points to prematurely freed memory"

Expected: *"Not certified; unsupported lifetime/publication protocol"*. Wave 9
left it unmapped because publish-then-free was `Safe` with no diagnostic, so the
verdict was neither the demanded class nor attributable to anything. Both halves
now hold. Measured on a candidate twin using CT43's own `q_*` contract surface:

```c
void *q_alloc(void);
void q_free(void *p);

void *_Atomic g_published;

void subject(void) { void *p = q_alloc(); g_published = p; q_free(p); }
```

| Row | Measured |
|---|---|
| `route` | `legacy_c` |
| `harness_verdict_mode` | `legacy_c_twin` |
| `semantic_profile` | `UnrestrictedC11ResourceProjection` |
| unit verdict | `unknown` |
| reason (verbatim) | `resource projection unresolved: 1 unsupported operation(s), 1 unresolved alias(es), 0 custody summary conflict(s)` |
| `function_status` | `{ "subject": "unknown" }` |
| diagnostics (complete, ordered) | `subject:4:unknown_alias:p`, ``subject:4:unsupported:publication of a tracked resource into the static-storage object `g_published` is outside the resource projection`` |
| control — publication statement removed | `safe`, no diagnostic, `{ "subject": "safe" }` |
| stages | `parse` executed (1, `tool_gate: ["clang"]`), `legacy_c_twin` executed (1, `tool_gate: ["clang"]`), every other stage `not_applicable` under the wave-9 route rule, `proof` `unassessed` |

The control removes exactly the publication statement and moves the verdict, so
the wave-9 attribution rule's clause (b) is satisfied **for the publication**.
One measurement the lane must weigh rather than inherit: the identical body with
a plain `void *g_published;` yields a **byte-identical verdict**, so `_Atomic` is
*not* load-bearing. The scenario's title names an atomic pointer; its `expected`
text names the publication protocol, and that is the construct the attribution
now rests on. This card does not decide whether that satisfies the lane's rule.

### CT41 — "`volatile` used as a completion flag"

**Stays unmapped, and the recorded blocker is now a decision rather than a
gap.** §5 asserts the invariance as correct: the gateway derives no
synchronization fact, so no verdict of its is evidence about one, and a mapping
would be vacuous under clause (b). Suggested blocker text for the lane:
*"the resource projection has no synchronization model; the `volatile`
invariance is asserted as correct by `CTWIN-PUBLISH` §5 rather than open"*.

### CT42 — unchanged, out of scope (`CT-CTWIN-DESCRIPTOR`).

## 8. Lean

`proof/QCore/PublicationCustody.lean`, audited by
`proof/AuditPublicationCustody.lean`: **30 theorems**, depending only on
`propext` and `Quot.sound`, **no new axiom**, no placeholder. `check_lean_audits.py`
count moves 67 → 68.

Covered: the decision positively (alias, acquisition, custody-yielding call,
`static` local, certificate withdrawal); **why refusal and not an escape**, with
the promotion exhibited concretely and stated existentially; every way it fails
closed (null store, opaque value, no-custody result, unbound source, caller-held
identity, formal, shadowing declaration, unrecognised name); what the decision
reads (the four name sets, and no per-site status); monotonicity of the scan and
of the destination set; the three load-bearing conjuncts; the published verdict
class; and non-vacuity.

Premises, not theorems, exactly as in `QCore.BorrowedActual` and
`QCore.BorrowLiveness`: that the readers extract file-scope declarators, body
declarations, formal names and flow-sensitive bindings faithfully from C source.

## 9. Tests

| Family | Count |
|---|---|
| `crates/qcore-c-gateway/tests/unit/publication.rs` — the scan, over token vectors | 13 |
| `crates/qcore-c-gateway/tests/publication_custody.rs` — the Clang-backed held-out family | 21 |

The acceptance family records each body's **measured** base answer in its
doc comment, asserts the **complete** ordered diagnostic vector (never a
convenient subset), and rechecks the certificate of every admitted control
through `check_legacy_safety_certificate`, so the rule reproduces from the
certificate's own source.

## 10. One deliberate pin update, and the standing gate

**Pin update.** `docs/reviews/r0_3/acceptance_matrix.json`'s `validation`
section carries two **derived** fields — `lean_audits` and `lean_audit_files`,
whose declared source is `proof/Audit*.lean on disk` — and
`scripts/tests/test_build_matrix_sections.py::test_the_derived_facts_are_recomputed_rather_than_carried`
pins that the committed values equal the derived ones. Adding an audit module
therefore *requires* moving them: `67` → **68**, with
`AuditPublicationCustody.lean` inserted in the sorted file list. That is the
entire diff to that file — two lines — and **no** hand-maintained section,
`named_followups` entry or CT number was touched; those stay for the integrator
(§12). The test's negative half is intact: it still compares the produced values
to the live sources on disk, never to the committed file, so it cannot be
comparing a value to itself.

| Gate | Result |
|---|---|
| `git submodule update --init --recursive` | ok (`third_party/libevent` at `d82464a2`) |
| `cargo test --workspace` | **2672 passed, 0 failed, 8 ignored** |
| `cargo fmt --all -- --check` | clean (both workspaces) |
| `cargo clippy --all-targets -- -D warnings` | clean (both workspaces) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | **68 Lean audits**, all pass |
| `python3 -B -m unittest discover -s scripts/tests` | 146 tests, OK |
| `cargo test` in `providers/` | 108 passed, 0 failed |
| pinned 185-unit corpus (canonical route) | exit 0, zero gateway failures, **all five artifacts byte-identical** |

## 11. What this does not establish

* **This is one destination shape.** A store into a file-scope *aggregate*, a
  store *through* a file-scope pointer, an array element and a
  `(`-carrying declarator remain refused or unrecognised by other, pre-existing
  rules; none of them is *modeled*.
* **Publication is refused, not understood.** No lifetime or publication
  protocol is analyzed. A body that publishes is `Unknown` because the
  projection declines, not because anything about the published object was
  decided.
* **Borrowed publication is untouched** (`CTWIN-PUBLISH-BORROWED`).
* **Nothing is promoted, and nothing moved.** The corpus artifacts are
  byte-identical; no unit, function or diagnostic changed.
* The `b_*` and `q_*` contracts in every fixture here are **trusted synthetic
  test assumptions**, not facts about any library.

## 12. Named follow-ons

| Id | What |
|---|---|
| `CTWIN-PUBLISH-BORROWED` | Publishing a **caller-held** (borrowed) pointer into a static-storage object is still admitted. Deciding it needs a lifetime property this projection does not have. |
| `CTWIN-PUBLISH-SCAN` | A file-scope declarator the reader cannot read — a function-pointer object, an `_Atomic(…)` parenthesised specifier, a K&R parameter run after the first — is not recognised, so that name keeps its pre-existing modelling. Monotone: a better scanner only adds refusals. |
| `CTWIN-PUBLISH-INDIRECT` | The already-refused indirect destinations (`g.f = p`, `g->f = p`, `g[i] = p`) are refused by the lexical gate, not modeled. Modelling them is the aggregate-store analogue of this card, and would need O04-B08's field-link relation extended to objects the body does not own. |
| `CTWIN-PUBLISH-OUT` | An `Out`-position actual spelled `&g` is not a place the projection names, so a callee writing a resource into a file-scope object through an out slot is invisible on a different path from the one this card closed. |
| `CT40-CANDIDATE` | The measured schema-2 rows in §7, for the next CT lane. `_Atomic` is measured non-load-bearing; the lane must decide whether publication attribution satisfies the scenario. |

## 13. Integrator carry list

New: `crates/qcore-c-gateway/src/publication.rs`,
`crates/qcore-c-gateway/tests/unit/publication.rs`,
`crates/qcore-c-gateway/tests/publication_custody.rs`,
`proof/QCore/PublicationCustody.lean`, `proof/AuditPublicationCustody.lean`,
this report and its `artifacts/`.
Changed: `crates/qcore-c-gateway/src/{legacy_c.rs,legacy_recursion.rs,lib.rs,typed_field_access.rs}`
(the last only widens two declaration-reader helpers to `pub(crate)`),
`crates/qcore-c-gateway/tests/unit/legacy_c.rs` (one hand-built CFG gains the
new field), `proof/QCore.lean` (one import), and
`docs/reviews/r0_3/acceptance_matrix.json` — **two lines only**, the derived
`validation.lean_audits` count and its sorted file list (§10). No
hand-maintained section of that file, and no CT number, was touched.

The integrator must:

1. **Not re-pin the corpus.** The five artifacts are byte-identical; the
   wave-15 baseline stands.
2. **Update `named_followups`** in `docs/reviews/r0_3/acceptance_matrix.json`:
   `CT-CTWIN-PUBLISH` is **closed** and `CT-CTWIN-SYNC` is **decided** (§5);
   `CT-CTWIN-DESCRIPTOR`, `CT-CTWIN-UNRESOLVED-LEAK` and `CT-CTWIN-ENTRY` are
   unchanged; `CTWIN-PUBLISH-BORROWED`, `CTWIN-PUBLISH-SCAN`,
   `CTWIN-PUBLISH-INDIRECT`, `CTWIN-PUBLISH-OUT` and `CT40-CANDIDATE` are new
   and open.
3. **Carry the gate line**: 2672 workspace tests (8 ignored), 108 provider
   tests, 146 Python tests, Lean audits 67 → **68**, both probes exit 0, fmt and
   clippy clean in both workspaces. 34 new tests; CT coverage **unchanged at
   32/50**; contract corpus **unchanged at 67**; `qcore-schemas` unchanged at
   12.0.0. **One version judgement recorded rather than made silently:**
   `LegacyCfg` is `pub` in `qcore_c_gateway::legacy_c` and re-exported by
   `qcore-compiler`, so the new `publications` field is a source-incompatible
   addition for any out-of-workspace consumer that constructs one literally.
   Neither crate's version was moved — `qcore-c-gateway` is 0.1.0 and
   `qcore-compiler` 0.4.0 — because no such consumer exists and the only literal
   construction in the tree is one hand-built test CFG, which this card updated.
   The integrator may decide otherwise.

---

## Increment paragraph (draft, for `DEVELOPMENT.md`)

> Publication custody (CTWIN-PUBLISH, 2026-09-10): the wave-9 `CT-CTWIN-PUBLISH`
> hazard is reproduced and closed — a store of an owned pointer into a
> file-scope object was `Safe` with no diagnostic (and so was publishing an
> already-released one), because the importer modelled the file-scope object as
> one more local name. The repair refuses rather than escapes, and the brief's
> escape proposal is **disproved by measurement**: escape discharges the
> residual, so the publish-without-release body — measured `Unsafe` at the base —
> would have been promoted to `Safe`. Both directions are withdrawn instead, with
> the leak claim retired through the existing unresolved-effect mechanism, which
> is also exactly CT40's "not certified; unsupported lifetime/publication
> protocol". Destinations are file-scope objects the body does not shadow plus
> its own `static` locals (the latter added by measurement, not in the brief);
> the value half fires only on an identity the body itself holds, so borrowed
> publication, null stores and scalar stores stay admitted, each argued. Corpus
> reach is **zero** — all five artifacts byte-identical, no re-pin — with a
> toggled-rule probe measuring which conjunct makes it so. `CT-CTWIN-SYNC` is
> converted from an omission into an asserted decision: C11 `volatile` is not a
> synchronization primitive, so a projection with no synchronization model must
> answer identically, pinned on the whole verdict in both directions. 30 audited
> Lean theorems (`QCore.PublicationCustody`), 34 new tests, CT candidate rows for
> CT40 recorded without touching the manifest.
