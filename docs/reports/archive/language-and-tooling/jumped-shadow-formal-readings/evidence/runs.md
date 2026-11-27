# Opt-in `current` runs (Fluent Bit 046accf6)

[../scripts/run.sh](../scripts/run.sh) extracts a `git archive` of the
measured revision into its own side tree, builds that tree's release
`quirin-audit` and runs [../scripts/opt-in-current.sh](../scripts/opt-in-current.sh).
Both scripts are NAME-KEYED-HOLDER-SHADOWING's, with only the paths
changed. The run uses `SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the
side tree's user load path and `--jobs 12`, over
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6` (read-only).
The logs and reports are in this worktree's `private/jsf/run-*` and are not
committed. The base and first tip runs overlapped in time, and the first
scoped gate ran alongside them. `befb0df69` adds only the prefilter that
asks no Clang reading where the body names no parameter; its run is
row-for-row identical to the first tip's (the same totals, no status move,
and `changed_rows.py` output byte-identical to
[changed-rows-base-tip.txt](changed-rows-base-tip.txt)).

| run | tree | sha256 (binary) | started / finished (UTC) | build / checkers / exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---|---|---:|---:|---:|
| base | `4fef267f4` | `80650c315cb3549cb9f350bc680be2f13f939a675aed759d22cbbd256bed17c4` | 02:55:05 / 03:46:16 | 0 / 0 / 0 | 1,187 / 2 / 3,607 / 8 | 2,105 | 215 | 8,917 |
| tip (first) | `243556432` | `7dc40e821b0c3a48acb08f83aec7a59b287010e5f540eb54643eb764e01505e2` | 03:06:18 / 03:54:36 | 0 / 0 / 0 | 1,187 / 2 / 3,607 / 8 | 2,105 | 215 | 8,917 |
| **tip** | `befb0df69` | `b095f1e50a1609c79e293e5502f5ed2ab50f8e1548bfa774cc59b31c21b4cced` | 05:39:03 / 06:27:28 | 0 / 0 / 0 | **1,187 / 2 / 3,607 / 8** | 2,105 | 215 | 8,917 |

The console's contract-dependency line is identical on both sides: `1187
Safe row(s): 448 on no declaration, 125 only on validated declarations, 45
on a partly validated declaration, 552 on a declared assumption, …`.

Files:

* [totals-base.json](totals-base.json) and [totals-tip.json](totals-tip.json),
  from `scripts/totals.py`.
* [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl), from
  `scripts/moved.py`. It records no status move (`{"moves": {}}`).
* [changed-rows-base-tip.txt](changed-rows-base-tip.txt), from
  `scripts/changed_rows.py`: two rows, both `Unknown -> Unknown`, leaves
  only (librdkafka `rdkafka_admin.c`, `rd_kafka_CreateAclsResponse_parse`
  and `rd_kafka_DeleteAclsResponse_parse`, each gaining the entry refusal
  for the formal `errstr`, which a block-scope `char *errstr = NULL;` shadows (the lexical guard already refused it there);
  the second also gains `unresolved resource identity reply`, a
  consequence of the entry refusal's unbounded writes).
