#!/usr/bin/env python3
"""Card K6: regenerate proof/AuditAddressArrowPlace.lean's `#print axioms` lines from
every theorem of proof/QCore/AddressArrowPlace.lean (the three arrow lemmas aside).

usage: audit_file.py REPO_ROOT
"""
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1])
audit = root / "proof/AuditAddressArrowPlace.lean"
module = (root / "proof/QCore/AddressArrowPlace.lean").read_text()
text = audit.read_text()
head = text[: text.index("#print axioms")]
names = [n for n in re.findall(r"^theorem ([A-Za-z0-9_]+)", module, re.M)
         if n not in {"place_arrow", "new_arrow", "old_arrow"}]
audit.write_text(head + "".join(f"#print axioms QCore.AddressArrowPlace.{n}\n" for n in names))
print(len(names), "theorems")
