# EXT-02 contract source migration

Base `e7341b1debb75002e65645ff713bdc3ebda52fb7` includes the integrated
compatibility readers and the [physical migration prediction](../extension-physical-migration/prediction.md).
This card's physical edits are held until the integrator reports the EXT-01
standing gate passed. Its preparation does not alter a source, reference or
lock.

## User-authorized archive policy change (before mutation)

The user subsequently directed outdated frozen historical files to be changed
or removed to prevent confusion. This instruction overrides the repository's
general frozen-file preservation rule for the extension migration. The contract
rename set now includes all **195** mapped `.qv` targets: the original 177 live
sources and 18 files under `docs/reports/`. Sixteen archived files parse as
contract mode; two are deliberately malformed attribution fragments with a
contract header and remain malformed after their path changes. The original
source bytes and original measurement labels/digests will be recorded beside
the migrated names. Historical reports must not claim a past run used a new
path or digest. Executable historical commands and links that target one of
these sources should point to the new path with explicit provenance for their
original spelling. No report directory is removed wholesale.

## Before-change evidence

The [parser-derived mapping](../extension-compatibility/mapping.tsv) names
195 `.qcore` contract files proposed for `.qv`: 177 live sources (123 under
`providers/`, 41 under `crates/`, 11 under `compiler/`, one under `contract/`,
and one under `examples/`) and 18 archived sources. Of these, 187 parse
directly as contract mode, six use recognized include syntax, and two archived
contract-header fragments are intentionally malformed.
The [baseline](baseline.json) records every old/new path, Git index blob and
source SHA-256. Six sources include nine `#include` targets that must change
from `.qcore` to `.qv`; the baseline also records the expected SHA-256 after
exactly those nine edits. It records all ten provider revision locks, with 125 pinned
and 105 flexible entries. Exactly 230 lock module fields point to mapped
paths. Every entry's declaration digest and, for pinned entries, reviewed
source identities are captured for a field-by-field comparison. The
[verification tool](verify_paths.py) refuses any other changed source byte, a missing
or added rename, a changed lock count, digest, reviewed identity or metadata.

The [reference candidates](reference-candidates.tsv) list 222 tracked,
non-frozen files containing an exact mapped source path or a provider module
name. This is a review queue, not permission for global string replacement:
some rows retain intentional legacy fixtures, historical commentary or
virtual names, and two logical module names appear in more than one source
scope. The migration must update active includes, provider exports, corpus
selection, package/load-path inventories, revision-lock module fields, and
tests/current code references according to each consumer's actual target.
The [historical candidates](historical-reference-candidates.tsv) record 147
possible references to the 18 archived sources across 35 report files. Some
are historical measurements or the original EXT-01 inventory and must retain
their original identity claims; executable commands and live links to the
renamed source need targeted edits and a provenance note.

## Required sequence after the compatibility gate

1. Record a pre-change `quirin-audit pin --verify-contracts` result on the
   old load path and archive its log. Then rename exactly the mapped files;
   preserve bytes except the nine mapped include target suffixes.
2. Rewrite includes and active exact references in the appropriate scope.
   Use [the guarded lock module migrator](migrate_lock_modules.py) to change
   each lock's `module` field by its owner crate's mapping only; keep
   all other fields, including declaration digests and reviewed identities,
   byte-equivalent as data. Do not regenerate or re-pin the lock.
3. Run `python3 providers/contracts.py sync`, then `check`, to regenerate
   declared package inventories and the load-path manifest from updated
   provider exports. Attribute every package digest change to the renamed
   path or changed include text.
4. Run `verify_paths.py verify`, the post-change pin verification, focused
   contract and provider tests, and the standing and provider gates in the
   integration sequence. Compare corpus and audit outcomes before claiming
   parity. A failing pin or Unknown-to-Safe movement blocks the card.

Preparation commands: `git submodule update --init --recursive` exited 0
at libevent `d82464a277d0f42703702c4dfd9af6af38595a83`;
`python3 docs/reports/extension-contract-physical/verify_paths.py baseline`
exited 0 and recorded 195 sources and ten locks. No cargo gate has run on this
preparation worktree; the integrator coordinates the warm target and physical
change start.
