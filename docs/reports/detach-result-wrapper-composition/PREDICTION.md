# Prediction: checked removed-selection composition (rule 33)

Frozen before any semantic change. Base `e282aaccc7763da989cf6f3c8f3c51cdd3c8de3c`
(main = origin/main, contract cycle4 merged). Written 2026-10-22 by card
DETACH-RESULT-WRAPPER-COMPOSITION; later results are compared against this file
and never edit it.

## Planned semantics

A **checked removed-selection composition** is source-derived for a function
whose complete body is exactly one of two frames (alpha-renamed, NULL spellings
normalized as the existing detach/lookup readers do):

- *selection detach* `R *F(R *P, int W) { if (W < 0) { return NULL; } return D(P, L(P, (size_t)W)); }`
  where `L` passes the existing checked indexed lookup frame reader and `D` is
  in force with the exact same-item `Removed` declaration shape, standing in this
  unit (its physical unlink frame and named `detach_list_representation` premise
  accepted, never withdrawn);
- *selection release* `void G(R *P, int W) { X(F(P, W)); }` where `F` is a
  selection detach of the same unit and `X` is declared exactly
  `resource_released(item, nullable)` with no other custody position.

The selected child is `sel(P, W) = (W < 0) ? NULL : L(P, (size_t)W)`, i.e. the
value the frame itself computes. The row of `F`/`G` is analysed by the ordinary
analyzer on the expanded frame under a *selection entry*: `P` is a live, settled,
non-NULL owned parent; `W` is a borrowed scalar; at the one checked lookup call
whose index is `W` (reached only when `W >= 0`, with no earlier call or write),
the selected child appears as one nullable child uniquely contained by `P`. That
membership is a **prerequisite transported to the caller**, never derived from
the representation premise or from borrowed provenance. The existing same-item
adapter then decides the detach exactly as today. Validation requires a fully
modeled, diagnostic-free run whose every exit keeps `P` live and either returns
NULL without having selected, or returns (F) / releases (G) exactly the detached
selected child; anything else publishes nothing.

Callers transport the summary by expanding a call `d = F(p, K);` or `G(p, K);`
with a plain parent name and a decimal constant `K` in 0–255 into the frame's own
constituent calls (`L(p, K)`, then `D`, then `X`). The existing lookup-identity,
physical-prefix, membership, NULL, alias and detach checks apply unchanged and
the detach declaration (with its named premise) is consulted at the caller.
Every other call shape (formal/nonliteral index, nested, whole-return, cast,
unknown parent) stays refused. Ordinary W4/fresh-result inference never sees the
selected child as fresh custody.

String-keyed object lookup (`get_object_item` / `cJSON_GetObjectItem*`) has only
a declared borrowed-from contract and no checked key-membership frame. This card
does **not** fake that membership: the two object detaches and their two delete
wrappers are predicted to stay Unknown with unchanged diagnostics.

## Predicted measurements (pinned corpus, BASE e282aaccc vs TIP)

- `cJSON_DetachItemFromArray`: Unknown → **Safe (conditional)**. Dependencies:
  declaration `cJSON_DetachItemViaPointer` with its named
  `detach_list_representation` premise, summary `get_array_item` and its null
  premise `null-deref:get_array_item:2770:current_child`.
- `cJSON_DeleteItemFromArray`: Unknown → **Safe (conditional)**, with the same
  dependencies plus declaration `cJSON_Delete` and the selection summary of
  `cJSON_DetachItemFromArray`.
- `cJSON_DetachItemFromObject`, `cJSON_DetachItemFromObjectCaseSensitive`,
  `cJSON_DeleteItemFromObject`, `cJSON_DeleteItemFromObjectCaseSensitive`,
  `cJSON_DetachItemViaPointer`: Unknown, diagnostics unchanged.
- Corpus custody: **Safe +2, Unknown −2**, provisional Unsafe unchanged; no other
  status change. The production cross-unit callers
  (`rdkafka_sasl_oauthbearer_oidc.c`, string keys) stay as they are.
- Null-property dictionaries: identical (the null analysis is not touched).
  Combined (custody+null) Safe may rise by at most 2.
- The detach declaration stays pinned-in-force / DeclaredAssumption; its
  dependent-row count may grow by the two promoted rows, all listing the premise.
- System and tail-pause slice select no cJSON function: no route change predicted
  and only the corpus is measured.

A measured outcome that differs (fewer promotions, any other status move, any
Safe regression, any premise not listed) is reported as such and attributed; it
is not tuned toward this prediction.
