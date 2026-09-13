# W31-CORPUS-CONTRACTS-2 — the two size-sibling contracts wave 29 named, each on its own body reading; five refusals decided, re-measured on the moved baseline — 2026-09-13

## Headline

The wave-31 arbiter card, on card W29-CORPUS-CONTRACTS' exact pattern. Two new
contract modules —
[`contract/qcore/chunkio/chunk_sizes.qcore`](../../../contract/qcore/chunkio/chunk_sizes.qcore)
(`cio_chunk_get_content_size`) and
[`contract/qcore/fluent_bit/input_chunk_total_size.qcore`](../../../contract/qcore/fluent_bit/input_chunk_total_size.qcore)
(`flb_input_chunk_total_size`) — each declaring **one** trusted assumption
read against its pinned body at Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, and the 185-unit corpus
re-measurement attributes its entire movement to **five** `unsupported`
leaves at the call sites of exactly those two callees:

| measure | BEFORE | AFTER |
|---|---:|---:|
| units | 185 — **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only** | **identical** |
| projected functions | 4,834 — **146 Safe / 85 provisional Unsafe / 4,603 Unknown** | **identical** |
| `leak` | **272** | **272 — unmoved** |
| `unknown_alias` | **4,208** | **4,208 — unmoved** |
| `double_release` | **1** | **1 — unmoved** |
| `unsupported` | **50,155** | **50,150** (−5, itemised leaf by leaf in §5) |
| unit status moves / function status moves / reason-code moves | — | **0 / 0 / 0** |
| units with any diagnostic change | — | **2** (`chunkio/cio_stream.c`, `fluent-bit-chunk/flb_input_chunk.c`) |
| leaves added / removed | — | **0 / 5** |

**Nothing is promoted.** Every function that carried one of the five refusals
is `Unknown` before and after, carrying its other refusals unchanged; both
touched units keep their statuses (`cio_stream.c` `Unknown`,
`flb_input_chunk.c` provisional `Unsafe` on its unchanged leak leaves). The
leak pin **does not move** — there is still no fifth move in constraint 7's
history beyond the one `C-GATEWAY-WAVE` itself recorded — and neither
`unknown_alias` nor `double_release` moves: at all five sites the actual
resolved, so the admitted calls produced **no** replacement refusal.

The candidates were the wave-29 arbiter's two named non-declarations, taken
on their own stated terms (§1): `W29-CONTENT-SIZE-UNDECLARED` is **resolved**
— its stated obstruction was module scope, not semantics, and this card lands
the ChunkIO-side module it asked for, with its own reading — and
`W29-SIZE-SIBLINGS-UNDECLARED` is resolved for **exactly one** sibling, the
one whose pinned body mutates no residency state; the remainder is **upheld**
with the readings and the baseline measurements recorded (§7,
`W31-SIZE-SIBLINGS-REMAINDER`).

Contract corpus **74 → 76 modules**, manifest-selected modules **19 → 21**,
resource-v1 declarations **41 → 43** — each pin updated deliberately
(constraint 5) in `compiler/tests/qcore_contract_corpus.rs`,
`compiler/tests/contract_loader.rs`,
`crates/qcore-c-gateway/tests/{prod_contracts,neighbours}.rs` and
`contract/cycle19-resource-contracts.json`.

No Rust source changes (test pins only), no new diagnostic code (the
`Q-NAMESPACE-DECISION` ice box needed nothing), no new keyword, no Lean module
(§8), no provider change.

---

## 0. The baseline moved under this card, and the card was re-measured against it

This card was interrupted mid-run by a model usage limit. Its predecessor's
work is inherited — the two contract modules, the pin edits, the measurement
scripts, the original prediction — and is committed history on this branch.
**While the card was down, the user landed their own wave-31 card
`C-GATEWAY-WAVE` (`2492825b`)**, which changes gateway analysis (scalar-fact
invalidation, complete-expression refusal frames, acyclic conditional custody)
and **moves the corpus baseline** from
`docs/reports/w4-recursive-summaries/artifacts/` to
`docs/reports/c-gateway-wave/artifacts/`.

Everything measured in this report is a **fresh run against the new baseline**.
The predecessor's runs and its BEFORE byte-identity — against the wave-30
archive, under the old analysis — are superseded and are **not** the evidence
here. Concretely, three things were redone rather than carried:

1. **The prediction was recomputed** against the `c-gateway-wave` archive
   before the AFTER run existed, in a commit of its own
   ([`prediction.md`](prediction.md) §0, [`predict.py`](predict.py),
   [`cmp_predictions.py`](cmp_predictions.py)). The original prediction is kept
   verbatim beneath it — a prediction is a commitment, not a draft.
2. **The BEFORE run reproduces `c-gateway-wave` byte for byte** (§3.1), which
   is what proves this card's AFTER delta isolates *its own* change: the
   gateway change is already inside the BEFORE.
3. **The gate was re-run at this tree.** The predecessor's gate logs describe
   a different tree and are not carried; its one disclosed sequencing record
   (a first full-suite run taken before two pin tables were updated) is a fact
   about that tree and is dropped rather than restated here.

### 0.1 The two predictions agree — and that agreement is itself a measurement

`predict.py` runs the same three inventories over whichever archive it is
given; [`logs/prediction-comparison.txt`](logs/prediction-comparison.txt) is
the section-by-section comparison. The gateway change moved the corpus totals
substantially and moved **nothing** in this card's candidate neighbourhood:

| inventory | wave-30 baseline | `c-gateway-wave` baseline |
|---|---|---|
| unit histogram | 2 / 42 / 137 / 4 | **identical** |
| projected functions | 4,834 — 150 / 84 / 4,600 | 4,834 — **146 / 85 / 4,603** |
| `leak` / `unknown_alias` / `unsupported` / `double_release` | 266 / 4,344 / 49,760 / — | **272 / 4,208 / 50,155 / 1** |
| the 4 candidate leaves naming the two declared callees | 4 | **byte-identical — same functions, lines, origins, texts** |
| every leaf of the 4 carrying functions (127 lines) | — | **byte-identical** |
| the 6 sibling-callee leaves behind `W31-SIZE-SIBLINGS-REMAINDER` | 2 + 1 + 3 | **byte-identical** |

So the recomputed prediction is the original one restated on the new totals,
and the **measured outcome is also the same**: the same five leaves, at the
same functions and the same internal lines, with the same origins and the same
zero movement everywhere else. Two independent analyses, four months of
gateway rules apart, decide this declaration's footprint identically. That is
a result worth stating plainly, and it is a **stability observation about this
neighbourhood**, not a general claim about the gateway change.

The one substantive thing it changes about the card's own story: the
predecessor observed the fifth leaf (L19962) as an **attributed deviation from
its prediction, in the deciding direction**. That deviation is **reproduced**
here under the new analysis (§5), so it is now a twice-measured mechanism
rather than a one-run surprise — and it is still reported as a deviation from
the recomputed range (−4 to −2), not as an expectation met.

---

## 1. The candidates, and how their stated reasons were honored

The brief named the wave-29 arbiter's two non-declarations as this card's
candidates and required their stated reasons to be read **first**:

* **`W29-CONTENT-SIZE-UNDECLARED`** declined `cio_chunk_get_content_size`
  because "a ChunkIO-side declaration is a different library's module with
  its own body reading". That is a scope obstruction, not a semantic one —
  the wave-29 card had itself already read the body
  (`lib/chunkio/src/cio_chunk.c:315-334`) while justifying the wrapper
  `flb_input_chunk_get_size`, and found "nothing allocated, nothing freed, no
  pointer stored". This card **resolves** the follow-on by landing the
  distinct ChunkIO-side module with its own line-cited reading (§2.1).
* **`W29-SIZE-SIBLINGS-UNDECLARED`** declined the neighbouring queries —
  `flb_input_chunk_is_up`, `flb_input_chunk_set_up`,
  `flb_input_chunk_total_size`, `input_chunk_metadata_view` "and others" —
  because "each needs its own body reading (several mutate residency state,
  where borrowed-only is not obviously the whole story)", and declaring them
  on their neighbours' shape would be card P07's unfaithful-declaration
  defect. This card read each named sibling (§7). For
  `flb_input_chunk_total_size` the stated obstruction **dissolves under its
  own reading**: the body (`src/flb_input_chunk.c:2947-2955`) mutates no
  residency state — no `cio_chunk_up`/`down`, no list relink — so it is
  declared, on this card's reading rather than its neighbours' shape (§2.2).
  For `is_up`, `set_up` and `metadata_view` the non-declaration is
  **upheld** with reasons and measurements (§7).

The corpus footprint was bounded from the **new** baseline before anything was
declared: `docs/reports/c-gateway-wave/artifacts/diagnostics.json` carries
exactly **4** `unsupported` leaves naming the two declared callees — two per
callee — plus one adjacent indirect-access refusal at the one chained-actual
`flb_input_chunk_total_size` call site, all bounded **before** the AFTER run
in [`prediction.md`](prediction.md).

## 2. The contracts, read against the pinned bodies

Every citation below was re-verified against the freshly prepared pin's own
checkout (`build/cycle19/clang-pin/fluent-bit-clang-source`) on this run, not
carried from the predecessor's report. Both modules' own comment blocks carry
the full line-cited readings; the substance:

### 2.1 `cio_chunk_get_content_size` (`lib/chunkio/src/cio_chunk.c:315-334`)

`cio_error_reset(ch)` (`lib/chunkio/src/cio_error.c:56-59` — one store,
`ch->error_n = 0;`, into the chunk's own error field, the callee
`chunkio/error_frame.qcore` and `chunkio/error_state.qcore` already declare
borrowed), a `ch->st->type` field read, then the backend's length field on
either storage arm (`mf->buf_len` or `cf->data_size`), or `-1` on an
unrecognized storage type. Nothing allocated, freed, stored or published on
any path.

### 2.2 `flb_input_chunk_total_size` (`src/flb_input_chunk.c:2947-2955`)

The whole body: `storage = (struct flb_storage_input *) in->storage;` (a
cast field read — the cast reinterprets a `void *` field, it neither
allocates nor releases), one call
`cio_stream_size_chunks_up(storage->stream)`, one scalar return. The one
callee was read too (`lib/chunkio/src/cio_stream.c:258-276`): a
`mk_list_foreach` walk of the stream's `chunks_up` list (pointer arithmetic,
no allocation) summing `cio_chunk_get_content_size` — §2.1's query — over
each up chunk into a local, skipping non-positive results. Note the receiver
is the input **instance**, not an input chunk: this is the aggregate query
over the instance's stream.

**Which mode is faithful.** On every path of both bodies the caller keeps
exactly the obligations it had before the call, so both declarations are
borrowed with `resource_return_none()` — the same reading, for the same
reasons, as `fluent_bit/input_chunk_sizes.qcore`. Writes into the receiver's
own object graph (the error-field reset) are not custody movement, exactly as
that module and `chunkio/chunk_residency.qcore` recorded.

**ABI and modes (constraint 8).** The C declarations in force, re-read on this
run, are `lib/chunkio/include/chunkio/cio_chunk.h:66`
(`ssize_t cio_chunk_get_content_size(struct cio_chunk *ch);`) and
`include/fluent-bit/flb_input_chunk.h:174`
(`size_t flb_input_chunk_total_size(struct flb_input_instance *in);`).
Under LP64 `ssize_t` resolves to `long int`, admitting `{i64, isize}` — the
module spells `isize` — and `size_t` to `unsigned long`, admitting
`{u64, usize}` — the module spells `usize`. Each parameter is one
object-pointer level, `ptr<mut, u8>`. The mode gate compares a declared duty
against a validated W4 summary of the body in the unit; both bodies contain
calls or casts the projection cannot model, so W4 derives no summary and
nothing is compared — and the measured corpus confirms it: **zero**
`SummaryConflict` movements, zero withdrawal descriptions, and the five
sites simply admit.

**Nullability is deliberately not claimed.** Both bodies dereference their
receiver unconditionally, but `resource_borrowed` imposes no non-null
precondition — the weaker statement, deliberately (a mode demanding entry
custody would delete the caller's failure paths). The wave-23
`nullable`/`nonnull` decision belongs to `resource_released` declarations;
nothing here releases.

**Declared although defined in corpus units.** Both defining units —
`chunkio/cio_chunk.c` and `fluent-bit-chunk/flb_input_chunk.c` — are in the
pinned corpus, so this is the PROD-CONTRACTS arrangement again: a declared
contract always wins over a W4-inferred summary, a disagreement is a
`SummaryConflict` diagnostic, and each unit still analyses the defined body —
its own function row is `Unknown` before and after.

**NOT CLAIMED**, named in the modules: the returned values' meanings (the
`-1` arm; the skipped non-positive chunks), which chunks are up (residency is
`chunkio/chunk_residency.qcore`'s subject), retention (B03/R13), and the
remaining size family (`cio_chunk_get_real_size`, `cio_meta_size`) at
measured payoff zero outside their declared wrappers.

---

## 3. The route

```sh
git submodule update --init --recursive          # FIRST: the libevent pin gate fires at the END of a run
sh measure/prepare-pin.sh                        # archived as prepare-pin.sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-w31
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cp target/release/examples/cycle19_fluent_bit_libraries_audit measure/bin/audit
python3 -B measure/predict.py docs/reports/c-gateway-wave/artifacts   # committed before AFTER existed
sh measure/run-audit.sh before measure/bin/audit # manifest still at 19 modules
python3 -B measure/byte_identity.py measure/runs/before
# ... land the manifest edit selecting the two new modules ...
sh measure/run-audit.sh after measure/bin/audit
python3 -B measure/census.py --before measure/runs/before --after measure/runs/after
```

A **fresh** Clang pin, per the wave-20 discipline: no earlier card's pin and
not the developer database. The pin's own production database is copied into
`<source>/build/compile_commands.json` and **both runs read the same bytes**
(SHA-256 `2f526ca7d5f6431d…`, **1,415 entries**, `/usr/bin/clang` spelled).
Both counted runs: **exit 0, zero gateway failures, all six scopes summing to
185 units** (31 + 17 + 94 + 13 + 1 + 29), per-scope lines byte-identical
([`scopes.py`](scopes.py), [`logs/scopes.txt`](logs/scopes.txt)).

**This card changes no Rust, so the BEFORE and AFTER runs executed the same
binary** — SHA-256 `959a487f4fd21463…`, built once in this worktree and
executed from worktree-private `measure/bin/`, never from `target/`. The only
input difference between the two runs is the contract manifest and the two
modules it selects (the wave-23 AOM-DESTROY-SIBLINGS shape, wave 29's exact
route). Both new `.qcore` files were on disk during the BEFORE run but
**unselected** — the manifest was reverted to its 19-module form for that run
and restored only after the BEFORE run and its byte-identity check had
completed — so they were not inputs of that run, and
[`prediction.md`](prediction.md)'s recomputation was committed before
`measure/runs/after` existed.

### 3.1 The BEFORE run reproduces the *current* baseline byte for byte

The authoritative baseline is card `C-GATEWAY-WAVE`'s archive
(`docs/reports/c-gateway-wave/artifacts/`) — the run the tip's leak pin of
272, `unknown_alias` 4,208, `unsupported` 50,155, `double_release` 1 and the
74-module contract corpus describe. The chain beneath it, every predecessor
frozen and untouched (constraint 21):
`docs/reports/wave23-c-operations/artifacts/` →
`docs/reports/aom-destroy-siblings/artifacts/` →
`docs/reports/aom-binding-poison/artifacts/` →
`docs/reports/gateway-c-location/artifacts/` →
`docs/reports/w29-corpus-contracts/artifacts/` →
`docs/reports/w4-recursive-summaries/artifacts/` →
`docs/reports/c-gateway-wave/artifacts/`.

[`byte_identity.py`](byte_identity.py),
[`logs/byte-identity-before.txt`](logs/byte-identity-before.txt):

| artifact | archived baseline | BEFORE run |
|---|---|---|
| `diagnostics.json` | `4fea48c688ff0aa9…` | **identical** (raw and canonical) |
| `chunk-contracts.json` | `c4f438e4ed3dab3c…` | **identical** (raw and canonical) |
| `source.json`, `source-effects.json`, `contracts.json` | — | differ: the checkout-root and identity-digest class every prior re-pin has recorded |

and `diff_artifacts.py --archived docs/reports/c-gateway-wave/artifacts`
([`logs/diff-summary-before.txt`](logs/diff-summary-before.txt),
[`artifacts/diff-baseline-to-before.json.gz`](artifacts/diff-baseline-to-before.json.gz))
itemises **zero** differences in every section that carries a verdict:

```
--- chunk-contracts: results 96/96 +0 -0 ~0;  backend_teardown 21/21 +0 -0 ~0
--- contracts:       sources 19/19, manifest 19/19, audit_selection 5/5
--- diagnostics:     units 185/185, +0 -0 ~0
--- source:          envelope key `compile_entry`
--- source-effects:  results 96/96, 3 result rows changed
```

The residues are the classes every prior re-pin has recorded — this
worktree's absolute path differs from the primary checkout's — and this card
can say so exactly: `source.json`, `source-effects.json` **and
`chunk-contracts.json` are byte-identical between this card's own two runs**
([`logs/byte-identity-before-to-after.txt`](logs/byte-identity-before-to-after.txt),
[`artifact-manifest.json`](artifact-manifest.json)).

**This is the load-bearing fact of the whole re-measurement.** Because the
BEFORE reproduces `c-gateway-wave`'s diagnostics byte for byte, the gateway
change is entirely *inside* the BEFORE, and every difference in §4 and §5 is
this card's two declarations and nothing else.

---

## 4. BEFORE → AFTER, itemised

[`logs/diff-summary-before-to-after.txt`](logs/diff-summary-before-to-after.txt),
[`artifacts/diff-before-to-after.json.gz`](artifacts/diff-before-to-after.json.gz):

```
--- chunk-contracts: results 96/96 +0 -0 ~0;  backend_teardown 21/21 +0 -0 ~0
--- contracts:       sources 19 -> 21 (+2: qcore/chunkio/chunk_sizes.qcore,
                     qcore/fluent_bit/input_chunk_total_size.qcore)
--- diagnostics:     units 185/185, +0 -0 ~2;  envelope diagnostic_counts
                     {double_release 1, leak 272, unknown_alias 4208,
                      unsupported 50155 -> 50150}
--- source:          (no differences)
--- source-effects:  results 96/96 +0 -0 ~0
```

Three of the five artifacts are **byte-identical**; `contracts.json` grows by
exactly the two selected modules; `diagnostics.json` moves in exactly two
units.

## 5. Every moved leaf, attributed

[`census.py`](census.py), transcript [`logs/census.txt`](logs/census.txt):

```
units: 185 -> 185;  unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834;  function status moves: 0
leak: 272 -> 272 (+0);  unknown_alias: 4208 -> 4208 (+0)
double_release: 1 -> 1 (+0);  unsupported: 50155 -> 50150 (-5)
units with diagnostic-count changes: 2
leaves added: 0; leaves removed: 5
```

The five removed leaves, each `(function, preprocessed line, origin,
category, leaf)`. For the first four the mechanism is the wave-29 one: *the
call rule now finds a declared contract, the actual resolves, and the call is
admitted with every parameter borrowed and a custody-free scalar result.* The
fifth is attributed separately below.

| unit | function | line | origin | category | leaf |
|---|---|---|---|---|---|
| `chunkio/cio_stream.c` | `cio_stream_size_chunks_up` | L2280 | `cio_stream.c:268` | `unsupported` | uncontracted call `cio_chunk_get_content_size` |
| `fluent-bit-chunk/flb_input_chunk.c` | `flb_input_chunk_map` | L18822 | `flb_input_chunk.c:1606` | `unsupported` | uncontracted call `cio_chunk_get_content_size` |
| `fluent-bit-chunk/flb_input_chunk.c` | `flb_input_chunk_set_limits` | L19889 | `flb_input_chunk.c:2974` | `unsupported` | uncontracted call `flb_input_chunk_total_size` |
| `fluent-bit-chunk/flb_input_chunk.c` | `flb_input_chunk_set_up_down` | L19957 | `flb_input_chunk.c:3108` | `unsupported` | uncontracted call `flb_input_chunk_total_size` |
| `fluent-bit-chunk/flb_input_chunk.c` | `flb_input_chunk_set_up_down` | L19962 | `flb_input_chunk.c:3118` | `unsupported` | indirect memory access is outside the resource projection |

(The `origin` column is the wave-27 presented coordinate of each internal
line, read from the artifact.)

**The fifth leaf is the one deviation from the prediction, in the deciding
direction, and it is attributed rather than absorbed.** The prediction bounded
the movement at −4 to −2 and said the L19962/L19963 refusals at the third
`flb_input_chunk_total_size` call site — `:3118`, the chained actual
`total = flb_input_chunk_total_size(ic->in);` — were "expected to stay",
because they are indirect-access refusals about the argument expression, not
uncontracted-call leaves. That expectation is wrong for L19962 and right for
L19963: with the callee **uncontracted**, the statement's member-access
argument refuses as an indirect access before any contract is consulted;
with the declared contract present, the call rule admits the member actual —
card ABI-TYPEDEF-ADDRESS-OF-MEMBER's admission landed for exactly this
chained-actual-at-a-declared-contract shape — resolves `ic->in` through the
same one resolution the named sites use, and the statement no longer refuses.
No replacement leaf appears at that line. L19963 (`:3119`,
`in->mem_chunks_size = total;`, a store through `in` with no contract in
play) keeps its refusal, as predicted. The result is strictly more decided
than predicted, with nothing promoted: `flb_input_chunk_set_up_down` stays
`Unknown` on its four remaining refusals.

This deviation was also measured by the card's interrupted first attempt, under
the *pre*-`C-GATEWAY-WAVE` analysis, at the same function, the same internal
line and the same origin. Two analyses reach it independently; it is a
mechanism, not a scheduling artefact. It is nevertheless still reported as a
**deviation**, because the recomputed prediction deliberately declined to
promote the earlier observation to an expectation (`prediction.md` §0).

Where the prediction landed otherwise: item 1's four named leaves went to
zero with **zero replacements** — both chancy actuals (`ch` at L2280, bound
from the `mk_list_entry` walk; `in` at L19957, bound from the member read
`in = ic->in;`) resolved, so the `unresolved resource identity` substitution
allowed for appeared nowhere; items 2, 3, 4, 5 and 6 exactly (leak,
`unknown_alias` and `double_release` unmoved leaf for leaf, zero status moves,
zero `SummaryConflict`, `contracts.json` 19 → 21 / 41 → 43, three artifacts
byte-identical between the runs).

## 5.1 The pin statements

* **Leak 272 → 272.** Unmoved. Constraint 7's recorded history stands as
  written, including `C-GATEWAY-WAVE`'s own 266 → 272 move and its six named
  new provisional rows. **This card adds no move to that history.** Not one
  leak leaf was added, removed or re-attributed.
* **`unknown_alias` 4,208 → 4,208.** Unmoved, leaf for leaf.
* **`double_release` 1 → 1.** Unmoved. That single row is `C-GATEWAY-WAVE`'s
  provisional `rd_kafka_jwt_b64_decode_payload` finding under its named
  follow-on `C-GATEWAY-CONSTANT-DO-WHILE`; this card declares nothing in
  librdkafka and did not touch it.
* **`unsupported` 50,155 → 50,150.** Moved by exactly the five decided leaves
  above, in the deciding direction, zero absorbed. Four are the
  uncontracted-call leaves the two declarations name; the fifth is the
  argument-expression refusal the second declaration retires at its one
  chained-actual site, attributed in §5.
* **Zero promotions.** Unit histogram, function histogram, every unit
  `reason_code`: byte-identical. `flb_input_chunk.c` stays provisional
  `Unsafe` on its unchanged leak leaves — a provisional `Unsafe` is never an
  upstream defect claim.

---

## 6. Gate

Measured at the final tree of this worktree, with the manifest at 21 modules
and every pin updated.

| step | result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **GATE_TESTS** |
| `cargo fmt --all -- --check` | **GATE_FMT** |
| `cargo clippy --all-targets -- -D warnings` | **GATE_CLIPPY** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **GATE_PROBE** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **GATE_FOLLOWUP** |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | **GATE_LEAN** |
| `python3 -m unittest discover -s scripts/tests` | **GATE_PYTHON** |

Provider code and packages are unchanged; the ledger's 122 provider tests are
carried from the preceding checkpoint. The five packages were **built** for
the audit route (the pinned route selects the ChunkIO package through
`QCORE_CHUNKIO_PACKAGE`), but no template and no `qcore-schemas` version
moved, so constraint 18's rebuild-and-verify obligation is not engaged.

---

## 7. Boundaries and named follow-ons

Each remaining wave-29 sibling was **read on this card** before its
non-declaration was upheld; the readings and the baseline measurements are
the record. All six sibling-callee leaves are byte-identical between the
wave-30 and `c-gateway-wave` baselines
([`logs/prediction-comparison.txt`](logs/prediction-comparison.txt)), so the
measurements below hold on both.

| name | statement |
|---|---|
| `W31-SIZE-SIBLINGS-REMAINDER` | the three remaining callees `W29-SIZE-SIBLINGS-UNDECLARED` named stay undeclared, each for its own read reason. **`flb_input_chunk_is_up`** (`src/flb_input_chunk.c:3128-3131`, one statement wrapping the already-declared `cio_chunk_is_up`) and **`flb_input_chunk_set_up`** (`:3142-3149`, a guarded `cio_chunk_up` — residency mutation, though `chunkio/chunk_residency.qcore` shows the family's faithful borrowed reading exists) carry **2 + 1** baseline `unsupported` leaves (`flb_input_chunk_destroy` L19737/L19716, `input_chunk_append_raw` L20103); they are a residency state-machine pair whose declarations belong together in the residency family's module, not singly in a size card's — landed on a later card's own reading and measurement, not this one's. **`input_chunk_metadata_view`** (`:308`…, 3 baseline leaves at `flb_input_chunk_get_direct_routes` L19328, `flb_input_chunk_get_tag` L20494 and `flb_input_chunk_has_direct_routes` L19263) is `static` and publishes a borrowed view of the chunk's mapped metadata through its out-parameter (`view->buffer = buf` from `cio_meta_read`, re-read on this run), a shape the v1 vocabulary has nowhere to carry — borrowed/none on `view` would say nothing about the interior pointer it receives; declaring it anyway would be the P07 defect. Upheld on semantics, not scope. |
| `W29-CONTENT-SIZE-UNDECLARED` | **resolved by this card**: `chunkio/chunk_sizes.qcore` is the ChunkIO-side module that follow-on asked for, with its own line-cited reading; both baseline leaves naming the callee are decided (§5). |
| `W29-SIZE-SIBLINGS-UNDECLARED` | **resolved for one sibling** (`flb_input_chunk_total_size`, whose body mutates no residency state) **and narrowed** to the remainder above; the P07 concern was honored by reading, not by inheritance — nothing was declared on a neighbour's shape. |

One boundary of this re-measurement is worth stating rather than leaving
implicit: §0.1's stability observation is about **this neighbourhood**. The
gateway change moved 813 additions and 547 removals corpus-wide by its own
report; that none of them fall on these five leaves or these six sibling
leaves is measured, not predicted, and says nothing about any other card's
candidate set.

---

## 8. Lean

No Lean module is added or amended, on the AOM-DESTROY-SIBLINGS and
W29-CORPUS-CONTRACTS precedent: this card changes **no semantic surface** —
no checker rule, no loader rule, no emitted byte; the BEFORE and AFTER runs
executed the same binary, and the only change is a declarative input (two
contract modules and the manifest rows selecting them) plus deliberately
updated test pins. Invariant 6 binds semantic changes; a grown premise set is
not one. The audit count is expected unchanged at `C-GATEWAY-WAVE`'s 123.

---

## 9. What was touched

* `contract/qcore/chunkio/chunk_sizes.qcore` — new (inherited from this card's
  interrupted first attempt; re-verified against the pin on this run).
* `contract/qcore/fluent_bit/input_chunk_total_size.qcore` — new, likewise.
* `contract/cycle19-resource-contracts.json` — two manifest rows.
* `compiler/tests/qcore_contract_corpus.rs` — corpus pin 74 → 76, two
  `EXPECTED_CONTRACTS` rows.
* `compiler/tests/contract_loader.rs` — (19, 41, 145) → (21, 43, 145), plus
  the two old-table rows and the certificate-replay pin.
* `crates/qcore-c-gateway/tests/prod_contracts.rs`,
  `crates/qcore-c-gateway/tests/neighbours.rs` — manifest count 19 → 21.
* `crates/qcore-c-gateway/tests/std_socket_inet.rs` — inherited worktree
  census hygiene (§10).
* `docs/reports/w31-corpus-contracts-2/` — this report and its evidence, all
  re-measured against the moved baseline.
* `docs/development/increments/wave-31.md` — one increment paragraph;
  `AGENTS.md` — headline numbers (corpus row, contract corpus, baseline
  pointer), per the sync rule.

## 10. Inherited alongside the card: worktree census hygiene

The card's interrupted first attempt also fixed an environment-shape defect it
tripped over, committed on its branch and merged here. The
`SOCKET-INET-ADDRESSES` census test
(`std_socket_inet.rs::this_card_reserves_no_word_and_its_address_rule_collides_with_nothing`)
walked the repository root recursively and, when run from the **primary
checkout**, recursed into `.claude/worktrees/*` — sibling checkouts of this
same repository — finding their copies of the two published examples and
failing against the pinned two-file expectation. The walker now skips any
subdirectory carrying its own `.git` entry (a nested git worktree or embedded
checkout is another tree, not this one), the same environment-shape family as
the wave-29 submodule-diagnostic precondition. The census itself is
unweakened: over one checkout's tree it visits exactly the files it always
visited, and the >400-file floor and the two-example pin are untouched.

---

## 11. For the integrator

The corpus baseline moves to
`docs/reports/w31-corpus-contracts-2/artifacts/`. The chain beneath it, every
predecessor frozen (constraint 21):
`docs/reports/wave23-c-operations/artifacts/` (the frozen corpus baseline) →
`docs/reports/aom-destroy-siblings/artifacts/` (wave 23) →
`docs/reports/aom-binding-poison/artifacts/` (wave 25) →
`docs/reports/gateway-c-location/artifacts/` (wave 27, presentation-only) →
`docs/reports/w29-corpus-contracts/artifacts/` (wave 29) →
`docs/reports/w4-recursive-summaries/artifacts/` (wave 30, identical verdict
content) → `docs/reports/c-gateway-wave/artifacts/` (wave 31, the user's own
gateway card) → **this archive** (wave 31). The ledger's corpus row changes in
exactly one figure: `unsupported` 50,155 → **50,150** (−5, all five decided by
declaration, attributed in §5). Leak stays **272**, `unknown_alias` stays
**4,208**, `double_release` stays **1**, the unit and function histograms are
unchanged, and nothing is promoted. Contract corpus **76 modules**.
