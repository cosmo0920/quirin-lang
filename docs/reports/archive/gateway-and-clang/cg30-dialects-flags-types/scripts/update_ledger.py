#!/usr/bin/env python3
"""CG30-DIALECTS-FLAGS-TYPES: the ledger edit, applied once.

usage: update_ledger.py MATRIX_JSON

Amends (does not retire) CG30-DIALECTS-FLAGS-TYPES, as the card's brief asks:
the identities, the target admission and the named refusals landed (gateway
rule 11y, Lean `QCore.DialectTargetIdentity`), and the entry keeps the
residuals with their re-measured sizes; registers G-TYPED-AST-INDEX-DEPTH-LIMIT
(found out of scope); retires CG20-ABI-READER-ARCH-FLAG (discharged by the
unit-level target admission). `dimensions.named_followups` is set to the list length. Writes the matrix
back in its own form (indent=1, ensure_ascii=False, trailing newline).
"""

import json
import sys

ID = "CG30-DIALECTS-FLAGS-TYPES"

TITLE = (
    "Dialect, target and type identities landed (rule 11y); residual: adopting C23/C2y "
    "differences and extension families, non-LP64 targets, reader use of CTypeId"
)

WHY = (
    "Candidate implementation card from the supplied Quirin-Clang gap analysis (entries I01 I02 I03 I05 N12 N13 "
    "N14). Amended by CG30-DIALECTS-FLAGS-TYPES (wave 48 round 10, base b6a3dc42, gateway rule 11y, "
    "docs/reports/cg30-dialects-flags-types/, Lean QCore.DialectTargetIdentity, 18 audited theorems). LANDED: "
    "C17, C23 and C2y are explicit CCompilerDialect identities (exact -std= names; aliases refused); each "
    "dialect's AST differences are registered (C23 17, C2y 6 more) and join the request digest and the "
    "compile-task identity (baseline dialects keep their historical byte; no earlier digest moves); route G "
    "withdraws a definition spelling a registered difference to one reserved marker call before any reader "
    "(Unknown, no summary, never a misread Unsafe), a digit separator refuses the unit, route T refuses by node "
    "name; structured --target/-target/-mabi (closed vocabularies, transported as CFrontendFlag::Target/Abi, "
    "digested) admitted only under the readers' LP64 premise, -arch checked alike, no passthrough; CTypeId is "
    "one identity for route G, route T and the lexical ABI reader; _Complex, vector, _BitInt and fixed-point "
    "types are refused by name on G (read from each definition's typed AST, its own filtered AST where the unit "
    "cannot be indexed) and T. Corpus/system/slice: SIZES_PLACEHOLDER. RESIDUAL (this entry stays open): "
    "(a) adopting any registered C23/C2y difference (nullptr as a null pointer constant, bool/true/false, "
    "constexpr, attributes, ...) needs every reader's semantics; census: 0 C17/C23/C2y units in the corpus, "
    "system and slice databases; (b) adopting an extension family; census: _Complex only in glibc floatn.h "
    "typedefs, vectors in 5 Fluent Bit definitions (flb_pack.c 2, flb_utils.c 2, tail_file.c 1, none Safe), "
    "_BitInt and fixed point 0; (c) non-LP64 targets (ILP32, LLP64) wait for CG20's widths keyed on "
    "CTargetIdentity (rule 11q); census: 0 target/ABI words in any measured database; (d) readers deciding "
    "admission by CTypeId (T's typedef profile, I03's refusal `result typedef outside unsigned scalar profile`, "
    "stands); (e) the pinned Cycle 19 route (compiler_options_compatibility) reads no target word, unchanged "
    "(constraint 11). DISCHARGE of the residual: each adoption with measured demand, Rust controls and audited "
    "Lean; the queue's section 4 acceptance applies."
)

RETIRE = "CG20-ABI-READER-ARCH-FLAG"

REGISTER = {
    "id": "G-TYPED-AST-INDEX-DEPTH-LIMIT",
    "title": "Route G has no typed AST index for a unit whose Clang JSON nests beyond serde_json's recursion limit (corpus: rdhttp.c, 21 definitions)",
    "why": (
        "Found by CG30-DIALECTS-FLAGS-TYPES (gateway rule 11y). typed_access_path's unit index decodes the "
        "translation-unit AST with serde_json's default recursion limit (128); librdkafka's rdhttp.c nests 223 "
        "levels deep, so StatementIndex::read returns None for every one of its 21 definitions and every typed "
        "route of G declines in that unit (measured on the corpus arbiter: CG30's first AFTER refused all 21 "
        "for an unreadable AST until its hook read each definition's own filtered AST instead). Measured among "
        "units that spell an extension (1,287 of 1,396 Fluent Bit units); other units were not probed. "
        "typed_dispatch_census already decodes without the limit on a dedicated stack. DISCHARGE: decode the "
        "index without the limit (bounded stack) or per definition, re-measure the corpus with every moving "
        "function attributed, no Unknown -> Safe without the typed route's own admission."
    ),
    "source": "docs/reports/cg30-dialects-flags-types/README.md",
    "recorded_by": ID,
}


def main() -> int:
    path = sys.argv[1]
    with open(path, encoding="utf-8") as handle:
        matrix = json.load(handle)
    entries = matrix["named_followups"]
    matches = [entry for entry in entries if entry.get("id") == ID]
    if len(matches) != 1:
        raise SystemExit(f"expected one {ID} entry, found {len(matches)}")
    entry = matches[0]
    entry["title"] = TITLE
    sizes = sys.argv[2] if len(sys.argv) > 2 else None
    if sizes is None:
        raise SystemExit("usage: update_ledger.py MATRIX_JSON SIZES")
    entry["why"] = WHY.replace("SIZES_PLACEHOLDER", sizes)
    entry["amended_by"] = ID
    entry["source"] = "docs/reports/cg30-dialects-flags-types/README.md"
    if not any(other.get("id") == REGISTER["id"] for other in entries):
        entries.append(REGISTER)
    # CG20-ABI-READER-ARCH-FLAG: discharged by the unit-level admission (every
    # admitted unit is LP64 by its own selection; an `-arch i386` unit is
    # refused), with the gateway test named in the report.
    entries[:] = [other for other in entries if other.get("id") != RETIRE]
    matrix["dimensions"]["named_followups"] = len(entries)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(matrix, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
