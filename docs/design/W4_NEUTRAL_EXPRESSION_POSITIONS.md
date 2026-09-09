# W4 next admission: neutral expression positions with pointer-field reads

**Status: design only.** No production file implements anything specified here,
and nothing in this note changes a verdict, a diagnostic code or an admission
rule. It is the disposition of card **C03-REACH** step (3): the measured
identification of what the next W4 admission must cover, and why the obvious
smaller options do not.

Measurement of record: [`docs/reports/remaining-c03-reach/README.md`](../reports/remaining-c03-reach/README.md),
whose census artifact carries every number quoted below.

## 1. The problem this note is scoped by

Neutral-frame recursion (card C03-NFR, `crates/qcore-c-gateway/src/legacy_neutral.rs`)
is refusal-complete and reaches **zero** components of the pinned Cycle 19
corpus. C03-NFR predicted its payoff would arrive with C02-P. C02-P, O04-ARMS,
O04-B08 and O04-READ-BOUND have since landed, and the re-run census measures
the reach as still zero: **46 recursive components / 96 members, 0 admitted**.

The refusals are not close. Ninety-four of ninety-six members refuse at the
same place — the **V5 containment gate**, which refuses a body whose resource
CFG retains any `Unknown` instruction — and they refuse with a *conjunction* of
blocker families, median 3 distinct families per member and up to 9.

## 2. Why a single-family admission is measured not to work

The census computes, for every blocker family, how many components would be
admitted if that family were admitted **completely and soundly** — an upper
bound no real admission reaches, since every real admission is a conjunctive
whitelist over a fragment of its family.

| family admitted completely | components admitted | members |
|---|---|---|
| indirect memory access (642 occurrences, 77/96 members) | **0** | 0 |
| unmodeled access in a branch condition (450, 73/96) | **0** | 0 |
| call to an uncontracted function (239, 52/96) | **0** | 0 |
| unmodeled access in a return expression (113, 28/96) | **1** | 1 |

The largest family in the corpus, by a factor of 1.4 over the next and present
in 80% of all refused members, admits **nothing at all** on its own. This is the
central finding: NFR's reach is gated on a *joint* threshold, not on any one
blocker. A card that picks the biggest number off the histogram and admits a
fragment of it will move the reach by zero and will have been correct about the
histogram the whole time.

## 3. The measured joint threshold

Grouping the families by the gate that produces them:

* **POSITION** — the six `unmodeled call, aggregate or indirect access in
  {branch, loop, for-loop, do-while, switch, return}` families. All six come
  from one shared token predicate, `supported_neutral_expression` in
  `crates/qcore-c-gateway/src/legacy_c.rs`, evaluated at six syntactic
  positions. Together: 618 occurrences.
* **INDIRECT** — `indirect memory access is outside the resource projection`,
  642 occurrences: the reads C02 admitted for direct *scalar* fields in
  statement position and C02-P/O04-ARMS/O04-READ-BOUND widened for specific
  *borrowed-actual* shapes.

| admitted | components | members |
|---|---|---|
| POSITION alone | 3 | 3 |
| INDIRECT alone | **0** | 0 |
| **POSITION + INDIRECT** | **11** | **17** |
| + braced initializers | 12 | 19 |
| + assignment-destination families | 15 | 30 |

`POSITION + INDIRECT` is the step that matters: 11 components and 17 members,
against 3 and 0 for either half alone. **The next admission must cover both, or
it is not worth building.** The named beneficiaries at that threshold are the
four-member `otlp_*_destroy` families in `cmt_encode_opentelemetry.c` and
`ctr_encode_opentelemetry.c`, five `rdavl.c` components, `cJSON_Delete`,
`cJSON_Compare`, `pb_check_proto3_default_value`, `rd_kafka_anyconf_get` and
`digits10`.

## 4. What the INDIRECT half actually contains

Classified from the 642 recorded blocking source lines (not from the published
diagnostics, which are computed under a different environment — see the report):

| shape | occurrences |
|---|---|
| chain of two or more member selections | 291 |
| single member selection | 238 |
| explicit dereference (`*p`) | 57 |
| subscript (`a[i]`) | 36 |
| other | 20 |

The single-selection majority is **not** the shape C02 already admits. C02
admits a *scalar* field read in statement position; what these bodies do is read
a **pointer-typed** field and immediately hand it somewhere:

```c
otlp_array_destroy(value->array_value);     /* pointer field as a call actual */
array = value->data.as_array;               /* pointer field bound to a local */
free(kvlist->values);                       /* pointer field at a Released position */
```

Two of those three are already inside an existing admission's stated scope and
one is deliberately outside it:

* `otlp_array_destroy(value->array_value)` is exactly C02-P's *borrowed actual*
  shape at a `Borrowed` position of a contract in force — under `N(m)` every
  member's parameters are `Borrowed`. Whether the existing admission's premises
  (1)–(5) actually hold for these protobuf-c generated records is **not
  established here**; the census records the refusal, not its cause. Deciding
  that is the first task of any implementing card, and it may turn out that part
  of this half needs no new rule at all.
* `array = value->data.as_array` is the *opaque field read* shape, refused by
  the chain rule: a `.` after a `->` crosses a nested aggregate sub-object.
* `free(kvlist->values)` is a **release of an identity the projection cannot
  name**, and must stay refused. Admitting it would let a body discharge an
  obligation over an unnamed place, which is the C02 §2 hazard in its purest
  form. Any implementing card that finds itself admitting this shape has made a
  mistake.

## 5. What the POSITION half would have to say

`supported_neutral_expression` refuses any expression containing a call, braces
or an unmodelled access, at all six positions, with no access to the contract
environment. The minimal sound widening is:

> A call inside an expression position is resource-neutral when the contract in
> force for the callee declares **every** parameter `Borrowed` and result
> `None`, and every argument is a token sequence this same predicate already
> accepts.

Under those conditions the call has no custody effect the projection could
observe: no argument can carry a tracked identity, and the result is untracked.
The premise is already computed — it is the contract environment
`build_function_cfg` is handed — but it is not currently reachable from the
predicate, which is a token-level function called during parsing. Threading it
there is the bulk of the work, and it is why this is **not** the "trivial
compositional gap in an existing admission" that card C03-REACH was permitted to
implement inline.

The cost is corpus-wide and in the permissive direction: every function in all
185 units has its branch conditions and return expressions re-classified. An
implementing card therefore needs the full C02 discipline — a conjunctive
whitelist, held-out fixtures in both precision directions, certificate
rederivation from the certificate's own content, audited Lean theorems, and two
full corpus measurements with every moved leaf attributed and any newly surfaced
provisional `Unsafe` unit named prominently.

## 6. The minimal validation case

`digits10` in `lib/monkey/mk_core/mk_string.c` is the whole POSITION half in one
function:

```c
uint32_t digits10(uint64_t v) {
    if (v < 10) return 1;
    /* … nine more literal and scalar-arithmetic returns … */
    return 12 + digits10(v / 1000000000000UL);
}
```

Its single blocking instruction is that last return. It carries no resource, no
pointer and no aggregate; the recursive callee's `N(m)` is in force at the call
site; and admitting it moves NFR's corpus reach from 0 to 1. It is the smallest
end-to-end proof that the POSITION rule is implementable and sound, and an
implementing card should land it first, alone, and measure the corpus before
widening to the other five positions.

## 7. Two boundaries this note does not cross

* **Heap-link chain teardown stays refused.** `cJSON_Delete`, `rd_avl_remove_elm0`
  and the `otlp_*_destroy` family are chain teardowns. Admitting their
  *positions* and their *reads* does not admit their *releases*; they reach the
  neutral route only because `N(m)` claims no effect, and a body that releases
  through an unnamed place still refuses at V5/V6. The design's §8 boundary
  (`docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md`, unit U6) is untouched, and no
  `ReleasedReachable` mode is proposed here.
* **Function-pointer parameters are a separate, smaller gap.**
  `rd_kafka_q_apply` is the corpus's only member that refuses at **V3**:
  `function_parameters` cannot read its `int (*callback)(...)` declarator, so no
  `N(m)` candidate exists for it at all. That is recorded as follow-on
  **C03-REACH-FNPTR-PARAM**. It clears one component and is independent of
  everything above.
