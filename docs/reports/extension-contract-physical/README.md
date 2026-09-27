# EXT-02 contract source migration

Base `e7341b1debb75002e65645ff713bdc3ebda52fb7` includes the integrated
compatibility readers and the [physical migration prediction](../extension-physical-migration/prediction.md).
This card's physical edits are held until the integrator reports the EXT-01
standing gate passed. Its preparation does not alter a source, reference or
lock.

## Before-change evidence

The [parser-derived mapping](../extension-compatibility/mapping.tsv) names
177 live `.qcore` contract files proposed for `.qv`: 123 under `providers/`,
41 under `crates/`, 11 under `compiler/`, one under `contract/`, and one under
`examples/`. Of these, 171 parse directly as contract mode and six use the
recognized include syntax; no frozen report input is in the rename set.
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

## Required sequence after the compatibility gate

1. Record a pre-change `quirin-audit pin --verify-contracts` result on the
   old load path and archive its log. Then rename exactly the mapped files;
   preserve their bytes and every frozen report file.
2. Rewrite includes and active exact references in the appropriate scope.
   Change each lock's `module` field by its owner crate's mapping only; keep
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
exited 0 and recorded 177 sources and ten locks. No cargo gate has run on this
preparation worktree; the integrator coordinates the warm target and physical
change start.
