#!/usr/bin/env python3
"""Apply the drafted declarations. Usage: apply_contracts.py exact|interior"""
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
LIBC = ROOT / "providers/quirin-provider-libc/contracts/libc"
DRAFTS = ROOT / "measure/drafts"
stage = sys.argv[1]


def rewrite(path, text):
    path.write_bytes(text.encode())
    assert path.read_bytes() == text.encode()


if stage == "exact":
    string = (LIBC / "string.qcore").read_text()
    old = """// ## Deliberately NOT declared here (recorded in the card report)
//
//   `memcpy`, `memmove`, `memset`, `strcpy`, `strncpy` return their FIRST
//   ARGUMENT, and `strchr`, `strrchr`, `strstr`, `strcasestr`, `memchr`,
//   `memrchr` return a pointer INTO their first argument. v1 has no result
//   mode meaning "aliases parameter n"; `resource_return_none()` would drop the
//   alias and is therefore not exact (`LIBC-MEM-RESULT-ALIAS`).
"""
    new = """// ## Declared later in this module (card W43-LIBC-RESULT-ALIAS)
//
//   `memcpy`, `memmove`, `memset`, `strcpy`, `strncpy` return their FIRST
//   ARGUMENT, and `strchr`, `strrchr`, `strstr`, `strcasestr`, `memchr`,
//   `memrchr` return a pointer INTO their first argument. Card W43-LIBC-CONTRACTS
//   could not state either (`LIBC-MEM-RESULT-ALIAS`); they are declared at the
//   end of this module with the derived-result modes of contract spec C04.3.1.
"""
    assert string.count(old) == 1
    string = string.replace(old, new, 1)
    string = string.rstrip("\n") + "\n" + (DRAFTS / "string-exact.qcore").read_text()
    rewrite(LIBC / "string.qcore", string)
    time = (LIBC / "time.qcore").read_text()
    old = """//   `gmtime_r`/`localtime_r` return their SECOND argument (or NULL), an alias
//   v1 cannot state.
"""
    new = """//   (`gmtime_r`/`localtime_r` return their SECOND argument or NULL; card
//   W43-LIBC-RESULT-ALIAS declares them at the end of this module.)
"""
    assert time.count(old) == 1
    time = time.replace(old, new, 1)
    time = time.rstrip("\n") + "\n" + (DRAFTS / "time-exact.qcore").read_text()
    rewrite(LIBC / "time.qcore", time)
elif stage == "interior":
    string = (LIBC / "string.qcore").read_text()
    assert "extern fn strchr(" not in string
    string = string.rstrip("\n") + "\n" + (DRAFTS / "string-interior.qcore").read_text()
    rewrite(LIBC / "string.qcore", string)
else:
    raise SystemExit("usage: apply_contracts.py exact|interior")
print(stage, "applied")
