"""update_ledger.py -- RDHTTP-HEADER-SUMMARY-GAP: amend
DECLARED-ASSUMPTION-BODY-CHECK-RESIDUALS and register the card's two
residuals in docs/reviews/r0_3/acceptance_matrix.json (indent=1,
ensure_ascii=False; dimensions.named_followups = list length). Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "RDHTTP-HEADER-SUMMARY-GAP"
SOURCE = "docs/reports/rdhttp-header-summary-gap/README.md"
MARK = "AMENDED by card RDHTTP-HEADER-SUMMARY-GAP"

AMEND = {
    "DECLARED-ASSUMPTION-BODY-CHECK-RESIDUALS": (
        " " + MARK + " (gateway rules 13o-13p, base b21f0c9a2, binary b51d4b20): the rdhttp.c"
        " unit's missing reading isolated and repaired. Clang's JSON AST of rdhttp.c nests 224"
        " levels (four definitions calling curl_easy_setopt, whose GCC type-check macro expands"
        " into nested conditionals), beyond serde_json's default recursion limit (128), so the"
        " whole-unit typed index was absent and no definition of the unit, header bodies included,"
        " had a typed reading: rd_assert(p) (glibc's assert, a statement expression under gnu17)"
        " was never read in rd_malloc, rd_calloc and rd_strdup, W4 published no summary and rule"
        " 13m had no conflict to check. Rule 13o reads a document within 256 levels without the"
        " limit; rd_malloc (43 units), rd_calloc (46) and rd_strdup (32) are now validated (13m"
        " in every examining unit); corpus Safe roll-up 418 / 91 / 23 / 349 -> 418 / 94 / 23 /"
        " 346, declarations 51 -> 54 validated, 247 -> 244 declared assumptions; no status move."
        " Class (3) re-measured: 24 -> 9 declarations, 24 -> 20 Safe-row mentions. Correction: 12"
        " of the 24 (cb_queue_chunks and the 11 sb_* storage-backlog declarations) examine no body"
        " on the corpus (withdrawn_units 180, all C06.3-no-body; the ranking fell through to this"
        " class) and are reclassified no-examined-body, 0 Safe rows. The 9 left have their own"
        " causes, none this one (a depth census of every unit in which any is unvalidated: all"
        " nest 45-82 but rdhttp.c): rd_atomic32_get (29 units, 8 Safe rows), rd_atomic64_get (5,"
        " 5), rd_atomic64_add (9, 3) (an __atomic builtin with no owning crate);"
        " mk_utils_libc_error (5, 3); rd_file_read (1, 1); flb_log_check (1, 0); mk_list_add (18,"
        " 0); mk_list_del (17, 0); rd_realloc (8, 0). Other classes unchanged. The DISCHARGE"
        " clause 'the rdhttp.c unit's missing reading isolated' holds; the per-class clauses"
        " remain open."
    ),
}

NEW = [
    {
        "id": "DEEP-AST-FILTERED-TRANSPORTS",
        "title": "Filtered and closed-world Clang AST transports still refuse JSON nesting 128 or more",
        "why": (
            "Registered by RDHTTP-HEADER-SUMMARY-GAP (gateway rules 13o-13p). Rule 13o reads only"
            " the whole-unit index (typed_access_path::AstIndex::from_json) beyond serde_json's"
            " default recursion limit. typed_c_ast::ast_declarations (Clang's name-filtered dump,"
            " used by definition_ast, function_ast and the appended-declaration probe) and"
            " typed_c_ast::translation_unit_ast (the closed-world whole-unit Value) still parse"
            " under the default limit, so a definition itself nesting 128 levels or more has no"
            " filtered AST: the four rdhttp.c definitions (rd_http_ssl_configure 224, rd_http_get"
            " 214, rd_http_req_init 212, rd_http_post_expect_json 212). No corpus row is measured"
            " to depend on it after the card (rule 11y's extension check reads the unit index"
            " first). DISCHARGE: each transport either reads a document within the rule-13o bound"
            " the same way (with its readers' recursion measured on the deepest admitted"
            " definition) or records why it must not."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "RDHTTP-DEEP-UNIT-WALL-TIME",
        "title": "The corpus arbiter takes 68 s longer since rdhttp.c's typed index is read",
        "why": (
            "Registered by RDHTTP-HEADER-SUMMARY-GAP. Corpus arbiter wall time at --jobs 4, lock"
            " time excluded, two runs each: 459.7 / 459.6 s (binary 18cf8371, base b21f0c9a2) ->"
            " 527.7 / 528.5 s (b51d4b20), +68 s (+15%); system 64.4 -> 64.1 s, slice 28.6 -> 29.6"
            " s. Every report and parity difference is in rdhttp.c, whose Clang JSON (about 750 MB)"
            " is now indexed and read by every typed reader (20,864 more parity rows, 19,870 of"
            " them in the four definitions nesting beyond 200 levels). DISCHARGE: the unit's own"
            " time measured and either reduced without changing any row or recorded as the price"
            " of the reading."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    data = json.loads(MATRIX.read_text())
    followups = data["named_followups"]
    for entry in followups:
        text = AMEND.get(entry["id"])
        if text and MARK not in entry["why"]:
            entry["why"] += text
            entry["amended_by"] = CARD
    present = {entry["id"] for entry in followups}
    for entry in NEW:
        if entry["id"] not in present:
            followups.append(entry)
    data["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
    print(len(followups))


if __name__ == "__main__":
    main()
