# Source Identity rules

Current reference, routed by [AGENTS.md](../../AGENTS.md). Rule numbers are stable.
Paths in backticks are repository-relative. Read every applicable rule,
including cross-topic rules, before editing; the index is not a substitute.

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
