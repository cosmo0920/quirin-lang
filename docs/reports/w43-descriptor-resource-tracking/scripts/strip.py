#!/usr/bin/env python3
"""Strip line markers from captured -E outputs and de-duplicate by content."""
import hashlib
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
source = ROOT / "measure/capture/pp"
out = ROOT / "measure/capture/plain"
out.mkdir(exist_ok=True)
seen = set()
for path in sorted(source.glob("*.i")):
    text = "\n".join(line for line in path.read_text(errors="replace").split("\n") if not line.startswith("# "))
    digest = hashlib.sha256(text.encode()).hexdigest()
    if digest in seen:
        continue
    seen.add(digest)
    (out / (digest[:16] + ".i")).write_text(text)
print(len(seen))
