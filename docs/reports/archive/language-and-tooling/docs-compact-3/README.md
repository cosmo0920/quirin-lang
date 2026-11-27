# DOCS-COMPACT-3 — third compaction pass, plus the examples style pass

**Date:** 2026-09-12. **Base:** main `f122cc03` (wave-26 close).
**Scope:** documentation and published example *spelling* only. The diff
touches `AGENTS.md`, `CLAUDE.md`, `README.md`, `examples/README.md`, five
`examples/**/*.qcore` programs and this report directory. No crate source, no
contract module, no provider file, no test, no spec, no acceptance file, no
frozen archive and no existing report changed. **Corpus-neutral by
construction:** nothing this card touches is read by the gateway, the audit
driver or a provider, so no arbiter is engaged and no corpus row moves.

Two deliverables, and they are independent:

1. the **recurring compaction** the cadence rule in `CLAUDE.md` schedules
   (§1–§5);
2. a **user-directed examples style pass**, to be reviewed file by file (§6).

## 0. Cadence

The cadence rule says `AGENTS.md`, `CLAUDE.md` and `README.md` are compacted
every 2–3 waves. Precedents: DOCS-COMPACT (2026-09-10, wave 16),
DOCS-COMPACT-2 (2026-09-11, `cb2905f0`, after wave 23). This card runs after
wave 26 — **three waves on, the cadence is honored** — and `CLAUDE.md` now
records it as the third precedent with the **next pass due around wave 29**.

---

# Part one — the compaction

## 1. What this card did and did not do

It changed **no fact about the system**. No verdict, count, pin, boundary or
premise moved. One piece of prose that had drifted away from its source was
re-read from that source and corrected (§4).

Each file had drifted a different way since wave 23:

- `AGENTS.md` grew 567 → 828 lines across waves 24–26. The accretion is in two
  places: five new standing constraints (15e–15i) and four diagnostics rows
  that reached card-report length.
- `CLAUDE.md`'s platform records accreted discharge and residual entries, all
  narrated in one run per platform, so a reader could not see at a glance which
  claims are discharged and which are open.
- `README.md` had fallen many routes behind, as recorded at the last two
  closes: `source.loop.v1` and its unroll licence, `source.socket.v1` and
  declared INET addresses, `source.http.v1` and computed lengths, the string
  composition band, the reactor composition, and the CT row were all absent.

## 2. Before / after

| File | Before | After | Change |
|---|---|---|---|
| `AGENTS.md` | 828 lines / 67,921 bytes | **838 / 67,125** | +10 lines, **−796 bytes (−1.2%)** |
| `CLAUDE.md` | 118 / 7,259 | **122 / 7,360** | +4 lines, +101 bytes — grew, see below |
| `README.md` | 156 / 8,188 | **180 / 10,259** | **+24 lines** — a refresh, not a compaction |
| `examples/README.md` | 65 / 4,434 | **112 / 7,381** | +47 lines — the style rule (§6) |
| `DEVELOPMENT.md` | unchanged | unchanged | nothing needed relocating |

**`AGENTS.md` gained lines while losing bytes, and that is the honest shape of
this pass.** Constraint 14 went from a 78-line run-on paragraph to a per-card
ledger: more lines, fewer bytes, far less re-reading. The byte saving comes
almost entirely from the four diagnostics rows (§3.1).

**`CLAUDE.md` grew, deliberately.** The cadence rule had to record this card as
the new precedent and the next pass's due date, and it gained one sentence
telling a future pass to expect single-digit percentages rather than cut to hit
a number. The header and two working rules were tightened against `AGENTS.md`
in the same edit; the net is +101 bytes.

**`DEVELOPMENT.md` was not touched.** As at DOCS-COMPACT-2, every fact this card
tightened is still present in shorter words or at a named authority, so none
needed a new home.

## 3. What changed, section by section

### 3.1 `AGENTS.md` — the diagnostics rows (the byte saving)

Four rows had grown to card-report length while every other row sat at 100–200
bytes:

| Row | Before | After | Change |
|---|---|---|---|
| `Q068`–`Q073`, `Q075`–`Q079` `StandardString` | 1,320 B | **1,000 B** | −24% |
| `Q080`–`Q089` `StandardSocket` | 1,704 B | **1,385 B** | −19% |
| `Q090`–`Q096` `StandardHttp` | 1,492 B | **1,486 B** | −0.4% |
| `Q097`–`Q099` `Loop` | 1,510 B | **1,505 B** | −0.3% |

The last two barely moved, and that is the measurement rather than a shortfall.
Their length is per-code conditions and the widening statements that
constraint 1 makes load-bearing — `Q090`–`Q096`'s seven conditions and
`HTTP-V1-COMPUTED-LENGTH`'s two widenings, `Q097`–`Q099`'s three conditions and
`LOOP-UNROLL-BUDGET`'s three-layer budget — none of which may be summarised
away. They were read for removable prose and had almost none.

The table's own header says it is **"a summary of §L21, not a second
allocation."** That authority was re-read before anything moved:
`docs/QCORE_SPEC_r0_3.md` lines 961–984 carry one row per band with the full
per-condition reasoning, in every case longer than the `AGENTS.md` summary. The
rows now give **the code → condition map and every widening verbatim**, and
point at §L21 for the reasoning.

Nothing searchable was lost, and one thing was gained. The socket row's ten
conditions were an unnumbered prose list; re-reading §L21 confirmed the
positional mapping exactly (`Q081` operand rule, `Q082` transport state
machine, `Q084` ranges, `Q085` determinism, `Q086` crossing, `Q088` certificate
recheck, `Q089` backend refusal), so the row now names each code — and `Q088`
is individually searchable for the first time.

The two condition **widenings** are kept verbatim, because constraint 1 makes
them load-bearing: `Q083`'s from "the inherited ownership checker refused" to
"an inherited, unedited **custody model** refused", and `Q084`'s from a
byte/payload-length/receive-count range to those and a **port** range.

### 3.2 `AGENTS.md` — the rest

| Section | Action |
|---|---|
| Status ledger header | wave-26 provenance paragraph 12 → 11 lines; every claim kept — the wave-23 baseline, both chained AFTER runs, "wave 26 ran **no arbiter**" with its five-card justification, "carried" labelling, the CT and gate rows as the only movers, constraint 21's freeze |
| Constraint 7 | reworded, 2,841 → 2,777 chars (§3.3) |
| Constraint 14 | reworded, 5,140 → 5,062 chars (§3.3) |
| Constraints 15e–15i | **deliberately untouched** (§3.4) |
| Q-namespace note | restatements removed; the decision, its ground, the operative consequence, the precedent and wave 26's three uses all kept |
| Key entry points | one drift corrected (§4); the published-examples bullet gained a pointer to the new spelling rule |

### 3.3 The standing-constraint inventory (the load-bearing check)

**39 entries before, 39 after. Zero dropped, zero added, zero renumbered.**
Verified mechanically by `measure/inventory.py` (output in
`measure/constraint_inventory.txt`), which re-extracts the numbered entries,
the hard invariants, every `docs/reports/` evidence pointer and every
`Q…`/`QP…` code from `git show f122cc03:AGENTS.md` and from the working tree
and diffs the label sets.

```
standing constraints: 39 before, 39 after   lost: none   gained: none
  identical bodies: 37      reworded: ['7', '14']
hard invariants:       6 before,  6 after   lost: none   gained: none
evidence pointers:    87 before, 87 after   lost: none   gained: none
diagnostics codes:    93 before, 94 after   lost: none   gained: ['Q088']
RESULT: nothing dropped
```

**37 of 39 bodies are byte-identical after whitespace normalisation.** The two
reworded entries, and why each rewording preserves its content:

| # | Subject | Disposition |
|---|---|---|
| **7** | leak pin **266** | **Reworded**, 2,841 → 2,777 chars, in the "no fifth move" tail only. All four moves keep their arrows, wave numbers, line numbers, directions and residual names (`CONSTRUCTOR-RESIDUALS-LEAK-271`, `AOM-DESTROY-JOINED-PROVENANCE`, `AOM-SIBLINGS-BINDING-POISON`); the `nullable`/`nonnull` contrast and its `if (!routes) { return; }` ground stand; the six retired leaves and the standing L19544 exception stand; "five standing claims, not the six wave 17 recorded" and "266 rather than 265" stand; and the wave-25 finding — the L19399 leaf decided instead by the contract's **discharging** arm, its first firing in the pinned corpus, changing no count — stands with every word of its content, one sentence shorter |
| **14** | new keywords are a compatibility hazard | **Reworded**, 5,140 → 5,062 chars. A 78-line paragraph became a per-card ledger. Every count survives (T03-F +5, T08-EPOCH-SRC +8, `succeeds` +1, STD-MODULES +8, quiet-emission +2, `QUIRIN-SOCKET-V1` +10, `LOOP-V1` +1, `LOOP-UNROLL-BUDGET` +0 words/+1 token, five cards at +0), as does every keyword spelled out, every census figure (412 files, 41,529 string literals, 418 files, 422 files, eight `where` occurrences), both rejected candidates with their grounds (`stream`, `socket`), both "do not" rules (the route-selector exclusion, `in` not promoted), the `work`→`effort` rename, the contextual `error` rule, and `EXAMPLES-SUITE-WIRE`'s non-engagement. The only text removed is the phrase "both censuses run rather than assumed" repeated across four cards, now stated once over the bullet that lists them. All thirteen evidence pointers retained |

**Six hard invariants:** extracted before and after — identical label set and
byte-identical bodies.

**Evidence pointers:** 87 before, 87 after in `AGENTS.md`; 7 before and after in
`CLAUDE.md`; 1 before and after in `README.md`. Zero lost.

**Diagnostics codes:** 93 distinct `Q…`/`QP…` before, 94 after — none lost,
`Q088` gained (§3.1). Every owning-pass name preserved.

**Status vocabulary:** the Safe/Unsafe/Unknown verdict words, the "unpromoted"
phrasing, "a grown denominator is not new conformance" and "a provisional
`Unsafe` is never presented as an upstream defect" are present and unedited in
`AGENTS.md` invariant 2 and constraint 23, `CLAUDE.md`'s status-language rule
and `README.md`'s status section — where the CT sentence is newly written *in*
that vocabulary rather than around it.

### 3.4 What was deliberately not compacted, and why

**Constraints 15e–15i were not touched.** They are the wave-24-to-26 accretion
and the obvious target, and they were read line by line looking for
explanation-rather-than-rule. There is very little: nearly every sentence is a
rule, a "do not", or a named residual (`STRING-SLICE-CONCAT-RUNTIME`,
`STRING-SLICE-FREEZE`, `STRING-CONCAT-LOOP-WINDOW`, `SOCKET-EVENT-CROSSED-SPANS`,
`SOCKET-EXTERNAL-PEER`, `LOADER-ABI-LP64`'s neighbours). Their only other home
is their card reports. Compacting them would have bought a few percent at the
risk of a premise, which is the trade the cadence rule exists to refuse. A
fourth pass should reach them only when a later card has superseded one.

`measure/inventory.py` reports those five bodies byte-identical, which is the
mechanical form of this paragraph.

### 3.5 `CLAUDE.md`

- **Platform records** restructured so the *shape* of each platform's evidence
  is visible: a discharged residual and an open one are now separate
  sub-bullets rather than consecutive clauses. Every claim and its exact scope
  survives — `SOCKET-V1-WINDOWS-RUN` discharged with MSVC 19.51.36248, Winsock
  loopback, byte comparison, the 49-test `std_socket` target, and **no
  sanitizer claim**; `EVENT-V1-APPLE-RUN` discharged with Apple ASan/UBSan and
  byte comparison and **no LSan claim**; `HTTP-V1-WINDOWS-RUN`,
  `STRING-SLICE-IO-WINDOWS-RUN`, `SOCKET-INET-WINDOWS-RUN`,
  `SOCKET-V1-APPLE-RUN` and `SOCKET-INET-APPLE-RUN` open with their exact
  discharge conditions; `WINDOWS-RUN-TEST-NAME-SANITIZED`'s four tests; the
  `47712702` commit condition; the ARM64 record with both candidate residual
  names; the `/var` → `/private/var` canonicalisation; Linux's scope statement.
- **Working rules deduplicated** against `AGENTS.md`: the fail-closed rule now
  cites invariants 1 and 5 instead of restating them, and the status-text rule
  cites the sync rule instead of quoting it. The operational halves — *update
  the pin deliberately and say so in the commit message and report*, *do not
  re-grow a chronicle here* — are what remain, because they live nowhere else.
- **Header** tightened from an enumeration of `AGENTS.md`'s contents to one
  sentence plus the constraint-section instruction, which is the part that
  tells an agent to *do* something.
- **Cadence rule** updated with this card and the next due date, plus the
  floor expectation (§2).

### 3.6 `README.md`

Refreshed as a human summary, same register, no marketing, honestly per the
ledger vocabulary.

- The `std` module table gained **`std.socket`** (checked TCP/UDP over
  loopback, both ends in one program, including declared IPv4/IPv6 addresses
  with ports resolved at check time against the program's own binds) and
  **`std.http`** (framing the compiler constructs, `Content-Length` it
  decides, written or computed). `std.string`'s row gained the **composition
  band**: `concat(…)` into static storage, `dec`/`hex`, `borrow_utf8`.
- The prose beneath gained the **counted loop** `for i in 0..n` with its
  opt-in, **checked-never-trusted** unroll licence under a three-layer budget,
  and the **reactor composition** (a checked socket registered with the event
  reactor) beside the three composed profiles already listed.
- A new paragraph states what the examples buy and what they do not: every
  published program is **executed and byte-compared**, and the HTTP route is
  **"an executed HTTP/1.1 subset with the boundary named"** — the ledger's own
  words — not HTTP/1.1 conformance, with the socket route's loopback-only,
  no-external-peer boundary beside it. The pre-existing honesty framing
  ("Everything outside an admitted profile is refused, not guessed"; general
  task/mutex composition remains open; **full language conformance is not
  claimed**) is unchanged.
- Status gained the **CT row** in the ledger's vocabulary: 36 of 50 **mapped**,
  14 unmapped, "that denominator grows by **mapping, not conformance**", 5
  sanitized-native and 0 proof of the 36.
- Quick start now shows `cargo test --workspace --no-fail-fast` with its
  one-sentence reason, matching the gate as of the wave-26 close.
- `examples/` gained a paragraph and a documentation-map row; it had been
  reachable only through the threads link.

Every `.md` path and link target in all four edited files resolves to a real
file (checked mechanically; zero unresolved in each).

## 4. Drift corrected (one item)

**The entry points listed the configured-audit command twice, once wrongly.**
The CLI bullet said audits "run as `quirin-audit CONFIG.json` (crate
`qcore-audit`)" and the next bullet said "Configured audits:
`qcore-audit CONFIG.json`". Re-read from source:
`crates/qcore-audit/Cargo.toml` declares

```toml
[[bin]]
name = "quirin-audit"
```

`qcore-audit` is the **package and library** name, unchanged by the Quirin
rename; it is not a command. The two bullets are now one, with the correct
command and the package/library distinction stated. This corrects a
documentation drift, not a rename: nothing in the crate moved.

Every other headline number was re-read from its source and **agrees with what
the wave-26 close set** — the close is not wrong anywhere this card looked:

| Number | Source re-read | Result |
|---|---|---|
| Contract corpus 73 modules | `find contract/qcore -name '*.qcore' \| wc -l` | **73** ✓ |
| Lean audits 110 | `find proof -name 'Audit*.lean' \| wc -l` | **110** ✓ |
| Workspace tests 4,209; provider 122; Python 193 | `docs/reports/wave26-close/gate.json` | ✓ |
| CT 36/50 mapped, 14 unmapped | `docs/reevaluation/ct_manifest.json`, `mapping_status` tally | **36 / 14** ✓ |
| Sanitized-native 5 of 36, proof 0 of 36 | same manifest, `native_sanitized` / `proof` stage status | **5** (CT03, CT17, CT26, CT34, CT49) / **0** ✓ |
| Named follow-ons 35 | `acceptance_matrix.json` `named_followups` | **35** ✓ |
| Ten workspace members, nine crates, six-member providers workspace | `Cargo.toml`, `providers/Cargo.toml`, `ls crates/` | ✓ |
| Cycle records `CYCLE_2` … `CYCLE_20` | `ls docs/cycles/` | ✓ |

## 5. Census (what was checked before moving anything)

Searched every `*.rs`, `*.py`, `*.sh`, `*.yml`, `*.yaml`, `*.toml` and `*.ps1`
outside `target/` for anything that reads or pins these files' content.

| Finding | Disposition |
|---|---|
| **Nothing reads the top-level `README.md`, `AGENTS.md` or `CLAUDE.md`.** Every `include_str!` of a markdown file names an `examples/**/README.md` | Confirms the brief's premise. The top-level files are free to change. |
| `compiler/tests/examples_suite.rs:1245` **does** `include_str!("../../examples/README.md")` | The *examples index*, which this card edits for deliverable 2. Its three assertions — every family directory is linked as `](family/)`, every `/` link is a real directory, every `.qcore` link exists — were re-run after each edit; the added section links only existing directories and files. |
| Six Rust files cite `AGENTS.md` invariants/constraints **by number** (`borrow_liveness.rs`, `address_of_member.rs`, `legacy_enumeration.rs`, `reason.rs`, `qcore_contract_corpus.rs`) | No constraint was renumbered, so every citation still resolves. This again ruled out merging numbered entries. |
| `DEVELOPMENT.md` quotes old `AGENTS.md`/`CLAUDE.md` line ranges throughout its DOCS-COMPACT section | Frozen quotes describing the past. **Not edited.** |
| `docs/reports/remaining-d02/update_acceptance.py:109` names `AGENTS.md` in a string | A historical note in report text, not a read. |

**Nothing the census found blocked a planned compaction.**

---

# Part two — the examples style pass

User-directed, and explicitly **to be reviewed one file at a time**: *"apply
your recommendations and I'll review them one-by-one later on."* Hence one
commit per respelled file, each titled by that file, each carrying its own
byte-identity evidence.

## 6. The rule

Recorded in `examples/README.md` under **"How these programs are spelled: pipe
the flow, parenthesize the assembly"**:

* **Pipe the flow** — a value moving in a straight line from one binding into
  one sink takes `value |> print dec;`.
* **Parenthesize the assembly** — an n-ary construction takes the call:
  `concat("Content-Length: ", dec(length), "\r")`. Pipe insertion is
  first-argument-only **by design**, so an n-ary assembly has no honest
  pipeline spelling.
* **A value that fans out stays a statement** — one binding read by two or more
  later statements is not a linear flow, and piping the first reader would
  suggest the value ends there.
* **Never contort one into the other.** A pipeline is linear and
  first-argument-only; those are design decisions, not gaps, and the statement
  form is a right spelling rather than a fallback.
* A **written literal at a sink** is not a flow.

Two families sit **above** the rule:

* **`examples/events/` keeps its standing pipes-only rule** — the user's
  earlier directive, stated on that page: *every line in these programs is
  spelled as a pipeline*, fan-out included. The index says in so many words
  that where the two rules disagree, **the events page wins**.
* `examples/pipes/` and `examples/stdio/` are the *demonstration* of the two
  spellings. `pipes/rendered_lines.qcore`'s own header comment names
  `stdio/renderings.qcore` and `stdio/severities.qcore` as its **unpiped
  twins** that emit the same C11. Respelling either side would delete the
  contrast, which is the content.

## 7. Method: byte-identity, per file

`|>` is a token permutation performed before any checker runs, so a respelling
*should* emit identical C. That is a prediction, and this card measured it
rather than asserting it — the precedent being the pipe cards' own byte-equality
pins.

For each file, the emitted C11 and the reference-observation projection were
captured **before and after** through `compile_files` with the same declared
contract environment `compiler/tests/examples_suite.rs` uses, by a
worktree-private tool (`measure/emitter/`, gitignored; the CLI cannot emit the
extern-calling examples — `Q023`, the named follow-on `UNSAFE-LINKAGE-CLI`).
A rewrite that moved a byte would have been reverted and listed as skipped.

**All 14 captured programs — the 5 respelled and the 9 examined-and-skipped —
are byte-identical before and after, C and observation alike**
(`diff -r measure/emit/before measure/emit/after`, empty).

## 8. Per-file table

### Respelled (5 files, 5 commits)

| File | What changed | Byte identity | Tests |
|---|---|---|---|
| `examples/strings/computed_length.qcore` | `print status;` / `print kind;` / `print declared;` → piped. `print content;` and `print "\r";` deliberately kept, with a new comment saying why (`content` is read twice, by `len` and by its own line; the `\r` is a literal) | **cksum 2027129972, 6,847 B — identical**; observation identical | `string_concat` 53 ✓, `examples_suite` 38 ✓ |
| `examples/loops/buffer_copy.qcore` | `print dec k;` in the loop and `print dec a\|b\|c\|d;` → piped (five single-reader flows). `log info dec n;` kept with a comment: `n` is read by the loop header too | **cksum 588800544, 3,906 B — identical**; observation identical | `examples_suite` 38 ✓ |
| `examples/http/exchange.qcore` | `print dec status;` / `print dec length;` → piped | **cksum 718661906, 38,691 B — identical**; observation identical | `std_http` 67 ✓, `http_computed_length` 33 ✓, `examples_suite` 38 ✓ |
| `examples/http/raw_request.qcore` | `print dec status;` → piped | **cksum 2152429501, 38,118 B — identical**; observation identical | `std_http` 67 ✓, `examples_suite` 38 ✓ |
| `examples/extern/borrowed_render.qcore` | `log info dec total;` → piped. Both `borrow b` renderings kept, with a new header comment: a live resource four statements read is not a linear flow | **cksum 1729426534, 8,697 B — identical**; observation `["out: 1", "err: [INFO] 56", "err: [WARN] 0x00000001"]` identical | `examples_suite` 38 ✓, `examples_cli` 6 ✓ |

No family page needed an edit: none of the five programs has its text quoted on
its page, and every published transcript is unchanged because every observation
is unchanged. `examples/extern/README.md` quotes `print dec borrow b`, which
this card deliberately did not touch.

### Examined and skipped, with reason (9 files)

| File | Reason |
|---|---|
| `examples/sockets/tcp_echo.qcore` | `first` is read by `print dec first;` **and** `print hex first;` — fan-out. Already on the rule's correct side. |
| `examples/sockets/udp_echo.qcore` | `echoed`, same two-reader shape. |
| `examples/sockets/inet_v4_echo.qcore` | `first`, same. (`port` also fans out, to the bind and the connect — which is the file's stated point.) |
| `examples/sockets/inet_v6_echo.qcore` | `first`, same. |
| `examples/http/computed_length.qcore` | `count` is read by `concat(…, dec(count), …)` and by `print dec count;`; `length` by the `Content-Length` header and by `print dec length;`. Both fan out — and that the header and the transcript read the *same* binding is the card's whole argument. |
| `examples/stdio/renderings.qcore` | The **declared unpiped twin** of `pipes/rendered_lines.qcore`, named as such in that file's own header comment. Respelling deletes the contrast. |
| `examples/stdio/severities.qcore` | The second twin, same reason. |
| `examples/stdio/pipeline.qcore` | `total` is read by `print dec total;` and by `total \|> demo_scale(2) …` — fan-out, and the file already shows both spellings three lines apart. |
| `examples/strings/status_line.qcore` | Already conformant on the literal/flow split, but see §9 — it is the rule's one open case. |

Not examined, and why: `examples/events/*` (four programs) are covered by the
standing pipes-only rule and are already all pipelines; `examples/pipes/*` is
the demonstration family; `examples/threads/*` and
`examples/unsafe-linkage/unchecked_calls.qcore` render only literals, which are
not flows (and the thread programs sit under constraint 15a);
`examples/branching/*` and `examples/cycle12.qcore` have no rendering lines.

## 9. The rule's one open case — `EXAMPLES-STYLE-SLICE-FANOUT`

`strings/status_line.qcore` writes

```qcore
let body = "hello, world";
body |> print;
body[7:12] |> print;
```

so `body` is read by **two** statements and belongs on the fan-out side of the
rule, while being spelled as a pipe.

It was **not** repaired, on two grounds, and the examples index says so rather
than letting the page state a rule a published example contradicts. First, the
repair would mean **un-piping** a line — the opposite direction from the one
this style pass was asked to move lines in, and the brief is explicit that
fan-out pairs stay exactly as they are. Second, the case turns on a real
question about the rule rather than about the program: a **slice of** a value
may be a second reader of it, or it may be the *assembly* half of "pipe the
flow, parenthesize the assembly", and the two readings disagree only here.

Named `EXAMPLES-STYLE-SLICE-FANOUT` and left for the user's decision.

## 10. Commits, for file-by-file review

| Commit | File |
|---|---|
| `515673df` | `examples/README.md` — the rule |
| `7beb77aa` | `examples/strings/computed_length.qcore` |
| `e7e55669` | `examples/loops/buffer_copy.qcore` |
| `689a5144` | `examples/http/exchange.qcore` |
| `4d1848c0` | `examples/http/raw_request.qcore` |
| `3fc7af0c` | `examples/extern/borrowed_render.qcore` |
| `3b506144` | `examples/README.md` — the open case (§9) |
| `d323d33a` | the compaction (part one) |

---

## 11. Gate

Run at the card tip on Linux with `$HOME/.elan/bin` on `PATH`. The gate command
is `cargo test --workspace --no-fail-fast` as of the wave-26 close
(`GATE-NO-FAIL-FAST`), and the Lean step is read from its **text** line rather
than its exit code, per that script's own behaviour.

| Step | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **4,209 passed, 0 failed, 8 ignored**, exit 0 |
| `cargo fmt --all -- --check` | clean (**exit 0**) |
| `cargo clippy --all-targets -- -D warnings` | clean (**exit 0**) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **`Passed 110 Lean audits`**, exit 0 |
| `python3 -m unittest discover -s scripts/tests` | **193 tests, OK** |

No provider rebuild was required: `providers/package.py` inputs are untouched
and `qcore-schemas` stays 14.0.0, so constraint 18's obligation is not engaged.
The provider row carries at 122.

### 11.1 A first run that failed five targets, and why none of it was this card

The first full run ended `error: 5 targets failed`. All five are reported here
rather than re-run quietly, and all five were **environmental**. The finding is
worth recording because it is a trap for any delegated card working in a fresh
worktree:

**Four were the uninitialized submodule.** `git submodule status` showed
`-d82464a2…` — the leading `-` meaning `third_party/libevent` was an empty
directory, because the gate's own first line,
`git submodule update --init --recursive`, had not been run in this worktree.
The failure mode is worse than "files missing", and is the reason this is worth
a paragraph: with the submodule directory empty, `profile_loader`'s pin check
read the **superproject's** HEAD and reported

```
SourceRevisionMismatch { pin: "libevent",
  expected: "d82464a277d0f42703702c4dfd9af6af38595a83",
  actual:   "d323d33add708b5f7d5011808066b2f814e4d6bb" }
```

— `d323d33a` being this card's own documentation commit. A reader who trusted
that line would think a docs-only card had moved a pinned submodule revision.
It had not; invariant 5's pin is intact and the test is doing its job. After
`git submodule update --init --recursive` checked out `d82464a2`
(`release-2.2.2-alpha-11-gd82464a2`), all four pass:

| Target | After initialisation |
|---|---|
| `-p qcore-compiler --test libevent_submodule` | **1 passed** |
| `-p qcore-compiler --test profile_loader` | **18 passed** (was 16 passed / 2 failed) |
| `-p qcore-compiler --test native_library_header` | **17 passed** |
| `-p qcore-compiler --test native_library_header_decoration` | **22 passed** |

The last two read libevent's real core headers for their census, so they failed
for the same one cause.

**The fifth was constraint 16, behaving exactly as that constraint describes.**
`-p qcore-verifier-host --test confined_process_execution` holds the confined
fixture write test, one of the five named load- or parallelism-sensitive rows.
`uptime` during the run reported a load average of **22.97 / 42.73 / 43.67** —
this host was running other delegated worktrees. Verified in isolation as
constraint 16 prescribes: **12 passed, 0 failed.** No assertion was weakened and
nothing was retried until green; the row was run once, alone, per its own rule.

The table above is therefore the **second** full run, taken with the submodule
present: **4,209 passed, 0 failed, 8 ignored, exit 0** — the wave-26 close's
number exactly, as a documentation-and-spelling card's must be.

## 12. Boundaries and follow-ons

- **The constraint sweep is a label check, not a semantic one.** The verifier
  proves 39 labels in and 39 out with 37 bodies byte-identical; that the two
  reworded bodies preserve their content is argued in §3.3 and rests on
  reading, as at both predecessors. The pre-card text remains in git history at
  `f122cc03`.
- **`AGENTS.md` is at its floor for this method.** Three passes in, the file is
  a dense reference. The remaining large entries (constraints 7, 8, 10a, 14,
  15e–15i) are rule text whose only other home is their card reports. A fourth
  pass should expect to find its material in whatever waves 27–29 append, and
  the cadence rule now says so in `CLAUDE.md` so a future agent does not cut to
  hit a number.
- `EXAMPLES-STYLE-SLICE-FANOUT` (§9): the rule's one outstanding case, needing
  a decision rather than an edit.
- `DOCS-COMPACT-2-W4-DESIGN-POINTER` (carried, not addressed here):
  `docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md` still attributes constraint
  4's assume-guarantee rule to `CLAUDE.md`, where it has not lived since
  2026-09-10.
- **`DOCS-COMPACT-3-WORKTREE-SUBMODULE`** (§11.1): in a fresh worktree with
  `third_party/libevent` uninitialised, `profile_loader`'s pin check reports the
  **superproject's** HEAD as the submodule's `actual` revision, which reads as a
  pin violation by whatever commit is at HEAD. The pin is fine and the test is
  correct to fail; only the message misleads. A one-line "is the submodule
  initialised?" precondition in that assertion would turn a scary diagnostic
  into an accurate one. Not fixed here — this card touches no test.
- **`README.md` grew again**, to 180 lines. The `std` table is now eight rows
  and is the candidate a future pass would move behind the implemented-grammars
  link — but it remains the only place a human learns what the language can
  express today, and it is further behind after every wave that lands a route.
