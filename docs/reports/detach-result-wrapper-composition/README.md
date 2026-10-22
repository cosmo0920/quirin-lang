# Checked removed-selection composition (rule 33)

Card DETACH-RESULT-WRAPPER-COMPOSITION transports the existing same-item
`Removed` identity through cJSON's two array wrappers. Fresh pinned corpus
BASE/TIP: **975 → 977 Safe, 3,850 → 3,848 Unknown, 9 provisional Unsafe
unchanged**. Exactly `cJSON_DetachItemFromArray` and
`cJSON_DeleteItemFromArray` become Safe, both conditional on the detach
declaration's `detach_list_representation` premise and `get_array_item`'s null
premise. The two object detaches and their two delete wrappers keep their old
refusals, because string-keyed membership has no checked frame. No prior Safe
regresses, and all 4,921 null dictionaries are unchanged.

The [frozen prediction](PREDICTION.md) precedes every semantic change, and its
row prediction holds exactly. Base is main
`e282aaccc7763da989cf6f3c8f3c51cdd3c8de3c`. The measured analyzer source is
`9ecc941d5f18739296cf573a2476dbb51a59fbb1`, whose code is that of `c3301ff41`;
later commits change only the report. Gateway rule
[33](../../agents/gateway.md#constraint-33) is the authority.

## What changed

**Frames.** `legacy_selection.rs` holds the gateway hooks and
`removed_selection.rs` the pure decisions. Two exact source frames are
recognized after renaming and the existing NULL normalization:

- a *selection detach* `F`: `if (W < 0) { return NULL; } return D(P, L(P, (size_t)W));`
- a *selection release* `G`: `X(F(P, W));`

The callees must have exactly these contracts in force:

- `L` passes rule 13a's complete indexed lookup frame and has its
  borrowed-from summary;
- `D` has exactly the same-item `Removed` shape and the exact named
  `detach_list_representation` premise, with no container, lifetime, retaining
  or guarded clause;
- `X` has one nullable heap `Released` formal and no result.

`P` must name the lookup's canonical record, `W` must be `int` and `size_t`
must be unsigned. An external `inline` definition is not a frame. The selected
child is `sel(P, W) = (W < 0) ? NULL : L(P, (size_t)W)`.

**Rows.** A frame's own body is read as its constituent calls. This is a
build-stream rewrite beside the generic-selection and literal-premise
rewrites, and the reserved `__qcore_selection_*` places must not appear in the
source. The ordinary analyzer runs the rewritten body under a *selection
entry*, where `P` is one live, settled, non-NULL owned site. The frame has one
lookup call, reached only through branches and reserved initializations. At
that call, the selected child appears as one nullable child uniquely contained
by `P` and is recorded as a contained identity, not a borrow. That membership
is the **prerequisite the caller transports**: it is never read off borrowed
provenance or off the representation premise. The unchanged same-item adapter
then decides the detach.

A row validates only when its run is `Safe` with no diagnostic and selects on
some path. Every exit must also keep `P` live with no other caller-visible
effect, and must take one of these forms:

- for `F`, return nothing without selecting (the `W < 0` arm), or return
  exactly the escaped detached child;
- for `G`, release the detached child.

Otherwise the contract-free row stands. No `ResourceFunctionContract` or
summary record is published. As a result, no other unit, route or
W4/fresh-result reading sees the selected child as fresh custody.

**Callers.** A caller statement expands into the frame's own constituent
calls (`L(q, K)`, then `D`, then `X`) when it has one of these forms, with a
plain parent `q` and a decimal constant `K` from 0 to 255:

- `d = F(q, K);`, `R *d = F(q, K);` or `struct T *d = F(q, K);`
- `F(q, K);`
- `G(q, K);`

Rule 13a's physical-prefix lookup identity and record binding then decide at
the caller, together with the same-item membership, NULL and alias checks and
`D`'s declaration with its premise. Every other use keeps the
uncontracted-call refusal. The pinned corpus has no such caller: the only
production callers of the six wrappers are the string-keyed
`cJSON_DeleteItemFromObjectCaseSensitive` calls in
`rdkafka_sasl_oauthbearer_oidc.c`.

**Not changed.** String-keyed object lookup (`get_object_item` and
`cJSON_GetObjectItem*`) has only a declared borrowed-from contract, and the
body of `get_object_item` is itself Unknown (two unmodeled loop conditions and
two unresolved cursors). No checked frame binds a key to one child, and no
caller can resolve a key. The card therefore does not fake that membership,
and the two object detaches and their two delete wrappers keep their old
refusals. `detach_list_representation` stays a `DeclaredAssumption`. Guarded
metadata is not consumed.

## Lean

`QCore.RemovedSelection` (`proof/QCore/RemovedSelection.lean`, audited by
`proof/AuditRemovedSelection.lean`) has 19 theorems. They state the frame's
selection and the facts below, and the three shared vector tables
(materialization, exit acceptance, selection) are executed by Rust's
`removed_selection` unit tests:

- negative selects nothing; a caller's nonnegative constant is the lookup;
- a present selection is a member of the represented list, which is the
  caller prerequisite;
- the detach composition returns exactly the selection, moves nothing without
  one, removes it from parent custody into direct custody and preserves every
  other child and the identity inventory;
- the release composition releases exactly the selection and nothing else;
- a materialization needs every fact; an accepted `F` exit returns nothing or
  the escaped selection; an accepted `G` exit never keeps or returns it.

The model reuses `QCore.ListLookup.lookup` and the
`QCore.DetachSameItem` transition. Source extraction (the exact frame match),
the analyzer's abstract-state reading and the Rust/Lean correspondence remain
boundary-28 premises. Targeted builds: `lake build QCore.RemovedSelection`
attempts 1–3 failed (console only, not separately logged): an unsolved cast
goal, then `omega` over `Int.ofNat`, then a deprecated `if_false`. Attempt 4
exits 0, and the 19 axiom prints use only `propext` and `Quot.sound`.

## Corpus measurement and attribution

Both sides are exported snapshots (`git archive`, with a `third_party/libevent`
clone at the pinned `d82464a`), each built in its own directory. They run
through the committed [driver](scripts/measure.py) (copied from batch5, with
the worktree-HEAD identity check dropped because the snapshot is the measured
input) with jobs 4, the batch4 [link map](link-map.corpus.json) (`98a4db76…`),
Clang 21.1.8 and both projections:

| Side | Binary SHA-256 | Window (UTC) | Exit | Safe / Unknown / prov. Unsafe |
|---|---|---|---|---|
| BASE `e282aaccc` | `8f915218…4b072db7` | 17:10–18:30 | 0 | 975 / 3,850 / 9 |
| TIP `9ecc941d5` | `6eb216ea…a68f981ac` | 14:41–16:02 | 0 | **977 / 3,848 / 9** |

BASE attempt 1 (13:19–14:39) exits 1. It failed the Cycle 18 libevent pin
because the exported snapshot had no libevent checkout, so `git rev-parse`
resolved to the enclosing worktree. It is retained and is not evidence.

[`corpus-summary.json`](artifacts/corpus-summary.json),
[`field-review.json`](artifacts/field-review.json),
[`site-review.json`](artifacts/site-review.json) and
[`projection-review.json`](artifacts/projection-review.json) attribute every
difference. Only two rows change in content: the two promoted rows, which each
lose their single refusal leaf. All other raw differences across the 4,834
rows are identity substitutions caused by the two snapshot directories
entering environment texts. The [field review](scripts/field_review.py) proves
each one is a consistent one-to-one substitution, or (for reading sites)
equal after removing only digests:

- every row's `hypothesis_context.assumptions`;
- 185 units' `home_environment_sha256`;
- 45 reading-site environment texts, equal after rewriting the directory name
  and masking digests;
- 55 rows' reading-site evidence, which reorders as multisets.

The cJSON unit's `diagnostic_counts` and `reason` drop one `unsupported` leaf
each, and the document's `unsupported` count drops by 2.

[`nine-body-review.json`](artifacts/nine-body-review.json) keeps all nine rows
on both sides:

| Function | BASE | TIP |
|---|---|---|
| `cJSON_DetachItemFromArray` | Unknown: nested `get_array_item` | **Safe**: declaration `cJSON_DetachItemViaPointer` (nine `detach_*` hypotheses of `detach_list_representation`), summary `get_array_item`, null premise `null-deref:get_array_item:2770:current_child` |
| `cJSON_DeleteItemFromArray` | Unknown: nested `cJSON_DetachItemFromArray` | **Safe**: declarations `cJSON_Delete`, `cJSON_DetachItemViaPointer` (same hypotheses), summary and null premise as above |
| `cJSON_DetachItemFromObject`, `…CaseSensitive` | Unknown: return expression | unchanged |
| `cJSON_DeleteItemFromObject`, `…CaseSensitive` | Unknown: nested object detach | unchanged |
| `cJSON_DetachItemViaPointer` | Unknown, 11 leaves | unchanged |
| `get_array_item`, `cJSON_GetArrayItem` | Safe | unchanged |

Rows depending on the detach declaration go from **4 rows (0 Safe) to 5 rows
(2 Safe)**; `cJSON_DeleteItemFromArray` now consults it. The declaration itself
is unchanged. All **4,921** null dictionaries are identical: only the two rows'
`custody` and `combined` fields move to Safe, so combined Safe goes **847 → 849**.

The runtime projection shows no gain. In all three loaded scenarios (`all`,
`flb_external_plugin`, `rk_plugin_library`), both promoted rows move
**Safe → Unknown** (`outer-uncovered: premise:detach_list_representation …
no envelope line covers it`). Runtime custody therefore stays 945 Safe / 30
Pending / 3,850 Unknown / 9 provisional Unsafe. This promotion is static and
conditional; it is not a runtime envelope.

The fresh planner (`bottom_up_plan.py`; plans
[BASE](artifacts/base-plan.json.gz) and [TIP](artifacts/tip-plan.json.gz))
moves:

- candidate bodies 2,215 → 2,214;
- single-body waits 254 → 253;
- hypothetical ceiling 362 → 360;
- no-extra-body closure 6/6 → 4/4, because the two promoted rows leave the
  Unknown denominators.

These are census quantities, not promised gains.

System and the tail-pause slice select no cJSON function, so neither route is
measured.

## Validation

The standing gate ([record](gate.json), Clang recording beside it) ran at
`9ecc941d5` in this worktree under the shared lock, with every step's exit
captured and no pipes (`private/detach-wrapper/gate-2/`):

| Step | Exit | Result |
|---|---|---|
| `git submodule update --init --recursive` | 0 | |
| `clang --version` | 0 | Clang 21.1.8 |
| `run_test_binaries.py --jobs 6 -- --locked --offline` | 0 | 542 units, 555 summaries; **8,549 passed, 0 failed, 15 ignored, 0 filtered** |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `check_lean_audits.py` | 0 | **366 Lean audits**; `AuditAll` axiom closure clean |
| `python3 -m unittest discover -s scripts/tests` | 1 | 374 tests, **one failure**: the stale committed matrix (`lean_audits` 365, produced 366) |

The single Python failure is the expected stale-matrix check that regeneration
closes. The matrix is integrator-owned and shared with the other running cards,
so it is not regenerated here. `providers/` is unchanged, so no provider gate
applies. Gate attempt 2 builds without incremental cache or debug info
(`CARGO_INCREMENTAL=0`, dev/test `debug=0`). This is build tuning only, made
after attempt 1 exhausted the shared disk. The lowest free space recorded
during attempt 2 is 85 GB.

Focused development runs, all exit 0 unless noted:

- `tests/detach_wrapper_composition.rs`, attempts 1–5. Attempts 1–3 failed
  and are retained in `private/detach-wrapper/probe/tests-{1,2,3}.log`:
  callers declared through the opaque defining alias, then a fixture typo with
  the missing `struct T *d =` prefix, then a header frame outside the W44
  callee scope. Attempts 4 and 5 pass 6/0/0.
- `removed_selection` unit tests: 5/0/0.
- Pre-gate regressions: `detach_lookup_membership` 8/0 and `detach_same_item`
  7/0.
- Pre-gate gateway crate suite: 191 summaries, 3,194 passed / 0 failed / 9
  ignored.
- Clippy: the first run found two `bool::then` lints; the second passes.
- Pre-gate Python: 374 tests with one expected stale-matrix failure (Lean
  audits 365 → 366), owned by the integrator's matrix regeneration.

The temporary `zz_selection_probe` test and its debug prints were removed
before commit. Gate attempt 1 ran the shared disk out of space during the test
build (ld bus errors, then `No space left on device`). It was stopped through
its captured task handle and is retained in
`private/detach-wrapper/gate-1-failed-disk-exhausted/`; only this card's
95 GB debug target was removed.

## Follow-ons

- `DETACH-RESULT-WRAPPER-COMPOSITION` (amended, not retired): the array
  detach/delete wrappers are discharged as conditionally Safe. The object
  detaches/deletes remain open, waiting on string-keyed membership.
- `DETACH-LOOKUP-MEMBERSHIP` (narrowed): what remains is string-keyed lookup,
  whose `get_object_item` body is itself Unknown and has no checked key frame,
  and callers that cannot resolve a key. The other residuals also remain:
  nonliteral and formal indices at callers, opaque defining-typedef caller
  record aliases (production callers spell `cJSON *`), successor-link closure
  through contained children, and wrapper callers outside the main-source or
  W44 callee scope.
- `DETACH-LIST-REPRESENTATION-CLOSURE`: unchanged. Both new Safe rows rest on
  it, and the runtime projection keeps them Unknown until an envelope covers
  it.
- New boundary: caller expansion assumes that this unit's frame definition is
  the body the call runs. This is the same premise selected summaries use; an
  external `inline` definition is refused.
