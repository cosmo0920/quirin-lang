# LEXICAL-ROUTES-PREEMPT-TYPED-FALLBACKS

Completed on an isolated branch. Original prediction and brief were committed on exact base
`654e2b7eed72374b02ea39ef41059f7a9149b776` before implementation;
[PREDICTION.md](PREDICTION.md) is unchanged. The card was subsequently rebased
onto the combined W48/RK integration source
`854920e3e12a86c34231d4b46fa42918e29044ca`, preserving its provider contracts,
interceptor metadata and ledger fields. The integrator's fresh combined
three-route measurement supplies the exact-base observation, explicitly reused
with its actual source/binary identity in [measurement.json](artifacts/measurement.json).

The precedence rule preserves the exact older admitting route. Only a scalar
route's flow refusal consults a separately read Clang plan at the same source
occurrence, under its own declaration and closure premises. The candidate
must have the same scalar destination and refused-write frame, no pointer
store/destination/readback, and every original same-type alias read. All base,
per-hop and alias obligations must discharge on the incoming state before it
can replace the lexical refusal. Otherwise the old result and diagnostics
stand. No route is deleted and no callee's contract or entry custody changes.

`legacy_scalar_precedence.rs` attaches the bounded candidate inside the existing
outer guards. `legacy_transfer.rs` evaluates virtual scalar outcomes using a
cloned incoming state, fresh diagnostics, a nested null-premise frame and a
saved consultation log. The original result wins when it discharges. Otherwise
the typed result wins only with zero diagnostics. Only the chosen outcome and
its named premises are committed; scalar effects occur once and no candidate
can allocate a site or call a function. Other CFG consumers keep the original
instruction's identical effect and violation evidence.

`QCore.ScalarPrecedence` proves composition under each independent reader's own soundness premise, abstract same-effect selection, preservation
of old admissions, complete typed discharge, preservation of original aliases,
failed-probe identity and once-only effects. Faithful Clang extraction and
Rust–Lean correspondence remain explicit, unproved trust-boundary premises.
The Rust controls cover the W42 store, W43 scalar access, W44 condition chain,
alias, null, released/owned resources, pointer stores, unresolved calls,
scalar-fact forgetting, candidate effect mismatches and speculative null-premise
rollback. Fixtures use synthetic contracts, not production library claims.

The first complete standing gate retained a Rust exit 101 (**8,247 passed /
2 failed / 15 ignored**, 514 units / 527 summaries); all other steps passed,
including **339 Lean audits / 374 Python tests**. The two failing tests encoded
three lexical-only refusal assertions: `disable_then_store` after a removal
with no record, a scalar store through a target-free interior alias, and a
chain read through that alias. Each is now independently discharged by the
new rule. The latter two custody decisions retain their named null premise;
none establishes null safety. The three pins were deliberately amended with
source comments, while genuine literal-null, tracked, released, alias, pointer
store and unresolved-effect controls remain. The caller-held-context
certificate control additionally rejects omission of the selected W42 store's
new null premise. All 23 pin/certificate-focused cases pass. The original gate,
latent third assertion's failed focused replay, and exact reasons remain under
[attempts](artifacts/attempts/first-standing-gate/explanation.json) and private logs.

The second full gate passes all **8,250 Rust tests / 0 failed / 15 ignored**
(514 units / 527 summaries), **339 Lean audits / 374 Python tests**, but retains
Clippy exit 101: amending the old pin left its diagnostic helper field unread.
The fix strengthens the Safe helper to require no diagnostics; all 11 focused
null/certificate controls and Clippy with warnings denied then pass. There is
no production code change. [The second attempt](artifacts/attempts/second-standing-gate/explanation.json)
is retained separately. The clean third gate supersedes those failures.

The clean third standing gate passes all ten commands with exit 0: **8,250
Rust tests / 0 failed / 15 ignored**, 514 units / 527 summaries, **339 Lean
audits / 374 Python tests**. It tested `bd12e15e3`, whose analyzer, proof and
test source is unchanged from `913a72a916242165ed0dc586fd1affdb3093fd65`.
[The gate receipt](gate.json) and [commands with private log paths](artifacts/final-standing-gate/commands.json)
retain the source boundary and explicitly borrowed provider measurement.
The saved release binary was built at that implementation source, SHA-256
`2dc2cef4e0b39e1ee48061d9c17cfc4261c71e904820d0ec37f36ed6683d8431`.

All three tip audits exit 0. Raw logs are worktree-private under
`private/lexical-typed/{tip-corpus-1,tip-system-1,tip-slice-1}`; exact commands,
input identities, exits and private log paths are retained in the measurement
record. The report-only projection comparator had two initial schema/sorting
failures; [both attempts and corrections](artifacts/attempts/projection-helper/explanation.json)
are retained. They changed no measured output.

## Measurements and every changed row

Both sides use six conditional linked homes, their explicit actual-link/type/
calling-convention/interposition assumption, and both null/runtime projections.
The base is the integrator's fresh `854920e3e` run, binary `daf942d7…`; the tip
uses the saved `913a72a91` binary `2dc2cef4…`, recorded head `b4ecf16de` whose
later changes are report evidence only. Actual C sources remain corpus
`66910c10`, system `046accf6`, slice `34155955`; the compile databases,
selections and source identities are recorded, not inferred from a profile pin.

| Static custody scope | Before | After | Diagnostic changes |
| --- | --- | --- | --- |
| Corpus, 4,834 functions | 959 Safe / 9 provisional Unsafe / 3,866 Unknown | 963 / 9 / 3,862 | 63 rows; 206 leaves removed, 0 added |
| System, 448 configuration rows | 97 Safe / 4 provisional Unsafe / 347 Unknown | 99 / 4 / 345 | 18 rows; 68 leaves removed, 0 added |
| Slice, 404 configuration rows | 52 Safe / 4 Pending / 1 provisional Unsafe / 347 Unknown | 53 / 4 / 1 / 346 | 25 rows; 50 leaves removed, 1 added |

No old Safe is lost and no new Unsafe is published. Full raw row differences
also retain metadata-only changes: corpus 65, system 22, slice 34. Duplicate
compile occurrences stay separate. Every changed row, diagnostic occurrence,
added premise/dependency and affected source line was read; see each route's
`full-row-differences.json.gz`, `row-comparison.json`,
`moved-source-occurrences.json` and `projection-comparison.json` under
[corpus](artifacts/corpus/), [system](artifacts/system/) and [slice](artifacts/slice/).

The corpus gains are `skip`, `exit_snappy_decompressor`,
`read_uncompressed_length` and `flb_input_chunk_set_up_down`. The first is the
scalar `s->curoff = 0`; the next two inherit its independently checked summary
and existing `iv` null premises, with the assert declaration still a premise.
The chunk function's two `in->mem_chunks_size` stores add their exact named
null premises. System gains are its two compile occurrences. Slice's
`flb_tail_file_dormant_scan_begin` settles the scalar context flag and two
scalar flags on declared dormant-list members; this proves scoped custody,
not list bounds or deployment behavior. The 0–15 corpus prediction contains
four actual promotions. `cio_options_init` remains Safe; net-property-check
loses its two `ins` condition leaves per occurrence and engine-shutdown loses
two `config` store leaves per occurrence, both remaining Unknown. The three
test setters retain every diagnostic: the bounded independent premise does
not discharge their remaining access/closure obligations.

Most removed leaves are scalar stores or scalar chain checks; downstream
identity reads and same-unit summary calls can then discharge. Corpus removes
193 alias leaves and 13 unsupported leaves while leak and summary-conflict
counts remain 40 and 2. Complete differences preserve all remaining custody,
held-contract, source/ABI and effect refusals. Eleven corpus public
`null_premises` arrays grow without removing old entries. The metadata-only
`unpack_meta_aggregation_type` and `unpack_basic_type_meta` rows additionally
carry the former's exact dependency premise; their public premise arrays are
unchanged. The contracts safe rollup carrying a null premise grows 32 → 37,
including that existing Safe row. These dependency changes are semantic
metadata, not dismissed as identity noise.

At `tail_file.c:1838`, the slice replaces one generic unsupported indirect
access with an unresolved `file` alias leaf, still Unknown. The [one-unit CFG
probe](artifacts/tail-append-actual/identity.json), run from the gate-built
executable without rebuilding, pins `Call tail_signal_manager(file->config)`
with `reads["file->config"]` at L20871/node 94 and its actual-read refusal.
The preceding hash-table call remains outside its literal-argument premise;
all other refusals remain (168 diagnostics). The `ctx` scalar increments now
discharge, but no call effect or actual identity is assumed. The remaining
actual-identity boundary is separately registered below.

All **4,921 / 454 / 406 null-property dictionaries are identical**. Combined
verdicts are identical on every raw row: corpus remains 841 Safe / 4,053
Unknown / 27 provisional Unsafe. The raw null streams retain the excluded
custody records (87 / 6 / 2); they are not custody promotions. All-loaded
runtime custody is corpus **931 Safe / 32 Pending / 3,862 Unknown / 9
provisional Unsafe**, system **99 / 0 / 345 / 4**, slice **53 / 4 / 346 / 1**.
Every runtime moved row, reason and scenario is identical. Corpus runtime
combined remains 808 Safe / 33 Pending / 4,053 Unknown / 27 provisional Unsafe.
Thus the four corpus custody gains establish no new combined-property or
deployment safety. No native violation validation is added.

All five primary artifact families are retained on both sides; `current`
emits no Monkey-source artifact, whose bilateral absence is explicit. Corpus
source callbacks/wrappers, 96 chunk results and 21 teardown results are
unchanged. All 64 normalized contract module texts are identical, with all 60
raw checkout-key renames retained. Three corpus source-effect records change
only configuration/build/task/evidence identities: sources, assumptions,
bounds and witness paths are checked identical after explicit identity
mapping. All 185 unit fields outside function changes, their diagnostic
tallies/reasons and linked-home identities are checked unchanged. Configured
artifacts retain 64 identical module texts, exact empty source/chunk effect
artifacts and all review holds; configuration digests and held module paths
change with checkout provenance. Complete raw and normalized comparisons
remain durable; normalization never changes checker output.

These are deliberately historical **original-envelope** measurements. After
they were frozen, the user requested removing the four Fluent Bit instance.p
kept paths and equivalent English promise from `rk_plugin_library`. That
provider correction is outside this branch and these evidence identities.
The later integration must measure the corrected envelope explicitly.

The [review index](artifacts/review-index.json) accounts for all 121 full-row
changes across the three routes. Matrix regeneration with this card's gate
receipt exits 0; the required post-matrix Python replay passes **374 tests**
(exit 0, 26.133 seconds). Report-index, whitespace and implementation-equivalence
checks also exit 0. The global residual sweep remains 192 names with no newly
untracked name; the [closeout receipt](artifacts/closeout/commands.json) retains
commands, exits, private logs and its corrected usage attempt. The original
card is retired, with two bounded residuals retained/registered: ledger
**797 → 798**. Exactly one increment is added.

## Follow-ons and integration

`LEXICAL-SCALAR-NOOP-ALIAS-READ` is a separate observed analyzer-boundary
candidate, not a validated violation or an upstream defect: releasing a
same-type formal peer then storing a scalar through an unbound formal lowers
the store to the older `NoOp`, where this card is never consulted.
[The observation](artifacts/noop-alias-observation.json) records the exact
synthetic source, custody profile, raw failure log and source-state boundary.
Discharge needs exact-base reproduction and an independently reviewed
alias-preserving reading of that older route, with matching Lean and a re-pin.
This card does not silently change established `NoOp` admissions.

The integrator should preserve the rebased RK tracking and runtime projection
metadata, then run the normal failure-gated integration discipline. No provider
sources are changed by this card; provider measurements belong to the combined
base gate, with explicit borrowing provenance if quoted here.

`LEXICAL-TYPED-TAIL-APPEND-ACTUAL-IDENTITY` retains the conservative actual-read
refusal at `tail_file.c:1838`. A future source/CFG review must independently
discharge the file identity and every preceding effect/literal-argument
premise, or state why it must remain refused; it must not turn the old generic
refusal into assumed safety.

## For the integrator

Merge with the provider envelope correction while preserving this report's
frozen original-envelope identities. Resolve STATUS, the matrix, current
increment and rule/import conflicts by keeping both cards' provenance. Run
the standing integration gate and corrected-envelope three-route measurement;
the two registered residuals above remain open. No provider source is changed
by this branch, and no deployment is certified.
