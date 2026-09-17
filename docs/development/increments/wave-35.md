# Wave 35 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`VERIFIER-HOST-CONTROL-BUDGET` (2026-09-17)** dispositioned
`VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED` by finding its mechanism, and the
mechanism was a defect rather than the host property two waves had assumed. The
transport loop's backoff charged a **fixed 1 ms quantum for an event taking
~5 µs**; with this host's measured pipe capacity of **8,192 bytes** (two pages,
where `PIPE_DEF_BUFFERS` gives 65,536 — unexplained, `HOST-PIPE-CAPACITY-UNEXPLAINED`)
that is a ceiling of 8 KiB/ms, so the control's 8 MiB request needs ≥1,024
quanta and consumes a 1,000 ms budget whole — **the same arithmetic wave 33
repaired, surviving on the write side**, that card having removed only the
*unconditional* sleep. The inversion `docs-compact-6` recorded is explained:
the quantum is charged only when the host comes back around before its peer is
scheduled, so a **quiet** machine pays it and a contended one does not. CPU
frequency scaling, the brief's leading candidate, is **falsified** — the passing
heavy-load arm ran at 2,954–3,082 MHz against up to 3,654 MHz in the failing
ambient arm. Landed as disposition (c): `backoff_until_ready` waits for pipe
readiness via `poll(2)`, bounded by the same quantum and the same deadline, with
every assertion, deadline, cap and refusal arm untouched and **the test file not
edited at all**. Paired and interleaved under one ambient load, the old backoff
was 4/8 red with phase 4 taking 350–1,000 ms and the new one 8/8 green at
42–112 ms; per-wait cost fell from 1,057 µs to 5.5 µs, a factor of 190. Also
measured, and **not** repaired: the row's *first* arm carries an opposite
sensitivity, its fixed one-second guard over a 40 ms deadline failing 3 of 6
repetitions at load ≈100 (`VERIFIER-HOST-EARLY-ARM-OVERLOAD`, constraint 16's
shape, amendment text proposed to the integrator). The residual's
"deterministic 3/3 in isolation" **does not reproduce**: 22 pre-repair isolated
repetitions gave 7 pass / 15 fail. Invariant 6 checked rather than inherited —
the only two Lean modules naming `ProcessProvider` model the construction
discipline and the declared-limit-kind plan, neither the transport loop. Two
backoff arms are recorded **defensive, not verified**
(`HOST-BACKOFF-FALLBACK-ARMS-UNDRIVEN`); Linux only, no macOS run claimed, and
no Windows sibling exists since the non-Unix arms refuse before any transport.
`docs/reports/verifier-host-control-budget/README.md`.

