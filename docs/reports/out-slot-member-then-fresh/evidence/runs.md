# Opt-in runs

`sh run.sh NAME TAR` (`docs/reports/fb-retry-or6-element-identity/scripts/run.sh`
and `opt-in-current.sh`, with the worktree path and the archived
`openssl-provider-plugin/evidence/opt_in_providers.py` path changed). Each
revision runs from its own `git archive` tree with its own release
`quirin-audit`, using `current --monkey-linked` over the prepared Fluent Bit
`046accf6` inputs (`private/fb-046accf6`, read-only) and `--jobs 12`. Outputs
are in the worktree's `private/m/run-NAME/`.

| run | binary sha256 | started / finished (UTC) | exit |
|---|---|---|---:|
| base `287dce8ed` | `6c64ced465fed5e7…3772a733` | 2026-11-27 23:57:25 / 2026-11-28 01:04:46 | 0 |
| tip `6aa742484` | `54dfa7d550cecfaa…518c8330f7` | 00:07:44 / 01:12:21 | 0 |

The two binaries differ in bytes. They were built from different extraction
directories, and the tip tree changes only one test file, which is not part
of the binary. The raw reports (`resource-diagnostics.json`, not committed)
have these sha256 digests:

* base: `72085c82e543fe340223bfa4ec4d387536edb0eecace8a40f0c0af89746538e2`
* tip: `17035324a5804cf6a7aa7964b2ca02742fd6f4a1e1d5e341c68d191b6089c317`

The digests differ, but no row's status or leaf multiset does.

Totals ([totals-base.json](totals-base.json), [totals-tip.json](totals-tip.json)):
both runs have 1,209 Safe / 2 Pending / 3,586 Unknown / 7 Unsafe, with 2,091
access rows, 201 alone and 8,909 leaves.

[moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl) is `{"moves": {}}`.
[changed-rows-base-tip.txt](changed-rows-base-tip.txt) is empty: no row
changes status or any leaf. [named-rows.txt](named-rows.txt) lists the two
rows and their forwarding caller, identical at base and tip.

## The implementation (USER DECISION 2026-11-28, (a1)+(b))

| run | binary sha256 | started / finished (UTC) | exit |
|---|---|---|---:|
| tip `b1cfce380` | `c1c04fa8549b562d…` | 2026-11-28 03:03:42 / 03:53:37 | 0 |

The tip was compared against the same base run. Its raw report digest is
`bff8f6b1f623b748d05da3737c00e01a5fb495ef15ee91d6b7bb2457dc501550`.
[totals-tip-b1cfce380.json](totals-tip-b1cfce380.json) gives 1,209 / 2 /
3,586 / 7, with 2,091 access rows, 201 alone and 8,909 leaves.
[moved-rows-base-tip-b1cfce380.jsonl](moved-rows-base-tip-b1cfce380.jsonl) is
`{"moves": {}}`. [changed-rows-base-tip-b1cfce380.txt](changed-rows-base-tip-b1cfce380.txt)
is empty. [named-rows-tip-b1cfce380.txt](named-rows-tip-b1cfce380.txt) lists
the two rows and their forwarding caller, with leaves identical to base.
