# LANDING-PUBLISH integration

Merged card tip `14eab6610` into main `da13b7d12` as `52029e5c1` at the
user's explicit request. The `scripts/land.py` conflict combined the
CONTRIBUTION-LEDGERS contributions-page regeneration with baseline publication
after the committed records and Python suite. No analyzer semantics changed.

The full landing gate uses fresh logs under
`private/measure-tool-landing-publish/landing-1/`. Each command's exit is saved
beside its log; `raw-gate.json` records the commands, counts and slot receipts.
The Python suite runs after the regenerated matrix is committed.

## Cancellation correction

The card report's exit-130 observations describe the interrupted parent
commands. Host process inspection during integration found their child
processes still alive: measure plugin PID 3083741 with immutable `rev:` sides,
and scoped gate PID 3088618 with slot child 3089811. The latter tree was
explicitly terminated after matching its command and private output path.
The immutable snapshot measurement was retained for collection. Thus exit 130
did not establish complete process-tree cancellation; the new named follow-up
`MEASURE-TOOL-PROCESS-TREE-CANCELLATION` records that defect. No other
worktree's process was signalled.

## Publication identity

The final records commit is the landing revision published by
`python3 scripts/land.py publish`. Its private immutable
`private/baselines/<sha>/<profile>/baseline.json` records own the exact
revision, inputs, bundles and report digests. Publication runs after the full
gate and committed record check and before pushing main. The original
LANDING-PUBLISH follow-up remains open for a subsequent card's verified reuse.
