# W48-QR-SCALAR-TYPES-R4: integration onto main

The card was gated alone at `0691a1cc`. While it ran, main moved to `f19204fa`: CG10-COMMON-TYPED-PLACE and CG02-ELABORATION-CORE landed from another session. It was merged onto that main as `integrate/w48-r4` `db0677be`.

## Composition

- **Code:** merged without conflict.
- **Ledger:** merged structurally against `0691a1cc`. Main retired 4 and registered 5; the card retired 1 and registered 2. Both sides recorded 623; the merged ledger is **624**.
- **STATUS** keeps every row, and the increment log lists the card first.

## Combined gate

Figures and the full account are in `gate.json`, exits in `exits/`, and logs in `private/w48-int-r4/gate-db0677be/`.

Three steps needed reruns, all load-related; the load average from other sessions was 20–40:

| Step | First run | Rerun |
|---|---|---|
| `cycle20_verifier_plugins` (wall-clock `elapsed < 500ms`) | failed | failed once more at load ~20; passed 20/20 at load 14 |
| Provider `configured_audit` | 2 tests failed, and cargo stopped at that binary | 7/7 alone; then the whole provider suite with `--no-fail-fast`: 129 passed, 0 failed, 1 ignored |
| Python | failed on the matrix's stale `lean_audits` (230 against 231 on disk) | passed after the matrix was regenerated from this `gate.json` |

## Results

The C-audit outputs were compared with round 9's gate outputs (`private/w47-int-l/gate-6cacfb1b`) using CG02's `compare.py`:

| Route | Figures | What moved |
|---|---|---|
| System | 32 / 2 / 200 | equal to CG02's recorded comparison in totals, moves, move counts and leaf deltas (`system-vs-round9.json`) |
| Slice | 26 / 1 / 242 / 1 Pending | the same (`slice-vs-round9.json`) |
| Corpus | 543 / 10 / 4,281 | 17 moves: exactly CG02's 16 plus CG10's `rd_kafka_anyconf_is_modified` (`corpus-vs-round9.json`) |

This card moves nothing on any route.

## Final gate after CG11 landed

Main moved to `cb20c28e` (CG11-POINTER-VALUED-STORES, C gateway only) before landing. It was merged in as `fa733dda`, with the ledger at **628**, and the standing and provider gates were re-run:

- **Rust tests:** 6,637 passed, 0 failed.
- **Lean:** 232 audits.
- **Provider:** `configured_audit` failed again under load average ~37, with the check failing closed (0 checked, 3 Unknown). The whole provider suite, rerun with `--no-fail-fast`, passed: 129 / 0 / 1.
- **Python:** passed after the matrix was regenerated.

The arbiters were not re-run for this merge: the card moves no C-audit output (see above), and CG11 measured its own. Exits are in `exits-fa733dda/`.
