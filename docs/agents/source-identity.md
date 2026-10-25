# Source Identity rules

Current rule text, routed by the [rule router](README.md), which says what to
read. Rule numbers and anchors are stable; backticked paths are repository-relative.

<!-- generated:rules:start — edit docs/agents/rules/<id>.md, then run python3 scripts/gen_rule_docs.py -->

<a id="constraint-11"></a>

<!-- preserved:constraint-11:start -->
11. **Do not normalize the spelled input path** (that changes `__FILE__`), **do
    not** let a declared identity survive onto derived text, and **do not** move
    the pinned Cycle 19 compatibility route off `HistoricalStdin` without
    re-measuring its 185-unit artifacts. `CompileTaskIdentity` binds this
    transport as `source_input` — keep it there, or a standard-input run and a
    declared-file run record the same identity for different preprocessing.
    `remaining-g02`, `remaining-g01`.
<!-- preserved:constraint-11:end -->

<a id="constraint-12"></a>

<!-- preserved:constraint-12:start -->
12. A `#line` directive in analysed source must not be able to move the audited
    scope; attribution is by include-depth flags no directive can move, and
    `Safe` requires full analysed coverage at the certificate gate.
    `remaining-e01-tail`.
<!-- preserved:constraint-12:end -->

<a id="constraint-12a"></a>

<!-- preserved:constraint-12a:start -->
12a. **A presented coordinate is derived, and no file spelling is ever read from
    a flagless marker.** A gateway diagnostic presents the original,
    pre-preprocessing `file:line`; the preprocessed `L`-number stays the
    **internal** coordinate of the certificate, `source-effects.json`,
    `chunk-contracts.json`, the census APIs and the probe examples, which is the
    user's ruling and not an omission (`GATEWAY-LOCATION-INTERNAL-CHANNELS`). The
    coordinate is derived by pairing the analysed `-E -P` text with a second,
    **presentation-only** marked `-E` run, and the pairing is **checked, not
    assumed**: same length, same trimmed text line for line, or the whole map is
    refused. Three rules carry the anti-forgery property and must survive
    refactors — a frame's spelling is adopted **only** from an *entering* marker
    (flag `1`), so a `#line`, which Clang re-emits flagless, can never write one;
    include depth 0 is not named by a marker at all but resolved by the caller
    against the audit's own configured file, which is what keeps the pinned
    transport's guessable `<stdin>` out of every report; and a flagless marker
    naming a file other than the one its frame was entered as **withdraws** the
    coordinate for every line it governs rather than relabelling it, the
    diagnostic falling back to `preprocessed line N`, which names itself as
    internal. Proved in general as `presented_files_are_entered_files`
    (`QCore.SourceCoordinate`), not on witnesses. What a `#line` *can* still do is
    renumber the file it is written in, which is its C11 §6.10.4 meaning and
    indistinguishable from one of Clang's own resynchronisations
    (`GATEWAY-LOCATION-LINE-ONLY-DIRECTIVE`) — it moves no file identity, no
    include depth and **no audited scope**, so constraint 12 is untouched: the
    scope reader (`legacy_enumeration`) is not read, not written and cannot be
    influenced by this module, and the map is built only *after* a unit's result
    is settled. All four refusal grounds are
    `ReasonCategory::EnvironmentUnavailable` on the producer-supplied path, so
    §L21.1 rule 1 applies and none can contribute to a provisional `Unsafe`; a
    refused unit keeps every internal position and is never given an approximate
    coordinate. `gateway-c-location`.

<!-- preserved:constraint-12a:end -->

<a id="constraint-12b"></a>

12b. **A contract is in force at a revision no person reviewed only through a
    tip identity, and that is never silent.** Card CONTRACT-PIN-TIP-MODE
    (contract specification C06.2, tip identities; user decision 2026-10-02).
    A lock entry holds reviewed identities or one tip identity (a repository,
    a full refname, the reviewed commit and its sites), never both. The tip
    is resolved before any unit is analysed, with read-only Git commands only
    and never a fetch or a working tree: located by
    `QUIRIN_TIP_REPOSITORIES` then the recorded path, the ref's current
    commit must descend from the reviewed commit, and the function's sites
    are read from that commit's blobs. It is in force exactly when the
    contract text is the pinned one and those sites equal the audited ones
    (`tip_decision`, bound to `QCore.ContractTipMode.tipVectors`); anything
    unresolvable is `tip-unresolved` and held like any pin mismatch. **Do
    not** let a tip entry keep a reviewed identity beside it, resolve a tip
    from a working tree, default an unresolvable tip into force, map a
    vendored path prefix onto the tip's paths, or let the notice (the
    contract targets a future, unreleased release; `info` in force, `warning`
    not) or the free-text release label reach a verdict, status or count. An
    in-force tip entry is the named assumption `tip-tracked:FUNCTION` and its
    inventory status is `tip-in-force`, never `pinned-in-force`; a run with
    no tip entry is byte-identical to one before the card.
    [Report](../reports/contract-pin-tip-mode/README.md).

<!-- generated:rules:end -->

## Certificate target scope

A legacy-C Safe certificate carries its complete selected target inventory and
the retained source evidence from which the independent checker reconstructs
that denominator. `checked_functions` is only a checked compatibility index;
it must never define the intended target set. Every selected target requires
an analyzed Safe row, with matching canonical inventory and request/artifact
identities. A duplicate, omitted, excluded or Unknown selected row cannot
support a Safe certificate. This discharges `E01-TAIL-CERT-SCOPE` at the
[certificate-scope card's boundary](../reports/review-certificate-scope/README.md).

Structural self-consistency is distinct from intended-target authentication.
Consumers requiring the latter must compare with an independently held expected
context using `check_legacy_safety_certificate_for`, rather than copying the
expected context from the certificate. The inner request/artifact identity
does not itself establish compiler/build provenance or the caller's intended
contract/profile/trust environment. Configured audit paths additionally use
`check_audit_legacy_safety_certificate_for`: the complete `CompileTaskIdentity`
must match the caller-held record and the prepared request/artifacts, and exact
contracts, retained contract sources, source profile and declared-assumption
trust policy must match independently loaded expected inputs. Admission
rederives trust identity from the presented certificate; a stale wrapper cannot
authenticate substituted contracts. Missing or unequal context publishes
Unknown for the unit and every affected function, retaining the denominator.

This binding compares recorded provenance; it is not compiler-execution or
environment attestation. It does not hash the compiler executable or each
individual header, prove declared-driver equivalence, or verify trusted
contract claims against library bodies. Header effects are bound through the
retained preprocessed content. Prepared audit paths use the exact retained raw,
marked and plain preprocessing artifacts; they do not reprocess the input to
manufacture certificate scope evidence. See the
[context-binding card](../reports/certificate-context-binding/README.md) for
the measured boundary and remaining assumptions.

## Assumed build settings are a scope, never an equivalence

`quirin-audit contracts` (W44-CONTRACTS-ONLY-NO-BUILD) analyses a unit under an
**assumed** configuration: the user's `-I`/`-D`/`--std` plus the stated
defaults (`gnu17`, the unit's own directory for quoted includes, the host
Clang's system headers), never a compilation database's command. Every
verdict it publishes, `Safe` included, is **relative to the recorded
assumptions only**, and must never be presented as equivalent to, comparable
with or a substitute for a build-configured verdict of the same file. A
different configuration can select different code (`QCore.NoBuildScope`'s
`favourable_direction_exists`). Keep the three carriers of that scope:

- each report row's `assumptions` (with `verdict_scope`);
- the report's `evidence` block (`evidence: "contracts-only-no-build"`,
  `build: "none"`, `equivalence_to_build_configured: "none: …"`);
- the assumption record as each compile task's `source_profile` dependency,
  so a Safe certificate minted under assumed settings is never admitted for a
  configured compile task.

A unit that does not preprocess or parse under the assumptions is `Unknown`
with its reason code and first diagnostic, never dropped. Nothing is guessed,
so no include directory is searched for and no configuration header is
generated. Headers are read as found on disk, tracked or not, and are bound
only through the preprocessed text. A comparison with a configured run
attributes each verdict difference to a preparation refusal or a text
difference (`verdict_difference_needs_text_difference`). It must never pool
the two runs' counts. [Report](../reports/w44-contracts-only-no-build/README.md).
