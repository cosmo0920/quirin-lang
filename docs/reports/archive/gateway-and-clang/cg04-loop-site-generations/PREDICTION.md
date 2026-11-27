# CG04-LOOP-SITE-GENERATIONS: frozen prediction

Frozen before any build of the repair was run or measured. Base: local `main`
= `origin/main` `ca634dc1` (round 4 and W48-QR-C-STRUCT-IMPORT landed; later
than the brief's `3e69322c`, which it contains).

**Deviation, stated:** the brief asks for the prediction before implementing.
The exposure census below was measured first, with a behaviour-neutral
instrumentation build (`030e90a5`: parity rows only). The repair's code was
then written and compiled (`cargo build -p qcore-c-gateway`; the Lean model
checked with `lean`), but no test of it had run and no analysis of any input
by it had run when this file was committed (`ebe68a53`). CG00 froze no
prediction for this card (it is a repair the queue did not list), so there is
no `predictions/CG04-LOOP-SITE-GENERATIONS.md` to re-freeze; this file is the
first one.

## 1. BEFORE on the base

Binary built at `030e90a5` (the base plus the census rows, which write parity
rows only), `--jobs 8`:

| Route | Figures | Identity |
|---|---|---|
| corpus (`66910c10`) | 586 / 10 / 4,238 | `resource-diagnostics.json` `c012e785…`, byte-identical to CG06's AFTER, as the brief states |
| system (`046accf6`) | 34 / 2 / 198 | as the brief states (`compare.py` totals, de-duplicated by unit and function) |
| tail-pause slice (`90978ac6`) | 26 / 1 / 242 / 1 Pending | as the brief states |

So the instrumentation moved nothing.

## 2. Exposure: re-acquisitions an earlier generation reaches

Census: every acquisition node whose own site is already present in the
settled state it runs in (an earlier generation reaches its re-acquisition,
so the node lies on a CFG cycle), from every flow run including W4
derivation runs, parity `position: "loop-site-generation"`
([scripts/exposure.py](scripts/exposure.py)). A function is **exposed** when
some such earlier generation may be live (`LIVE` bit) and carries no
unresolved effect: that is the only case in which the one-site flow could
drop an obligation it would otherwise report.

| Route | Functions with a re-acquisition | Exposed | Exposed and Safe |
|---|---|---|---|
| corpus | 31 (29 Unknown, 2 Unsafe) | **0** | **0** |
| system | 5 (all Unknown) | **0** | **0** |
| slice | 3 (2 Unknown, 1 Safe: `flb_tail_mult_destroy`) | **0** | **0** |

In every one of the 39, the earlier generation is either already unresolved
(a refused effect reached it, so no leak was claimed for it and the function
is not Safe on that ground) or not live: `mk_server_listen_exit` (corpus),
`sb_destroy_backlogs` and `flb_task_queue_destroy` (system), and
`flb_tail_mult_destroy` (slice) reach their re-acquisition with the earlier
generation released (status 2). No W4 summary was validated on an exposed
body (no row names a function outside the reported rows, and none is
exposed), so there is no summary cascade.

**Reading:** the defect is real (the pinned test reproduces it on the base)
but no currently Safe function of the three populations depends on it. The
corrections this card exists for are therefore predicted to be **none** in
the measured populations; the repair closes the hole for code the flow will
admit later (the placement guard, CG08 and the other widening cards).

## 3. Predicted outcome of the repair

Step 1, the flow's generations (rule 10x's guard kept):

| Claim | Prediction | Confidence |
|---|---|---|
| Corpus status moves | **none**; 586 / 10 / 4,238 | 80% |
| Safe → non-Safe | **none** (0 exposed) | 90% |
| Same-status diagnostic changes | at most the 31 functions with a re-acquisition; bindings to a merged summary become unknown, so a later use may add an `unknown_alias` leaf; the 2 Unsafe (`rd_kafka_snappy_java_uncompress`, `rd_kafka_sasl_safe_string`) stay Unsafe | 70% |
| System and slice | no status move; `flb_tail_mult_destroy` stays Safe (its earlier generation is released) | 85% |
| New provisional Unsafe | none | 85% |

Step 2, lifting rule 10x's `acquisition-on-cycle` guard, measured on its own:

| Claim | Prediction | Confidence |
|---|---|---|
| Consultations the guard refused | 2 (6 rows), `rd_kafka_lz4_decompress` (`!(tmp = rd_realloc(out, …))`) and `rd_gz_decompress` (`!(decompressed = rd_malloc(…))`), both admitted | 80% |
| Their functions | both stay Unknown (other leaves hold them; for the reallocation the loop joins `out` over two identities, which the flow refuses) | 75% |
| Other moves | none in corpus, system or slice | 80% |

If a Safe → non-Safe move does appear, it is named as a correction with its
loop, its acquisition and a reading of whether the leak is real.

## 4. What would falsify the reading

- A Safe function moving at step 1: the exposure census missed a path (it
  reads settled states, the same states the leak report reads).
- A system or slice move: the same.
- A new refusal in a Safe function without a re-acquisition row: the shift
  ran on a node the census does not see (it should run only where a produced
  site is already mentioned).
