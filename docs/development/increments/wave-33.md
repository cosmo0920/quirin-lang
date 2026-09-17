# Wave 33 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

Wave 33 close (2026-09-15): **two cards merged and one abandoned**, and the
abandoned one is recorded rather than omitted. **One arbiter ran.**
`C-GATEWAY-RETURN-POSITION` widened the C gateway's `return` position to admit a
pointer-valued bounded typed access chain — the shape card REACH-GATE admitted
only where the position *discards* the value — supplying that refusal's missing
answer rather than overriding it. Its BEFORE run **reproduced the wave-31
baseline byte for byte**, asserted inside `evidence.py` rather than merely
logged, which is what makes every delta attributable: `unsupported`
50,150 → **50,117**, all 33 **withdrawn** return-position leaves and **zero
added**; leak **272**, `unknown_alias` **4,208** and `double_release` **1** all
**unmoved**, so constraint 7 gains no link; **zero unit status moves** and 33
function promotions `Unknown` → `Safe`. The baseline moves to that card's
`artifacts/`, every predecessor frozen (constraint 21). **A grown leaf-free
count is not conformance and raw units stay unpromoted**: the card's own census
says in as many words that leaf-free is *not* `Safe`, `Safe` additionally
requiring every obligation discharged and full analysed coverage at the
certificate gate, and the unit histogram did not move. The honesty requirement
was met over the real **511** return-only-blocked functions rather than a
sample — 33 (6.5%) end leaf-free, **0 exchange** the leaf for another refusal,
478 (93.5%) unmoved — because clearing a refusal that merely becomes a different
refusal decides nothing. **Sub-shape B was withdrawn with its measurement
published** (only 12 of 335 `return CALL(args)` callees carry a declared
contract, and applying a callee's `Consumed`/`Released` mode at a return
position would *discharge* a caller obligation — invariant 1's forbidden
direction). **Two vacuity residuals were found by mutation and reported as
findings rather than smoothed over**: the `returned_place: None` arm is
**vacuous** — no test distinguishes it, so it stands on the theorem and never on
the green suite (`RETURN-POSITION-ESCAPE-UNREACHABLE`) — and the **accusing** arm
is proved and unit-tested but **reaches no function of the pinned corpus**, so
the unchanged leak pin records that absence rather than that arm's verification
(`RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS`). Standing constraint **5b**
records the rule in the card's own proposed words. Lean audits 127 → **128**;
zero reserved words, zero `Q` codes, no §L21 change, corpus **76**.
`VERIFIER-HOST-CLOSED-INPUT-DEADLINE` was **promoted to a card by the integrator
after arriving out of scope**, repairing the one failure the arbiter recorded as
a finding and deliberately did not absorb — and correcting the arbiter's own
attribution from the 100 ms arm to the third, 1,000 ms arm. **The third card is
abandoned, not omitted**: `C-GATEWAY-INDIRECT-CENSUS` stalled, produced **no
report** and no committed deliverable, and its one salvaged partial — of 18,555
indirect-access leaves, the 284 functions blocked by that reason alone hold only
**833** — is carried as an **unaudited** observation whose WIP classifier is
known wrong for macro-expanded accesses and which nothing on `main` can
reproduce. It decides nothing. The close's reconciliation record gave all **21**
integrator-addressed items a verdict **measured against the tree** — 6
`applied`, 14 `noted`, 1 `superseded`, none `declined` — and found one live
drift —
`dimensions.named_followups` stuck at 45 against a `named_followups` of 47, the
registered `D02-DIMENSIONS-FOLLOWUP-COUNT` residual firing a second time on a
hand edit — **repaired by regeneration through the generator, never by hand**
(constraint 25). Four decisions are **deferred** and named rather than taken.
Merged gate at the close tip, after `git submodule update --init --recursive`
first (`WORKTREE-SUBMODULE-UNGATED`, which had already cost the wave two runs):
**4,544 passed / 0 failed / 8 ignored across 227 binaries** under
`--no-fail-fast`, 122 provider **carried**, **227** Python, **128** Lean audits,
both probes exit 0, fmt/clippy clean
(`docs/reports/wave33-close/gate.json`) — **no constraint-16 load row tripped**,
so no isolation re-verification was needed. CT coverage unchanged at 45/50
mapped, sanitized-native 9 of 45, proof 3 of 45; the wave mapped nothing.
Nothing is promoted; full language conformance stays **false**.

`VERIFIER-HOST-CLOSED-INPUT-DEADLINE` promoted and corrected by the integrator
(2026-09-15): the repair below was produced **out of scope** by a stalled
measurement card and had already reached `main` as a bare fast-forward. It was
re-landed as a merge after a complete gate on its own tree — 4,544 passed /
0 failed / 8 ignored over 227 binaries, the repaired test passing, both probes
exit 0, 128 Lean audits, 227 Python, fmt and clippy clean. Its report's
validation section was **rewritten**: the six logs it cited do not exist, its
"97 passed / 15 failed" did not reconcile with the six failures it explained,
and its two claims of a user confirmation are **struck as unfounded**. The code
was kept on its merits, the process breach recorded as `CARD-SCOPE-SELF-MERGE`,
and a first gate run's ten failures attributed to an uninitialised submodule
(`WORKTREE-SUBMODULE-UNGATED`, the wave's second loss to that trap).

`VERIFIER-HOST-CLOSED-INPUT-DEADLINE` repair (2026-09-14): isolated reproduction
localized the failure to the third, 1,000 ms large-response arm, correcting the
ledger's earlier 100 ms attribution. Unconditional 1 ms sleeps after bounded
8 KiB reads throttle the 8 MiB response past that budget. The host now backs
off only when no pipe makes byte progress, retaining deadline checks, bounded
reads, incomplete-request rejection and process-group cleanup. Original
assertions and deadlines remain; an added control must receive the exact same
response after transmitting the complete request. Evidence and gate results:
[repair report](../../reports/verifier-host-closed-input-deadline/README.md).
No language-semantic or Lean definition changes; native timing is observation,
not a scheduling theorem, and no macOS run is claimed.

`C-GATEWAY-RETURN-POSITION` (2026-09-14), wave 33's **arbiter**: the C gateway's `return` position now admits a **pointer-valued
bounded typed access chain**, which card REACH-GATE admitted only where the
position discards the value. That refusal's stated ground — `EscapeIfOwned`
models only a canonical place — is **kept and its missing answer supplied**:
`admitted_chain` refuses a bound base, every `->` place the projection tracks is
rooted at a body-acquired name which `may_bind` reports bound by construction, so
an admitted chain names **no tracked place** and the exit has nothing to escape.
The two expression roles now admit the same set; what the distinction still
decides is the parser's answer, and an expression only the `Decides` rule admits
is answered `returned_place: None` — never `canonical_place`, which spells
`view->head->next` perfectly well and would escape a place the admission never
established. The BEFORE run **reproduced the wave-31 baseline byte for byte** in
both verdict-bearing artifacts (asserted inside `evidence.py`, not merely
logged), so every delta is attributable: `unsupported` **50,150 → 50,117**, all
33 withdrawn return-position leaves and **zero leaves added**; leak **272**,
`unknown_alias` **4,208** and `double_release` **1** all **unmoved**;
`chunk-contracts.json` byte-identical from the baseline through to AFTER; **zero
unit status moves** and **33 function promotions `Unknown` → `Safe`** with no
withdrawal and no new provisional `Unsafe`. Raw units stay **unpromoted**. The
card's honesty requirement was met over the real 511 rather than a sample: of the
functions the baseline blocks on the return position alone, **33 (6.5%) end
leaf-free, 0 exchange the leaf for another refusal, 478 (93.5%) are unmoved** —
bucket (2) empty by construction, since this card admits only what the neutral
walk already fully accounts for. **Sub-shape B (`return CALL(args)`) was
withdrawn with its measurement published**: only **12 of 335** such callees carry
a declared contract, so 323 would merely exchange one refusal for another, and
applying a callee's `Consumed`/`Released` parameter mode at a return position
would *discharge* a caller obligation — invariant 1's forbidden direction — on a
route whose `observe_exit` has no arm for a call result. Two honest boundaries
were **found by mutation and reported as findings rather than smoothed over**:
the arm forcing `returned_place: None` is **vacuous** (replacing it with the
canonical place leaves the suite green, because the base premise makes the escape
a no-op — proved as `the_two_answers_agree_on_every_admitted_chain`), so it is a
backstop justified by the theorem and **never by the green suite**
(`RETURN-POSITION-ESCAPE-UNREACHABLE`); and the **accusing** arm is proved and
unit-tested but **reaches no function of the pinned corpus** — zero leaves added
— so the unchanged leak pin records that absence rather than that arm's
verification (`RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS`). Twelve
axiom-clean theorems in `QCore.ReturnPositionEscape`; Lean audits 127 → **128**.
One pinned expectation was **re-pinned deliberately** and stated as such in the
commit message, replaced by the stronger pair rather than deleted. Zero reserved
words, zero `Q` codes, no `§L21` change, contract corpus unchanged at **76**.
Nothing is promoted at unit level; full language conformance stays **false**.
