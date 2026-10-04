#!/usr/bin/env python3
"""W45-CALLEE-CASCADE-RESIDUAL: the increment paragraph (newest first in
docs/development/increments/wave-48-part5.md) and the STATUS rows, from this
card's primary evidence (README sections 1-5). Idempotent."""
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MARK = "W45-CALLEE-CASCADE-RESIDUAL"

PARAGRAPH = (
    "**`W45-CALLEE-CASCADE-RESIDUAL` (2026-10-04, base `3905139cd`; prediction `19ef75241`).** "
    "Round 20, the user's top priority: the 704 Unknown functions blocked by uncontracted calls "
    "alone ([report](../../reports/w45-callee-cascade-residual/README.md)). Contracts only: no "
    "gateway rule (ids `12s`–`12w` unused) and no Lean. The BEFORE reproduces (corpus `ce57d194…`, "
    "756 / 9 / 4,069; the 704 and every cited head). The 90 callees deciding at least 2 functions "
    "alone were read at both revisions and classified: 17 contractable in existing vocabulary, the "
    "rest decision classes (1: 94 functions, 3: 19, 2: 18, 8: 16, 7: 2), reader shapes (enum "
    "typedefs 28, `const T **` 7), destructors whose callers would move one each (8), and two "
    "others. Twenty-one declared contracts, each read at `66910c10` and `046accf6` and pinned at "
    "both (`pin --check` per crate 0 at both, `--verify-contracts` 0): `cJSON_New_Item` (owned "
    "nullable), three one-shot LZ4 functions, `load_descriptor_values`, "
    "`rd_kafka_aborted_txns_next_offset`, `rd_kafka_anyconf_desensitize`, `rd_kafka_dump0`, "
    "`rd_kafka_metadata_copy_internal` (owned nullable) in `qcore-provider-librdkafka`; two "
    "`ctr_mpack_consume_*` readers and four copying `ctr_attributes_set_*` in "
    "`qcore-provider-ctraces`; `cfl_kvlist_count` and four copying `cfl_kvlist_insert_*` in "
    "`quirin-provider-cfl`; `_cio_file_up` in `qcore-provider-chunkio`. Corpus **756 / 9 / 4,069 → "
    "803 / 9 / 4,022** (`2734991a…`), **+47 Unknown → Safe** (predicted +35 to +52), each attributed "
    "to the callee it lost; no Safe → non-Safe move, no leak or violation added; system and slice no "
    "row moves; parity compared in full (system and slice multiset-identical, corpus differences "
    "confined to moving functions, declared callees and their callers). Five predicted movers stay "
    "Unknown on an exposed obligation (a pointer store into the new cJSON node; a nested "
    "`LZ4_compressBound` actual). Remaining 652, 473 decided by one callee alone; new head "
    "`LZ4_decompress_generic` 9, blocked by the enum reader shape. Amended "
    "`W45-CALLEE-CASCADE-RESIDUAL`, `W48-CALLEE-DECISION-TABLE` (custody-taking inserts and "
    "retaining LZ4 stream states added to class 1), `W43-LIBC-ABI-READER-SHAPES` and "
    "`W44-CALLEE-DESTRUCTOR-SUMMARY`; follow-ups 705 unchanged. Pins: `contract_loader` 272 → 293 v1 "
    "declarations, `qcore_contract_corpus` 21 clause rows."
)

OLD_HEAD = "| Projected functions | **756 Safe / 9 provisional Unsafe / 4,069 Unknown**, 4,834 total (CG10-INDIRECT-ACCESS-SOLE-RESIDUALS,"
NEW_HEAD = (
    "| Projected functions | **803 Safe / 9 provisional Unsafe / 4,022 Unknown**, 4,834 total "
    "(W45-CALLEE-CASCADE-RESIDUAL, 21 declared, pinned contracts, no rule, arbiter on main "
    "`3905139cd` plus the card: `ce57d194…` → `2734991a…`, **+47 Unknown → Safe**: seven "
    "`cJSON_Create*` constructors over `cJSON_New_Item`, twelve ctraces attribute setters and span "
    "setters over the copying `cfl_kvlist_insert_*`/`ctr_attributes_set_*`, nine LZ4 wrappers, "
    "nanopb's field iterator (3), and two each over `_cio_file_up`, `cfl_kvlist_count`, the "
    "`ctr_mpack_consume_*` readers, `rd_kafka_dump0`, `rd_kafka_anyconf_desensitize`, "
    "`rd_kafka_metadata_copy_internal` and `rd_kafka_aborted_txns_next_offset`; no Safe → non-Safe "
    "move, no leak or violation added; system 85 / 4 / 359 and slice 45 / 1 / 354 / 4 Pending, no "
    "row moves ([report](reports/w45-callee-cascade-residual/README.md)); before it 756 / 9 / "
    "4,069, CG10-INDIRECT-ACCESS-SOLE-RESIDUALS,"
)

OLD_UNCON = "| Uncontracted-call refusals | With rule 12k"
NEW_UNCON = (
    "| Uncontracted-call refusals | After W45-CALLEE-CASCADE-RESIDUAL (corpus on main `3905139cd` "
    "plus the card): **652** Unknown functions are blocked by uncontracted calls alone, 473 by one "
    "callee alone (704 before it; 47 freed by 21 contracts, 6 now hold another leaf); every callee "
    "that still decides 2 or more is a decision class of `W48-CALLEE-DECISION-TABLE` (class 1: 94 "
    "functions, 3: 19, 2: 18, 8: 16, 7: 2), a declared-ABI reader shape (enum typedefs 28, head "
    "`LZ4_decompress_generic` 9; `const T **` 7), a destructor (8, at most one each) or one of two "
    "others ([ranking](reports/w45-callee-cascade-residual/artifacts/tally-after.txt)). Before it, with rule 12k"
)

OLD_CORPUS = "| Contract corpus | **124** modules; **264** resource-v1 declarations by the loader pin (`compiler/tests/contract_loader.rs`; "
NEW_CORPUS = (
    "| Contract corpus | **124** modules; **293** resource-v1 declarations by the loader pin "
    "(`compiler/tests/contract_loader.rs`; W45-CALLEE-CASCADE-RESIDUAL appends 21 pinned "
    "declarations to six already-selected modules, 272 before it, which CONTRACT-NORETURN-AND-BORROWED-TAILS "
    "(+1) and CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (+7) reached from 264; "
)


def main() -> int:
    part5 = REPO / "docs/development/increments/wave-48-part5.md"
    text = part5.read_text()
    if MARK not in text:
        anchor = "**`CHUNKIO-META-LEN-BOUNDS-CONTRACTS`"
        assert text.count(anchor) == 1
        text = text.replace(anchor, PARAGRAPH + "\n\n" + anchor)
        part5.write_text(text)
    status = REPO / "docs/STATUS.md"
    text = status.read_text()
    if "**803 Safe / 9 provisional Unsafe / 4,022 Unknown**" not in text:
        for old, new in ((OLD_HEAD, NEW_HEAD), (OLD_UNCON, NEW_UNCON), (OLD_CORPUS, NEW_CORPUS)):
            assert text.count(old) == 1, old[:60]
            text = text.replace(old, new)
        status.write_text(text)
    print("records written")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
