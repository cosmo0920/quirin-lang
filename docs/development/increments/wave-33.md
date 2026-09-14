# Wave 33 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

`C-GATEWAY-RETURN-POSITION` (2026-09-14), wave 33's **only card and therefore its
arbiter**: the C gateway's `return` position now admits a **pointer-valued
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
