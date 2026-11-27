# MEASURE-LOOKUP-RECORD-REUSE prediction

Against base `74105dd0ebe4ea25befe9fafe17eb9508224d56c`. Committed before the
pinned-route, gateway and measure code changes; the only edit preceding it on
the branch is the evidence-only lookup recorder in `qcore-schemas`
(invisible to serialization, `Debug` and equality, and read by nothing).

The card adds a per-unit lookup record (names found and missed, relied
callback candidates, whole-map reads) recorded beside the analysis, a
restricted stage-environment digest in the pinned group checkpoint identity,
and a contract-only analyzer acceptance record. None of it is read by the
analysis: a run without `--checkpoint-dir` executes the code it executed
before, and a checkpointed run reuses a group only where its published rows
are the recomputed ones. So BASE -> TIP is an identity on all three live
routes: zero verdict movement and byte-identical reports (up to the checked
substitutions the measure tool already accepts: revision-derived digests and
the Monkey clock key).

```measure-prediction
route corpus
none
route system
none
route slice
none
```
