#!/usr/bin/env python3
"""Census of acquire/release callees whose resource is none of the four kinds.

Usage: kind_census.py DIAGNOSTICS_JSON[.gz] [OUT_JSON]

Card CG25-RESOURCE-KINDS. For every callee in CALLEES (acquire/release pairs
that are not Heap, Descriptor, Stream or Directory today) it counts the
`call to uncontracted function NAME` leaves, the functions carrying one (with
their status), and the Unknown functions whose *only* distinct diagnostic
description is that leaf (sole-blocked, the w43-descriptor-resource-tracking
inventory's definition). Every other description that names a callee in
backticks is counted separately, so a call that already meets another route
is visible. The shape is that card's `descriptor_inventory.py`.
"""

import collections
import gzip
import json
import re
import sys

# family -> callees (acquire first, then release)
FAMILIES = {
    "library handle": "dlopen dlmopen dlclose",
    "regex (in place)": "regcomp regfree",
    "pthread mutex (in place)": "pthread_mutex_init pthread_mutex_destroy",
    "pthread condvar (in place)": "pthread_cond_init pthread_cond_destroy",
    "pthread rwlock (in place)": "pthread_rwlock_init pthread_rwlock_destroy",
    "pthread attr (in place)": "pthread_attr_init pthread_attr_destroy",
    "pthread mutexattr (in place)": "pthread_mutexattr_init pthread_mutexattr_destroy",
    "pthread condattr (in place)": "pthread_condattr_init pthread_condattr_destroy",
    "pthread spin/barrier (in place)": "pthread_spin_init pthread_spin_destroy pthread_barrier_init pthread_barrier_destroy",
    "pthread key": "pthread_key_create pthread_key_delete",
    "C11 threads (in place)": "mtx_init mtx_destroy cnd_init cnd_destroy tss_create tss_delete",
    "semaphore": "sem_init sem_destroy sem_open sem_close",
    "glob (in place)": "glob globfree",
    "iconv": "iconv_open iconv_close",
    "addrinfo": "getaddrinfo freeaddrinfo",
    "ifaddrs": "getifaddrs freeifaddrs if_nameindex if_freenameindex",
    "timer": "timer_create timer_delete",
    "mqueue": "mq_open mq_close",
    "process stream": "popen pclose",
    "fts": "fts_open fts_close",
    "mntent": "setmntent endmntent",
    "mapping": "mmap munmap shmat shmdt",
    "locale": "newlocale freelocale duplocale",
    "spawn (in place)": "posix_spawnattr_init posix_spawnattr_destroy posix_spawn_file_actions_init posix_spawn_file_actions_destroy",
    "thread": "pthread_create pthread_join pthread_detach thrd_create thrd_join thrd_detach",
}
CALLEES = [c for names in FAMILIES.values() for c in names.split()]
FAMILY_OF = {c: f for f, names in FAMILIES.items() for c in names.split()}

UNCONTRACTED = re.compile(r"call to uncontracted function `([^`]+)`")


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def main() -> int:
    data = load(sys.argv[1])
    leaves = collections.Counter()
    functions = collections.defaultdict(set)
    sole = collections.Counter()
    sole_functions = collections.defaultdict(list)
    status = collections.defaultdict(collections.Counter)
    other = collections.Counter()
    for unit in data["units"]:
        for entry in unit["functions"]:
            diagnostics = entry.get("diagnostics") or []
            reasons = {d["description"] for d in diagnostics}
            for diagnostic in diagnostics:
                match = UNCONTRACTED.fullmatch(diagnostic["description"])
                if match and match.group(1) in FAMILY_OF:
                    callee = match.group(1)
                    leaves[callee] += 1
                    key = (unit["file"], entry["function"])
                    if key not in functions[callee]:
                        status[callee][entry.get("status")] += 1
                    functions[callee].add(key)
                elif any(f"`{c}`" in diagnostic["description"] for c in CALLEES):
                    other[diagnostic["description"]] += 1
            if entry.get("status") == "Unknown" and len(reasons) == 1:
                match = UNCONTRACTED.fullmatch(next(iter(reasons)))
                if match and match.group(1) in FAMILY_OF:
                    sole[match.group(1)] += 1
                    sole_functions[match.group(1)].append(f"{unit['file']}:{entry['function']}")
    rows = []
    for callee in CALLEES:
        if leaves[callee]:
            rows.append(
                {
                    "family": FAMILY_OF[callee],
                    "callee": callee,
                    "leaves": leaves[callee],
                    "functions": len(functions[callee]),
                    "sole_blocked": sole[callee],
                    "sole_blocked_functions": sorted(sole_functions[callee]),
                    "function_status": dict(status[callee]),
                    "function_list": sorted(f"{u}:{f}" for u, f in functions[callee]),
                }
            )
    families = collections.defaultdict(lambda: {"leaves": 0, "functions": set(), "sole_blocked": 0})
    for row in rows:
        family = families[row["family"]]
        family["leaves"] += row["leaves"]
        family["functions"].update(row["function_list"])
        family["sole_blocked"] += row["sole_blocked"]
    all_functions = set().union(*functions.values()) if functions else set()
    summary = {
        "source": sys.argv[1],
        "callees_present": len(rows),
        "leaves": sum(leaves.values()),
        "functions": len(all_functions),
        "sole_blocked": sum(sole.values()),
        "families": {
            name: {"leaves": f["leaves"], "functions": len(f["functions"]), "sole_blocked": f["sole_blocked"]}
            for name, f in sorted(families.items(), key=lambda item: -item[1]["leaves"])
        },
        "absent": [c for c in CALLEES if not leaves[c]],
        "rows": rows,
        "other_descriptions": dict(other.most_common()),
    }
    text = json.dumps(summary, indent=1) + "\n"
    if len(sys.argv) > 2:
        with open(sys.argv[2], "w") as out:
            out.write(text)
    for name, family in summary["families"].items():
        print(f"{name:34} leaves {family['leaves']:4}  functions {family['functions']:4}  sole {family['sole_blocked']:3}")
    for row in rows:
        print(f"  {row['callee']:28} {row['leaves']:4} {row['functions']:4} {row['sole_blocked']:3} {row['function_status']}")
    print("other descriptions naming a callee:", len(summary["other_descriptions"]))
    for description, count in list(summary["other_descriptions"].items())[:30]:
        print(f"  {count:4}  {description}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
