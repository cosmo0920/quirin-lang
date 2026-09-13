# Development documentation structure and retrieval

Date: 2026-09-13. Original document checkpoint: `1e56ba3b`.
Scope: documentation structure, links and reading policy; no compiler,
contract, proof, verdict, conformance or acceptance-matrix changes.

## Result

`DEVELOPMENT.md` is now the stable development table of contents. Four focused
guides cover references, repository structure, recorded cycle evidence, and
build/platform setup. The increment index routes to individual waves, early
integration groups and labelled historical guidance. The source was split
into 41 payload documents, with one additional directory index.

The root guide is **6,966 bytes**, down from **362,048 bytes**: a **98.08%**
reduction in the bytes needed to read that entry point. The largest child page
is **21,199 bytes**. These are UTF-8 document measurements, not runtime tokens,
total repository storage or billed usage. `AGENTS.md` still carries the full
standing constraints and its startup size is not reduced by this change.

The main guides now apply the compact-reading strategy: choose a topic or wave
from the index, retrieve matching sections, and keep current rules distinct
from historical checkpoints. A brief for a resumed branch must include its
base-to-HEAD changes as well as local/planned paths and topic. The automated
packet generator and always-loaded constraint routing remain separate work in
the restart plan.

## Preservation and navigation

[migration.json](migration.json) records the original SHA-256, introduction,
ordered section ranges, payload hashes and each presentation change.
[preservation.json](preservation.json) records the verified measurements.

- Reversing the declared transformations and concatenating the payloads
  reconstructs the complete original 362,048-byte document, byte for byte,
  also checked against Git at the original checkpoint.
- All 28 original headings remain in the root guide, preserving fragment
  landings. Historical compatibility links jump to the matching child heading.
- All 70 original relative-link occurrences still resolve to their original
  targets. Sixty-eight were rebased after relocation; the two overview links
  to root README/AGENTS retain their targets. The external link is unchanged.
- Root traversal reaches all 42 development pages. Link verification covers
  353 local links and their Markdown fragments in the new tree and main guides.
- All 44 standing constraints, the six hard invariants, the standing gate and
  Lean CI hygiene are byte-identical to the source checkpoint.
- The only current-guide text corrections inside migrated payloads are two
  `cargo test --workspace` examples gaining `--no-fail-fast` and three binary
  name corrections to `quirin`. The package name `qcorec` is preserved.
  Historical checkpoint prose is unchanged.
- No pre-existing report or measured baseline is rewritten. The original
  guide's introduction is retained in the migration manifest; the new root
  introduction describes the new navigation structure.

## Reproduction

At the documentation revision that publishes this report, with the original
Git checkpoint available locally, run from the repository root:

```sh
python3 docs/reports/development-docs-structure/verify.py
git diff --check
```

The checker is read-only and prints the preservation measurement as JSON.
Its historical reconstruction and rule checks use `git show` at the source
checkpoint; it does not execute historical instructions. This report records
the migration, so later documentation revisions should not overwrite its
hashes or measurements to describe a different tree.

## Maintenance

The [history index](../../development/increments/README.md#adding-a-record)
names the current record file. Prepend one paragraph per new increment there;
start a new wave file and update the index when the wave changes. Keep the
root table of contents short. New history pages target less than 32 KiB when
a complete record boundary permits it; do not cut a record merely to meet a
byte target.

The four pending implementation worktrees predate this split. Their incoming
`DEVELOPMENT.md` additions must be compared against their own recorded bases
and moved into the current wave page during integration, preserving the root
table of contents. The migration manifest provides the old-to-new line ranges.

The `migrated-section` comments delimit original checkpoint text for this
report. New records go above that original section, as the current index
specifies. Current operational guidance can evolve with an explicit increment;
it must not silently rewrite old evidence or create a second rule authority.

## Validation

Fresh commands, exit codes, counts and log hashes are in [gate.json](gate.json),
with compressed raw output under [logs/](logs/). Tests ran on Linux outside
the managed ptrace sandbox, with compiler/contract/Lean sources unchanged.

| Check | Result |
|---|---|
| Workspace tests, `--no-fail-fast` | Exit 0: 4,473 passed, 0 failed, 8 ignored |
| Formatting and Clippy | Both exit 0 |
| Both reevaluation probes | Both exit 0 |
| Lean audits | Exit 0: 122 passed |
| Python suite | Exit 1: 199 of 200 passed; the existing derived-facts tripwire remains |
| Report-local preservation/navigation verifier | Exit 0; matches `preservation.json` |
| Whitespace check | Exit 0 |

The Python failure is the same parent-checkpoint mismatch already measured:
the acceptance matrix pins 121 Lean audits while the tree contains 122.
This documentation change does not reconcile that integration-owned matrix,
weaken its test, or claim an all-green aggregate gate. Two Sol/medium reviewers
independently checked authority, preservation and root reachability; their
material findings were corrected before publication.
