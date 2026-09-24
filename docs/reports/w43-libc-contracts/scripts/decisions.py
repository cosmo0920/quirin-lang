#!/usr/bin/env python3
"""Join the libc/POSIX callee inventory with this card's per-callee decision.

Usage: decisions.py INVENTORY.json OUT.json OUT.md MODULE.qcore...

INVENTORY.json is `libc_inventory.py`'s output over the baseline. Every callee
of class `libc`, `libc-with-tree-body` or `libc-inline` must receive exactly one
decision: `declared` (its name is declared by one of the MODULE files) or one of
the refusal grounds below. A libc callee with no decision, or a decided name
that is not in the inventory, fails the script (exit 1). OUT.md is the
Markdown table the report embeds.
"""

import json
import pathlib
import re
import sys

REFUSED = {
    "result-alias": (
        "the result (or a stored out-pointer) aliases an argument or retained storage; "
        "v1 has no result mode meaning 'aliases parameter n' (`LIBC-MEM-RESULT-ALIAS`)",
        ["memcpy", "memmove", "memset", "strcpy", "strncpy", "strchr", "strrchr", "strstr",
         "strcasestr", "memchr", "memrchr", "gmtime_r", "localtime_r", "realpath",
         "pthread_getspecific", "strtol", "strtoll", "strtoull", "strtod", "getpwnam_r",
         "getgrnam_r"],
    ),
    "abi-restrict": (
        "faithful in v1, but the prototype spells a qualifier after the `*` (`T *__restrict p`); "
        "the declared-ABI reader withdrew the declaration at every call site "
        "(`LOADER-ABI-RESTRICT-QUALIFIER`, measured by this card's trial run)",
        ["printf", "fprintf", "snprintf", "sprintf", "stat", "strftime", "gettimeofday",
         "getsockopt", "getpeername", "dlsym", "cnd_timedwait", "pthread_sigmask", "regexec",
         "regerror"],
    ),
    "abi-sockaddr-union": (
        "`__CONST_SOCKADDR_ARG` is a transparent union under `_GNU_SOURCE`; the reader "
        "withdrew the declaration at every call site",
        ["bind", "connect"],
    ),
    "abi-enum-typedef": (
        "first parameter is the enumeration typedef `__rlimit_resource_t`, which the reader "
        "does not project; withdrawn at every call site",
        ["getrlimit", "setrlimit"],
    ),
    "abi-va-list": (
        "takes a `va_list` (`__builtin_va_list`), for which v1 has no ABI spelling",
        ["vprintf", "vsnprintf"],
    ),
    "no-return": (
        "does not return; v1 has no no-return mode and a `normal` clause would add a "
        "continuation the C does not have",
        ["exit", "abort"],
    ),
    "descriptor-or-handle-lifecycle": (
        "creates or releases a descriptor, stream, directory, library or result-list handle; "
        "v1 custody is pointer-only and kind-less, so a neutral declaration would erase the "
        "obligation and an owned/released pair would conflate it with `malloc`/`free`",
        ["open", "close", "socket", "accept4", "fcntl", "fopen", "fclose", "opendir",
         "closedir", "dlopen", "dlclose", "getaddrinfo", "freeaddrinfo", "fts_open",
         "fts_close", "glob", "globfree"],
    ),
    "in-place-object-lifecycle": (
        "initialises or destroys an object the caller owns in place (a destroy obligation "
        "v1 cannot state)",
        ["regcomp", "regfree", "mtx_init", "mtx_destroy", "pthread_rwlock_init",
         "pthread_rwlock_destroy", "pthread_attr_init", "pthread_key_create"],
    ),
    "retains-pointer": (
        "retains its pointer argument past the call without an owner v1 can name",
        ["pthread_setspecific"],
    ),
    "callback": (
        "invokes a caller-supplied function whose effects the declaration cannot state",
        ["qsort", "call_once", "pthread_once"],
    ),
    "pointer-to-pointer-borrow": (
        "writes through `void **`; `ptr<mut, ptr<mut, u8>>` admits only `resource_out`",
        ["backtrace"],
    ),
    "header-inline-body": (
        "defined `static __inline` by a glibc header inside the unit: a checkable body, not "
        "an external prototype, so a name-keyed contract would replace it with an assumption",
        ["__bswap_16", "__bswap_32", "__bswap_64", "__uint64_identity"],
    ),
}
LIBC_CLASSES = {"libc", "libc-with-tree-body", "libc-inline"}


def main() -> int:
    inventory = json.loads(pathlib.Path(sys.argv[1]).read_text())
    out_json, out_md = pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
    declared = {}
    for module in sys.argv[4:]:
        text = "\n".join(l.split("//", 1)[0] for l in pathlib.Path(module).read_text().split("\n"))
        for name in re.findall(r"extern fn (\w+)\(", text):
            declared[name] = pathlib.Path(module).name
    grounds = {}
    for ground, (_, names) in REFUSED.items():
        for name in names:
            assert name not in grounds and name not in declared, name
            grounds[name] = ground
    rows, errors = [], []
    libc = [r for r in inventory["rows"] if r["class"] in LIBC_CLASSES]
    names = {r["callee"] for r in libc}
    for r in libc:
        name = r["callee"]
        if name in declared:
            decision, detail = "declared", f"libc/{declared[name]}"
        elif name in grounds:
            decision, detail = "refused", grounds[name]
        else:
            decision, detail = None, None
            errors.append(f"no decision for {name}")
        rows.append({
            "callee": name, "class": r["class"], "decision": decision, "detail": detail,
            "leaves": r["leaves"], "functions": r["functions"], "sole_blocked": r["sole_blocked"],
            "blocked_only_by_uncontracted_calls": r["blocked_only_by_uncontracted_calls"],
            "statuses": r["statuses"], "headers": r["libc_headers"],
            "tree_definitions": r["tree_definitions"], "blocked_functions": r["blocked_functions"],
        })
    for name in set(declared) - names:
        errors.append(f"declared name {name} is not a baseline libc callee")
    for name in set(grounds) - names:
        errors.append(f"refused name {name} is not a baseline libc callee")
    rows.sort(key=lambda r: (-r["sole_blocked"], -r["functions"], r["callee"]))
    excluded = {}
    for r in inventory["rows"]:
        if r["class"] not in LIBC_CLASSES:
            excluded.setdefault(r["class"], []).append(r)
    record = {
        "inventory_callees": inventory["callees"],
        "class_counts": inventory["class_counts"],
        "libc_callees": len(rows),
        "declared": sum(r["decision"] == "declared" for r in rows),
        "refused": sum(r["decision"] == "refused" for r in rows),
        "refusal_grounds": {g: {"reason": reason, "callees": sorted(n for n in ns if n in names)}
                            for g, (reason, ns) in REFUSED.items()},
        "libc_rows": rows,
        "excluded": {
            cls: {
                "callees": len(items),
                "leaves": sum(i["leaves"] for i in items),
                "sole_blocked": sum(i["sole_blocked"] for i in items),
                "top_by_sole_blocked": [
                    {"callee": i["callee"], "functions": i["functions"], "sole_blocked": i["sole_blocked"],
                     "evidence": (i["corpus_occurrences"] or i["other_headers"])[:2]}
                    for i in sorted(items, key=lambda i: (-i["sole_blocked"], -i["functions"], i["callee"]))[:25]
                ],
            }
            for cls, items in sorted(excluded.items())
        },
        "errors": errors,
    }
    out_json.write_text(json.dumps(record, indent=1) + "\n")
    lines = [
        "# W43-LIBC-CONTRACTS: every libc/POSIX callee of the uncontracted-call family",
        "",
        "Generated by `scripts/decisions.py` from `scripts/libc_inventory.py` over the",
        "W43-RETURN-EXPRESSION baseline diagnostics. Columns count, over all 4,834",
        "projected functions: the functions whose diagnostics name the callee in a",
        "`call to uncontracted function` leaf; the Unknown functions for which that",
        "leaf is the only distinct description (sole-blocked); and the leaves.",
        "Headers, tree bodies and every blocked function are in",
        "[libc-inventory.json](libc-inventory.json); refusal grounds are defined in",
        "[the report](README.md#refused-callees).",
        "",
        "| Callee | Blocks (functions) | Sole-blocked | Leaves | Decision |",
        "|---|---:|---:|---:|---|",
    ]
    for r in rows:
        detail = f"declared, `{r['detail']}`" if r["decision"] == "declared" else f"refused: {r['detail']}"
        lines.append(f"| `{r['callee']}` | {r['functions']} | {r['sole_blocked']} | {r['leaves']} | {detail} |")
    out_md.write_text("\n".join(lines) + "\n")
    print(json.dumps({k: record[k] for k in ("inventory_callees", "class_counts", "libc_callees", "declared", "refused", "errors")}, indent=1))
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
