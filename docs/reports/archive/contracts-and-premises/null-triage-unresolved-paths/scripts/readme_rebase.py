#!/usr/bin/env python3
"""readme_rebase.py -- NULL-TRIAGE-UNRESOLVED-PATHS: restate the report's
measurement on the rebased tip (BEFORE b1 at main d7b0882ac, AFTER a3 at
3b2cdb636); the first measurement (b0 at c77efedbe, a2) is kept as history."""
import pathlib

README = pathlib.Path(__file__).resolve().parents[1] / "README.md"
t = README.read_text()
EDITS = [
    ("card-high). Base: local `main` at `c77efedbe`. Prediction:",
     "card-high). Base: local `main` at `c77efedbe`, rebased onto `d7b0882ac`\n"
     "(NULL-MEMBER-INVARIANT-SCOPE landed; its `nonnull_` names adopted, section 4\n"
     "re-measured on the rebased tip). Prediction:"),
    ("**Rule ids.** The brief assigned `15c`–`15g`, but the router already uses\n"
     "`15c`–`15p` (io, effects, buffers, concurrency topics). This card takes\n"
     "`15q`–`15u`, leaving `15q`–`15u` free for the parallel card, whose own\n"
     "assignment (`14x`–`15b`) also collides at `15a`–`15b`.",
     "**Rule ids.** The brief assigned `15c`–`15g`, but the router already uses\n"
     "`15a`–`15p` (source-routes, io, effects, buffers, concurrency topics). On\n"
     "the integrator's correction this card uses `15q`–`15u`; every id is unique\n"
     "across `docs/agents/*.md` (the duplicate-id check returns nothing)."),
    ("| Corpus, 139 strong-source candidates | BEFORE `b0` | AFTER `a2` |",
     "| Corpus, 139 strong-source candidates | BEFORE `b1` | AFTER `a3` |"),
    ("**Null property** (BEFORE `b0`, binary `3f2fdc78…` at `c77efedbe` → AFTER\n"
     "`a2`, binary `e8141daa…`; [compare-corpus.json](artifacts/compare-corpus.json)):",
     "**Null property** (BEFORE `b1`, binary `9da12dbe…` at main `d7b0882ac` → AFTER\n"
     "`a3`, binary `3ff018d6…` at `3b2cdb636`; [compare-corpus.json](artifacts/compare-corpus.json)):"),
    ("| Safe unconditional | 1,115 → 1,115 |", "| Safe unconditional | 1,113 → 1,113 |"),
    ("| Safe conditional | 1,421 → **1,423** |", "| Safe conditional | 1,433 → **1,435** |"),
    ("| Unknown | 2,363 → **2,365** |", "| Unknown | 2,353 → **2,355** |"),
    ("| candidates | 12,809 → **12,804** |", "| candidates | 12,648 → **12,643** |"),
    ("**Wall time** (inside the lock, null and parity selected): corpus 504.4 s →\n"
     "562.7 s (+11.6 %); system 74.7 → 92.8 s (rerun 83.1 s); slice 33.8 → 45.7 s\n"
     "(rerun 43.6 s). The second phase reruns the body per path up to the budgets\n"
     "(1,024 paths, 40,000 steps); the host carried a second card's load (load\n"
     "average 7–8) during the AFTER runs, so the system and slice figures are upper\n"
     "bounds, not a controlled comparison.",
     "**Wall time** (inside the lock, null and parity selected; `b1` → `a3`):\n"
     "corpus 508.8 s → 563.0 s (+10.6 %); system 75.4 → 91.8 s (+22 %); slice\n"
     "34.0 → 62.9 s (+85 %). The second phase reruns the body per path up to the\n"
     "budgets (1,024 paths, 40,000 steps): the slice's two candidates, both in\n"
     "`do_start`, exhaust the 1,024-path budget. The host carried other load (load\n"
     "average 7–14) during the AFTER runs, so these are upper bounds, not a\n"
     "controlled comparison. The first measurement before the rebase (`b0` at\n"
     "`c77efedbe` → `a2`, binary `e8141daa…`) gave the same triage, row moves and\n"
     "custody identity (corpus 504.4 → 562.7 s, system 74.7 → 92.8 s, slice\n"
     "33.8 → 45.7 s)."),
]
for old, new in EDITS:
    assert t.count(old) == 1, old[:60]
    t = t.replace(old, new)
README.write_text(t)
print("ok")
