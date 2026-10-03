# CG10-INDIRECT-ACCESS-SOLE-BLOCKERS: prediction (frozen before implementation)

Frozen on base `cdc2fa878` before any implementation code. Every figure below is
from the BEFORE runs in `private/cg10sole/` (measured binary
`36e60dd9…`, release build of the base):

| route | Safe / Unsafe / Unknown (Pending) | `resource-diagnostics.json` sha256 |
| --- | --- | --- |
| corpus arbiter (`66910c10`) | 654 / 9 / 4,171 | `d13ed4c1ab2dcf3c…` |
| system (`system-selection.json`) | 83 / 4 / 361 | `68cca54065a5919c…` |
| tail-pause slice | 45 / 1 / 354 (4 Pending) | `2222e03d5dd3bfec…` |

The corpus totals and digest equal the brief's (654 / 9 / 4,171, `d13ed4c1…`).
The census run (`scripts/census-run.sh`, the measurement-only `indirect-census`
feature on) reproduced the corpus report byte for byte (`d13ed4c1…`).

## Task 1: the 513 functions, classified by the readers' decisions

Reproduced: **513** Unknown functions whose only refusal family is
`indirect memory access is outside the resource projection` (977 leaves); the
family touches **2,971** functions and **13,283** leaves; the next families are
uncontracted calls 479, return expressions 58, branch conditions 49.

`scripts/classify.py` classifies each of the 977 leaves from the census rows
(the gate's own record, rule W44's per-actual record, the return census) and
the typed readers' parity rows. Functions freed alone, leaves, functions
touched:

| sub-class (reader's decision) | freed alone | leaves | touched |
| --- | ---: | ---: | ---: |
| `uncontracted-callee`: a call statement whose callee has no contract or summary in force; the gate refuses its actual before the contract lookup | 218 | 578 | 285 |
| `return-uncontracted-callee`: the same in a whole-call `return` | 103 | 108 | 105 |
| `return-contracted-call`: a whole-call `return` of a **contracted** callee whose actual reads memory; rule W44 is never consulted at the return position | 15 | 19 | 19 |
| `elaboration: no value form` (read by hand below) | 15 | 63 | 39 |
| `record-object-place`: every access the gate read is a `.`-only selection from a record object the function declares (no pointer read) | 13 | 41 | 25 |
| `pointer-store:other` (rule 10v's value class: a cast local, arithmetic, an interior address, …) | 12 | 16 | 16 |
| `custody-position:released` (an actual at a `Released` position) | 12 | 43 | 26 |
| `pointer-store:formal-pointer` (rule 10v: a formal's pointer stored into a member) | 8 | 23 | 20 |
| `probe-declined` (owned call results stored into members, …) | 6 | 13 | 12 |
| 15 smaller classes | 26 | 73 | — |

Fixed point in rank order (functions freed when the classes up to the row are
lifted together): 218, 323, 338, 361, 382, 397, 421, 436, 446, …, 495.

**Correction to the brief's ranking.** 321 of the 513 functions (218 + 103) are
blocked by an uncontracted callee hidden behind the access refusal: the
lexical gate refuses the statement before its call's contract is looked up, so
lifting the access frees none of them (each would carry
`call to uncontracted function`). They belong to the P4 callee closure, not to
this card. The largest liftable sub-classes are `return-contracted-call` and
`record-object-place`; 6 of the 15 `elaboration: no value form` functions are
also record-object places the gate's token view does not parse (a
parenthesised root, `(max_rtt).int_value = …`, read by hand), so the
record-object sub-class frees **19** functions alone.

## What this card attempts (in rank order of the liftable sub-classes)

1. **Rule 12i, whole-call return actuals** (`return-contracted-call`). A
   `return` whose expression is one contracted call (or B01's one call inside a
   scalar expression) and whose call the gate refused is read with rule W44's
   call-argument access exactly as the same call as a statement: the typed
   reads, their second obligation, then the call, then the return position's
   unchanged acceptance (`escapes_only_fresh_custody`; a derived result only in
   the derived-summary CFG). A custody-taking position, an unrecognised actual,
   a declined probe or a reallocated result keeps the refusal.
2. **Rule 12j, record-object places with no pointer read**
   (`record-object-place`). A place whose every step is a `.` member selection
   (rule 10w's union and bit-field steps included) from a record object that is
   an automatic local or a by-value parameter of the function is read with no
   root premise: no pointer is read, so no binding of the root or of a place
   rooted at it is read or changed. A scalar read or store only; a static,
   file-scope or `extern` root, a hop (`->`, a pointer subscript or `*`), a
   pointer-valued, address or aggregate access keep the existing refusal (rule
   10t's never-bound premise).

Not attempted (each keeps its refusal by name): the uncontracted callees (P4),
custody positions (`W44-CALL-ARGUMENT-CUSTODY-POSITION`, by design), the
pointer-store values rule 10v refuses (Q36's missing premise), owned results
stored into members (the field-link model), and the small classes.

## Expected movement

* **Corpus:** Unknown → Safe **+23 to +38** (rule 12i: 9–15 of its 15; rule
  12j: 14–19 of its 19; callers whose callee's validated summary now applies:
  0–6). Indirect-access leaves −40 to −90. Unsafe +0 to +2, each read and
  classified (a path the refusal hid). **No Safe → non-Safe.**
* **System route and tail-pause slice:** 0 to +3 Unknown → Safe each (the
  selections contain few telemetry, comparator or wrapper functions).
* **mk_vhost_set / mk_vhost_set_property** (the shared tip probe): no change
  (their refusals are a loop-condition `va_arg`, a `va_arg` value and an
  uncontracted callee, none of these sub-classes).
