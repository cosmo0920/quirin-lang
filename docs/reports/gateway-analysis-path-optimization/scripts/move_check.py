#!/usr/bin/env python3
"""GATEWAY-ANALYSIS-PATH-OPTIMIZATION: prove a split commit is a move.

Usage: move_check.py REV   (run from the repository root; REV is the split commit)

Compares, as multisets of lines, everything REV removed from
`crates/qcore-c-gateway/src/legacy_c.rs` against everything it added to the
new `legacy_*.rs` module files, after erasing the one edit split.py makes to
moved text (a `pub(super) ` visibility prefix) and dropping the lines split.py
writes itself (each new file's header and `impl Parser<'_> {` wrapper, and the
parent's module declarations). Blank lines are ignored (a cut segment loses
its trailing blank line). The prefix is erased on both sides, so a line that
was already `pub(super)` compares too. Prints the counts and every line that is not
accounted for; exit 0 only when nothing is.
"""
import collections
import subprocess
import sys

rev = sys.argv[1]
PARENT = "crates/qcore-c-gateway/src/legacy_c.rs"
diff = subprocess.run(
    ["git", "show", "--format=", "--unified=0", "--no-renames", rev, "--", "crates/qcore-c-gateway/src"],
    capture_output=True, text=True, check=True).stdout

removed, added, parent_added = collections.Counter(), collections.Counter(), []
current = None
new_files = set()
for line in diff.splitlines():
    if line.startswith("+++ "):
        current = line[6:] if line.startswith("+++ b/") else None
        continue
    if line.startswith("--- "):
        if line == "--- /dev/null":
            new_files.add("pending")
        continue
    if line.startswith("@@") or line.startswith("diff ") or line.startswith("index ") or line.startswith("new file"):
        continue
    if current is None:
        continue
    if line.startswith("-") and current == PARENT:
        text = line[1:]
        if text.strip():
            removed[text.replace("pub(super) ", "", 1)] += 1
    elif line.startswith("+"):
        text = line[1:]
        if current == PARENT:
            parent_added.append(text)
            continue
        if not text.strip():
            continue
        if text.startswith("//! ") or text == "//!" or text == "use super::*;":
            continue
        added[text.replace("pub(super) ", "", 1)] += 1

# split.py's own wrapper lines: one `impl Parser<'_> {` / `}` pair per impl module
wrappers = added["impl Parser<'_> {"]
unexplained_removed = removed - added
unexplained_added = added - removed
# each wrapper adds one `impl Parser<'_> {` and one `}` that nothing removed
for text, allowance in (("impl Parser<'_> {", wrappers), ("}", wrappers)):
    if unexplained_added[text] <= allowance:
        unexplained_added.pop(text, None)
parent_unexplained = [
    text for text in parent_added
    if text.strip() and not (
        text.startswith("// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION:")
        or text.startswith("#[path = \"legacy_")
        or text.startswith("#[cfg(feature = ")
        or text.startswith("mod ")
        or text.startswith("pub use self::"))
]
print(f"removed from legacy_c.rs: {sum(removed.values())} non-blank lines")
print(f"added to the new modules: {sum(added.values())} non-blank lines (headers excluded)")
print(f"impl wrappers: {wrappers}")
for text, count in sorted(unexplained_removed.items()):
    print(f"REMOVED, not re-added x{count}: {text}")
for text, count in sorted(unexplained_added.items()):
    print(f"ADDED, not removed x{count}: {text}")
for text in parent_unexplained:
    print(f"PARENT ADDED: {text}")
ok = not unexplained_removed and not unexplained_added and not parent_unexplained
print("MOVE-ONLY" if ok else "NOT A PURE MOVE")
sys.exit(0 if ok else 1)
