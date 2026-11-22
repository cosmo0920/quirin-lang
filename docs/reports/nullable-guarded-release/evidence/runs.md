# Opt-in `current` runs (Fluent Bit 046accf6)

Script: [../scripts/opt-in-current.sh](../scripts/opt-in-current.sh), a
verbatim copy of the held card's (RK-TOPIC-PARTITION-LIST-CONSTRUCTORS,
`4174d25e5`), itself OPENSSL-SSL-CTX-CALLBACK-SLOTS': `SOURCE_DATE_EPOCH=1767225600`,
`--monkey-linked`, the side tree's user load path, `--jobs 12`, over the
prepared inputs `/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`.
Each run reads its own side tree. Logs and reports are in this worktree's
`private/ngr/run-*` (not committed).

Binaries (`cargo build --release -p qcore-audit --bin quirin-audit` in each
`git archive` tree):

* base `96d41921c`: sha256 `b9793e15130367aafdff52d9702da4713cd7752a8f73ba4f3877c5a55aec4f23`;
* tip `686ddf8f4` (the last commit changing Rust; later commits change rules
  and this report only): sha256 `120fc24171f700c36a2f3f5ae4612a3fa5a4da14f858203d4871d9a19e90ed5d`.

| run | tree | binary | started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `git archive 96d41921c` | base | 10:26:37 / 11:16:46 | 0 | 1,169 / 2 / 3,626 / 7 | 2,123 | 199 | 9,013 |
| tip | `git archive 686ddf8f4` | tip | 12:08:47 / 12:55:28 | 0 | **1,169 / 2 / 3,626 / 7** | 2,123 | 213 | 9,012 |
| base + held (private) | worktree at `96d41921c`, `git merge --no-commit --no-ff 4174d25e5` | base | 10:26:57 / 11:16:58 | 0 | 1,171 / 2 / 3,622 / 9 | 2,117 | 201 | 8,977 |
| tip + held (private) | worktree at `686ddf8f4`, `git merge --no-commit --no-ff 4174d25e5` | tip | 12:08:50 / 12:54:32 | 0 | **1,171 / 2 / 3,623 / 8** | 2,118 | 215 | 8,992 |

Both merges were clean; neither was committed to the card branch. A first
tip pair started at `4d5325d05` (the unnarrowed part A) was stopped before it
finished, when the gateway suite showed that rule withdrawing leaks a refused
release did not cause; it is not evidence.

Console dependency lines: base and tip `1169 Safe row(s): 439 on no
declaration, 124 only on validated declarations, 44 on a partly validated
declaration, 546 on a declared assumption`; both merged runs `1171 …, 548 on a
declared assumption`.

Files: [totals-base.json](totals-base.json), [totals-tip.json](totals-tip.json),
[totals-base-held.json](totals-base-held.json), [totals-tip-held.json](totals-tip-held.json)
(`scripts/totals.py`), [moved-rows.jsonl](moved-rows.jsonl) and
[moved-rows-merged.jsonl](moved-rows-merged.jsonl) (`scripts/moved.py`),
[changed-rows-base-tip.txt](changed-rows-base-tip.txt) and
[changed-rows-merged.txt](changed-rows-merged.txt) (`scripts/changed_rows.py`:
every row whose status or leaves differ).

Census (`--features qcore-c-gateway/indirect-census`, base binary, base + held
tree, the selection `{"librdkafka": ["…/rdkafka_mock_handlers.c"]}`,
`QCORE_LEAK_WITNESS` set; `private/ngr/q-mock-census/witness.jsonl`): the
Metadata leak at preprocessed line 23014 (`:1490`) is `settled: false`,
`unresolved: true`, `status: 3`, with the refused guards at 23011 and 23016
untied (`tied: []`).
