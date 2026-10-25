# GENERATED-ROOTS: generated Lean roots and rule documents

Card of refactoring round 1, seams C and D of the
[pluggable-cards plan](../../plans/pluggable-cards/README.md). Base `main`
`fb5412930`. No analyzer, contract, theorem or verdict change, so no identity
run; the evidence is that the generated files reproduce what they replace.

## Why

Almost every implementation card edited the same four shared files, and they
conflicted at every landing: `proof/QCore.lean` (one import line),
`proof/AuditAll.lean`, a numbered constraint section of
`docs/agents/gateway.md` and a row of the router `docs/agents/README.md`. Now a
card adds its own files and reruns two generators; a hand edit to a generated
file fails the Python suite.

## Lean roots (seam C)

`scripts/gen_lean_roots.py` writes `proof/QCore.lean` (a generated-file
header, `import` of every module under `proof/QCore/` in sorted order, the
module docstring) and `proof/AuditAll.lean` (header plus its unchanged audit
text; it imports only the root, so nothing in it is per card).

**Order does not matter, so there is no order file.** Lean 4 builds each module
from its own `import` lines; a root's import list fixes no build order and no
declaration, and modules cannot form a cycle through it. Import order could
only affect elaboration in a file that imports the root (instance and
attribute ordering); the only such file is `AuditAll.lean`, whose detector
self-test and audits ran green on the sorted root (gate below).

**Two modules were not in the old root**: `QCore.DescriptorCustody` and
`QCore.ModuleImports`. Both were already reached transitively (through
`QCore.StdEvent` / `QCore.SocketEventCompose` and `QCore.ConcurrencyPrelude`
among others), so importing them directly adds no declaration. The old root
also repeated its docstring and had stray blank lines; the generated one has
neither.

**Identical declaration set, measured.** After `lake build QCore` at base and
at the generated root, `scripts/DumpDecls.lean` (run by `scripts/dump.sh`)
listed every constant of the environment importing `QCore`, with its kind:

| Set | Base | Generated root | sha256 (both) |
|---|---|---|---|
| Audited (`QCore.*`, `_private.QCore.*`, the set `AuditAll` checks) | 102,367 | 102,367 | `c759fe0c712d85f685fc59e9630c665d5eb26c009ed24376d6a01618f88c3275` |
| Whole environment | 310,354 | 310,354 | `9fc02cc2c983500ac0c57eeca3c8676a079938650cdcb19f2a708291f8ebbbef` |

The two lists are byte-identical (`cmp`). `check_lean_audits.py` discovers
the same audits, by the same `Audit*.lean` glob: 370 files at base and tip
(the card brief's 369 was one short; no `Audit*.lean` file was added or
removed).

## Rule documents (seam D)

All topic pages are done, not only `gateway.md`. Each numbered or lettered
constraint the router indexes (210 rules on 11 pages) plus gateway's
`11-series` lead-in is one file `docs/agents/rules/<id>.md` (211 files,
706,607 bytes of rule text), with front matter `rule`, `topic` and `summary`
(the router's navigation-only sentence, absent for `11-series`). Format and
link resolution: [rules/README.md](../../agents/rules/README.md).

`scripts/gen_rule_docs.py` rewrites only the text between `generated:rules`
markers on each topic page (anchor plus rule text, in natural rule order
4, 4a, …, 10z, 11-series, 11a, …) and the stable rule index table between
`generated:router` markers in the router. Everything outside the markers
(preambles, gateway's compaction note, the amendment sections of
`buffers-records.md`, `source-routes.md`, `diagnostics.md` and the trailing
sections of the other pages) stays hand-edited and unchanged. The one-time
split is `scripts/split_rule_docs.py`; its segmentation (a rule runs from its
anchor to the next rule anchor, another anchor or a heading) was checked to be
contiguous on every page.

Evidence (`scripts/verify_split.py`, against `fb5412930`, 0 failures):

- every rule file's text occurs byte for byte in its base page, immediately
  after its anchor and one blank line;
- every page equals its base page after deleting the marker lines and blank
  lines; the only blank-line edits are eight places in `gateway.md` where a rule
  was followed by two blank lines (5e, 10y, 11p) or none (10x, 11x, 13z, 14z,
  36a) and now by one, and the end of `gateway.md` and `io.md`;
- every `<a id=...>` anchor (gateway 170, all pages included) and every
  `<!-- preserved:... -->` marker is present the same number of times;
- the router's 210 rows are the base rows as a set. **Their order changed**:
  the table is now in natural order, so rows 13, 14, 15v and 15q–15z moved to
  their numeric places (they had been appended out of order). An order file
  would have reintroduced the shared edit this card removes.

Links: `scripts/constraint_links.py` lists every link in a tracked file to a
`docs/agents/*.md#constraint-*` anchor. Base: 603 links, 210 distinct
targets, 0 unresolved. Tip: 623 links (the 20 added are the rule files' own
links, resolved from `docs/agents/`), 210 distinct targets, 0 unresolved.

Tests (`scripts/tests/test_generated_roots.py`, 11): committed roots equal
generated; every module imported once; committed pages and router equal
generated; every router row has a rule file on the same page and every rule
with a summary has a row; every rule anchor occurs once on its page; natural
order; on a temporary copy a new rule lands in order with a router row while
the trailer is kept, a hand edit is stale, and six malformed rule files fail
closed. Inputs come from `tracked_files.py` (constraint 29); the generators
themselves read tracked plus untracked-not-ignored files
(`scripts/listed_files.py`), so a card's new file is seen before `git add`
while `private/` and `.lake/` never are.

## Card workflow

[Operations](../../agents/operations.md#card-briefs-and-delegation) now says
how a card adds a rule and a Lean module. Generated files still conflict
textually when two cards add neighbouring entries (two new rules both append
after rule 37; two modules adjacent in sorted order); the resolution is to
rerun the generator, which the integrator can do without reading either side.

## Gate

Measured tip `9caee67ba`: the card commit merged with `main` `1e3967b97`
(GATE-ELASTIC landed meanwhile). The merge's only conflict was
`docs/agents/validation.md`, where GATE-ELASTIC inserted the scoped-gate and
slot sections before rule 29; resolved by taking `main`'s page and re-inserting
the `generated:rules` markers around rule 29, whose text `main` did not change.
`verify_split.py` against `main` then passes for every page except
`operations.md`, which differs only by this card's note; links 623, 0
unresolved. `main` added no module under `proof/QCore/`, so the roots stayed at
426 modules.

`python3 scripts/gate_scope.py --base main --full` (the whole suite; the scope
itself was full, since the proof roots are read by gateway, contract and audit
tests), [gate.json](gate/gate.json), all 8 steps exit 0:

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1, none moved |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| Rust suite (`run_test_binaries.py` under the `rust-suite` slot) | 0 | 550 units, 563 summaries; 8,645 passed; 0 failed; 15 ignored (5,343 s, most of it waiting for audit slots) |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `check_lean_audits.py` (`lean` slot) | 0 | 370 audits; AuditAll: 102,367 declarations axiom-checked, 20,186 public theorems statement-checked |
| `python3 -m unittest discover -s scripts/tests` | 0 | 443 tests |

Logs: `private/generated-roots/gate-scope/` in the card worktree. Clang
recording beside the gate file. An earlier full gate on the unmerged card
commit `e5e031a17` was green through the reevaluation examples (549 units,
8,639 passed, 0 failed) and was stopped before Lean when the merge was
requested; it is not the evidence here.

## Follow-ups

- `RULE-AMENDMENTS-OUTSIDE-RULE-FILES`: amendment sections that qualify a
  rule but sit outside its file (`buffers-records.md` and `source-routes.md`'s
  `constraint-15k-amendments`/`constraint-14-amendments` and post-migration
  sections, `effects.md`'s amendment to 15m, `diagnostics.md`'s namespace and
  amendment sections, `providers.md`'s CONTRACT-INHERIT amendment, the
  trailing sections of `concurrency.md`, `evidence.md`, `source-identity.md`
  and `validation.md`) remain hand-edited page text, so a rule file is not yet
  the whole authority for those rules. Discharge: fold each into its rule file
  (or its own rule) in a documentation card.
- The task-context packet (`scripts/task_context.py`) still reads whole topic
  pages, which remain complete; reading per-rule files instead is a possible
  retrieval saving, not measured here.
