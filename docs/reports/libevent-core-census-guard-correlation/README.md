# LIBEVENT-CORE-CENSUS-GUARD-CORRELATION

Card LIBEVENT-CORE-CENSUS-GUARD-CORRELATION (named follow-up registered by
TAIL-FILE-DEFINITION-CENSUS-INCOMPLETE). Measurement only; no code change.
Base `fe6d5760d` (the integration line's revision immediately before rule 49
entered it: the first parent of `15d072e45`, which merged the TAIL card into
integrate/r11); tip `0f6e17118` (main when the card started). The
[prediction](PREDICTION.md) was committed at `d0a4da43b`, after the base run
and before any tip run.

## Headline

* **Census.** Both refusals are gone: `buffer.c` and `bufferevent_sock.c`
  now have `completeness: established`, no refusal. Route totals: refused
  units 2 → 0, `absent_from_raw_scan` 62 → 21 (the 28 and 13 names now come
  from the raw half; `event.c`'s 13 and `evutil.c`'s 8 are unchanged).
  Denominator 727 → 729: `buffer.c` gains `evbuffer_cb_suspend` and
  `evbuffer_cb_unsuspend`, which sit inside `#if 0` (lines 3481–3505). They
  are `not_projected` (excluded from the build and kept in the denominator, as
  the route's other guarded-out definitions are; 49 → 51).
* **Verdicts.** No unit row and no function row moves. 21 units: 0 Safe /
  0 Pending / 0 provisional Unsafe / 21 Unknown on both sides. 678 function
  rows: 80 Safe / 2 Pending / 596 Unknown on both sides. With the run
  directory and the build commit normalised, the only differences on any row
  are the enumeration blocks of the two units, `buffer.c`'s
  `source_functions`/`not_projected`, and the `hypothesis_context.scope` of
  `buffer.c`'s 101 rows (the unit's function list, now with the two `#if 0`
  names). This is what the prediction said, line for line. The declared block
  (`route libevent-core` / `none`) was checked by hand against
  [compare.py](scripts/compare.py), not by the measure plugin (section 2):
  0 moves, so it matches.
* **Per unit.** Both changes are a correct decision that is now available;
  neither is still refused; nothing is suspicious and nothing moved toward
  Safe. No defect in rule 49 was found, so nothing was fixed.
* **Found on the way: the route does not load as committed.**
  `contract/profiles/libevent-core/contracts.json` still names
  `libevent_memory.qcore`, but `f4018a925` ("Migrate contract sources and
  lock paths to .qv") renamed the file to `.qv`, so `quirin-audit` stops with
  `audit rejected: …/libevent_memory.qcore: No such file or directory` at
  both revisions (run `base-1`, kept). The measurement used a per-run copy of
  each side's own profile directory with only that one module name
  corrected. This is a follow-up, not this card's fix (section 5).

## 1. Inputs

* **Trees.** `git archive` exports of `fe6d5760d` (`private/base-src`) and
  `0f6e17118` (`private/tip-src`). In each, `third_party/libevent` is a real
  clone checked out at the pin `d82464a2`; the pin is the same on both sides.
  `quirin-audit` (with `quirin-audit-plugin-libevent`) was built in place,
  `cargo build --release --locked`, with `QCORE_AUDIT_BUILD_COMMIT` set to the
  full revision. Binaries: base `6862e1d2…`, tip `40ea9003…`
  ([digests](artifacts/digests.txt)).
* **Prepared inputs.** One preparation for both sides:
  `quirin-audit libevent prepare` (tip plugin `879b95c8…`, Ubuntu clang 21 at
  `/usr/lib/llvm-21/bin`) into `private/libevent-prep/`. Its
  `compile_commands.json` is `e1ff3134…` (55 entries); log
  `private/logs/prep-libevent.log`.
* **Configuration.** The plugin's `libevent-core-audit.json.config` with
  three changes. Each run gets its own report path. Both sides read the same
  checkout and compile database. The profile directory is a copy of the
  side's own `contract/profiles/libevent-core`, whose `contracts.json` names
  `libevent_memory.qv`
  ([run_side.py](scripts/run_side.py); configs
  [base](artifacts/config-base-3.json), [tip](artifacts/config-tip-1.json)).
  The profile, the audit selection and the contract module are byte-identical
  at the two revisions (`git diff fe6d5760d 0f6e17118 -- contract/` is
  empty).
* **Runs.** `quirin-audit CONFIG.json` (the plain configured route E01-TAIL
  and G03-F ran), each in an `audit` gate slot (`scripts/gate_slot.py audit
  --memory-budget 7G`), `QCORE_CLANG=/usr/lib/llvm-21/bin/clang`:
  `base-3` (exit 0), `tip-1` (exit 0), and the control `base-4` (exit 0, the
  base again in a fresh directory). Outputs under `private/runs/<run>/`. The
  failed `base-1` (the `.qcore` name) and `base-2` (a profile copy whose
  directory name did not match its `source_set`, refused by the profile
  check) are kept and are not evidence.

## 2. Why not `quirin-audit measure --declared`

The measure plugin has two route kinds. `pinned-corpus` runs
`run_pinned_audit.py`. `configured-selection` runs `quirin-audit current
--selection S` and requires a link map. `libevent-core` is neither: it is the
plain `quirin-audit CONFIG.json` route, which selects units from the profile's
library group and the configuration's `compile_target` object prefix.
Expressing it as a configured selection would measure a different route:
`current`'s historical sidecars, an explicit selection file that does not
exist in either tree, and the Fluent Bit corpus link map. The figures could
then not be compared with E01-TAIL's. Two more things stood in the way: a
`rev:` side needs user namespaces, which this host refuses, and the route
cannot load its contracts as committed. So the configured audit was run the
same way at both revisions with the same inputs, and the rows were compared
by [compare.py](scripts/compare.py) and
[sidecars.py](scripts/sidecars.py). A `libevent-core` route kind for the
plugin belongs with making the route live, which is a user decision.

## 3. Comparison

[base-tip.json](artifacts/base-tip.json) (compare.py, every changed unit and
row) and [sidecars-base-tip.txt](artifacts/sidecars-base-tip.txt) (the
report, `.components.json` and `.assumptions.json` after normalisation):

| | base `fe6d5760d` | tip `0f6e17118` |
|---|---|---|
| units (Safe / Pending / prov. Unsafe / Unknown) | 0 / 0 / 0 / 21 | 0 / 0 / 0 / 21 |
| function rows (Safe / Pending / Unknown) | 80 / 2 / 596 | 80 / 2 / 596 |
| refused enumerations | 2 (`buffer.c`, `bufferevent_sock.c`) | 0 |
| `absent_from_raw_scan` | 62 | 21 |
| denominator (`source_functions`) | 727 | 729 |
| `not_projected` | 49 | 51 |
| rows moved | | 0 units, 0 functions |

**What the normalisation removes, and why it is safe.** Every function row on
every unit differs in `hypothesis_context.evidence_version` (the build commit)
and `hypothesis_context.assumptions`. The `.components.json` file also differs
in 21 `resource-contracts` digests. The control shows these are spellings of
the run directory: `base-3` against `base-4`, the same binary and inputs in a
fresh directory, differs in exactly the same `assumptions` digests and the
same 21 `resource-contracts` digests, and in nothing else
([base-base.json](artifacts/base-base.json),
[sidecars-base-base.txt](artifacts/sidecars-base-base.txt)). The contract
module path in the assumptions spells the per-run profile copy. With the run
directory, 40-hex commits and that one digest normalised, the
`.assumptions.json` files are equal, and the base pair is equal everywhere
except the 21 `resource-contracts` lines.

After normalisation, base against tip differs only in these places:

* `buffer.c`: enumeration `refused` (`conditional-arm-depth-disagreement`,
  28 `absent_from_raw_scan`) → `established` (0); `source_functions` +2 and
  `not_projected` +2 (`evbuffer_cb_suspend`, `evbuffer_cb_unsuspend`); the
  `scope` string of its 101 rows lists the same two names. Status,
  diagnostics and contract dependencies are equal on all 101 rows (95
  Unknown, 6 Safe).
* `bufferevent_sock.c`: enumeration `refused` (13 `absent_from_raw_scan`) →
  `established` (0). Denominator 21 = 21. Rows equal (20 Unknown, 1 Safe).

The other 19 units are equal after normalisation. The tip's other gateway
changes since base (member paths, the callback route, contract loading in
r11/r12) do not reach this route, so attributing the change to rule 49 needs
no third side.

## 4. Per-unit reading

**`buffer.c` (correct decision, now available).** The refusing groups are
the two `#ifdef USE_SENDFILE` groups in `evbuffer_write_atmost`. The first
(lines 2572–2577) opens an `else {` in one arm only, and the second
(2591–2593) closes it. E01-RAW-COND refused each group alone, so every later
file-scope definition was read one brace deep and the raw scan saw 73 names. Guard correlation splits on `USE_SENDFILE` and follows both
worlds through both groups. It decides 103 names, the 73 plus 30, and loses
none ([raw-census.txt](artifacts/raw-census.txt),
[raw_census.rs](scripts/raw_census.rs)). 28 of the 30 are exactly the base's
`absent_from_raw_scan` list. Those definitions were already in the
expansion, so the analyzer already read them; only their attribution moves.
The other two are the `#if 0` pair. No configuration compiles them, but
the raw half collects every guarded-out definition by design (rule 49's
claim: every definition any configuration keeps at file scope, guarded-out
arms included, so `#if 0` counts as an ordinary uncorrelated group whose
arms agree). They enter the denominator as build-excluded `not_projected`
names, the fail-closed direction, as the route's 49 other guarded-out
definitions already do. The unit stays Unknown on resource grounds
(`contract-pin-mismatch`; 482 unsupported operations, 345 unresolved
aliases). A complete census removes only the enumeration bar to a whole-unit
Safe, and every other bar remains.

**`bufferevent_sock.c` (correct decision, now available).** It has the same
shape: in `bufferevent_socket_connect` an `#ifdef _WIN32` group
(lines 409–419) opens an `else {` that a second (423–425) closes; a third,
balanced `_WIN32` group follows (427–435). The raw scan
saw 8 names; correlation decides 21, adding 13 that are exactly the base's
`absent_from_raw_scan` list. The denominator is unchanged at 21. The unit
stays Unknown (`contract-pin-mismatch`; 144 unsupported operations,
37 unresolved aliases).

No unit is still refused and none is suspicious.

## 5. Follow-ups and amendments

[integration-amendments.json](integration-amendments.json) retires
`LIBEVENT-CORE-CENSUS-GUARD-CORRELATION` and registers
`LIBEVENT-CORE-PROFILE-CONTRACT-SPELLING`: the committed `libevent-core`
profile names `libevent_memory.qcore`, which no longer exists, so the
configured route refuses before it audits anything. The minimal repair is the
one-word change this card made in its private copies. Whether to repair it,
and whether `libevent-core` should become a measured route (a measure-plugin
route kind for the plain configured route), are decisions the integrator and
the user make. Also noted there: 18 of the 21 units are Unknown with
`reason_code: contract-pin-mismatch` on both sides. This was not
investigated; it is the same at both revisions and outside this card. No
evaluation-profile baseline changes (`profiles: []`). The increment paragraph
is the file's `increment_paragraph`.

## 6. Gate

`sh private/toolchain/env.sh python3 scripts/gate_scope.py --base 0f6e17118
--out private/gate-1` on `901909bce` (the measurement and report commit;
this record commit adds only this section and [gate.json](gate.json)):
**exit 0**, mode scoped (no crate, proof root or provider changed), 3 steps.
Logs are in `private/gate-1/`; the console is `private/logs/gate-1.console.log`.

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 submodule |
| fmt | 0 | |
| Python suite | 0 | 761 tests, 0 skipped |

No Rust suite, Lean audit or provider gate is assigned: the diff is
documentation and report scripts only.
[raw_census.rs](scripts/raw_census.rs) is not a workspace target. It was
compiled only inside the tip export, as `qcore-c-gateway`'s example, and that
does not change the measured `quirin-audit` binary (its digest is the same
before and after).
