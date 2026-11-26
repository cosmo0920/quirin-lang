# Opt-in `current` runs (Fluent Bit 046accf6)

The scripts are [../scripts/run.sh](../scripts/run.sh) and
[../scripts/opt-in-current.sh](../scripts/opt-in-current.sh), taken from
RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST. Only this worktree's paths are
changed in `run.sh`. `opt-in-current.sh`, `totals.py`, `moved.py` and
`changed_rows.py` are verbatim copies. `census.py` and `rows_by_unit.py` are
this card's.

`run.sh` extracts a `git archive` of the measured revision into its own side
tree, builds that tree's release `quirin-audit`, and runs the opt-in route
with `SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's user
load path and `--jobs 12`, over
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/rsm/run-*` and are not committed.
The tip run overlapped the first scoped gate.

| run | tree / binary | sha256 (binary) | route started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `497c859fc` | `2e8a716ed4daeb527b3075b415388b6b24b5fd4232a839f6e1addd5402be8374` | 05:43:50 / 06:48:15 | 0 | 1,171 / 2 / 3,623 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `8434d92a3` | `305373bdfa8d3795b829f138b7afd90dfbffd58e0f031f376d67c3e81e294626` | 06:56:58 / 08:02:38 | 0 | **1,171 / 2 / 3,623 / 8** | 2,118 | 215 | 8,992 |

On both sides the build, `providers/package.py checkers` and route steps
exited 0. The console dependency line differs in one figure:
`… 16 only on a null premise (42 carrying one)` at the base and
`(44 carrying one)` at the tip. The route reports 181 units (180 distinct
files, because `src/flb_input_chunk.c` is reported twice).

The evidence files are:

* [totals-base.json](totals-base.json) and [totals-tip.json](totals-tip.json),
  from `scripts/totals.py`;
* [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl), from
  `scripts/moved.py`, which reads `{"moves": {}}`;
* [changed-rows-base-tip.txt](changed-rows-base-tip.txt), from
  `scripts/changed_rows.py`, which is empty. No row changes its status or
  its leaf multiset;
* [rows-by-unit-base-tip.jsonl](rows-by-unit-base-tip.jsonl), from
  `scripts/rows_by_unit.py`, keyed by unit index. It also compares contract
  dependencies. It finds the two rows that gain a null premise: both units
  of `flb_input_chunk_get_name`, which stay `Safe` with
  `null-deref:flb_input_chunk_get_name:20522:ch`;
* [census-route.txt](census-route.txt), from `scripts/census.py` over the
  base report, with 169 hits.

The rows also differ in `hypothesis_context.evidence_version`,
`reading_site_evidence` and the `plugin_links` package digests. These name
the measured revision and the side tree's packaged providers, and are not
verdict data.
