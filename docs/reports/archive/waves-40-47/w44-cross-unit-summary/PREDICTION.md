# W44-CALLEE-CROSS-UNIT-SUMMARY prediction

Base `fa04aab0fd43b242d2a996da2c6c507f1eb2a113`. This prediction is
committed before implementation. It is calibrated against the W48
BOTTOM-UP-CALLEE-BODIES AFTER checkpoint (`bcc98d2d…`, 648 Safe / 9
provisional Unsafe / 4,177 Unknown corpus functions), not a fresh base run.
The fresh base baseline will be recorded before the first implementation run.

The old checkpoint's call-only model gives six potential cross-unit promotions
at its ceiling, or ten when calls held in position leaves are counted. This is
an upper bound: it does not prove external symbol resolution, and position
leaves can retain other refusals. Later declared contracts have already
removed some calls. Therefore the predicted **movement on this base is
bounded by 0–10 corpus Unknown → Safe, 0 system, 0 slice**, with no Safe →
non-Safe or new provisional Unsafe move. Exact leaf changes and current-base
held-call counts are to be measured, not inferred from the old checkpoint.

An inferred home summary alone may not be transported. The intended route
requires one explicitly selected external definition per symbol in a pinned
linked program, a unique home unit, matching source and environment digests,
and replay that independently re-derives the home summary and the caller
result. Missing or ambiguous resolution keeps the caller Unknown. A C99
external inline body is never taken as the linked external definition merely
because its text appears in a caller unit. The linked-program assertion is a
named premise of any conditional Safe result; no source-only measurement can
prove it. The arbiter will compare corpus, system and slice against the fresh
base and attribute every verdict and diagnostic leaf move.
