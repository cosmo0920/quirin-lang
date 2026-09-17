# Wave 35 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

`C-GATEWAY-CONSTANT-DO-WHILE` (2026-09-17), wave 35's first arbiter, modelling
the constant `do … while (0)` exit and withdrawing the corpus's only
`double_release` row. `do S while (0)` executes `S` exactly once (C11
§6.8.5.2), so the CFG without a back edge is the construct's **exact**
semantics rather than an approximation; the builder had given every `do`-`while`
header both `body_entry` and `continuation` as successors whatever its
condition. The census the card was asked to re-derive is the finding that sizes
the problem: on the **preprocessed** text the 185 units carry **15,053**
`while (0)` occurrences (librdkafka 13,361, cmetrics 841, ctraces 435,
monkey-libevent 286, fluent-bit-chunk 130, chunkio **0**), against the 266 a
source-level grep of two directories sees — a factor of about 57, because the
idiom is a macro body and macro expansion is where it lives; 185 units
measured, 0 missing from the compile database, 0 preprocessor failures, and the
brief's `librdkafka-2.10.0` is **2.15.0** in the pin, its 121 reproducing
exactly. Because removing an edge is the **admitting** direction, the rule is
keyed on the single **literal token** `0` and on nothing else (constraint 5a —
never an evaluated expression): `while ((0))`, `while (1 - 1)`, `while (0 == 0)`
and `while (0 && 0)` each keep the CFG they had, pinned by a test, and the
header node and its **exit** edge are never removed, so no leaf coordinate
moves for an unrelated reason. `QCore.ConstantDoWhile` (Lean audits **129 →
130**) states both halves: `constant_do_while_is_its_body` is the `≡ S`
equality, `removing_the_back_edge_only_removes_paths` and
`the_successor_list_never_grows` are the conservativity — the new CFG's paths
are contained in the old one's **for every condition**, so this change can
withdraw a refusal and can never manufacture one — and
`the_repeated_release_is_a_path_the_c_does_not_have` names the parting witness
as the corpus's own row, one release under the C against two under the back
edge. The re-pin is surgical and the BEFORE run reproduces
`c-gateway-return-call-w4`'s archives byte for byte: **four of the five
artifacts are byte-identical between BEFORE and AFTER**, only `diagnostics.json`
moves, and within it exactly **one leaf removed and zero added** —
`double_release` **1 → 0**, the `rd_kafka_jwt_b64_decode_payload` row at
internal L21340 (origin `rdkafka_sasl_oauthbearer_oidc.c:192`, acquisition
`:136`, reached through `RD_IF_FREE`'s own `do { … } while (0)`). It is
**withdrawn, not discharged**: the function keeps its other 18 leaves and goes
provisional `Unsafe` → **`Unknown`**, so functions read **186 Safe / 84
provisional Unsafe / 4,564 Unknown** with **Safe unmoved and nothing promoted**,
the unit stays `Unsafe` on two sibling leak-carrying functions, and there are
**zero unit status moves** — the wave-31-through-35 record holds. Leak **272**,
`unknown_alias` **4,214** and `unsupported` **50,106** are all **unmoved**
(constraint 7 has nothing to attribute), and the 32 `unmodeled … do-while
condition` leaves are a different question and measured **unmoved** at 32. The
gap between 15,053 expansions and one leaf is published rather than glossed: a
spurious back edge is only observable where the body releases or acquires a
tracked identity that survives to the header, which in a corpus with 4,564
`Unknown` functions is rare — so the rule's measured reach today is one leaf and
its forward value, foreclosing the same false accusation in every expansion a
later card promotes, is left explicitly unquantified
(`CONSTANT-DO-WHILE-LATENT-REACH`). Two findings came out of the campaign rather
than the plan. All six mutations were caught, **including the control** that
swapped successor *order* on the **unchanged** arm — so successor order is
observable in the worklist schedule, recorded not repaired, the same
sensitivity constraint 6 carries for `report_leaks`
(`CONSTANT-DO-WHILE-SUCCESSOR-ORDER`). And a neighbouring route was found and
**measured pre-existing on the unmodified base**: where a scalar binding's value
is modelled precisely, a `do`-`while` on it was already decided from the value,
answering conservatively on all four probed shapes (`flag = 0` loses the edge;
`= 1`, a tracked reassignment to 1, and an unmodelled initializer each keep it),
so it is not a false-`Safe` source, it is pinned by a test so a refactor cannot
move it silently, and it is why the card still had work — a macro's `while (0)`
is a token and no binding (`CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED`).
Wave 35's **second arbiter gets a measured hand-off rather than an assurance**, and it is the largest thing this card does: `legacy_joined_provenance::derive` returns **no** conditional-provenance facts at all for a cyclic CFG (constraint 10b), so removing a back edge can flip a whole function into deriving them. An instrumented pair of corpus runs — the probe placed in `legacy_c.rs` and never in the sibling's file, reverted on every exit path — measures **654** of the 4,771 analysed functions moving **cyclic → acyclic**, 34% of the 1,926 that were cyclic, with **0** moving the other way; and the measured diagnostic effect of all 654 is **zero** — `unknown_alias` unmoved, no leaf added anywhere. So that pre-pass's reach grew by 654 functions before the second arbiter changed anything, its coverage figures must be quoted against this tree rather than `a300a39d`, and the 1,272 that stay cyclic are closer to the genuine loops now that the macro idiom's spurious cycles are gone (`CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH`). Follow-ons also name the unclaimed literal-zero spellings
(`0u`, `0L`, `0x0`, `'\0'` — `CONSTANT-DO-WHILE-ZERO-SPELLINGS`) and the
deliberately untaken `while (1)` / `for (;;)` generalisation, whose direction is
the opposite one: modelling *those* exactly would remove an **exit** edge, which
is accusing and needs its own Lean statement and re-pin
(`CONSTANT-DO-WHILE-OTHER-CONSTANTS`).

