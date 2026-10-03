#!/usr/bin/env python3
"""GATEWAY-ANALYSIS-PATH-OPTIMIZATION: register the card's findings as named
follow-ups in docs/reviews/r0_3/acceptance_matrix.json. Idempotent (an entry
with the same id is replaced). Writes with indent=1, ensure_ascii=False and
keeps dimensions.named_followups equal to the list length.
"""
import json
import pathlib

CARD = "GATEWAY-ANALYSIS-PATH-OPTIMIZATION"
REPORT = "docs/reports/gateway-analysis-path-optimization/README.md"
MAP = "docs/reports/gateway-analysis-path-optimization/ROUTE-MAP.md"

ENTRIES = [
    ("GAPO-PARITY-DECISIONS-NONDETERMINISTIC",
     "The corpus typed-path parity stream varies between runs of one binary: a thread-local place decision leaks into the next typed-place row",
     "Measured by " + CARD + ": four corpus runs of the BEFORE binary (main 2ecd39531, --jobs 4, QCORE_TYPED_PATH_PARITY on) wrote 626,469 or 626,470 rows; the extra row is a `typed-place` row at `cio_file_st_get_meta_len` line 2223 (chunkio cio_file.c's unit) whose one place decision belongs to another consultation and differs between runs (`st` in one, `u` in another). Mechanism read: `typed_place::record_place_row` (rule 12j's `record_object_place::record`) pushes into the thread-local DECISIONS of every reader run, but only some consumers clear or flush it (`typed_access_path::StatementIndex::statement_with` clears only after a node matched; `typed_path_plan` then records whatever is left), so a decision left by a reader whose consumer records nothing is flushed into the next statement row its worker thread writes, which depends on which unit the scheduler gave that thread. resource-diagnostics.json and every sidecar are unaffected (identical in every run); the parity stream is evidence other cards compare as a multiset, so a comparison can see a phantom move. DISCHARGE: every consumer clears the decisions it does not record (or the reader records into a value the consumer owns), and repeated --jobs 4 runs of one binary write identical multisets."),
    ("GAPO-RETURN-PROBE-COUNTER-RESTORATION",
     "Declined classification probes restore next_site only: element lowerings, list frames and element origins may leak (returns, 12h, 11w)",
     "Read by " + CARD + " (ROUTE-MAP section 9, findings 1, 2, 5): the W4, B01 and W45-K probes of a refused return (legacy_return_routes.rs: return_call_instruction, return_expression_instruction; legacy_return_family.rs:660) restore `next_site` only, while `classify_simple` also moves `element_lowerings` (which decides a `for` loop's counted lowering), the list frames and the element origins; the doc comment of return_call_instruction says `next_site` is its only mutation. Rule 12h's uncast_call_actuals classifies the uncast statement after the first classification minted sites (gaps in site numbering); variadic_tail_statement keeps counters when its widened result is not a ContractCall; the W43 descriptor guard restores nothing. Not measured as a verdict defect. DISCHARGE: each probe saves and restores every parser state classify_simple mutates, or a test pins that a declined probe leaves the state unchanged."),
    ("GAPO-RETURN-POSITION-ROUTE-GAPS",
     "The return position asks fewer statement routes (12h H-U0, 11o no-return exit, 11w, 10u), and 11o reads only the first lowering",
     "Read by " + CARD + " (ROUTE-MAP sections 4 and 9, findings 3 and 4): a statement is asked rule 12h's uncast actuals, rule 11o's no-return exit, rule 11w's tail and va_* hooks and rule 10u's substituted call; `return CALL(...)` is asked none of them (e.g. a cast Borrowed actual in a returned call is read by W4 without 12h). call_exit_statement sees only the instruction classify_simple/12h produced, so a no-return call lowered later by W44, 10u, 10y or 11w never becomes a PathEnd. Not measured. DISCHARGE: each gap is either asked at the return position with its own controls or recorded as out of the route's scope in gateway.md, with a corpus measurement."),
    ("GAPO-ROUTE-ORDER-FINDINGS",
     "Route-order observations: the if arm's access route ignores its documented null test, 11i ends the statement table with a refusal, the census reads the stream without 11p",
     "Read by " + CARD + " (ROUTE-MAP section 9, findings 6, 7, 9): (a) the `if` arm asks condition_access without the null test its doc comment requires; CfgBuilder::compile lets the null decision win, so only a wasted consultation and a parity row result; (b) atomic_routed_statement (11i) answers Some for a re-worded Unknown, so 11u, 11w and 12h are never asked for a statement it matched; (c) recursive_component_census reads the token stream without the rule 11p rewrite, contrary to its comment (census only). DISCHARGE: each is either changed with its own measurement or documented as intended in gateway.md."),
    ("GAPO-REMAINING-REDERIVATIONS",
     "Next analysis-path speed targets after GATEWAY-ANALYSIS-PATH-OPTIMIZATION: AST JSON parse, per-build whole-unit token passes, repeated classifications and AST walks",
     "Measured by " + CARD + " (ROUTE-MAP sections 6-8 and the AFTER profile in the report): after the contract-environment readings were shared, the corpus arbiter's worker time is led by the analysis proper and serde_json parsing of Clang's whole-unit AST (6.65 GB of JSON per corpus run); per CFG build, `literal_premise::restrict_to_declared_literals` and the other whole-unit token passes of build_function_cfg_with run once per function per derivation round; a refused `return f(a)` classifies the same synthetic statement three or four times; every Clang statement hook re-walks the function AST (`elaboration_node`); the pinned route runs the same line-marked preprocessing twice per unit and validation again for Unsafe coordinates; ClangIdentity::capture is uncached. Each of these is observable (parity rows, site numbering, files read) and needs its own identity proof. DISCHARGE: each optimised with a byte-identity measurement, or recorded as not worth it with its measured share."),
    ("GAPO-CLANG-CROSS-RUN-CACHE",
     "A fail-closed cross-run Clang cache was not built: Clang processes are 3-4 % of arbiter worker time",
     "Measured by " + CARD + " (scripts/clang-census.sh over the BEFORE corpus arbiter: 1,549 launches, 90.1 Clang-seconds against about 2,630 worker-seconds, 601 repeated launches worth 26.9 s; profile: 3.9 % corpus / 2.7 % system waiting on Clang): a content-addressed cache keyed by the Clang binary identity, every flag and the preprocessed input digest could save at most that share across runs, while the preprocessing launches read include files from disk and so need every read file in the key. The in-process reuse of the unit AST and layout dump across a unit's scopes was implemented and measured verdict-identical but saved no measurable wall time, and was kept. DISCHARGE: a cache whose key covers every file Clang reads, off by default, with byte-identical outputs on and off, if a later profile shows Clang's share has grown."),
    ("GAPO-DISPATCHER-COVERAGE",
     "Only the statement and return fallback chains are explicit tables; the condition routes and classify_simple's internal chain stay in code order",
     "Recorded by " + CARD + ": legacy_route_table.rs holds the statement fallbacks (15 rows) and the return fallbacks (8 rows); the condition routes (whose parse-time order differs from CfgBuilder::compile's precedence, ROUTE-MAP section 3), classify_simple_unguarded's typed fallbacks and classify_simple_base's call holds are documented in ROUTE-MAP but remain in code order, because their entries interleave state (sites, lowerings, parity rows) a table would have to thread. DISCHARGE: each chain moved into a table with the identity runs, or ROUTE-MAP kept as their authority."),
]


def main():
    path = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
    matrix = json.loads(path.read_text())
    followups = [entry for entry in matrix["named_followups"]
                 if entry["id"] not in {identifier for identifier, *_ in ENTRIES}]
    for identifier, title, why in ENTRIES:
        followups.append({"id": identifier, "title": title, "why": why, "source": REPORT,
                          "recorded_by": CARD})
    matrix["named_followups"] = followups
    matrix["dimensions"]["named_followups"] = len(followups)
    path.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(followups))


if __name__ == "__main__":
    main()
