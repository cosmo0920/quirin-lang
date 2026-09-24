#!/usr/bin/env python3
"""Apply W43-LIBC-RESTRICT-PROTOTYPES' hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `W43-LIBC-RESTRICT-PROTOTYPES-UNDECLARED`, which must be registered,
  recording the retired entry verbatim with its ground beside the report;
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections (including `dimensions.named_followups`) are left to
`docs/reports/remaining-d02/build_matrix.py`, which the caller runs afterwards.
Adapted from docs/reports/w43-libc-result-alias/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w43-libc-restrict-prototypes"
GROUNDS = {
    "W43-LIBC-RESTRICT-PROTOTYPES-UNDECLARED": (
        "Its discharge condition: a declarative card lands the ten non-printf prototypes (or refuses "
        "them by name) with its own leaf-by-leaf attribution, and the array-bound qualifier is either "
        "read (C11 6.7.6.3p7) with Lean parity or refused on record. Card W43-LIBC-RESTRICT-PROTOTYPES "
        "declared all ten as the neutral element in providers/quirin-provider-libc: stat (sys_stat), "
        "strftime and gettimeofday (time), getsockopt and getpeername (socket), dlsym (dlfcn), "
        "cnd_timedwait and pthread_sigmask (threads), and regexec and regerror in the new "
        "libc/regex.qcore. It read the array-bound qualifier: legacy_declared_abi.rs "
        "`array_bound_is_read` accepts a bound of qualifiers and at most one size word, so glibc "
        "regexec's `regmatch_t __pmatch[__restrict __nmatch]` agrees in its one unit, with Rust table "
        "rows in crates/qcore-c-gateway/tests/loader_abi.rs and 13 audited theorems added to "
        "QCore.PointerQualifierReader; the reader change alone reproduced all five BEFORE artifacts "
        "byte for byte and moved none of the 10,686 census rows of the 130 selected declarations. "
        "The declarations removed 36 leaves and added 2, all attributed, and moved 3 functions "
        "Unknown -> Safe (mk_plugin_load_symbol, mk_signal_thread_sigpipe_safe, rd_regex_exec). "
        "getpeername stays withdrawn in the 93 `_GNU_SOURCE` units, its only pinned call site "
        "included, and is registered as W43-LIBC-GETPEERNAME-SOCKADDR-UNION; dlsym's handle lifetime "
        "is registered as W43-LIBC-DLSYM-HANDLE-LIFETIME. "
        "docs/reports/w43-libc-restrict-prototypes/README.md."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    retired = []
    for name, ground in GROUNDS.items():
        if name not in ids:
            raise SystemExit(f"{name} is not registered; nothing to retire")
        retired.append({
            "entry": next(entry for entry in entries if entry["id"] == name),
            "discharged_by": "W43-LIBC-RESTRICT-PROTOTYPES",
            "ground": ground,
        })
    (REPORT / "retired-followups.json").write_text(json.dumps(retired, indent=2) + "\n")
    matrix["named_followups"] = [entry for entry in entries if entry["id"] not in GROUNDS] + new

    corpus = matrix["corpus"]
    (REPORT / "previous-corpus-row.json").write_text(json.dumps(corpus["cycle19_raw_units"], indent=2) + "\n")
    diagnostics = json.loads(gzip.decompress((REPORT / "artifacts/diagnostics.json.gz").read_bytes()))
    functions = collections.Counter()
    units = collections.Counter()
    reason_codes = collections.Counter()
    for unit in diagnostics["units"]:
        units[unit["status"]] += 1
        reason_codes[unit.get("reason_code") or "none"] += 1
        for entry in unit["functions"]:
            functions[entry["status"]] += 1
    counts = diagnostics["diagnostic_counts"]
    row = dict(corpus["cycle19_raw_units"])
    row.update(
        {
            "note": "W43-LIBC-RESTRICT-PROTOTYPES, 2026-09-25. Staged arbiter: BEFORE (base binary) reproduces "
            "the W43-LIBC-RESULT-ALIAS diagnostics byte for byte; the card's binary with no new declaration "
            "(array-bound reader change alone) reproduces all five BEFORE artifacts byte for byte; ten "
            "neutral libc declarations then remove 36 leaves and add 2, all attributed. 3 functions "
            "Unknown -> Safe, each relative to the declared premises; 0 unit status moves; 0 violation "
            "candidates. Leak 272 preserved. Raw units remain unpromoted; provisional Unsafe is not an "
            "upstream defect claim. Previous row preserved in "
            "docs/reports/w43-libc-restrict-prototypes/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w43-libc-restrict-prototypes/artifacts/",
            "source": "docs/reports/w43-libc-restrict-prototypes/README.md; preceding baseline "
            "docs/reports/w43-libc-result-alias/artifacts/",
            "unknown_alias_diagnostics": counts["unknown_alias"],
            "unsupported_diagnostics": counts["unsupported"],
            "double_release_diagnostics": counts.get("double_release", 0),
            "function_statuses": {
                "safe": functions["Safe"],
                "provisional_unsafe": functions["Unsafe"],
                "unknown": functions["Unknown"],
            },
            "unit_reason_codes": dict(sorted(reason_codes.items(), key=lambda kv: (-kv[1], kv[0]))),
        }
    )
    corpus["cycle19_raw_units"] = row
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"follow-ups: {len(entries)} -> {len(matrix['named_followups'])}")
    print(json.dumps({k: row[k] for k in ("safe", "provisional_unsafe", "unknown", "data_only", "function_statuses", "unit_reason_codes")}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
