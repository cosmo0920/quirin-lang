#!/usr/bin/env python3
"""make_amendments.py OUT_JSON MEASURED_TIP -- K2: write the card's
integration-amendments.json (rule 79, USER DECISION 2026-11-25 "Build the 2 rows")."""
import json
import sys

out, tip = sys.argv[1:3]
SRC = "docs/reports/k2-no-typed-access/README.md"
CENSUS = ("RE-CENSUSED by K2-NO-TYPED-ACCESS (round 37, base 5087f9295, the user's 2026-11-24 current run: "
          "4,934 rows, 1,173 Safe / 3,750 Unknown / 11 Unsafe; 11-07 tags joined by file, function, family and "
          "preprocessed line, Monkey after a per-file line shift): bundle K2 is 240 leaves (COND 133, IND 74, "
          "RET 29) on 181 Unknown rows, 19 freed alone (11-07: 1,058 leaves, 601 rows, 40 alone). 801 of the 952 "
          "11-07 COND leaves were exchanged by AG3's rule 63 (2) for the named uncontracted-callee text. Of the 19 "
          "freed rows, 6 wait on an Unknown callee summary, 3 on an uncontracted cross-unit callee, 4 are refused "
          "on purpose (formal-aliasing multi-declarators, a returned `mk_list_entry`, a nested call on a "
          "descriptor), 1 is glibc's `_Generic` strchr (CG07-GENERIC-TOKEN-AST-MATCH), 2 are bit-count builtins "
          "(K2-BUILTIN-BIT-COUNTS, a user decision) and 2 are decidable. USER DECISION 2026-11-25 \"Build the 2 "
          "rows\": gateway rule 79 (type_query_positions.rs; Lean QCore.TypeQueryPositions; every condition a "
          "formula over the new K2 block of the C04.18.9 typed rendering, checked by guarded_proposition) admits "
          "(1) a controlling expression every older route refused whose elaboration holds a value region, a "
          "sizeof/_Alignof of a type name that is not variably modified and only Branch/Merge events and reads of "
          "unaliased automatic objects by name (LZ4_hashPosition), and (2) drops `NAME * sizeof` from the "
          "declaration candidates, so a function holding `(n * sizeof(T *))` has its declared types read "
          "(rd_kafka_share_partition_offsets_list_new). Forecast: both rows Unknown -> Safe on current, corpus and "
          "twelve-home.")
document = {
    "schema": "quirin-amendments/2",
    "card": "K2-NO-TYPED-ACCESS",
    "base": "110a69c8e",
    "measured_tip": tip,
    "increment_paragraph": (
        "**K2-NO-TYPED-ACCESS** (round 37, USER DECISIONS 2026-11-25 \"Launch K2, K4 and K6 cards\" and "
        "\"Build the 2 rows\"; gateway rule 79; Lean `QCore.TypeQueryPositions`; "
        "[report](../../reports/k2-no-typed-access/README.md)). Re-census of bundle K2 of "
        "CURRENT-ACCESS-ALIAS-CENSUS on the user's 2026-11-24 `current` run (1,173 / 3,750 / 11): 240 "
        "no-typed-access leaves on 181 Unknown rows (11-07: 1,058 on 601), 19 rows freed alone (11-07: 40); 801 "
        "of the 11-07 condition leaves were AG3 rule 63 (2) re-attributions to uncontracted callees. Read from the "
        "bodies, 15 of the 19 keep a callee's or an earlier card's deliberate refusal, 2 need a user decision "
        "(the bit-count builtins, registered) and 2 were built. Rule 79 (1) lowers a controlling expression every "
        "older route refused, whose elaboration holds a value region, a `sizeof` of a type name that is not "
        "variably modified and only Branch/Merge events and reads of unaliased automatic objects by name, as one "
        "branch refining neither edge; (2) drops `NAME * sizeof` from the declaration candidates of the "
        "total-or-reject declaration reader. Every condition and refusal is a formula over the new `k2_` block of "
        "the C04.18.9 typed rendering (14 terms, 4 sorts, appended after MES), validated by "
        "`guarded_proposition`. Forecast: `LZ4_hashPosition` and "
        "`rd_kafka_share_partition_offsets_list_new` Unknown -> Safe on `current`, corpus and twelve-home; system "
        "and slice unchanged; the tip unit probe reads `LZ4_hashPosition` Safe."),
    "status_edits": [],
    "ledger": [
        {"id": "W46-LEXICAL-GATE-NON-ACCESS", "operation": "amend", "text": CENSUS, "source": SRC},
        {"id": "W44-CONDITION-CALL-REFUSED-SHAPES", "operation": "amend",
         "text": ("RE-CENSUSED by K2-NO-TYPED-ACCESS (round 37, the user's 2026-11-24 current run): the "
                  "condition share of bundle K2 is 133 leaves (11-07: 952); 801 were exchanged by AG3's rule 63 "
                  "(2) for named uncontracted callees. Of what remains, negated direct calls of run-defined "
                  "callees with no summary in force 33 leaves / 21 rows, other direct calls (nested, pointer "
                  "arithmetic actuals, uncontracted externals) 25 / 22, calls through a function pointer 12 / 12, "
                  "`rd_kafka_buf_*_tags` loop macros 13 / 4, OpenSSL `sk_*_num` 4 / 3, pointer comparisons 4 / 3; "
                  "sole blocker of 2 rows (`thrd_setname`: `!pthread_setname_np(pthread_self(), name)`, nested and "
                  "uncontracted; Monkey `mk_socket_set_nonblocking`: a nested `fcntl` on a descriptor formal)."),
         "source": SRC},
        {"id": "W44-CONDITION-CALL-ASSIGNMENT", "operation": "amend",
         "text": ("RE-CENSUSED by K2-NO-TYPED-ACCESS (round 37, the user's 2026-11-24 current run): 33 K2 "
                  "leaves on 31 Unknown rows are an assignment in a condition (`if ((err = f(...)))`, "
                  "`(err = ErrorCode)`); none is the sole blocker of a row."),
         "source": SRC},
        {"id": "CG07-GENERIC-TOKEN-AST-MATCH", "operation": "amend",
         "text": ("OBSERVED by K2-NO-TYPED-ACCESS (round 37): `rd_kafka_conf_validate_single`'s only leaf, "
                  "`return !strchr(val, ',') && !strchr(val, ' ');`, is glibc's const-preserving `strchr` "
                  "`_Generic` (`_Generic (0 ? (val) : (void *) 1, const void *: (const char *) (strchr (val, ',')), "
                  "default: strchr (val, ','))`); the value-connective and nested-call readers answer `no unique "
                  "Clang node` (real-unit probe with the parity stream on base 5087f9295). The same body spelled "
                  "with a non-macro interior-result callee is Safe; a sole-blocker row of this entry."),
         "source": SRC},
        {"id": "K2-BUILTIN-BIT-COUNTS", "operation": "register",
         "title": "`__builtin_clz`/`__builtin_ctz` calls stay refused: `LZ4_NbCommonBytes`, `log2_floor`, 2 sole-blocker rows on current; needs a user decision",
         "text": ("Registered by K2-NO-TYPED-ACCESS. No contract or table names the bit-count builtins "
                  "(``call to uncontracted function `__builtin_clz` ``; the elaborator reads a builtin callee as "
                  "an indirect call, CG22). Their result is undefined at 0. OPEN USER CHOICES: (a) "
                  "obligation-bearing, call = __builtin_c{l,t}z{,l,ll}(x) ∧ (path ⊢ x ≠ 0) ⇒ "
                  "custody_neutral(call): admits `log2_floor` (`n == 0 ? -1 : 31 ^ rd_clz(n)`) and not "
                  "`LZ4_NbCommonBytes` (its `assert(val != 0)` is `((void)0)`); (b) value-only, call = "
                  "__builtin_c{l,t}z{,l,ll}(x) ⇒ custody_neutral(call): admits both, departing from rule 10s's "
                  "rule that a node with a C fault case is never custody-neutral; and where the declarations "
                  "live (not libc). DISCHARGE: the decision, then a table or provider with a Lean model and "
                  "fixtures, measured."),
         "source": SRC, "recorded_by": "K2-NO-TYPED-ACCESS"},
    ],
}
json.dump(document, open(out, "w"), indent=1, ensure_ascii=False)
open(out, "a").write("\n")
