# Validation of the QCore-C r0.2 design experiment

**Executed:** 2026-09-05.  
**Status:** Bounded executable evidence for a proposed design, not full language conformance or a formal proof.

## Executed checks

| Check | Observed result |
|---|---|
| Python test suite | **48 passed; 0 failures; 0 errors; 0 skipped** |
| Python syntax compilation | All three prototype Python files compiled successfully |
| Four-holder share assignments with model FULL = 8 | **6,561 assignments examined** |
| Assignments satisfying total share <= FULL | **495 valid assignments**; a full holder leaves every other holder at zero |
| Balanced nonnegative allocation traces | **13 traces**, of length 2 or 4, with increments in {-2, -1, 1, 2} |
| Ordered pairs of these allocation traces | **169 pairs** |
| Order-preserving two-trace interleavings | **9,154 enumerated interleavings**; each observed peak was bounded by the sum of local peaks |
| Markdown structure | Balanced fenced blocks; consolidated chapters 0–59 present in order |
| BNF reference coverage | Every referenced nonterminal has a definition, excluding declared scanner tokens |
| Unified patch reconstruction | Applying the patch to the exact baseline reproduced the consolidated bytes |

The interpreter used for this execution was Python **3.13.5**. The prototype is dependency-free and uses Python 3.10-or-later syntax; only the stated interpreter was executed here. No execution-time benchmark claim is made.

The specification's default FULL is **1024**. The finite model uses **8** to make exhaustive small-domain enumeration straightforward. A share is authority, not a number of allowed reads. For the assignment check, any unassigned complement can be held by an explicit reserve custodian; the check establishes exclusivity, not a complete well-typed program state.

## What the executable model covers

`prototype/permission_model.py` models concrete finite footprints with fresh generations, integer shares, unique release custody, selected structured task transitions, completed-but-unjoined capsules, fixed-resource mutex handoffs, vector-clock happens-before, and a separate credit ledger. Its methods independently check pointwise permission conservation and unique live-resource accounting.

The tests include complementary concurrent readers, rejected competing writers, disjoint and overlapping ranges, creation failure, moved resources, returned loan authority, child normal/error outcome tags, explicit join, blocked or failed join behavior, destruction before and after authority return, mutex handoff and lifetime restrictions, credit partition/abandonment, reused identity generations, and resource-bound accounting. Some negative tests deliberately bypass authorization or corrupt a fragment to check witness detection or the independent invariant validator; those operations are explicitly not safe source constructs.

The **9,154 interleavings are interleavings of numeric allocation/release traces**, not an exhaustive exploration of arbitrary threaded QCore programs, C source, or weak-memory executions. The 48 unit tests include bounded-property checks; the above counts are not additional independent full-language test cases. Child errors in this reduced model are outcome tags and custody transitions, not a complete implementation of typed error payloads or cleanup frames.

## What was not executed or proved

No QCore parser, Rust compiler, legacy C front end, C emitter, real OS-thread implementation, sanitizer, C11 weak-memory checker, or Lean development was executed. No `lake build` was run, and no machine-checked theorem is claimed. The prototype does not implement source-language borrow/view typing, Send/Sync derivation, symbolic alias analysis, target C layouts, arbitrary cleanup, dynamic protected ownership, or atomic publication/reclamation.

The BNF check is only **definition/reference coverage**, not a proof of unambiguity, parser correctness, successful parsing of examples, or type checking. The source examples remain explicitly illustrative.

The 50 CT01–CT50 cases in specification §55 are the required **future compiler/analyzer conformance corpus**. The 48 Python tests are not represented as completion of that corpus. Full revision acceptance, backend validation, and the inherited continuous Lean/no-`sorry` gates remain outstanding.

## Reproduction

From the extracted bundle directory:

```sh
python prototype/run_validation.py
```

The command exits unsuccessfully on a failing test or bounded assertion. It writes `test_output.txt` and `validation.json` and prints the structured result. For individual test output:

```sh
cd prototype
python -m unittest -v
```

The model uses only the Python standard library. `validation.json` and `test_output.txt` record the execution accompanying this revision.
