# Binary-count label provenance for the wave-33 close

Published **beside** this directory's frozen close artifacts by card
`DOCS-COMPACT-6` (wave 34), discharging the residual
`LEDGER-BINARY-COUNT-CORRECTED-IN-ONE-FILE-ONLY`. Nothing in `gate.json` or
`README.md` is edited: constraint 21 forbids editing a frozen archive in place,
and the dual publication is what makes the edit unnecessary.

## What the two numbers count

`docs/reports/lean-proof-integrity/validation.json` settles the split, and it
is the authority for it:

| Field | Value | What it counts |
|---|---|---|
| `workspace.test_binaries` | **216** | test **executables** cargo builds and runs |
| `workspace.result_summaries` | **227** | `test result:` **lines** in the run's output |
| `workspace.doc_test_result_summaries` | **11** | doc-test summary lines |

216 + 11 = 227. A binary that carries doc tests prints a second `test result:`
line, so the two figures differ by exactly the doc-test summaries and **both
are correct measurements of different things**. The correct phrasing is
**"216 test executables / 227 result summaries"**, which is what `AGENTS.md`'s
gate row carries.

## The sites this record covers

The wave-33 close predates the split being written down, so four sites in this
directory and its increment record say "227 binaries", meaning
*result summaries*. They are not wrong measurements — they are an imprecise
**label** on the 227 figure:

| Site | Text | Reading |
|---|---|---|
| `gate.json` `note` | "all 227 binaries green in one --no-fail-fast run" | 227 **result summaries**; 216 executables |
| `README.md:169` | "**227 test binaries**" | 227 **result summaries** |
| `README.md:181` | "over 227 binaries" | 227 **result summaries** |
| `README.md:184` | "All 227 binaries were green" | 227 **result summaries**; all 216 executables green |

`docs/development/increments/wave-33.md` carried the same label at two places
and, being the increment history rather than a close artifact, was **corrected
in place** by `DOCS-COMPACT-6` to "216 test executables / 227 result summaries".
That file also held the corrected and uncorrected labels side by side — the
`d3225f91` union merge took line 37 from one parent and lines 98/110 from the
other, and neither parent contained both.

## What is not claimed

No count moves. The wave-33 close measured **4,544 passed / 0 failed /
8 ignored**, and that figure is untouched by this record; only the word
attached to 227 is clarified. The separate question of whether that `0 failed`
generalises beyond the machine it was measured on is a different residual,
`VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED`, recorded beside constraint 16 in
`AGENTS.md` and measured in `docs/reports/stability-reeval-1/README.md` §2.1.

Evidence: `docs/reports/stability-reeval-1/README.md` §4.4 and §4.5,
`docs/reports/lean-proof-integrity/validation.json`,
`docs/reports/docs-compact-6/README.md`.
