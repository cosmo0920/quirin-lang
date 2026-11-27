# Opt-in `current` runs (Fluent Bit 046accf6)

Script: `docs/reports/openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh`
(`SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's user load
path and `opt_in_providers.py` selection, `--jobs 12`) over the prepared inputs
at `/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`, used directly
(their compile-database prefix was fixed before this card; no run failed on
paths). Binaries: `cargo build --release -p qcore-audit --bin quirin-audit`
([../scripts/build.sh](../scripts/build.sh)), exit 0 each. Each run reads its
own revision's tree (`git archive`). Logs and reports in this worktree's
`private/oom/run-*` (not committed).

| run | tree | binary sha256 | started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `8e7e8d509` | `512d7a69…` | 01:01:29 / 02:01:24 | 0 | 1,159 / 2 / 3,636 / 7 | 2,135 | 209 | 9,027 |
| tip 1 (not evidence) | `bedc1a5c1` | `6070c09a…` | 01:33:06 / 02:29:07 | 0 | 1,159 / 2 / 3,636 / 7 | 2,135 | 209 | 9,027 |
| **tip 2** | `af8c51fc0` | `6070c09a…` | 02:33:42 / 03:14:26 | 0 | **1,169 / 2 / 3,626 / 7** | **2,125** | **200** | **9,011** |

A first base attempt (`private/oom/run-base-failed-relative-bin`) passed the
binary by a relative path the script's `cd` broke: exit 127 before analysis, not
evidence. Tip 1's tree did not select the card's modules in
`cycle19-resource-contracts.json`, so no new declaration was consulted (641 on
both) and its report equals the base row for row; tip 2 is the same binary on
the tree that selects them.

Console dependency lines: base `1159 Safe row(s): 439 on no declaration, 124
only on validated declarations, 44 on a partly validated declaration, 536 on a
declared assumption, … 16 only on a null premise (38 carrying one); 641
declaration(s) consulted (75 validated, 52 partly validated)`; tip 2 `1169 …,
546 on a declared assumption, … (42 carrying one); 646 declaration(s)
consulted`. `refuted contracts: 0` on both.

Files: [totals-base.json](totals-base.json), [totals-tip2.json](totals-tip2.json)
(`scripts/totals.py`), [moved-rows.jsonl](moved-rows.jsonl) (`scripts/moved.py`),
[changed-rows-base-tip2.txt](changed-rows-base-tip2.txt) (`scripts/changed_rows.py`:
every row whose status or leaves differ). `totals.py`, `moved.py`, `rows.py` are
round 4's and `changed_rows.py` CG10-MEMBER-STORE-RETAINED-BY's, copied
unchanged.
