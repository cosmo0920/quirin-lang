#!/usr/bin/env python3
"""Card W37-CORPUS-CONTRACTS-4: resolve the rebase conflict in the matrix.

Both sides APPEND to `named_followups`: main reached 197 across the wave-36
close and wave 37's two code arbiters, this card adds ten. The resolution keeps
both runs of entries as siblings. The derived count is set consistently here
but is the GENERATOR's to own - `dimensions.named_followups` is derived and
`CommittedMatrixIsSelfConsistent` checks it against the list, so the file is
regenerated through build_matrix.py afterwards and never left hand-set
(constraint 25; the wave-36 close recorded exactly this trap).
"""
import json
import pathlib

p = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
lines = p.read_text().split("\n")

# --- hunk 2: the two runs of follow-on entries (0-based indices) ---
a, b, c = 7409, 7634, 7693
assert lines[a].startswith("<<<<<<<"), lines[a]
assert lines[b].startswith("======="), lines[b]
assert lines[c].startswith(">>>>>>>"), lines[c]
head_side = lines[a + 1:b]
my_side = lines[b + 1:c]
# the shared "  {" before and "  }" after stay put; join the runs as siblings
merged = head_side + ["  },", "  {"] + my_side
lines = lines[:a] + merged + lines[c + 1:]

# --- hunk 1: the derived count ---
i, j, k = 197, 199, 201
assert lines[i].startswith("<<<<<<<"), lines[i]
assert lines[j].startswith("======="), lines[j]
assert lines[k].startswith(">>>>>>>"), lines[k]
lines = lines[:i] + ['  "named_followups": 207'] + lines[k + 1:]

p.write_text("\n".join(lines))

d = json.loads(p.read_text())
ids = [e["id"] for e in d["named_followups"]]
dupes = sorted({x for x in ids if ids.count(x) > 1})
mine = [
    "LOADER-ABI-INLINE-FNPTR-DECLARATOR", "RDKAFKA-SORT-WITHDRAWN",
    "W37-COVER-FRONTIER-IS-STATIC", "NESTED-CALL-IN-ARGUMENT-POSITION",
    "CTRACES-FORMATTER-CHAIN", "LIBC-PRINTF-FAMILY",
    "LOADER-ABI-RESTRICT-QUALIFIER", "CONDITIONAL-RELEASE-BY-LITERAL",
    "CPROFILES-STRAY-DUPLICATE-TREE", "MPACK-WRITER-INTERIOR-REALLOC",
]
print("JSON parses OK")
print("named_followups list length:", len(ids))
print("stated dimensions count:", d["dimensions"]["named_followups"])
print("duplicate ids:", dupes or "none")
print("all ten of this card's present:", all(m in ids for m in mine))
print("main's newest still present (JOINED-PROVENANCE-BOUND-NOT-TIGHT):",
      "JOINED-PROVENANCE-BOUND-NOT-TIGHT" in ids)
print("main's W36-COVER-HIDDEN-BLOCKER still present:",
      "W36-COVER-HIDDEN-BLOCKER" in ids)
