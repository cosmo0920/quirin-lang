#!/usr/bin/env python3
"""TSAN-ROWS-2: give CT07's ledger row a ThreadSanitizer CITATION — not a stage flip.

Appends one sentence to the manifest's `semantic_profile` for CT07
(docs/reevaluation/ct_manifest.json) and to the fixture's `notes`
(compiler/tests/fixtures/ct/CT07/case.qcore's expected.json). Neither text is
byte-pinned anywhere (ct_corpus.rs asserts the profile is non-empty; the
generator copies it), no `stages` entry moves, and the "there is NO
ThreadSanitizer evidence" sentence each already carries is kept as the wave-37
state it describes, superseded by the citation appended after it. Idempotent.
"""
import json
import pathlib

MANIFEST = pathlib.Path("docs/reevaluation/ct_manifest.json")
EXPECTED = pathlib.Path("compiler/tests/fixtures/ct/CT07/expected.json")

CITATION = (
    " TSAN CITATION (card TSAN-ROWS-2, wave 39; a citation, not a stage flip - no stage of this row "
    "moves): compiler/tests/task_route_tsan.rs reads THIS fixture through the real pipeline "
    "(compile_files, entry null) and runs its own emitted C under ThreadSanitizer through the "
    "pthread binding of tests/support/thread_sanitizer.rs, changing no emitted byte - unbound it "
    "dies at the 0x18 fault every task program dies at (every_subject_dies_unbound_at_the_one_fault); "
    "bound it runs clean under the unforced schedule "
    "(every_subject_runs_clean_under_thread_sanitizer_when_bound) and under two FORCED schedules "
    "(every_subject_runs_clean_under_each_forced_schedule): both workers created and released "
    "before either body runs and both threads live until both bodies have run, and last-created "
    "first with each body to completion - zero races, stdout byte-identical to the reference "
    "execution, the realised schedule read back from the harness's own trace. The overlap "
    "mutation on this fixture is reported under all three schedules "
    "(overlapping_writers_are_reported_under_every_schedule), 20 of 20 runs each "
    "(docs/reports/tsan-rows-2/artifacts/catch-rates.json). So the sentence above that reads "
    "'there is NO ThreadSanitizer evidence' describes wave 37's state and is superseded by this "
    "citation; 'two threads write distinct memory locations without a race' is now ALSO carried by "
    "a race detector, over the schedules named, with TASK-ROUTE-TSAN-PTHREAD-BINDING's and "
    "TSAN-FORCED-BODY-OVERLAP-NOT-FORCED's boundaries. CT07's case is pinned as the write-lend "
    "flagship without its logger (ct07s_case_is_the_flagship_without_its_logger)."
)


def append(path, key_path):
    data = json.loads(path.read_text(encoding="utf-8"))
    node = data
    for key in key_path[:-1]:
        node = node[key]
    text = node[key_path[-1]]
    if "TSAN CITATION (card TSAN-ROWS-2" in text:
        print(f"{path}: citation already present")
        return
    node[key_path[-1]] = text + CITATION
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"{path}: citation appended")


manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
cases = manifest["cases"] if isinstance(manifest, dict) and "cases" in manifest else manifest
index = next(i for i, case in enumerate(cases) if case["id"] == "CT07")
if isinstance(manifest, dict) and "cases" in manifest:
    append(MANIFEST, ["cases", index, "semantic_profile"])
else:
    append(MANIFEST, [index, "semantic_profile"])
append(EXPECTED, ["notes"])
