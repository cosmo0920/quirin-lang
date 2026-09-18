# Wave 37 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.


**Wave 37 · `JOINED-PROVENANCE-ROUND-BOUND` (2026-09-18).** The
conditional-provenance fixed point's `FIXED_POINT_ROUNDS = 64` is replaced by
this graph's own **proved** bound, `nodes * (1 + sites + sites * names)`
(`QCore.JoinedProvenanceRoundBound`, 14 theorems; `a_cap_at_the_measure_settles`
is the operative half — a cap at or above the measure never declines). The card
started as a measurement: a decline and a convergence had returned the *same*
all-empty vector, so the decline rate was unobservable, and the smallest fix —
`iterate` returning a `Settled` outcome carrying its settling round, with
`Declined` mapped to precisely the vector `fixed_point` always returned — was
landed first and shown to reproduce the baseline archives byte for byte. Under
it, of **2,704** cyclic analysis visits over the pinned 185-unit corpus
(**1,273** distinct functions, correcting a 1,272 mis-transcription and not a
count), **2,545** never sweep at all for want of an off-cycle site; of the
**159** that do, 137 converged by round 59 and **22 declined**, every one of
them settling by round **148**. Nine of those 22, across four functions
including `flb_input_chunk_get_direct_routes`, publish facts on settling — so
by the card's own rule the bound had to move and the corpus had to be re-pinned.
It was, and **zero leaves moved**: leak **272**, `unknown_alias` **4,308**,
`unsupported` **49,922**, 54,503 leaves unmoved, zero unit and zero function
status moves, both verdict-bearing artifacts byte-identical to
`docs/reports/joined-provenance-call-frames/artifacts/`. The reason is the
finding: the relation has exactly one consumer, `release_site`, whose entire
reachable surface in this corpus is 14 visits over 6 sites — and none of the
four newly deriving functions is among them, so the new facts are published
where nothing asks for them. The two functions the wave-36 residual named need
**148** and **101** rounds and derive **nothing** on convergence, which
discharges that residual in both halves. The Lean measure had to be corrected
mid-card: counting only the names literally stored makes a site's *arrival*
increase the count, so absence must read as the top of that site's own lattice.
Six of seven mutations are caught; the survivor is recorded, the declining arm
being unreachable from `derive` unless the theorem is false. Retires
`JOINED-PROVENANCE-ROUND-BOUND`, `JOINED-PROVENANCE-DECLINE-INDISTINGUISHABLE`
and `JOINED-PROVENANCE-FIXED-POINT-BOUND-UNMEASURED`; registers
`JOINED-PROVENANCE-BOUND-DESCENT-PREMISE`,
`JOINED-PROVENANCE-DECLINE-ARM-UNREACHABLE`,
`JOINED-PROVENANCE-BOUND-COST-UNMEASURED` and
`JOINED-PROVENANCE-BOUND-NOT-TIGHT` (named followups 171 → 172; Lean audits
135 → 136). `JOINED-PROVENANCE-RELEASE-NULLABLE` is **not** retired — it is an
untaken decision, re-measured unchanged on the final tree, exactly one gate
visit (`rd_kafka_jwt_b64_decode_payload`, site 1, `payload`) blocked by the
nullable premise alone. Evidence:
`docs/reports/joined-provenance-round-bound/README.md`.
