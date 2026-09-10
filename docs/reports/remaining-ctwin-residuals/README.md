# CTWIN-RESIDUALS — the four publication shapes wave 16 left open

Date: 2026-09-10. Base: main `8ec8402` (wave-16 consolidation; corpus baseline
`docs/reports/wave16-corpus/artifacts/`). Branch:
`claude/wave-16-hatch-opening-43bc0d` (isolated worktree). Wave 17, card
`CTWIN-RESIDUALS`, kind gateway/soundness, scope L2.

Card `CTWIN-PUBLISH`
([`docs/reports/remaining-ctwin-publish/README.md`](../remaining-ctwin-publish/README.md))
closed the direct publish-then-free false `Safe` and named four residuals:
`CTWIN-PUBLISH-BORROWED`, `-INDIRECT`, `-OUT` and `-SCAN`. This card measures
all four, and lands the three that carried a live hazard.

**Headline, and one correction to the brief.** The brief directed
soundness order *"INDIRECT is the likeliest live launder — measure first."*
**Measurement refutes that.** INDIRECT is the one shape with **no** false
`Safe`: all nine indirect shapes probed were already refused at the base. The
live false `Safe`s were the other three — and the sharpest of them,
`CTWIN-PUBLISH-BORROWED`, is not a lifetime question at all, as wave 16
supposed, but a **custody launder across the summary boundary**: a helper that
publishes a borrowed formal certified, so W4 published `Borrowed` for it, and a
caller that acquired, called the helper and released certified too — the
wave-16 headline body with one level of indirection.

All four shapes are now closed: three by withdrawal, and INDIRECT by
withdrawing the residual `violation` claim it kept. **Nothing is promoted**:
raw Cycle 19 units stay 2 Safe / 42 provisional Unsafe / 137 Unknown plus four
data-only, the contract corpus stays 67 modules, CT coverage stays 32/50, and
the wave-16 baseline is **not** re-pinned.

---

## 1. What was measured at the base

Through the real gateway entry points (`analyze_unrestricted_c11`) over card
C02-P's synthetic `b_*` and card BORROW-RESIDUALS' `y_*` surfaces, at
`8ec8402`, before any line of this card:

### 1.1 Three live false `Safe`s

| # | Shape | Body (elided) | Base | After |
|---|---|---|---|---|
| S | `-SCAN` | `_Atomic(struct b_handle *) g; … g = h; b_close(h);` | **`Safe`, no diagnostics** | `Unknown`, `[publication:g, unknown_alias:h]` |
| B | `-BORROWED` | `static void publish(struct b_handle *q) { g = q; }` `… h = b_open(…); publish(h); b_close(h);` | **`Safe`, both functions** | `Unknown` for both |
| O | `-OUT` | `void *g; … y_fill(&g); y_release(g);` | **`Safe`, no diagnostics** | `Unknown`, `[publication:g, unknown_alias:g]` |

Row S is CT40's **literal** spelling: the scenario is titled "atomic pointer
points to prematurely freed memory", and `_Atomic(void *) g;` is exactly the
spelling wave 16's `CT40-CANDIDATE` measurement did **not** use — it used the
postfix `void *_Atomic g;`, which was already refused. The two spellings
declare the same object and answered differently.

Row O is the same publish-then-free hazard with the acquisition performed by
the callee through a declared `Out` slot. The unreleased form
(`y_fill(&g_published);` alone) was `Unsafe` with a `Leak`, which the
projection is in no position to claim.

### 1.2 One shape with no false `Safe`, and a claim it should not make

`CTWIN-PUBLISH-INDIRECT`, all nine probed shapes at the base:

| Shape | Base | Cause |
|---|---|---|
| `g_reg.cio = h` | `Unknown` + `Unsafe` `[leak]` when unreleased | indirect memory access |
| `g_reg->cio = h` | same | indirect memory access |
| `g_slots[0] = h` | same | indirect memory access |
| `static struct r s; s.h = h` | same | indirect memory access |
| `static T *c[4]; c[0] = h` | same | indirect memory access |
| `T **p = &g; *p = h` | same | unsupported assignment expression + store through `*p` |
| `g = (T *)h` (cast) | `Unknown` | unsupported assignment expression |
| `g = n ? h : 0` | `Unknown` | unsupported assignment expression |
| `loc.h = h; g_h = loc` | `Unknown` | indirect memory access |

**No `Safe` anywhere.** The residual was not a launder; it was an
*inconsistency*: the same body that answers `Unknown` when it publishes
directly answered provisional `Unsafe` with a `Leak` when it published through
a member — the very claim card `CTWIN-PUBLISH` §3 withdrew for the direct
shape, on the ground that the identity may well be released through the
published object.

## 2. The decisions, and the arguments

Every direction is argued; refusal is the default. No diagnostic code was added
or changed: each refusal is an existing `Unsupported`, published as `Unknown`.

### 2.1 `CTWIN-PUBLISH-BORROWED` — **withdrawn**

Wave 16 admitted a borrowed publication with the argument that *"publishing a
borrowed pointer is a lifetime hazard, not an acquire/release accounting
error, and this projection's documented property is one translation unit's
acquire/release accounting."* Measurement shows the second half is false
**within one translation unit**:

```c
struct b_handle *g_published;
static void publish(struct b_handle *q) { g_published = q; }
void w(int n) { struct b_handle *h = b_open(0,0,n); if (!h) return;
                publish(h); b_close(h); }
```

Both functions were `Safe`. Inline `publish` and you get the wave-16 headline
body, which is `Unknown`. **A rule that refuses a body inlined must refuse it
composed**, or the refusal is defeated by extracting a helper.

The mechanism is the summary, not the lifetime: W4 published `Borrowed(q)`,
and `Borrowed` asserts *the caller's obligations are unchanged across this
call*. A publication falsifies exactly that — after the call, releasing leaves
a dangling reference in an object that outlives everything, and not releasing
is no longer detectable as a leak.

**Why not a faithful mode instead of a refusal.** The two modes that would
describe the body — `Escaped`, `Consumed` — are card `CTWIN-PUBLISH` §3's
escape model at the summary boundary, and they promote in the mirror
direction: under `escaped`, the caller that does **not** release certifies.
Both directions are proved (`publishing_borrowed_launders_the_callers_release`,
`an_escaping_summary_promotes_the_unreleased_caller`), so the only fail-closed
answer is withdrawal, exactly as wave 16 concluded for the direct shape.

The rule therefore reads **no provenance**, matching the status-independence
wave 16 already proved: a body-acquired identity and a caller-held one are
refused alike (`the_rule_reads_no_provenance`, `the_rule_reads_no_status`,
whose caller-held hypothesis this card deletes).

**One exclusion added by measurement, not in the brief.** A candidate entry
custody binds **every** declared formal to a resource site, scalar parameters
included, so a provenance-free rule refused `g_count = n;` — a body card
`CTWIN-PUBLISH` measured `Safe` and pinned. That is a modelling artefact, not
a published identity, so a source the function's own declarations read as a
declared scalar is excluded. The exclusion is positive and fails closed: a
function whose declarations the type reader could not read contributes no
scalar name, and the store is refused.

**Two theorems retired by name**, because the admission they pinned was
measured unsound: `a_caller_held_identity_is_never_refused` and
`the_caller_site_exclusion_is_load_bearing`. The replacements are
`a_caller_held_identity_is_now_refused`, the two launder theorems above, and
`the_scalar_exclusion_is_load_bearing`.

### 2.2 `CTWIN-PUBLISH-OUT` — **withdrawn**

A declared `Out` slot whose actual *is* a publication destination acquires the
resource directly into the object. That is card `CTWIN-PUBLISH`'s own
`Acquire` arm — *"the acquisition is published at the moment it is created;
refused before the site exists"* — seen from the callee's side, so it takes
the same answer. Refused before the call's effects apply, so no site exists to
carry forward and the unreleased form's `Leak` is withdrawn rather than
retained: a downgrade, never a promotion.

**A latent false `Unsafe` closed along the way.** An early return skips the
call's declared effects, so a `Released` argument of the same call would stay
live to the exit and be reported as a leak the call was going to discharge.
The refusal therefore marks an unresolved effect on exactly the identities the
call's own arguments denote — an unrelated resource's residual is untouched.

The destination half is load-bearing: an `Out` slot naming an ordinary local
is untouched and still certifies.

### 2.3 `CTWIN-PUBLISH-INDIRECT` — **residual claim withdrawn**

No launder exists to close, so what is repaired is the inconsistency of §1.2:
a store whose destination is a *continued access* on a publication destination
(`g.f`, `g->f`, `g[i]`, and the `static`-local analogues) now withdraws the
residual claim on the identities the statement's own operands denote, and
names the publication.

Three properties keep this a downgrade and nothing more:

* **The statement keeps the refusal it already had.** The attribution stays
  with the gate that read it — `indirect memory access is outside the resource
  projection`, `store through …` — and this card's diagnostic is *added*, so
  the withdrawn residual is auditable rather than silent. Wave 16's
  `indirect_file_scope_destinations_keep_their_pre_existing_refusal` asserted
  those shapes gained **no** diagnostic of its card; that pin is deliberately
  updated, with its load-bearing half — the pre-existing refusal survives —
  kept and asserted.
* **Nothing here can certify.** The statement is an unmodeled instruction
  before and after, so its verdict was never `safe` and no input makes one
  (`the_indirect_record_never_certifies`).
* **Only the statement's own operands are marked.** A local base keeps its
  claim, and an untracked value withdraws nothing.

**Recorded, not repaired:** `T **p = &g; *p = h;` keeps its pre-existing
`Unsafe` `[leak]`, because the base this scan reads is the local `p`. Deciding
it needs a pointer relation the projection does not have. Named
`CTWIN-PUBLISH-INDIRECT-POINTER` and pinned as measured.

### 2.4 `CTWIN-PUBLISH-SCAN` — **narrowed, monotonicity preserved**

`_Atomic ( type-name )` is a *specifier*, and card `CTWIN-PUBLISH`'s
`(`-carrying skip — which exists so that no prototype's parameter name enters
the destination set — swallowed the declarator with it. The specifier is now
dropped before that test.

The extension is monotone in the sense the card requires, and it is asserted
rather than argued: `the_specifier_drop_only_adds_names` (every name the old
reader recognised, the new one recognises),
`a_declarator_parenthesis_still_skips_the_group` (the drop never reaches a
declarator parenthesis, so the original reason for the skip is preserved
exactly), and, in Rust, a per-spelling comparison over four declarations
plus three refusal shapes.

`_Atomic` is the only C11 specifier spelled `KEYWORD ( type-name )` (§6.7.2.4),
so the table has one entry. Declarations the reader still cannot read — a
function-pointer object, a K&R parameter run after the first — keep their
pre-existing modelling and `CTWIN-PUBLISH-SCAN` stays open for them, still
monotone.

## 3. Probe results: which conjuncts are live

Method: card `CTWIN-PUBLISH` §6.3's toggled-rule probe, and card
BORROW-RESIDUALS' before it. **The probes are not committed.**

### 3.1 `-SCAN`: reach is zero, measured statically

`_Atomic` does not occur **once** in any of the six audited scopes:

```
grep -rl _Atomic <clone>/lib/{cmetrics,ctraces,librdkafka-2.15.0,chunkio,monkey} \
                 <clone>/src/flb_input_chunk.c      ->  (no output)
```

The three occurrences anywhere in the checkout are in `jemalloc`, `nghttp2`'s
test harness and `wasm-micro-runtime` — none of them audited. So the
parenthesised-specifier extension recognises no additional destination on this
corpus, and its reach is **zero by measurement, not by assumption**. Its
non-vacuity is carried by the held-out fixture family instead, where the
shape's base answer was `Safe` with no diagnostic.

### 3.2 `-INDIRECT`: the value conjunct, disabled — a real unit moves

The indirect record's value test was relaxed from *"a source denotes at least
one site and is not a declared scalar"* to *"a source name is present in the
binding map"*, and the whole 185-unit corpus re-measured through the canonical
route.

**Result: one unit moves, `librdkafka/crc32c.c`, +2 / −0 leaves.**

| Unit | Function | Lines | Destination named |
|---|---|---|---|
| `librdkafka/crc32c.c` | `crc32c_init_sw` | 2892, 2898 | `crc32c_table` |

That is upstream's

```c
static uint32_t crc32c_table[8][256];
static void crc32c_init_sw(void) { … crc32c_table[0][n] = crc; …
                                   … crc32c_table[k][n] = crc; … }
```

Two things follow, and the second is the more important:

* **The destination scan is not vacuous on the pinned corpus.** It recognises a
  real file-scope object, and a real body really does store into a continued
  access on it, at a site the projection visits. The rule is live.
* **The corpus told the card its rule was too loose.** Every operand there is a
  declared scalar; nothing is published. Presence in the binding map is true of
  any assigned name, so the relaxed test was *refusal without a property* —
  exactly what card `CTWIN-PUBLISH` §4.2 declined to do for an untracked value.
  The value test was therefore tightened to the direct arm's, and the reduced
  shape is pinned in both directions by
  `a_scalar_table_fill_is_not_an_indirect_publication` and
  `a_tracked_source_into_the_same_shape_still_refuses`. This was found by
  measurement, not by review.

The probe run's artifacts are kept as
[`artifacts/probe-indirect-presence-only/`](artifacts/probe-indirect-presence-only/)
so the +2 leaves are inspectable rather than merely reported.

### 3.3 `-BORROWED` and `-OUT`: reach is zero, and the shipped diff says so

Neither withdrawal moves a leaf on the pinned corpus (§4). Both are live in the
held-out family, and both have a measured base answer of `Safe` with no
diagnostic — a false `Safe`, not an absence of opportunity. The honest reading:
the pinned corpus does not contain a publishing helper or an `Out` slot naming
a static-storage object at a site the projection reaches, and the wave-16
destination-scan probe already showed the destination half is not vacuous
there.

## 4. Corpus measurement

### 4.1 Provenance

Pinned Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97` and forced-Libevent
Monkey prepared fresh in this worktree by `scripts/prepare_cycle19_clang_audit.py`
(explicit `/usr/bin/clang`, Ubuntu clang 21.1.8); `third_party/libevent` at the
pinned `d82464a2`; all five provider packages rebuilt into
`build/cycle19/provider-packages-ctwin`. Card CORPUS-REPIN's recorded deviation
2 is reproduced: the prepared CMake build lives outside the clone, so its
`compile_commands.json` was copied verbatim to
`<clone>/build/compile_commands.json`, where the pinned route reads it.

```sh
QCORE_CLANG=/usr/bin/clang QCORE_CHUNKIO_PACKAGE=<packages>/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone> <monkey>/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

Exit 0, **zero gateway failures in every library**, both runs.

> **One process note, recorded because it cost a run.** The first AFTER attempt
> ran the whole 185-unit corpus and then refused at the Cycle 18 Libevent pin,
> reporting the *superproject's* HEAD as the submodule revision, because
> `git submodule update --init --recursive` had not been run in this worktree.
> That is the standing gate's first line behaving exactly as designed — a
> fail-closed pin refusing rather than analysing an absent submodule — and it is
> a worktree setup step, not a defect.

### 4.2 Headline: nothing moved

**185 units: 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only —
unchanged.** Zero unit status moves, zero function status moves across all
4,834 projected functions, zero unit `reason_code` moves, no newly surfaced
provisional Unsafe, **zero `Unknown`→`Safe` promotions**, and leak unchanged at
271.

Per library, each matching the wave-16 baseline exactly: cmetrics `0/16/15`,
ctraces `1/2/13` (+1 data-only), librdkafka `0/10/81` (+3), chunkio `0/2/11`,
fluent-bit-chunk `0/1/0`, monkey-libevent `1/11/17`.

### 4.3 The diagnostics artifact is byte-identical to the pin

| Artifact | vs `docs/reports/wave16-corpus/artifacts/` |
|---|---|
| `resource-diagnostics.json` | **byte-identical** (`46d0f17845ca…`) |
| `resource-diagnostics.chunk-contracts.json` | **byte-identical** (`c4f438e4ed3d…`) |
| `resource-diagnostics.contracts.json` | differs — path-literal provenance |
| `monkey-source.json` | differs — path-literal provenance |
| `resource-diagnostics.source-effects.json` | differs — package/task identity |

The one artifact that carries a verdict, a function status or a diagnostic is
**byte-identical to the pinned baseline**. That is a stronger statement than
"the before and after runs agree": the shipped tree reproduces the pin exactly.
So there is **nothing to re-pin**, and the wave-16 baseline is left as it
stands.

### 4.4 The three residues, attributed leaf for leaf

Each is one of the two classes waves 9, 15 and 16 already record, and each is
attributed **arithmetically or structurally**, not by assertion:

* **`contracts.json` — 24 leaves, 12 removed + 12 added, path-literal
  provenance.** The same twelve contract modules, keyed by absolute path;
  only the worktree prefix differs. No module added, removed or changed in
  content, so the **contract corpus stays at 67 modules**.
* **`monkey-source.json` — 17 leaves.** Three are the compile entry's
  `command`, `directory` and `file`. The other fourteen are seven
  `c_type`/`value` pairs where `char[150]` became `char[189]` — the `__FILE__`
  expansion. The arithmetic closes exactly: the baseline's recorded input path
  is 140 bytes, this worktree's is 179, and **179 − 140 = 39 = 189 − 150**.
* **`source-effects.json` — 45 leaves, all identity.** `package_digest`,
  `build_digest`, `configuration_digest`, `obligation_id`, `task_digest`, one
  `preprocessed_source_digest`, and the `bounds` map re-keyed by the new
  obligation id. **No result's verdict moved**: the split is
  **3 checked function contracts / 93 Unknown** before and after, and every
  per-result `(status, function)` pair is unchanged.

### 4.5 BEFORE, measured — the residues are not this card's

The card's three changed source files
(`publication.rs`, `legacy_c.rs`, `legacy_recursion.rs`) were checked out at
base `8ec8402` and the whole corpus re-measured through the same route, from the
same prepared clone and the same rebuilt packages.

| Comparison | Result |
|---|---|
| BEFORE vs the pinned wave-16 baseline | `diagnostics.json` and `chunk-contracts.json` **byte-identical**; the same three residues, byte-for-byte |
| **BEFORE vs AFTER** | **all five artifacts byte-identical** |

So the three residues are **measured** environmental — they appear with this
card's source changes reverted — and the card's own effect on the pinned corpus
is **exactly nothing**, on every artifact, not only the one that carries
verdicts.

Archived: [`census-baseline-vs-before.txt`](census-baseline-vs-before.txt),
[`census-baseline-vs-after.txt`](census-baseline-vs-after.txt),
[`census-before-vs-after.txt`](census-before-vs-after.txt),
[`census.py`](census.py), [`corpus-run.log.gz`](corpus-run.log.gz).

## 5. Tests

| Family | Before | After |
|---|---|---|
| `crates/qcore-c-gateway/tests/unit/publication.rs` — the readers, over token vectors | 13 | **19** |
| `crates/qcore-c-gateway/tests/publication_custody.rs` — the Clang-backed held-out family | 21 | **35** |

Twenty new tests. Every acceptance body records its **measured** base answer in
a `BASE:` doc line, asserts the **complete** ordered diagnostic vector, and — for
admitted controls — rechecks the certificate through
`check_legacy_safety_certificate`, so the rule reproduces from the certificate's
own source. Every landed shape has both directions: the refusal and the control
that keeps it from being a blanket refusal (an ordinary-local `Out` slot, a
non-publishing helper, a local aggregate base, a compound assignment, a
comparison).

Two spellings that matter for the unit family: this gateway's lexer emits every
punctuator as its own token, so `->`, `+=` and `==` are two adjacent tokens
there, and the tests use exactly that spelling.

## 6. Lean

`proof/QCore/PublicationCustody.lean`, audited by
`proof/AuditPublicationCustody.lean`: **50 theorems** (30 before this card),
depending only on `propext` and `Quot.sound`, **no new axiom**, no placeholder.
The audit **count is unchanged** — this card extends the existing module rather
than adding one.

Covered by the twenty new theorems: the provenance independence and the
now-refused caller-held identity; the launder and its mirror at the *summary*
boundary, with both promotions exhibited concretely
(`publishing_borrowed_launders_the_callers_release`,
`an_escaping_summary_promotes_the_unreleased_caller`) and the implemented
answer's two directions (`a_refused_helper_certifies_no_caller`,
`a_non_publishing_helper_still_certifies_its_caller`); the `Out` arm, its
static-local case, its ordinary-local control and its never-a-violation
property; the indirect record's withdrawal, its local-base control, its
untracked-value no-op and the proof that it can never certify on any input; and
the specifier drop's monotonicity, its preservation of the declarator skip, its
non-vacuity and its spelling-independence.

**Two theorems are retired by name** — `a_caller_held_identity_is_never_refused`
and `the_caller_site_exclusion_is_load_bearing` — because the admission they
pinned was measured unsound. `the_rule_reads_no_status` is **strengthened**
rather than retired: its caller-held hypothesis is deleted, so the conclusion
now holds for every caller-held predicate.

That the reader splits a declarator group's *specifier* parenthesis from its
*declarator* parenthesis faithfully is a premise, exactly as every other
reader's extraction already is in this module.

## 6.1 The standing gate

| Gate | Result |
|---|---|
| `git submodule update --init --recursive` | ok (`third_party/libevent` at `d82464a2`) — see §4.1 |
| `cargo test --workspace --no-fail-fast` | **2764 passed, 3 failed, 8 ignored**; all three are the recorded load-flakes and **each passes in isolation** (below) |
| `cargo fmt --all -- --check` | clean, both workspaces |
| `cargo clippy --all-targets -- -D warnings` | clean, both workspaces |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | **72 Lean audits**, all pass |
| `python3 -B -m unittest discover -s scripts/tests` | 161 tests, OK |
| `cargo test` in `providers/` | **115 passed, 0 failed** |
| `providers/package.py build` + `verify` × 5 | all five packages rebuild and verify |
| pinned 185-unit corpus | exit 0, zero gateway failures, **all five artifacts byte-identical to the BEFORE run** |

The three load-flakes, re-run alone on the same tree:

| Test | In the full run | In isolation |
|---|---|---|
| `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` | FAILED | **ok** (`1 passed`) |
| `a_confined_fixture_cannot_read_outside_the_admitted_paths` | FAILED | **ok** |
| `a_confined_fixture_cannot_write_outside_the_admitted_paths` | FAILED | **ok** |

Both are the flakes the wave brief names (`P02-FLAKE` and the confined-fixture
pair); no assertion was weakened, and this card touches neither the verifier
host nor the confinement profile. The whole
`qcore-verifier-host::confined_process_execution` target passes 12/12 alone.

## 7. What this does not establish

* **Publication is refused, not understood.** No lifetime or publication
  protocol is analyzed. A body that publishes is `Unknown` because the
  projection declines.
* **The borrowed withdrawal is a refusal, not a lifetime decision.** Whether a
  particular borrowed publication is *safe* is still undecided; what is decided
  is that no summary may assert the caller's obligations are unchanged across
  it.
* **The out rule reads the declared contract's `Out` positions**, which are
  trusted declarations, not facts about any callee's body.
* **`_Atomic` carries no synchronization meaning here.** Card
  `CTWIN-PUBLISH` §5's decision is unchanged: this projection derives no
  synchronization fact, and recognising the parenthesised spelling is a
  *declarator* repair, not a memory-model one.
* The `b_*` and `y_*` contracts in every fixture are **trusted synthetic test
  assumptions**, not facts about any library.

## 8. Named follow-ons

| Id | Status | What |
|---|---|---|
| `CTWIN-PUBLISH-BORROWED` | **closed (withdrawn)** | The admission is gone; the launder it enabled is measured and proved. |
| `CTWIN-PUBLISH-OUT` | **closed (withdrawn)** | A declared `Out` slot naming a publication destination is refused before the call's effects apply. |
| `CTWIN-PUBLISH-INDIRECT` | **closed at its measured boundary** | No launder existed; the residual `violation` claim is withdrawn and the publication named. The pointer-base case is split out below. |
| `CTWIN-PUBLISH-SCAN` | **narrowed, still open** | The parenthesised `_Atomic` specifier is now read. A function-pointer object and a K&R parameter run after the first still contribute nothing; monotone, so a better scanner only adds refusals. |
| `CTWIN-PUBLISH-INDIRECT-POINTER` | **new, open** | `T **p = &g; *p = h;` keeps its pre-existing `Unsafe` `[leak]`: the base this scan reads is the local `p`, and relating it to `g` needs a pointer relation the projection does not have. Pinned as measured. |
| `CTWIN-BORROWED-ATTRIBUTION` | **new, open** | When the publishing body's formal identity comes from a *derived* entry custody, the refusal fires inside the summary derivation, whose answer is then withheld — so the published diagnostic on that body is its unresolved identity, not the publication. The verdict is right and fail-closed; the attribution is imprecise. |
| `CT40-CANDIDATE` | **corrected** | Card `CTWIN-PUBLISH` recorded `_Atomic` as *not* load-bearing for CT40. That measurement used the **postfix** spelling `void *_Atomic g;`. The parenthesised spelling `_Atomic(void *) g;` — the one the scenario's title most directly names — **was** load-bearing at that base: it hid the destination and the body certified. Both spellings now answer alike; the lane should re-measure its rows on this tree. |

## 9. Integrator carry list

**New:** `docs/reports/remaining-ctwin-residuals/` (this report, `artifacts/`,
`census.py`).

**Changed:**
`crates/qcore-c-gateway/src/publication.rs` (the parenthesised-specifier drop,
the continued-access reader, the scalar-source set, and `PublicationFacts`),
`crates/qcore-c-gateway/src/legacy_c.rs` (the `Out` arm, the provenance
withdrawal, the indirect record in the `Unknown` arm, and the `LegacyCfg`
field's type),
`crates/qcore-c-gateway/src/legacy_recursion.rs` (the combine unions one record
instead of one set),
`crates/qcore-c-gateway/tests/{publication_custody.rs,unit/publication.rs,unit/legacy_c.rs,c03_neutral_frame_recursion_probe.rs}`,
`proof/QCore/PublicationCustody.lean` and `proof/AuditPublicationCustody.lean`.

**No** contract text, `borrowed_actual.rs`, CT manifest, acceptance file,
`AGENTS.md`, `CLAUDE.md` or `DEVELOPMENT.md` was touched. The Lean **audit
count does not move** — this card extends the existing
`AuditPublicationCustody.lean` rather than adding a module — so
`acceptance_matrix.json`'s derived `validation.lean_audits` needs no edit.

**Three pins updated deliberately, each keeping its load-bearing half:**

1. `publication_custody.rs::publishing_a_borrowed_pointer_is_not_newly_refused`
   — **retired by name**, replaced by
   `a_body_that_publishes_a_borrowed_formal_no_longer_certifies` and the
   composition test. The admission it pinned was measured to launder a
   caller's release.
2. `publication_custody.rs::indirect_file_scope_destinations_keep_their_pre_existing_refusal`
   — the expected vectors gain this card's diagnostic. Its load-bearing half —
   the pre-existing refusal survives and is not replaced — is unchanged and
   still asserted.
3. `c03_neutral_frame_recursion_probe.rs::assumed_neutral_refuses_every_body_that_carries_an_effect`
   — the `SummaryConflict` assertion for the store-a-formal-in-a-global body is
   replaced by "no summary is published". The refusal moved *earlier*: the store
   is now refused as a publication, so no `Owned` reading of the formal survives
   to conflict with anything. The two assertions that matter — `Unknown`, with
   an unresolved identity — are unchanged above it.

**One version judgement recorded rather than made silently.** `LegacyCfg` is
`pub` in `qcore_c_gateway::legacy_c` and re-exported by `qcore-compiler`; its
`publications` field changes type from `BTreeSet<String>` to the new `pub
PublicationFacts`. That is a source-incompatible change for any out-of-workspace
consumer that constructs or reads one. Neither crate's version was moved —
`qcore-c-gateway` is 0.1.0, `qcore-compiler` 0.4.0 — because no such consumer
exists and the only literal construction in the tree is one hand-built test CFG,
which this card updated. Card `CTWIN-PUBLISH` recorded the same judgement for
the field's addition. The integrator may decide otherwise.

---

## Increment paragraph (draft, for `DEVELOPMENT.md`)

> Publication residuals (CTWIN-RESIDUALS, 2026-09-10): the four shapes card
> CTWIN-PUBLISH left open are measured and closed, and the brief's soundness
> ordering is **refuted by measurement** — INDIRECT, named the likeliest
> launder, is the one shape with no false `Safe` at all (nine indirect shapes
> probed, all already refused), while three others each carried a live
> publish-then-free `Safe` with no diagnostic. The sharpest,
> `CTWIN-PUBLISH-BORROWED`, is not the lifetime question wave 16 supposed but a
> custody launder across the **summary** boundary: a helper publishing a
> borrowed formal certified, so W4 published `Borrowed` for it and a caller
> that acquired, called the helper and released certified too — the wave-16
> headline body with one call level of indirection. The exclusion is withdrawn,
> on the argument that `Borrowed` asserts the caller's obligations are
> unchanged and a publication falsifies exactly that, with both faithful
> alternatives proved to promote in the mirror direction; the rule now reads no
> provenance, and one exclusion (a declared scalar source) was added by
> measurement because a candidate entry custody binds every formal to a site.
> `CTWIN-PUBLISH-OUT` is withdrawn the same way — a declared `Out` slot naming
> such an object is the `Acquire` arm seen from the callee's side — and a
> latent false `Unsafe` is closed with it. `CTWIN-PUBLISH-INDIRECT` has no
> launder to close, so its residual `violation` claim is withdrawn instead
> (each statement keeping the refusal it already had) with the pointer-base
> case split out as a named residual. `CTWIN-PUBLISH-SCAN` narrows: `_Atomic (
> type-name )` is a specifier, not a declarator, and the parenthesised spelling
> — CT40's literal one — was hiding its destination, correcting CTWIN-PUBLISH's
> "`_Atomic` is not load-bearing" note, which had measured only the postfix
> spelling. Monotonicity is preserved and proved. Two theorems and one test are
> retired by name and one C03 pin moved earlier, each keeping its load-bearing
> half. `QCore.PublicationCustody` grows 30 → **50** audited theorems with no
> new axiom and no new audit module. Corpus reach is **zero**, measured
> twice: all five artifacts are byte-identical between the before and after runs,
> and `diagnostics.json` is byte-identical to the pinned wave-16 baseline, so
> there is nothing to re-pin. A toggled-conjunct probe names the one real unit
> that moves when the indirect rule's value test is relaxed —
> `librdkafka/crc32c.c`'s `crc32c_table[k][n] = crc;` — and that measurement is
> what caught the relaxed test as refusal-without-a-property and tightened it,
> pinned in both directions.
