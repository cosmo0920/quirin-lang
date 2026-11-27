# Opt-in `current` runs (Fluent Bit 046accf6)

The scripts are [../scripts/run.sh](../scripts/run.sh) and
[../scripts/opt-in-current.sh](../scripts/opt-in-current.sh), taken from
RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT. Only this worktree's paths are
changed in `run.sh`; `opt-in-current.sh`, `totals.py`, `moved.py`,
`changed_rows.py` and `rows_by_unit.py` are verbatim copies. `census.py` and
`leaf_diff.py` are this card's.

Each side is a `git archive` of the measured revision in its own tree, with
its own release `quirin-audit`, run under `scripts/gate_slot.py audit` with
`SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's user load
path and `--jobs 12`, over
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/unj/run-*` and are not committed.

| run | tree / binary | sha256 (binary) | route started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `2a598d2af` | `20eb3fe7aa4fc96b504f66332edd81224f2bbdb3c40980825b62710f452d976f` | 11:14:41 / 11:48:35 | 0 | 1,171 / 2 / 3,623 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `bd9aa123e` | `6ca783bc15a20bc44e6d243f0323f168b873d1165c4dd4d68f9853d08fad7264` | 12:05:41 / 12:39:43 | 0 | **1,171 / 2 / 3,623 / 8** | 2,118 | 215 | 8,994 |

On both sides the build, `providers/package.py checkers` and route steps
exited 0. The console's dependency line is identical on both sides
(`… 16 only on a null premise (44 carrying one)`). Both slot receipts note a
peak resident set (about 10 GB) above the 7 GiB admission budget; the runs
completed.

Evidence files:

* [totals-base.json](totals-base.json), [totals-tip.json](totals-tip.json)
  (`totals.py`);
* [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl) (`moved.py`):
  `{"moves": {}}`;
* [rows-by-unit-base-tip.jsonl](rows-by-unit-base-tip.jsonl)
  (`rows_by_unit.py`): 37 changed rows, all `Unknown` on both sides;
* [leaf-diff-base-tip.jsonl](leaf-diff-base-tip.jsonl) (`leaf_diff.py`): 15
  rows gain 36 diagnostics, none removed;
* [census-base.jsonl](census-base.jsonl) (`census.py` over this card's base
  report): 502 premise rows, 44 `Safe`, 2 `Safe` null-writing candidates, as
  the prediction's census over RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT's tip
  run found.
