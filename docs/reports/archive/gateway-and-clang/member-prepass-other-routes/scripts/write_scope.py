#!/usr/bin/env python3
"""write_scope.py -- MEMBER-PREPASS-OTHER-ROUTES: rewrite
`contract/profiles/fluent-bit/member-facts-scope.json` as schema 2: the
evidence revision (the pinned Fluent Bit revision the OP-APP-RK-CALLBACKS-VIA-POLL-CB
scope check read, `docs/reports/op-app-rk-callbacks/artifacts/scope-check.json`)
and, per entry, the typed evidence the pre-pass re-verifies at run time.
Every entry's fragment and recorded `why` are kept byte for byte. Idempotent
on a schema-2 file (the evidence is recomputed from the fragment)."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
SCOPE = REPO / "contract/profiles/fluent-bit/member-facts-scope.json"
REVISION = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"
LUAJIT = "lib/luajit-7152e154/src"
BACKTRACE = ["lib/libbacktrace-b9e4006/backtrace.h",
             "lib/libbacktrace-b9e4006/backtrace-supported.h.in"]
# Each parser pair: its two grammars (the bison grammar includes the flex
# header and the flex grammar the bison header) and the headers generated
# from them.
PARSERS = {
    "processor_sql": (["plugins/processor_sql/parser/sql-parser.y",
                       "plugins/processor_sql/parser/sql-parser.l"],
                      ["processor-sql-parser_lex.h", "processor-sql_parser.h"]),
    "record_accessor": (["src/record_accessor/ra.y", "src/record_accessor/ra.l"],
                        ["ra_lex.h", "ra_parser.h"]),
    "stream_processor": (["src/stream_processor/parser/sql.y",
                          "src/stream_processor/parser/sql.l"],
                         ["sql_lex.h", "sql_parser.h"]),
}
MEANING_V1 = ("Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (gateway rule 12n, user decisions R1 and F1): "
              "units of the production compile database the member-facts pre-pass does not read, "
              "each with its recorded evidence. Every other unit is preprocessed; a unit that names "
              "a token of an invoked member's closure and cannot be read as C opens every member.")
MEANING_V2 = (" Schema 2 (card MEMBER-PREPASS-OTHER-ROUTES): `evidence_revision` is the Fluent Bit "
              "revision the recorded evidence was taken at, and each entry's `evidence` is "
              "re-verified by the pre-pass against the run's own sources and its final tokens. "
              "`tokens-absent`: the unit's raw text, every file Clang resolves for it under its own "
              "command and the named generator sources (relative to the source root; a directory "
              "is every file under it) name no token, and every include Clang leaves unresolved "
              "is named in `generated`. `not-linked`: the named link lines (relative to the CMake "
              "build root) name `linked` and not `unlinked`. An exclusion that does not hold is "
              "dropped and its unit read like any other.")


def evidence(fragment):
    if "/src-cpp/" in fragment:
        return {"kind": "not-linked",
                "links": ["src/CMakeFiles/fluent-bit-bin.dir/link.txt",
                          "src/CMakeFiles/fluent-bit-shared.dir/link.txt"],
                "linked": "librdkafka.a", "unlinked": "rdkafka++"}
    if "luajit" in fragment:
        return {"kind": "tokens-absent", "sources": [LUAJIT],
                "generated": ["buildvm_arch.h", "lj_bcdef.h", "lj_ffdef.h", "lj_folddef.h",
                              "lj_libdef.h", "lj_recdef.h", "luajit.h"]}
    for needle, (sources, generated) in PARSERS.items():
        if needle in fragment:
            return {"kind": "tokens-absent", "sources": sources, "generated": generated}
    if fragment.endswith(("/src/fluent-bit.c", "/src/flb_dump.c")):
        return {"kind": "tokens-absent", "sources": BACKTRACE,
                "generated": ["backtrace-supported.h", "backtrace.h"]}
    raise SystemExit(f"no evidence rule for {fragment}")


def main():
    data = json.loads(SCOPE.read_text())
    out = {
        "schema_version": 2,
        "evidence_revision": REVISION,
        "meaning": MEANING_V1 + MEANING_V2,
        "outside_program": [
            {"fragment": entry["fragment"], "why": entry["why"],
             "evidence": evidence(entry["fragment"])}
            for entry in data["outside_program"]
        ],
    }
    SCOPE.write_text(json.dumps(out, indent=2, ensure_ascii=False) + "\n")
    print(f"{SCOPE}: {len(out['outside_program'])} entries")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
