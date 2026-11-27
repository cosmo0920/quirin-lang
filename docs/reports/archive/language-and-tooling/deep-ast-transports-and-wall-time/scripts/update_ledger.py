"""update_ledger.py -- DEEP-AST-TRANSPORTS-AND-WALL-TIME: retire
DEEP-AST-FILTERED-TRANSPORTS and RDHTTP-DEEP-UNIT-WALL-TIME (every DISCHARGE
clause holds) and register the card's residuals in
docs/reviews/r0_3/acceptance_matrix.json (indent=1, ensure_ascii=False;
dimensions.named_followups = list length). Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "DEEP-AST-TRANSPORTS-AND-WALL-TIME"
SOURCE = "docs/reports/deep-ast-transports-and-wall-time/README.md"

RETIRE = ("DEEP-AST-FILTERED-TRANSPORTS", "RDHTTP-DEEP-UNIT-WALL-TIME")

NEW = [
    {
        "id": "UNBOUNDED-DEPTH-AST-READERS",
        "title": "Two Clang JSON readers parse and recurse with no depth bound on their own 256 MiB stacks",
        "why": (
            "Registered by DEEP-AST-TRANSPORTS-AND-WALL-TIME (gateway rule 13r). The typed dispatch"
            " census (typed_dispatch_census, its streaming UnitSeed) and the bounds side pass"
            " (index_extent::analyze_bounds_preprocessed, only under QCORE_BOUNDS_PROPERTY) parse"
            " Clang's whole-unit JSON with serde_json's recursion limit disabled and no depth check,"
            " on dedicated 256 MiB stacks where their readers run too. Every other transport is"
            " bounded (rules 13o, 13q: 256 levels; rule 13r: the default 128). A stack overflow is"
            " a crash, not a refusal. Size: 2 readers; no document beyond 256 levels on the pinned"
            " corpus, the system route or the slice (the deepest is rdhttp.c, 223-224). DISCHARGE:"
            " each reader either applies rule 13o's bound (with its rows re-measured) or carries a"
            " measured per-level stack cost showing its stack covers any depth Clang can emit."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "CORPUS-DISPATCH-BY-SOURCE-SIZE",
        "title": "The pinned route dispatches rdhttp.c last, so the corpus arbiter ends on its analysis",
        "why": (
            "Registered by DEEP-AST-TRANSPORTS-AND-WALL-TIME. pinned_route dispatches units"
            " heaviest-first by current_route::source_weight, the source file's byte size."
            " librdkafka's rdhttp.c is a small source whose preprocessed unit (curl's headers) and"
            " Clang JSON (about 400-750 MB) are among the corpus's largest; it is started at 412-453"
            " s of a 500-545 s run (unit-timing runs bprof1, aprof1, aprof3) and is the last unit"
            " to finish in every one: the arbiter's tail is its analysis alone (132.5 s before the"
            " card, about 70 s after). Dispatch moves wall time only (consumption stays in step"
            " order), so a better weight changes no output. DISCHARGE: a dispatch weight that"
            " predicts a unit's cost, measured to shorten the corpus wall with byte-identical"
            " outputs on all three routes, or the measured reason none does."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "ELABORATION-NODE-PER-HOOK-WALKS",
        "title": "elaboration_node still walks a statement's whole macro-expanded subtree once per hook",
        "why": (
            "Registered by DEEP-AST-TRANSPORTS-AND-WALL-TIME (rule 13t). After the line tree and the"
            " span-token memo, rdhttp.c's own analysis is about 70 s (132.5 s before); 59 of its"
            " 140 thread-samples (aprof3) are elaboration_node's walk, called by"
            " elaborated_statement (24), call_once (18) and uncast_value_statement (14) for the same"
            " statement, each visiting every node of curl_easy_setopt's type-check expansion (all on"
            " the call's line) and asking its own accept closure of each. DISCHARGE: the walk's"
            " answer reused across hooks of one statement (keyed on the site's accept predicate, the"
            " line and the tokens, identity measured on all three routes) or the cost recorded as"
            " the price of the reading."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "SYSTEM-SLICE-WALL-TIME-AFTER-READING-REUSE",
        "title": "The system route and the slice measured about 5 % slower after rules 13s-13t, unattributed",
        "why": (
            "Registered by DEEP-AST-TRANSPORTS-AND-WALL-TIME. Interleaved medians of three, --jobs 4,"
            " lock time excluded, outputs byte-identical: system 65.8 -> 69.6 s (+5.8 %), slice 29.2"
            " -> 30.8 s (+5.5 %), while the corpus fell 550.2 -> 497.8 s. The card's readers are under"
            " 1 % of the system route's busy samples before and after; the arbiter's own CPU on the"
            " system route is 184.0 and 198.4 s for the BEFORE binary and 213.3 and 197.5 s AFTER (an"
            " 8 % spread within one binary). On the corpus the units over 5 s that do not exercise the"
            " deep readers are a median 6.5 % slower in their own CPU (wall 9-16 %) with no frame of"
            " the card's in their profiles, while the total CPU without rdhttp.c fell 1,751.1 ->"
            " 1,733.3 s. Candidates not isolated: allocator state from the retained per-function"
            " memos and line trees, code layout, host frequency. DISCHARGE: a quiet-host"
            " measurement (thread CPU per unit, several alternations) that attributes the"
            " difference to a frame and removes it with identical outputs, or shows it is not the"
            " card's."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "TOKEN-ROUTE-ELSE-IF-RECURSION-STACK",
        "title": "The token route's else-if recursion overflows a 2 MiB debug thread before the AST bound",
        "why": (
            "Registered by DEEP-AST-TRANSPORTS-AND-WALL-TIME from RDHTTP-HEADER-SUMMARY-GAP's"
            " measurement (its report, section 3): the legacy token parser recurses once per"
            " `else if` arm, and a debug test thread (2 MiB) aborts with a stack overflow at 130"
            " and at 200 arms, at the base as after both cards; that card's beyond-bound fixtures"
            " run on a 256 MiB thread for that reason. A stack overflow is a crash, not a refusal."
            " Not measured: the release worker (2 MiB) and the deepest chain on the corpus. The"
            " typed readers of rules 13o and 13q are not involved (measured within 236 KiB at 255"
            " JSON levels, debug). DISCHARGE: the chain depth at which a release analysis worker"
            " overflows measured, and either a bound that refuses before it or an iterative"
            " reading of the chain."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    data = json.loads(MATRIX.read_text())
    followups = [entry for entry in data["named_followups"] if entry["id"] not in RETIRE]
    present = {entry["id"] for entry in followups}
    for entry in NEW:
        if entry["id"] not in present:
            followups.append(entry)
    data["named_followups"] = followups
    data["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
    print(len(followups))


if __name__ == "__main__":
    main()
