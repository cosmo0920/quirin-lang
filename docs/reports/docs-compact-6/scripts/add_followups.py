import json, pathlib, sys

p = pathlib.Path(sys.argv[1])
m = json.loads(p.read_text(encoding="utf-8"))
nf = m["named_followups"]
have = {e["id"] for e in nf}

new = [
    {
        "id": "WRITE-LEND-WINDOWS-RUN",
        "title": "The wave-29 write lend's native test ran the Linux arm only; no Windows or Apple run is claimed",
        "why": "CLAUDE.md's platform records have named this residual since wave 29 and it reached no entry in this ledger until card DOCS-COMPACT-6 (wave 34) swept the three authoritative files for residual-shaped names absent from named_followups. SLICE-PERMISSIONS-WRITE-LEND's one native test ran the LINUX arm; the emitted difference is carried through the unchanged `windows_c` path, but no MSVC and no Apple run has happened and none is claimed (docs/reports/slice-permissions-write-lend/README.md SS13). Wave 30's CT07-WRITE-REMAP left it unchanged. It is the sibling of SLICE-PERMISSIONS-WINDOWS-RUN, which STD-MODULES-O's roll-up does mention; this one and WRITE-LEND-TASKS-WINDOWS-RUN fell outside that roll-up. Discharge: run the card's named native test on Windows through tests/support/windows_c.rs::run_msvc and record the result, or restate the row as a Linux-only measurement.",
        "source": "docs/reports/slice-permissions-write-lend/README.md SS13, CLAUDE.md Platform records",
        "recorded_by": "DOCS-COMPACT-6",
    },
    {
        "id": "WRITE-LEND-TASKS-WINDOWS-RUN",
        "title": "The wave-31 task-route range write lease's native test ran the Linux arm only",
        "why": "Named in CLAUDE.md's platform records since wave 31 and absent from this ledger until card DOCS-COMPACT-6 (wave 34). WRITE-LEND-TASKS' native test ran the Linux arm; macOS compiles the same test file; no MSVC run has happened and none is claimed (docs/reports/write-lend-tasks/README.md SS9), on the STRING-CONCAT-WINDOWS-RUN precedent. Discharge: the same exact-test-name condition as its two siblings - run it on Windows and record the result, or restate the row as a Linux-only measurement.",
        "source": "docs/reports/write-lend-tasks/README.md SS9, CLAUDE.md Platform records",
        "recorded_by": "DOCS-COMPACT-6",
    },
    {
        "id": "SLICE-PERMISSIONS-COMPOSED-LEND",
        "title": "A write lend beside borrow_utf8's read loans refuses by name, pending a cross-model overlap rule",
        "why": "Standing constraint 15h and the Q068-Q079 row both state that on the COMPOSED string/buffer route a `lend(...)` refuses by name at Q077 'until a cross-model overlap rule with borrow_utf8's read loans exists'. That 'until' is open work, and it was named only inside a constraint paragraph - the exact shape RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK describes - reaching no ledger entry until card DOCS-COMPACT-6 (wave 34). The refusal is fail-closed and nothing rests on it being lifted; what is open is the rule that would let the lend-extended ownership IR's byte ranges and the composed route's exact borrow windows be judged against each other. Discharge: land that cross-model rule with its own card, or record the refusal as a permanent measured boundary of the composed route.",
        "source": "AGENTS.md constraint 15h and the Q068-Q079 diagnostics row; docs/reports/slice-permissions-write-lend/README.md",
        "recorded_by": "DOCS-COMPACT-6",
    },
    {
        "id": "QUIRIN-ARM64-GATE",
        "title": "ARM64 Windows runs the compiler and builds emitted C; no test-suite, sanitizer or byte-comparison claim exists for that architecture",
        "why": "CLAUDE.md's cross-platform record offers QUIRIN-ARM64-GATE and QUIRIN-ARM64-RUN as 'candidate residual names' and neither reached this ledger until card DOCS-COMPACT-6 (wave 34). The recorded observation is that ARM64 Windows STARTS the quirin compiler and compiles emitted C sources. No standing per-change gate has been run on that architecture, so the workspace suite, the sanitizer arms and every byte comparison are unmeasured there - absence of evidence, recorded as such rather than as a limitation. Discharge for the GATE half: run AGENTS.md's standing per-change gate on ARM64 Windows and publish the result, or state the architecture as explicitly out of gate scope.",
        "source": "CLAUDE.md Platform records (cross-platform build)",
        "recorded_by": "DOCS-COMPACT-6",
    },
    {
        "id": "QUIRIN-ARM64-RUN",
        "title": "No emitted-program execution or byte comparison has been recorded on ARM64 Windows",
        "why": "The RUN half of the pair CLAUDE.md names as candidates (see QUIRIN-ARM64-GATE), registered by card DOCS-COMPACT-6 (wave 34). The environment report covers compiler start-up and emitted-C BUILD; it does not cover executing an emitted program and comparing its stream with the checked reference observation, which is the standard every published example is held to on Linux. Discharge: run the examples suite's native comparison on ARM64 Windows and record it, or state that the architecture carries the build claim only.",
        "source": "CLAUDE.md Platform records (cross-platform build)",
        "recorded_by": "DOCS-COMPACT-6",
    },
    {
        "id": "CONSTRAINT-NAMED-BOUNDARIES-NOT-OPEN-WORK",
        "title": "Nine residual-shaped names in AGENTS.md's constraints are measured boundaries, not open work - classified rather than registered",
        "why": "Card DOCS-COMPACT-6 (wave 34) swept AGENTS.md, CLAUDE.md and README.md for screaming-kebab residual-shaped names absent from named_followups, discharging the enumerate half of RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK. Fifteen names had no mention anywhere in this ledger. Five are open work and are now registered individually (WRITE-LEND-WINDOWS-RUN, WRITE-LEND-TASKS-WINDOWS-RUN, SLICE-PERMISSIONS-COMPOSED-LEND, QUIRIN-ARM64-GATE, QUIRIN-ARM64-RUN). One, DOCS-COMPACT-5-RECONCILIATION-LIST, is DISCHARGED - CLAUDE.md's own text records W32-RECONCILE-SWEEP discharging it - and is deliberately not registered as open. The remaining NINE name RULES the constraints impose, each a refusal or a stated ground that holds today rather than work anyone owes: REGION-JOINS-EPOCH-AXIS (the ground that creator-scope bytes lie outside the epoch allocation), REGION-JOINS-ERASURE-FIT, REGION-JOINS-MULTI-OWNER, REGION-JOINS-T07, SLICE-PERMISSIONS-CROSSED-WINDOWS, SLICE-PERMISSIONS-LOOP-WINDOW, WRITE-LEND-BOUND-FROM-BYTE, WRITE-LEND-COMPUTED-INDEX, WRITE-LEND-NESTED-ACQUIRE. They are recorded here so a later sweep finds the classification rather than re-raising them, and so the distinction the occasioning residual drew - a constraint states a RULE, this ledger states WORK - is written down once. This entry claims nothing about whether any of the nine should later become work: a card that lifts one of those refusals must name it then. Discharge: none required; retire this entry if the nine are ever given a different disposition.",
        "source": "docs/reports/docs-compact-6/README.md",
        "recorded_by": "DOCS-COMPACT-6",
    },
]

added = [e for e in new if e["id"] not in have]
nf.extend(added)
p.write_text(json.dumps(m, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
print("added:", [e["id"] for e in added])
print("named_followups now:", len(nf))
print("dimensions.named_followups (stale until regeneration):", m["dimensions"]["named_followups"])
