# W43-DESCRIPTOR-RESOURCE-TRACKING: kind-indexed custody, integer-carried descriptors and eight libc declarations

Card W43-DESCRIPTOR-RESOURCE-TRACKING, 2026-09-25, on base `f8b41209` (main
after CONTRACT-INHERIT-ADOPTION's matrix repair), merged with main `0a07cb57`
(W43-INDIRECT-ACCESS-GENERAL, CONTRACT-INHERIT-QCORE-PROVIDER-NAMING and
CONTRACT-VIEW-PROJECTION-CHECK) and re-arbitrated on the combined tree. File descriptors and other OS
handles could not be tracked: a descriptor is an `int`, which vocabulary
version 1 never treated as custody, and v1 custody had one kind, so a declared
`fopen`/`fclose` pair would have let `free(fopen(…))` and `fclose(malloc(…))`
both discharge. This card gives every custody position a **kind**, lets one
kind (`descriptor`) be carried by an `int` whose failure value is the sentinel
`-1`, carries both into the C gateway with the scalar-route consequences, models
them in Lean, and declares eight libc functions with them, only in
`providers/quirin-provider-libc`.

It discharges `W43-LIBC-DESCRIPTOR-LIFECYCLE` and the `CT-CTWIN-DESCRIPTOR` arm of
`CT-CTWIN-O`, addresses the sentinel half of `EXTERN-CALL-F` where it applies to
the resource loader, and registers five residuals.

## Headline

* **Vocabulary** (contract spec [§C04.6](../../QCORE_CONTRACT_SPEC_r0_3.md)):
  `ensures normal resource_kind(p, KIND);` and
  `ensures normal resource_return_kind(KIND);`, `KIND` one of `descriptor`,
  `stream`, `directory`. The heap is spelled by the **absence** of a kind
  clause, so every earlier module loads to exactly its previous contract, and a
  contract without a kind serializes and prints byte for byte as before.
  `descriptor` sits on an `i32` and spells its validity `sentinel` / `valid`
  instead of `nullable` / `nonnull`. An addition to vocabulary version 1, like
  the derived results before it; no new evidence version.
* **Gateway** (`legacy_c.rs`, new `legacy_descriptor_custody.rs`):
  * a release discharges only its own kind; a mismatched release that resolves
    to one identity is a `KindMismatchedRelease` candidate, published as
    `invalid_release`, and every other mismatched custody position refuses;
  * `fd < 0`, `fd == -1`, `fd >= 0`, `fd != -1`, `fd > -1` (and mirrors) narrow a
    descriptor's validity exactly; `fd > 0` narrows its true edge only; `if (fd)`,
    `!fd`, `fd == 0` refine nothing, because descriptor `0` is valid;
  * **one predicate**, `descriptor_custody::value_carries_descriptor`, decides
    whether a value carries a tracked descriptor; every statement that reads one
    is admitted only by an instruction that states the effect exactly (a call, a
    copy into a plain name, a return, a release, a validity test, a scalar
    computation into another name). The member, aggregate and indirect scalar
    stores, the unbound scalar-field no-op and any future instruction variant are
    refused, and the scalar-source publication exemption does not cover it;
  * a descriptor handed to a callee that no declaration covers is refused;
  * a released non-heap identity used at a `Borrowed` position is a
    `UseAfterClose` candidate, published as `use_after_release`;
  * W4 infers no kinded summary, and validates no heap summary against a body
    that releases a formal with another kind's release.
* **Lean**: [`QCore.KindedCustody`](../../../proof/QCore/KindedCustody.lean),
  30 theorems audited by [`AuditKindedCustody.lean`](../../../proof/AuditKindedCustody.lean),
  including the soundness of the validity table for every value a descriptor
  call can produce and all-heap conservativity. No placeholder, no project axiom.
* **Declared: 8**, each a trusted assumption agreeing with every captured unit
  that declares it: `open`, `close`, `socket`, `fopen`, `fclose`, `fileno`
  (neutral), `opendir`, `closedir` — seven of the brief's seventeen, plus
  `fileno`. **Refused by name: the other ten**, and six more spelled without a
  leaf, each with a measured ground (§4), including `dlopen`/`dlclose`, withheld
  because their one corpus effect was a leak candidate refuted by reading.
* **Arbiter** (combined tree). BEFORE, main `0a07cb57`'s own binary, reproduces
  the W43-INDIRECT-ACCESS-GENERAL diagnostics byte for byte. **(a) Vocabulary and gateway alone: all five artifacts byte-identical to
  BEFORE.** **(b) Declarations:** 60 leaves removed, 43 added, all attributed;
  56 are the declared callees' own leaves. **(c) Wrappers:** none declared.
  **(d) Candidates:** one leak added (`ctr_random_get`, `if (fd > 0)`), one leak
  withdrawn by refusal (`mk_rconf_read`, still Unsafe); no kind-mismatch or
  use-after-close leaf.
* **Totals.** Functions 269 / 84 / 4,481 → **269 / 85 / 4,480** (Safe /
  provisional Unsafe / Unknown); units 3 / 42 / 136 / 4 → **3 / 43 / 135 / 4**.
  Leak **271** (+1 −1), `unknown_alias` 4,727 → 4,765, `unsupported` 47,832 →
  47,777. The pre-merge arbiter on `f8b41209` measured the identical movement
  leaf for leaf ([pre-merge-totals.txt](pre-merge-totals.txt)). The frozen prediction ([PREDICTION.md](PREDICTION.md)) held on the
  function, unit and candidate rows; its three misses are listed in §1.

| Measure | BEFORE (main `0a07cb57`, W43-INDIRECT-ACCESS-GENERAL baseline) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 269 / 84 / 4,481 | **269 / 85 / 4,480** | 0 / +1 / −1 |
| Units Safe / provisional Unsafe / Unknown / data-only | 3 / 42 / 136 / 4 | **3 / 43 / 135 / 4** | 0 / +1 / −1 / 0 |
| `leak` / `summary_conflict` / `double_release` | 271 / 1 / 0 | 271 / 1 / 0 | +1 −1 / 0 / 0 |
| `invalid_release` / `use_after_release` | 0 / 0 | 0 / 0 | 0 |
| `unknown_alias` | 4,727 | **4,765** | +38 |
| `unsupported` | 47,832 | **47,777** | −55 |
| `call to uncontracted function` on Unknown functions: leaves / functions / sole | 11,642 / 2,684 / 298 | 11,601 / 2,673 / 297 | −41 / −11 / −1 |
| Descriptor/handle callee leaves / functions / sole-blocked ([inventory](inventory-before.txt) → [after](inventory-after.txt)) | 80 / 38 / 5 | 24 / 14 / 3 | −56 / −24 / −2 |
| Contract corpus modules / manifest-selected / resource-v1 declarations | 108 / 45 / 130 | **109 / 46 / 138** | +1 / +1 / +8 |

## 1. Inventory and prediction

[`scripts/descriptor_inventory.py`](scripts/descriptor_inventory.py) counts every
`call to uncontracted function X` leaf for 55 descriptor and handle names over
the baseline; [`scripts/source_call_sites.py`](scripts/source_call_sites.py)
counts where each is *spelled* in the 185 audited units' own text, which also
finds calls hidden behind another statement's refusal.

| Callee | Leaves | Functions | Sole-blocked | Decision |
|---|---:|---:|---:|---|
| `close` | 20 | 11 | 1 | declared, `descriptor` release |
| `fclose` | 22 | 6 | 0 | declared, `stream` release |
| `closedir` | 5 | 4 | 0 | declared, `directory` release |
| `open` | 3 | 3 | 1 | declared, `descriptor` result, variadic tail |
| `socket` | 3 | 3 | 1 | declared, `descriptor` result |
| `opendir` | 2 | 2 | 0 | declared, `directory` result |
| `fopen` | 1 | 1 | 0 | declared, `stream` result |
| `fileno` | 0 (1 nested) | 1 | 0 | declared, neutral: its number is borrowed |
| `freeaddrinfo` / `getaddrinfo` | 6 / 4 | 4 / 4 | 0 | refused: status-conditioned out list |
| `dlclose` / `dlopen` | 3 / 2 | 2 / 2 | 1 / 1 | refused: cast-return false leak (§3) |
| `fcntl` | 2 | 2 | 0 | refused: command-dependent creator |
| `accept4` | 1 | 1 | 0 | refused: unreadable `__SOCKADDR_ARG` |
| `fts_open` / `fts_close` | 1 / 1 | 1 / 1 | 0 | refused: unreadable `fts_open` |
| `glob` / `globfree` | 1 / 1 | 1 / 1 | 0 | refused: in-place lifecycle plus callback |
| `regcomp` / `regfree` | 1 / 1 | 1 / 1 | 0 | already registered (`W43-LIBC-IN-PLACE-LIFECYCLE`) |
| **total** | **80** | **38** | **5** | |

Spelled with no leaf of their own ([source-call-sites.txt](source-call-sites.txt)):
`accept` (2), `pipe` (1), `fdopen` (1), `fileno` (3), `mmap` (2), `munmap` (1).
Every other name in the brief's list — `dup`, `dup2`, `creat`, `openat`,
`mkstemp`, `popen`/`pclose`, `socketpair`, `epoll_create1`, `eventfd`,
`timerfd_create`, `signalfd`, `inotify_init1` — is spelled in no audited unit and
is not declared: a declaration no call site reads would be an untested trusted
assumption. The brief's "`fclose` sole-blocks 6" is "blocks 6"; its sole-blocked
count is 0.

[PREDICTION.md](PREDICTION.md) was frozen and committed (`65fe4182`) before any
implementation. Outcome against it:

| Predicted | Measured |
|---|---|
| class (a) byte-identical on all five artifacts | **yes** |
| 0 functions Unknown → Safe | **0** |
| 1 function and its unit Unknown → provisional Unsafe (`ctr_random_get`, `if (fd > 0)`), leak +1 | **yes**, and its unit |
| no scalar-route refusal in the corpus | **1**: `mk_vhost_fdt_open`'s `hc->fd = fd;` (§2.3) — the prediction missed this shape |
| no kind-mismatch or use-after-close candidate | **0 / 0** |
| `unsupported` −50…−65, `unknown_alias` +20…+35 | −55, **+38** (three above the band; `mk_rconf_read` alone adds ten, one per `fclose(f)`) |
| declare `dlopen`/`dlclose` | **withdrawn** after the first AFTER run (§3); the prediction did not foresee the cast-return shape |
| leak 272 → 273 | **272** (pre-merge; 271 on the combined tree): the predicted +1, and an unpredicted withdrawal (−1) in `mk_rconf_read` |

## 2. The design

### 2.1 Vocabulary, placement and version

`ResourceFunctionContract` gains `parameter_kinds` (non-heap entries only) and
`result_kind` (heap by default), both skipped when heap by serde and by a
hand-written `Debug`, so the `SummaryConflict` text the audit renders is
unchanged ([`qcore-schemas/src/contract.rs`](../../../crates/qcore-schemas/src/contract.rs)).
The loader ([`contract_loader.rs`](../../../crates/qcore-contract/src/contract_loader.rs),
`check_kinds`) admits a kind only on an owned, consumed, released or out
parameter or an owned result; a `descriptor` only on `i32`, spelled
`sentinel`/`valid`, never on an out slot; every other kind on a pointer, spelled
`nullable`/`nonnull`. A borrowed, stored, removed or reallocated position, a
derived or `none` result, the spelled-out `heap`, an unknown kind, a duplicate,
and a validity word on the wrong carrier each reject the whole module
([`crates/qcore-contract/tests/resource_kinds.rs`](../../../crates/qcore-contract/tests/resource_kinds.rs), 20 rejection rows). An `i32` custody mode without the
`descriptor` kind keeps the verbatim pre-card refusals. `extern_bridge` refuses a
kinded declaration by name: the checked extern-call model has one custody kind.

The kind vocabulary is exactly the kinds a shipped declaration uses. An `FTS *`
kind and a library-handle kind were drafted and withdrawn with the declarations
that needed them (§4), following the loader's standing rule of no vocabulary
without a use.

**Version.** An addition to resource vocabulary version 1: the marker is
unchanged, all 130 earlier declarations load to equal contracts (the old-table
parity test in `compiler/tests/contract_loader.rs` passes unchanged for them), and
a loader that predates the card rejects `resource_kind`, `resource_return_kind`,
`sentinel` and `valid` as unsupported rather than misreading them. Every verdict
already carries the exact module texts in `source_assumptions`, so no evidence
version is minted.

### 2.2 The gateway rules

| Rule | Where | Test (`crates/qcore-c-gateway/tests/descriptor_custody.rs`) |
|---|---|---|
| A site's kind is the kind of the position that created it (result, out slot, or a formal under the contract a body is checked against); absent is heap | `descriptor_custody::site_kinds`, `EntryCustody::site_kinds`, `TransferContext::site_kinds` | `kinds_load_onto_their_positions_and_the_heap_is_the_default` |
| A release of another kind discharges nothing; resolved → `KindMismatchedRelease` from settled states; every other mismatched position refuses | `kind_mismatch`, `refuse_kind_mismatch`, `report_kind_violations` | `a_release_of_another_kind_is_accused_and_discharges_nothing` |
| Validity guards on a descriptor name compile to `BranchOnValidity` and narrow like a null test; `fd > 0` true edge only; null-shaped tests refine nothing | `descriptor_custody::validity_test`, `refine_edge`, `refine_validity`; joined provenance excludes the site on the invalid edge | `a_guarded_descriptor_closed_once_certifies`, `a_null_shaped_test_of_a_descriptor_refines_nothing`, `a_positive_test_leaks_descriptor_zero`, `an_unguarded_descriptor_is_not_valid_for_a_valid_position` |
| Leak, double close, use after close | existing leak and double-release arms; `report_kind_violations` for `UseAfterClose` | `the_three_lifecycle_violations_are_candidates` |
| A tracked descriptor read by a custody-neutral route is refused | `Parser::classify_simple` guard over `statement_reads_descriptor` / `admits_descriptor_read` | `a_tracked_descriptor_on_every_scalar_store_route_is_refused` |
| Publication: the declared-scalar source exemption does not cover a tracked descriptor | `carries_integer_identity` in both publication checks | `publishing_a_tracked_descriptor_is_refused_not_admitted_as_a_scalar` |
| A copy aliases, arithmetic creates nothing, a return escapes, an overwrite leaks | existing `Alias`, `ForgetScalar`, `EscapeIfOwned` | `arithmetic_copies_and_returns_state_their_effect_exactly` |
| A descriptor at a callee no declaration covers is refused | `Parser::classify_simple` guard, `declared_callees` | `a_descriptor_passed_where_no_declaration_states_the_effect_is_refused` |
| W4 infers no kinded summary and validates no heap summary over a kinded release | `propose_result`, `out_mode`, `result_matches`, `out_slots_match` | `no_heap_summary_is_inferred_for_a_descriptor_wrapper` |
| Heap-only bodies unchanged | every rule is gated on a non-empty descriptor-name set or a non-heap site | `a_heap_only_body_is_unchanged`, class (a) |
| The CT twin loads | loader | `the_ct_twin_descriptor_shape_is_expressible` |

Two diagnostic kinds are new: `KindMismatchedRelease` and `UseAfterClose`,
published in the existing `invalid_release` and `use_after_release` categories
with their own wording. Both are safety issues and path-poisoning, like
`double_release`, and **neither is a validated upstream defect**.

### 2.3 The scalar-route interaction

The critical soundness point of the brief: once an `int` can carry custody,
every route that treats a declared scalar as custody-neutral is wrong for a
tracked descriptor. The card closes it with one predicate and one guard, not a
patch per route:

* `DescriptorNames::read` computes, per body, the names that may hold a tracked
  integer identity (assigned from an integer-kinded result, possibly behind one
  cast; copied from one; or a formal the function's own declared contract
  kinds), flow-insensitively, so it can only refuse more.
* `value_carries_descriptor(value, names)` is **the** predicate. A member name
  after `.` or `->` is not the variable.
* The guard runs on every classification of a complete statement. A statement
  that reads a descriptor name is admitted only by `ContractCall`, `Alias` into
  a plain name, `Release`, `Escape`, `EscapeIfOwned`, `Clear`,
  `BranchOnValidity`, `ForgetScalar` into a plain non-descriptor name, or an
  already-refusing `Unknown`. The match names every variant, so a new variant is
  refused until someone decides otherwise.

Measured on the corpus: one leaf, `mk_vhost_fdt_open`'s `hc->fd = fd;`
(`mk_vhost.c:229`), whose previous refusal was an `unknown_alias` on `hc`; the
function was and stays Unknown. The prediction had said zero.

**Composition with W43-INDIRECT-ACCESS-GENERAL** (branch tip `6bb838c8`, not on
this base; read, not merged). That card admits `*p = e`, `p[i] = e`, `p->f = e`
and `s.f = e` as a new `LegacyInstruction::IndirectScalarAccess` when `e` is a
declared scalar, by wrapping `Parser::classify_simple` exactly as this card does
(both rename the old body; the textual merge conflicts there). The composition
is: this card's descriptor guard outermost, their typed-scalar route inside it,
the old classifier innermost — so their admitted instruction is checked by the
guard. `admits_descriptor_read` names every variant with no wildcard arm, so the
merge does not compile until `IndirectScalarAccess` is placed; it belongs with
the refused variants (it treats the value as custody-neutral data). The test
`a_tracked_descriptor_on_every_scalar_store_route_is_refused` pins all four
shapes, plus W42's bound `ScalarMemberStore`, refused and never accused; on this
base the indirect shapes are refused by the lexical gate. Registered for the
combined tree (`W43-DESCRIPTOR-INDIRECT-ACCESS-WIRING`); this card wires and
re-tests it when main contains that card.

### 2.4 `dup`, `fdopen`, `fileno`, `fcntl`

* `dup`/`dup2`/`fcntl(F_DUPFD)` would each be an owned `descriptor` result over a
  borrowed descriptor. No audited unit calls `dup` or `dup2`; `fcntl` is refused
  (§4).
* `fdopen` moves the descriptor into the stream only when it returns non-NULL.
  A consumed descriptor on both arms would hide the failure arm's obligation, so
  it is refused (no leaf; one spelled call).
* `fileno` is declared neutral: its result is the stream's own descriptor
  number, borrowed. A later `close(fileno(fp))` refuses (a nested call at a
  custody position) and `int fd = fileno(fp); close(fd);` refuses (an untracked
  name at a release), so the stream's descriptor can never be released
  independently through this declaration.

### 2.5 Lean parity

[`QCore.KindedCustody`](../../../proof/QCore/KindedCustody.lean) models sites
with static kinds, bindings, failure-value possibility and the seven
instructions the rules speak about, beside the one-kind pre-card transfer. The
30 audited theorems cover:

* **own kind**: `a_mismatched_release_discharges_nothing`,
  `a_mismatched_release_is_a_candidate`, `a_matching_release_releases`, and the
  three named shapes (`freeing_a_stream_is_a_candidate`,
  `closing_a_heap_allocation_as_a_stream_is_a_candidate`,
  `closing_a_stream_as_a_descriptor_is_a_candidate`);
* **validity**: `facts_are_sound` (the comparison table is sound for every value
  in `{-1} ∪ ℕ`), `the_positive_tests_false_edge_includes_a_valid_descriptor`
  and `equality_with_zero_decides_no_validity` (why two edges stay unrefined),
  `an_invalid_edge_removes_the_descriptor`, `a_valid_edge_keeps_a_valid_descriptor`,
  `an_unrefined_edge_changes_nothing`, `a_validity_test_of_a_pointer_kind_is_refused`;
* **null tests**: `a_descriptor_null_test_refines_nothing` beside
  `a_pointer_null_edge_removes_its_site`;
* **scalar routes**: `a_tracked_descriptor_on_a_scalar_route_is_refused`,
  `a_tracked_descriptor_is_never_neutral_data`,
  `an_untracked_scalar_route_stays_neutral`, `arithmetic_creates_no_identity`;
* **use after close**: `a_use_after_close_is_a_candidate`,
  `a_heap_use_after_release_keeps_its_refusal`;
* **no `Safe`**: `the_kind_candidates_are_safety_issues`,
  `a_kind_candidate_is_never_safe`, `a_refusal_is_never_safe`;
* **conservativity**: `an_all_heap_program_is_unaffected`;
* **witnesses**: `a_guarded_close_discharges`,
  `the_positive_guard_leaks_on_its_false_edge` (the `ctr_random_get` shape),
  `a_double_close_is_a_candidate`, `a_mismatched_release_leaves_the_obligation`,
  `storing_a_descriptor_into_a_member_is_refused`.

As for every gateway module, the Rust–Lean correspondence is maintained by
review, not proved (trust boundary 28).

## 3. Declarations and the ABI gate

Appended to the end of their modules, with a pointer from each module's old
"deliberately not declared" note; one new module, `libc/dirent.qcore`:

| Module | Declarations | Census rows (185 captured units) |
|---|---|---|
| `libc/unistd.qcore` | `open` (`<fcntl.h>`, declared beside its release): borrowed path and flags, variadic data tail for `mode`, `resource_return_owned(sentinel)` + `descriptor`; `close`: `resource_released(fd, valid)` + `descriptor` | open 112 agreed / 63 undeclared; close 137 / 36 |
| `libc/socket.qcore` | `socket`: three borrowed scalars, `sentinel` owned `descriptor` result | 105 agreed / 16 undeclared |
| `libc/stdio.qcore` | `fopen` (`nullable` owned `stream`), `fclose` (`nonnull` released `stream`), `fileno` (neutral) | 168 agreed each |
| `libc/dirent.qcore` (new) | `opendir` (`nullable` owned `directory`), `closedir` (`nonnull` released `directory`) | 17 agreed / 1 undeclared each |

Every row is `agreed` or `undeclared`; 0 withdrawn
([abi-census.txt](abi-census.txt), [scripts/census.py](scripts/census.py) over the
line-marker-stripped captures of a tee-wrapped run whose diagnostics were
byte-identical to class (a)). The AFTER diagnostics contain no
`whose declared contract was withdrawn` leaf for any of the eight names. Each
module's comment carries the prototypes, the custody reading and a "what these
declarations do not say" list: nothing about `errno`, `EINTR`, a failed `close`
(read as a release, the Linux reading), buffering or entry order.

**`dlopen`/`dlclose` withdrawn, and why.** Drafted, agreed (4 / 4 units), and
selected in the first AFTER run (`measure/after1`), they moved one more function:
`rd_dl_open0` Unknown → provisional Unsafe with a leak at
`return (rd_dl_hnd_t *)handle;`. The handle *is* returned; the return route reads
`return (T *)p;` as returning no place. That shape reports the same false leak for
a heap pointer (`a_cast_return_is_not_an_escape_for_any_kind`), and three of the
271 combined-tree baseline leaks already sit at one ([cast-return-leaks.txt](cast-return-leaks.txt)).
A declaration whose one corpus effect is a refuted candidate adds no evidence, so
the pair and its kind were withdrawn before the published arbiter, and the
return route is registered (`RETURN-CAST-PLACE-NOT-ESCAPED`).

**Wrappers (class c).** None declared. Measured: `rd_socket_close` and Fluent
Bit's `flb_socket_close` are macros for `close` (their call sites already read
`close`); Fluent Bit's `flb_pipe_close` accepts the sentinel (`if (fd == -1)
return -1;`), so it is not an exact forwarding wrapper of `close` and
`contract inherit libc::close` would misstate it; `flb_pipe_create` wraps `pipe`,
which is refused; `mk_socket_create` sets flags and logs, so it is not
forwarding either. Fluent Bit core units are not in the corpus, and the
`qcore-provider-*` crates (`librdkafka::`, `libevent::`) are not on the load
path at this base.

## 4. Refused by name

| Callee | Leaves / functions / sole (AFTER) | Ground |
|---|---|---|
| `getaddrinfo`, `freeaddrinfo` | 4/4/0, 6/4/0 | The list is valid exactly when the call returns `0`: a status witness bound to the out slot. A `nullable` out slot would misstate the failure arm; the release alone decides nothing. |
| `fcntl` | 2/2/**1** | A creator only for `F_DUPFD`/`F_DUPFD_CLOEXEC`; the units spell `F_SETFL`, `F_GETFL`, `F_SETFD`, `F_SETLK`, and a declaration carries one literal premise per parameter. `rd_kafka_open_cb_generic` is now blocked by it alone. |
| `dlopen`, `dlclose` | 2/2/1, 3/2/1 | §3: the cast-return false leak. |
| `accept4` (`accept`) | 1/1/0 | `__SOCKADDR_ARG` transparent union: withdrawn in all 93 declaring units. |
| `fts_open`, `fts_close` | 1/1/0 each | `fts_open` is withdrawn (`char * const *`, inline comparator); the release alone decides nothing. |
| `glob`, `globfree` | 1/1/0 each | In-place lifecycle of a caller `glob_t` plus an error callback. |
| `pipe`, `socketpair` | — | An `int[2]` out slot; a descriptor has no out slot here. |
| `fdopen` | — | Transfer into the stream on success only. |
| `mmap`, `munmap` | — | Pointer sentinel `MAP_FAILED`. |

All registered as `W43-DESCRIPTOR-REFUSED-SHAPES` and `RETURN-CAST-PLACE-NOT-ESCAPED`.

## 5. Arbiter

One command shape, [`../w43-return-expression/scripts/run_audit.py`](../w43-return-expression/scripts/run_audit.py),
with `QCORE_CLANG=/usr/bin/clang` and the wave-20 ChunkIO package.

The published arbiter runs on the **combined tree** after merging main
`0a07cb57`:

| Run | Tree | Declarations | Exit | Record |
|---|---|---|---:|---|
| BEFORE | main `0a07cb57`'s own tree and binary (`9824e963…`), extracted by `git archive` to `measure/main-src` | main's | 0 | [before-run.json](before-run.json) |
| CLASS-A | merge `06ee9fe4` binary (`73d6d00a…`) with main's `unistd`/`socket`/`stdio` modules and manifest | main's | 0 | [class-a-run.json](class-a-run.json) |
| AFTER | merge `06ee9fe4` | + 8 | 0 | [after-run.json](after-run.json), [artifacts/run.json](artifacts/run.json) |

**BEFORE vs the W43-INDIRECT-ACCESS-GENERAL baseline** ([before-identity.json](before-identity.json),
`--root-alias <this worktree>=…/agent-a8d24f7f717a987c1`): diagnostics,
chunk-contracts and source byte-identical — 269 / 84 / 4,481 functions, 3 Safe
units, leak 271, as the integrator recorded. `contracts` and `source-effects`
carry the checkout-path identities the brief lists.

**Class (a): CLASS-A vs BEFORE.** diagnostics, chunk-contracts and source
byte-identical; `contracts` identical after rewriting BEFORE's checkout root to
this worktree's; `source-effects` differs only in the path-derived fields the
brief lists — `resource_contracts:<digest>`, `build_digest`,
`configuration_digest`, `task_digest`, `obligation_id`, `coverage[0]` and the
`contract_sources` digests ([class-a-path-alias-diff.txt](class-a-path-alias-diff.txt),
[`scripts/path_alias_diff.py`](scripts/path_alias_diff.py)). With the indirect
rule in force, the vocabulary and gateway alone move no verdict-bearing byte.

**Pre-merge record.** The same three stages on base `f8b41209` against the
W43-LIBC-RESULT-ALIAS baseline gave BEFORE byte-identical, CLASS-A byte-identical
on **all five** artifacts (one checkout), and an AFTER whose attribution matches
the combined tree's class by class and leaf by leaf; a private AFTER-1 with
`dlopen`/`dlclose` still declared is §3's measurement. Totals are in
[pre-merge-totals.txt](pre-merge-totals.txt). The capture run
(`scripts/capture_run.sh`, the tee wrapper) on the pre-merge CLASS-A binary
reproduced its diagnostics byte for byte and supplied the ABI census units and
the `-E -P` texts.

**AFTER vs CLASS-A.** chunk-contracts, source-effects and source byte-identical;
`contracts` differs in the four edited module texts and the one added module.
Attribution: [`../w43-libc-contracts/scripts/attribute.py`](../w43-libc-contracts/scripts/attribute.py)
with this card's eight names, then [`scripts/classes.py`](scripts/classes.py),
which accepts the two leaves that script leaves unassigned only as the
descriptor guard at its own coordinate. Every row is in
[attribution.json.gz](attribution.json.gz); the split is [classes.json](classes.json)
(`attribution_complete: true`):

| Class | Removed | Added | What it is |
|---|---:|---:|---|
| (b) declared-callee | 56 | — | `close` 20, `fclose` 22, `closedir` 5, `open` 3, `socket` 3, `opendir` 2, `fopen` 1 |
| (b) same-line | 2 | 41 | Removed: the `fileno` nested-call refusal and one indirect-access refusal on the `fstat(fileno(fp), …)` line. Added: 37 `unknown_alias` on the actual of a now-modelled release that the projection does not track — a formal (`socket_fd` in `net_plain_close`, `s` in `rd_kafka_transport_close0`), a global stream (`stderr`, `stdout`), or a descriptor or handle produced by an uncontracted wrapper or inside a refused statement (`fd` from `mk_socket_create`/`mk_socket_connect`/`open` in a condition, `listen_s`, `f` ×10 in `mk_rconf_read`, `fp`, `dir`, `tempfile`) — and the two `__qcore_returned_value` pairs of `rd_kafka_open_cb_linux`/`rd_kafka_socket_cb_linux`, which return a possibly-`-1` result under a contract-free exit (2 `unknown_alias` + 2 `unsupported`) |
| (b) same-function | 1 | 1 | Removed: `mk_rconf_read`'s leak at `mk_rconf.c:383` (buffer acquired at 254) — withdrawn by refusal: `fclose(f)` on an untracked `f` now marks every live site unresolved, where the uncontracted call poisoned only bindings. The function keeps its other leak and stays Unsafe. Added: `ctr_random_get`'s leak (d) |
| (a) rule, triggered by (b): descriptor guard | 1 | 1 | `mk_vhost_fdt_open` `hc->fd = fd;`: `unknown_alias hc` → the guard's refusal |
| **total** | **60** | **43** | net −17 = −55 `unsupported` +38 `unknown_alias` ±0 `leak` |

**(d) Violation candidates and their witness status.**

* **Added: `ctr_random_get`** (`ctr_random.c`), Unknown → provisional Unsafe,
  unit likewise: `fd = open("/dev/urandom", O_RDONLY); if (fd > 0) { read; close;
  return; }` and the fallback path returns with the descriptor still owned when
  `open` returned `0`. **Witness status: reachable only if `open` returns
  descriptor 0**, i.e. when the process runs with standard input closed. Not
  observed natively; a candidate under the provisional-Unsafe policy, not an
  upstream defect claim.
* **Withdrawn: `mk_rconf_read`**, one provisional leak removed by the
  unresolved-effect mechanism (above); a withdrawal, not a discharge.
* **Kind-mismatched release / use after close: 0 / 0** in the corpus
  (`W43-DESCRIPTOR-ACCUSING-ARMS-UNEXERCISED-IN-CORPUS`).

**Unit fields.** Every unit whose functions changed moves its
`diagnostic_counts` and count-summary `reason` by exactly its functions' leaf
delta, except `ctr_random.c`, whose status, `reason` and `reason_code`
(`unsupported-syntax` → none) move with its one function's status; this is
the attribution script's only failure line, and it is that unit move.

**Leak pin (constraint 7).** 271 → **271** (272 → 272 pre-merge): one
accusing addition (`ctr_random_get`, the first leak on an integer-carried
identity) and one withdrawal-by-refusal (`mk_rconf_read` L383). The new
baseline is [artifacts/](artifacts/) ([manifest.json](artifacts/manifest.json));
`docs/reports/w43-indirect-access-general/artifacts/` is its frozen predecessor.

## 6. Pin changes, each deliberate

* `compiler/tests/qcore_contract_corpus.rs`: corpus 108 → 109; eight clause
  pins in logical-path order (`open` 6, `close` 4, `socket` 6, `fopen` 5,
  `fclose` 4, `fileno` 3, `opendir` 4, `closedir` 4).
* `compiler/tests/contract_loader.rs`: modules 108 → 109; (selected, v1,
  rejected) (45, 130, 207) → **(46, 138, 207)**; two `source_assumptions` pins
  45 → 46; the old-table parity list gains the eight kinded entries.
* `crates/qcore-c-gateway/tests/{neighbours,prod_contracts}.rs`:
  `source_assumptions` 45 → 46.
* `scripts/tests/test_contract_corpus.py`: modules 108 → 109,
  `quirin-provider-*` share 36 → 37; the libc placement test passes unchanged.
* `cycle19-resource-contracts.json` (schema 2): `libc/dirent.qcore` selected
  after `libc/unistd.qcore`; every v1 declaration stays manifest-selected.
* `providers/quirin-provider-libc/src/lib.rs`: `CONTRACTS` gains
  `libc/dirent.qcore`; `providers/contract-load-path.json` regenerated by
  `providers/contracts.py sync` (one line). `contract/corpus.json` unchanged (the
  libc `contracts/` directory is already a root).
* Exhaustive `LegacyDiagnosticKind` renderers gain the two new kinds
  (`qcore-audit` report, `ct_corpus.rs`, four gateway tests, two examples);
  `alias_aggregate_probe` renders `BranchOnValidity`. `compiler/src/lib.rs`
  re-exports `ResourceKind`. `LegacyInstruction` carries
  `#[allow(clippy::large_enum_variant)]` with its reason, the crate's precedent.

No assertion is weakened.

## 7. Validation

GATE_SECTION

## 8. Ledger and status

* **Retired** ([retired-followups.json](retired-followups.json)):
  `W43-LIBC-DESCRIPTOR-LIFECYCLE`, whose discharge condition — a kind-indexed
  vocabulary carried into the gateway with Lean parity, then per-callee
  declarations — is met; its nine undeclared callees are rehomed by name.
* **Amended** ([amended-followups.json](amended-followups.json) keeps the previous
  texts): `CT-CTWIN-O` records `CT-CTWIN-DESCRIPTOR` discharged (the umbrella keeps
  its other residuals; CT42 stays unmapped); `EXTERN-CALL-F` records the sentinel
  half as it applies to the resource loader, and that the extern-call model still
  refuses kinded declarations, so `EXTERN-MODES-SENTINEL` stays open there.
* **Registered** ([followups.json](followups.json)):
  `W43-DESCRIPTOR-W4-SUMMARIES`, `W43-DESCRIPTOR-REFUSED-SHAPES`,
  `RETURN-CAST-PLACE-NOT-ESCAPED`,
  `W43-DESCRIPTOR-ACCUSING-ARMS-UNEXERCISED-IN-CORPUS`,
  `W43-DESCRIPTOR-INDIRECT-ACCESS-WIRING`.
* Follow-ups **299 → 303** on the card branch before the merge; **313 → 317** on the
  combined tree. `corpus.cycle19_raw_units` points at this report's
  artifacts; its previous contents are in [previous-corpus-row.json](previous-corpus-row.json).
  `docs/STATUS.md`'s corpus, contract-corpus, unit-reason and follow-up rows are
  updated from this arbiter.

## 9. For the integrator

* **Constraint 7:** the leak pin holds at 271 (combined tree) through one accusing addition on
  the first integer-carried identity and one withdrawal-by-refusal; both leaf
  coordinates are above.
* **Constraint 5g / W43-INDIRECT-ACCESS-GENERAL:** §2.3. Wire the indirect rule's
  value test to `descriptor_custody::value_carries_descriptor` (or confirm its
  instruction passes the classification guard) and keep
  `a_tracked_descriptor_on_every_scalar_store_route_is_refused` green on the
  combined tree.
* **Constraint 10:** the declared-scalar source exemption now excludes a scalar
  bound to an integer-carried identity.
* **Files likely to conflict:**
  * with main and CONTRACT-VIEW-PROJECTION-CHECK / the libc cards:
    `providers/quirin-provider-libc/contracts/libc/{stdio,socket,unistd}.qcore`
    (appends; W43-LIBC-PRINTF-FAMILY appends to `stdio.qcore`,
    W43-LIBC-RESTRICT-PROTOTYPES to `socket.qcore`), `providers/quirin-provider-libc/src/lib.rs`,
    `cycle19-resource-contracts.json`, `providers/contract-load-path.json`
    (regenerate with `sync`), `compiler/tests/qcore_contract_corpus.rs`,
    `compiler/tests/contract_loader.rs` (counts add up across cards),
    `scripts/tests/test_contract_corpus.py`;
  * with W43-INDIRECT-ACCESS-GENERAL and W43-RESULT-ALIAS-RETURNED:
    `crates/qcore-c-gateway/src/legacy_c.rs` (`LegacyInstruction`, `classify_simple`,
    `transfer`, the edge loop, `apply_contract_call`), `legacy_summary.rs`,
    `legacy_joined_provenance.rs`, `qcore-audit/src/report.rs`;
  * `docs/reviews/r0_3/acceptance_matrix.json`, `docs/STATUS.md`,
    `docs/development/increments/wave-43.md`, `docs/reports/INDEX.md`.
* The `contracts` artifact embeds this worktree's path in its source keys, as
  earlier isolated cards' artifacts did.

## 10. Remaining work, by measured leverage

1. **Kinded W4 summaries** (`W43-DESCRIPTOR-W4-SUMMARIES`): the socket/open
   callbacks and `mk_socket_create` certify or nearly certify on their own bodies
   and publish nothing to their callers.
2. **The cast return** (`RETURN-CAST-PLACE-NOT-ESCAPED`): three baseline leaks and
   the `dlopen`/`dlclose` pair.
3. **`fcntl` per command** and **the `getaddrinfo` status witness**
   (`W43-DESCRIPTOR-REFUSED-SHAPES`): `rd_kafka_open_cb_generic` is blocked by
   `fcntl` alone.

## Reproduction

```sh
python3 docs/reports/w43-descriptor-resource-tracking/scripts/descriptor_inventory.py docs/reports/w43-indirect-access-general/artifacts/diagnostics.json.gz inventory.json
python3 docs/reports/w43-descriptor-resource-tracking/scripts/source_call_sites.py docs/reports/w43-libc-result-alias/artifacts/diagnostics.json.gz <fluent-bit-clang-source>
git archive 0a07cb57 | tar -x -C measure/main-src    # plus a symlink to third_party/libevent
(cd measure/main-src && cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit \
  && python3 docs/reports/w43-return-expression/scripts/run_audit.py ../before-main 0a07cb57)
python3 docs/reports/w43-descriptor-resource-tracking/scripts/identity.py measure/before-main docs/reports/w43-indirect-access-general/artifacts before-identity.json agent-a8d24f7f717a987c1
# CLASS-A: this tree's binary with main's unistd/socket/stdio modules and manifest copied in, then restored
python3 docs/reports/w43-return-expression/scripts/run_audit.py measure/class-a3 06ee9fe4+main-contract-data
python3 docs/reports/w43-descriptor-resource-tracking/scripts/path_alias_diff.py measure/before-main measure/class-a3 $PWD/measure/main-src $PWD
python3 docs/reports/w43-return-expression/scripts/run_audit.py measure/after3 06ee9fe4
docs/reports/w43-descriptor-resource-tracking/scripts/capture_run.sh measure/capture-run  # tee wrapper, pre-merge CLASS-A binary
python3 docs/reports/w43-descriptor-resource-tracking/scripts/strip.py
python3 docs/reports/w43-descriptor-resource-tracking/scripts/census.py docs/reports/w43-descriptor-resource-tracking/measurement/drafts.qcore abi-census.txt
python3 docs/reports/w43-libc-contracts/scripts/unit_texts.py measure/class-a/resource-diagnostics.json measure/capture/ppP measure/unit-texts
python3 docs/reports/w43-libc-contracts/scripts/attribute.py measure/class-a3 measure/after3 measure/unit-texts attr.json docs/reports/w43-descriptor-resource-tracking/measurement/shipped-names.txt
python3 docs/reports/w43-descriptor-resource-tracking/scripts/classes.py measure/before-main measure/class-a3 measure/after3 attr.json classes.json
python3 docs/reports/w43-descriptor-resource-tracking/scripts/totals.py measure/before-main measure/class-a3 measure/after3
python3 docs/reports/w43-descriptor-resource-tracking/scripts/cast_return_leaks.py measure/after3/resource-diagnostics.json measure/unit-texts
python3 docs/reports/w43-libc-contracts/scripts/archive.py measure/after3 docs/reports/w43-descriptor-resource-tracking/artifacts
python3 docs/reports/w43-descriptor-resource-tracking/scripts/update_ledger.py
```

`measurement/drafts.qcore` is the measured draft set, including the withdrawn
`dlopen`, `dlclose`, `fts_open` and `fts_close` entries; it loads only with the
census binary that measured it, because the `file_tree` kind its `fts` drafts
use was withdrawn afterwards. Private raw outputs are in this worktree's
`measure/` (pre-merge `before/`, `class-a/`, `capture-run/`, `capture/`, `after1/`,
`after2/`; combined `before-main/`, `class-a3/`, `after3/`; `unit-texts/`,
`gate/`, `provider-gate/`, and the aborted pre-merge `gate-aborted-premerge/`).
`measure/main-src` was deleted after its run so no stray `.qcore` copy reaches
the repository-wide census tests.
