# Wave 48, seventh record

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Counts and observations describe their recorded dates. Current rules and
> headline status live in [AGENTS.md](../../../AGENTS.md) and
> [STATUS.md](../../STATUS.md); each report owns its evidence. The preceding
> [record](wave-48-part6.md) holds the wave-48 paragraphs through
> EXECUTION-QUOTA-LOAD-INDEPENDENCE (it reached 29,667 bytes); it is not extended further.

**`QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN` (2026-10-13, base `e0b92ece6`; prediction `b5da3bb4b`).** The verifier host's per-execution quota join is fail-closed ([report](../../reports/quota-procs-descriptor-dup-fail-open/README.md); constraint [26](../../agents/providers.md#constraint-26) amended in place; Lean `QCore.ProcessQuotaPlan` +6 theorems). `quota_descriptor` mapped a failed `dup` of the cgroup's `cgroup.procs` descriptor to "no join", so the child ran outside its cgroup while the record observed the empty cgroup under limits naming the quota. A failed duplicate now refuses before spawning (`RefusedByProfile`, no enforced limits, no observation, cgroup and errno named). An audit of the path from cgroup creation to the join found two more sites. `finish_quota` observed every created cgroup, including after an identity refusal or a failed spawn; it now observes only a spawned child, which has joined. The join's `write` accepted a short count; it is now a failed join (`EIO`), unreachable on Linux kernfs and pinned by a unit test. `MechanismAvailability::quota_join_descriptor`, a one-directional override, makes the duplicate fail with `EMFILE` at its real call site, because `RLIMIT_NOFILE` cannot isolate it. Three `execution_quota` rows and one unit row; reverting each repair alone fails exactly its rows (3 of 3 mutations caught, harness and logs in the report). Corpus, system and slice, with the null property selected, are byte-identical BEFORE and AFTER. Ledger 763: the card's entry retires and `HOST-PRE-SPAWN-PROTOCOL-RECORD-NAMES-LIMITS` registers (three pre-spawn `Protocol` records that name the planned limits, one reachable; evidence shape, not fail-open).
