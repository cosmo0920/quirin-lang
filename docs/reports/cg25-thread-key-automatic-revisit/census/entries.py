"""Write <n>.entry: the name of the first function in each unit's source that
names a thread-key call (read-only inputs). Usage: entries.py <units-dir>"""
import os
import re
import sys

out = sys.argv[1]
call = re.compile(
    r"pthread_(get|set)specific|pthread_key_(create|delete)|FLB_TLS_(GET|SET|INIT)|MK_TLS_(GET|SET|INIT)"
)
head = re.compile(r"^(?:static\s+|inline\s+|MK_INLINE\s+)*[A-Za-z_][\w\s\*]*?\b([A-Za-z_]\w*)\s*\(")
for line in open(os.path.join(out, "units.tsv")):
    n, _, path = line.rstrip("\n").split("\t")
    lines = open(path, errors="replace").read().split("\n")
    name = None
    current = None
    for text in lines:
        m = head.match(text)
        if m and not text.rstrip().endswith(";") and m.group(1) not in ("if", "while", "for", "switch", "return"):
            current = m.group(1)
        if call.search(text) and not text.lstrip().startswith("#") and current:
            name = current
            break
    open(os.path.join(out, f"{n}.entry"), "w").write((name or "main") + "\n")
    print(n, name)
