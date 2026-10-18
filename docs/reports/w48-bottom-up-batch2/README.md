# W48 bottom-up batch 2

The configured 66910c10 corpus gains **four Safe functions**, with no Safe
regression or new provisional Unsafe: **955 → 959 Safe / 9 provisional Unsafe /
3,870 → 3,866 Unknown**, 4,834 total. The four are `gf2_matrix_times`,
`gf2_matrix_square`, `LZ4_compressHC` and `LZ4_compressHC2`. This matches the
frozen prediction's 0–4 range. Base `1543259f0e32c8a003a9062e7de7f59a42dd4519`;
prediction and brief committed before implementation at `0dc191d3f`.

The base is a fresh exact-base arbiter (exit 0, 38m 46s); the tip arbiter also
exits 0 (37m 24s). Both use the conditional W44 route, with the tip extending
the five-home map to six homes. Corpus primary SHA-256 changes from
`43e7a1d9a79bf0396ac8cf6c285663eec645f4836b1f30657f5f8d061423ba58` to
`b06079404e681338a455749ef6e72cb0ad3ad43b275aa3ee985742497b79a1fb`.
All 12 changed rows are archived in [the diff](artifacts/after/compare.json):
19 leaves disappear, three retention leaves become visible, and the other
eight rows stay Unknown. In particular, the three stateful/continuation LZ4
wrappers retain `src` in their state or context and remain Unknown; the two
CRC helpers' callers and Monkey's `check_indent` lose leaves without becoming
Safe. This is static custody evidence, not a bounds or runtime certificate.

The bounded body rule admits a complete typed self-step of a scalar-pointer
formal only when every reference to its exact Clang declaration is a scalar
dereference read or a self-step whose pointer result is discarded, and its own contract fixes neither entry
custody nor an out slot. Copies, pointer actuals, addresses, pointee writes,
rebinding and unresolved references retain the old refusal. The existing
`PointerStep` transfer derives an interior pointer, leaves all custody state
unchanged and refuses losing the sole name of a live object. Its ordinary
statement, callee and caller obligations remain in force. The initial target
is `crc32c.c::gf2_matrix_times` (`mat++`); bounds are a separate property.

The batch link map adds the external Safe `LZ4_compressBound` body in `lz4.c`
to W44's five homes under the same explicit actual-link/type/calling-convention/
interposition premise. Source presence does not prove that premise. Five
legacy `lz4hc.c` wrappers are the measured-base candidates this import serves;
the audit still re-derives home and caller and may expose deeper obligations.

`cJSON_DetachItemViaPointer` remains undecided: success returns exactly the
child it removed from the parent list, while NULL transfers nothing. No
fresh-result or Borrowed-only declaration is introduced. The parent has the
conditional-detach decision options; work on this independent batch proceeds.

The model is `QCore.IndexTraversal.formalStepEligible` and `checkedFormalStep`:
non-scalar, fixed and exposed formals refuse; a checked step preserves the
custody component and caller slots. The Rust gate is bound to all eight Lean
boolean vectors. Extraction of Clang references and Rust/Lean correspondence
remain explicit review premises. The full-body frame is checked before this
exception is considered; unsupported references cannot be hidden by a local
step admission. Existing path, return, own-contract and W4 checks still decide
the function. No new syntax, contract declaration or diagnostic category is
introduced.

Focused validation admits the exact `gf2_matrix_times` body and its caller,
keeps calls/copies/address/storage refused, and keeps explicit Owned and Out
formals fixed. The amended deepest-admitted AST test walks an unused formal
through the entire depth-255 `else if` and depth-253 conditional trees, measuring
236 KiB in each case against the existing 1 MiB budget; the next deeper trees
still refuse. These are model and fixture checks, not corpus coverage claims.

The refreshed ranked plan is archived under `artifacts/{before,after}`:

| Hypothetical planning quantity | Exact base | After |
| --- | ---: | ---: |
| Functions waiting on exactly one in-corpus body | 259 | 259 |
| Candidate bodies | 2,227 | 2,225 |
| No-extra-body closure, same unit | 4 | 4 |
| No-extra-body closure, cross unit | 9 | 4 |
| All-body ceiling | 367 | 362 |

These are hypothetical admissions, not Safe outcomes. Five legacy LZ4 wrappers
leave the no-extra-body plan: two become Safe and three reveal retention.
The four admin wrappers remain hypothetical closure candidates: a Safe home
alone does not establish a usable W4 summary. `cJSON_DetachItemViaPointer`
remains the largest single body (six callers, seven own leaves), pending the
ownership decision. The next ranked bodies and their unresolved families are
in `plan.json.gz` and `family.json`; hooks and allocation choices remain open.
The frozen predecessor figures (260 / 2,228 / ceiling 372, cross-unit 12) refer
to its earlier checkpoint. A broader diagnostic closure of 378 is a different
model: it admits located provisional Unsafe and unanalysed homes that this
ranked plan excludes. `before/closure-model-distinction.json` records the 11
extra callers; 378 is not this card's ceiling.

The final semantic source is `78751c611`; its release executable SHA-256 is
`c6c0a7201b46887a297301833a12ec3ef0ec840257c8a15c1aee7fbdb0802ac0`.
The complete standing gate (`scripts/gate.sh final-v2`) exits 0 at each of ten
steps: submodules, Rust runner, fmt, Clippy with warnings denied, both
reevaluation probes, Lean audits, Python evidence validators, documentation
index and Clang. It records **8,235 passed / 0 failed / 15 ignored** across
513 Rust units and 526 summaries, **337 Lean audits** and **374 Python tests**.
The profiles omit debug symbols and incremental caching after a disk-exhausted
compilation; debug assertions and test controls remain enabled. The amended
focused controls pass (21 formal controls, 22 traversal integration tests and
one deepest-frame test). Provider sources are unchanged; the 133-provider-test
count in `gate.json` is explicitly borrowed from stage 2, not a fresh run.

All raw logs are worktree-private under `private/w48-bottom-up-batch2`:
`gate-final-v2`, `formal-controls-final.log`, `index-final-v3.log`,
`deep-frame-final.log` and `tip-build-final.log`. Compressed gate summaries,
exits and source identity are archived in `artifacts/final-gate`; Clang is
recorded beside `gate.json`.

The pre-tightening full gate passed separately (8,234 Rust tests); it is not
the final source gate. A newly added direct AST control then failed on a
consumed `note(s++)` result, exposing a predicate gap, not a demonstrated Safe
verdict defect. The discarded-result restriction and control pass are included
in the final gate. An earlier compilation exhausted disk space before running
tests; only this card's completed debug cache was removed. Early focused
initializer, fixture and lint failures were repaired. Initial zsh wrappers
also exited 1 after their subprocess because `status` is read-only; the base
audit's independent `run.json` records its actual exit 0. These attempts and
wrapper limitations remain under `artifacts/attempts`; no failed attempt is
counted as a passing gate.

The configured system and slice audits both exit 0. Strict comparison keeps
all duplicate configuration rows: system **97 Safe / 4 provisional Unsafe /
347 Unknown** over 448 rows and slice **52 Safe / 1 provisional Unsafe /
347 Unknown / 4 Pending** over 404 rows. Every status and complete diagnostic
is identical to the reused integration baseline `ad526b58`, whose implementation,
contracts and inputs are equivalent to this card's base. These are fresh tip
runs against explicitly reused base evidence, not new base runs. Both strict
and leaf comparisons exit 0. `artifacts/configured` archives their reports,
input identity, digests, comparisons and reuse provenance. These measurements
cover static custody; no new runtime or null-property claim is made.

Reproduce with `bash docs/reports/w48-bottom-up-batch2/scripts/measure.sh corpus RUN docs/reports/w48-bottom-up-batch2/link-map.corpus.json`, then the same helper
for `system` and `slice`;
its script pins the prepared sources, selections, jobs and report paths. The
base command is recorded verbatim in `artifacts/before/base-provenance.json`.
`python3 docs/reports/w48-bottom-up-batch2/scripts/archive.py BEFORE AFTER`
archives primary reports, identities and contracts and reruns the existing
`bottom_up_plan.py`, `family_leverage.py` and W44 comparator (all exit 0).
The unchanged strict configuration comparator also exits 0 for both routes.
Raw audit paths are `private/w48-bottom-up-batch2/base-corpus`,
`tip-corpus-final/corpus`, `tip-system-final` and `tip-slice-final`. The source,
binary and map identities and native exits are retained with each archive.

The acceptance matrix amends four existing follow-ups; no follow-up is retired.
The source-extraction/Rust-model correspondence premise, derived traversal exit
model and accusing traversal coverage follow-ups remain open. No provider
contract, lexical-route precedence rule, allocator decision or detach semantics
changes in this card.

After the four tracking amendments, `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w48-bottom-up-batch2/gate.json`
exits 0. The required fresh `python3 -m unittest discover -s scripts/tests`
replay also exits 0 (**374 tests**), with raw log
`private/w48-bottom-up-batch2/python-post-matrix-final.log` and an archived
compressed copy. The documentation index and shell syntax checks and
`git diff --check` exit 0 after closeout edits.
