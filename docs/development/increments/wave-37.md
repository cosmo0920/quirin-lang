# Wave 37 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`CT-LANE-5` (2026-09-18).** Wave 37's CT mapping lane, base `0f6eb0a6`. **One
case re-mapped, three re-measured and left unmapped, one not attempted, and no
count moved.** CT07's scenario is *two disjoint array halves mutated
independently*, and for two waves the word **independently** was carried by a
program with one thread in it: wave 28 mapped two disjoint *read* loans, wave 30
(`CT07-WRITE-REMAP`) two *write* authorities — and said so itself, "**NOT two
threads**; the task-crossing half is residual `WRITE-LEND-TASKS`". That
residual's successor landed in wave 31 and became exercisable in wave 36, whose
`WRITE-LEND-TASKS-EXERCISE` deliberately left the row alone and named the work as
its own follow-on item 7 with a discharge condition. This card discharges it:
CT07 moves `sequential` → `task`, `sequential_sanitized` →
`native_heap_scenarios`, becoming **CT03's write twin** — where CT03 is two
readers over one allocation, CT07 is two `thrd_create`'d writers over disjoint
byte ranges of one allocation, each exercising its own range write lease in its
own worker body, both leases outstanding before either byte is written, the owner
reading both bytes back after the joins, one allocation matched **by address**,
17 observation lines compared line for line, clean under ASan/UBSan/LSan.
**Mapped stays 46/50, sanitized-native 10/46, proof 3/46**; CT07 was mapped
before and after, so this is evidence quality and not coverage (constraint 23),
and the wave-30 record is superseded **from this card's own record** with its
bytes untouched (constraint 21). The remap is a **strict superset** rather than a
swap: wave 30's sequential program is retained as an accept control and its three
reject twins as rejection controls, which took two `serde`-defaulted harness
fields — `accept_controls`, because the heap mode had nowhere to put an accept
control, and `diagnostic_sequence`, because one retained control refuses in *two*
layers (`[Q054, Q053]`) and a head-only match would have let either fall silent.
Seven mutations, **seven caught**, so the row is green about something. The three
cases left unmapped are each published with their measurement rather than
inherited: **CT42** fails on both arms independently — the C twin needs a contract
loader change that cascades into `qcore-schemas` (a descriptor's failure value is
`-1`, which `ResourceReturnMode` cannot express at all), `qcore-core` and the
gateway, which this lane may not touch; and the socket cell has the model, the
Lean and `Q664` but **zero source routing**, so a row there would name stages the
harness cannot execute. **CT10**'s sub-byte half exists in exactly one
model-level module with zero frontend callers, whose own doc says the bit range
"never participates in the disjointness decision" — and the tempting byte
encoding was measured **unsound in both directions**, not merely coarse
(`CT10-BYTE-ENCODING-UNSOUND`). **CT41** is not re-attempted, there being no new
capability; **CT24** is not attempted, constraint 25 making a third withdrawal
actively hazardous. Two boundaries are published rather than papered over: there
is **no ThreadSanitizer evidence** for this case or this route
(`TASK-ROUTE-TSAN-SEGV`), so "two threads, distinct memory locations, no race" is
carried by C11 3.14 plus the sanitizer and allocation evidence and not by a race
detector; and the byte-identical trace **does not witness interleaving**, because
worker events are buffered privately and replayed by the parent
(`CT07-TWO-THREAD-REPLAY-ORDER`). The card also hit and measured a structural
hazard: eight CT fixture directories are read by `include_str!` from **outside**
`ct_corpus.rs` — three of them the proof-gated cases — so re-pointing a case can
silently re-point another card's tests, which is exactly what happened to
`write_lend.rs` here and was caught by a stale doc comment rather than by a
mechanism (`CT-FIXTURE-EXTERNAL-READERS`). Gate at the card tip: **4,611 passed /
0 failed / 8 ignored** over 231 summaries, fmt and clippy clean, both probes exit
0, **135 Lean audits**, `ct_stage_summary --check` exit 0 with no drift; the
first gate run was **killed rather than reported** because it predated the
`write_lend.rs` fix. `named_followups` 187 → **191**. +0 reserved words, +0
tokens, +0 `Q` codes, and no C gateway source change.

