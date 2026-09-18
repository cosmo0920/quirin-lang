#!/usr/bin/env python3
"""WAVE-39-CLOSE: give CT46's two texts their ThreadSanitizer CITATION - not a stage flip.

The CT07 precedent is docs/reports/tsan-rows-2/scripts/cite_ct07.py; this is the
two-text citation card TSAN-ROWS-2 section 11 item 6 left to the close, and it is
CT46-NOTES-PREDATE-TSAN-ROW's discharge in that entry's words: "a citation
appended to both CT46 texts on the CT07 precedent, naming the row and the
recorded decision that this route's native evidence stays serial by
construction". Appends one sentence to the manifest's `semantic_profile` for
CT46 (docs/reevaluation/ct_manifest.json) and to the fixture's `notes`
(compiler/tests/fixtures/ct/CT46/expected.json). Neither text is byte-pinned
(ct_corpus.rs asserts the profile is non-empty; build_matrix.py copies it), no
`stages` entry moves, and the wave-36 sentence each carries is kept as the
state it describes, superseded by the citation appended after it. Idempotent.
"""
import json
import pathlib

MANIFEST = pathlib.Path("docs/reevaluation/ct_manifest.json")
EXPECTED = pathlib.Path("compiler/tests/fixtures/ct/CT46/expected.json")
MARK = "TSAN CITATION (wave-39 close"

CITATION = (
    " TSAN CITATION (wave-39 close, 2026-09-18, discharging CT46-NOTES-PREDATE-TSAN-ROW on the CT07 "
    "precedent of docs/reports/tsan-rows-2/scripts/cite_ct07.py; a citation, not a stage flip - no "
    "stage of this row moves): the sentence above saying TSan is run nowhere in this repository "
    "describes wave 36's state and is superseded. Since wave 37 (card TASK-ROUTE-TSAN) the crossing "
    "has a ThreadSanitizer row on a CT46-shaped twin (crates/qcore-core/tests/unit/mutex_c_emit.rs), "
    "and since wave 39 (card TSAN-ROWS-2) on THIS fixture's own lowering: "
    "compiler/tests/task_route_tsan.rs::ct46s_own_case_runs_clean_bound_and_the_forcing_header_finds_no_seam "
    "lowers the case through compile_files, dies unbound at the one 0x18 fault, runs clean bound "
    "under ThreadSanitizer with stdout byte-identical to interpret_mutex_program's observation, and "
    "shows the schedule-forcing header attaches to nothing on this backend (the mutex backend calls "
    "thrd_create bare: no worker released, no schedule line). The row is serial by construction and "
    "claims no contention - the recorded decision: merge 0150c904 records "
    "MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION as RETIRED on its own second arm, in its words 'a "
    "DECISION, recorded as such, that this route's native evidence stays serial by construction', "
    "taken by card TSAN-ROWS-2 in its section 6 and recorded at that merge as accepted."
)


def append(path, key_path):
    data = json.loads(path.read_text(encoding="utf-8"))
    node = data
    for key in key_path[:-1]:
        node = node[key]
    text = node[key_path[-1]]
    if MARK in text:
        print(f"{path}: citation already present")
        return
    node[key_path[-1]] = text + CITATION
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"{path}: citation appended")


manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
cases = manifest["cases"]
index = next(i for i, case in enumerate(cases) if case["id"] == "CT46")
append(MANIFEST, ["cases", index, "semantic_profile"])
append(EXPECTED, ["notes"])
