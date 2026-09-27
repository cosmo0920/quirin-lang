# O04 — Validate one constructor-to-caller custody path — 2026-09-08

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Execution of the single bounded path card O00 recommended
([`docs/reports/remaining-o00/README.md`](../remaining-o00/README.md) §8):
**per-arm constructor-to-caller cleanup discharge on the early-return arms of
`flb_input_chunk_create`.**

The property now holds, measured on the real upstream bodies:

> On each of the caller's three modeled failure arms, the chunk identity
> produced at the `cio_chunk_open` call is released **exactly once**, by the
> `cio_chunk_close` on that arm, and no other tracked identity is left live.

The exits that cross an aggregate this projection cannot follow stay
**Unknown**, in both directions: no discharge is claimed and no residual is
alleged. Both functions keep function-level status **Unknown**.

**Nothing is promoted.** No raw Cycle 19 unit changes status, the canonical
CMetrics audit is byte-identical, CT coverage is untouched, the contract
corpus stays at 59 modules, and `qcore-schemas` is unchanged.

## 1. What was measured

Byte-verbatim upstream regions, digest-pinned in
[`provenance.json`](../../../crates/qcore-c-gateway/tests/fixtures/o04/provenance.json)
and asserted by `the_extracted_regions_are_the_upstream_bytes`, at Fluent Bit
`90a010910e5b3328f496cc20fadc937547133439`:

| Region | Upstream | Lines | Bytes |
|---|---|---|---|
| `cio_chunk_open` | `lib/chunkio/src/cio_chunk.c` | 30–109 | 1,931 |
| `flb_input_chunk_create` | `src/flb_input_chunk.c` | 2506–2621 | 3,590 |
| `struct flb_storage_input` | `include/fluent-bit/flb_storage.h` | 57–61 | 153 |
| `struct flb_input_instance` | `include/fluent-bit/flb_input.h` | 244–554 | 11,545 |

Raw ledger: [`logs/exit_ledger.txt`](logs/exit_ledger.txt), regenerable with the
`print_the_measured_exit_ledger` reporting aid.

### 1.1 Caller `flb_input_chunk_create` — six exits

| O00 exit | Upstream line | Arm | Measured | Residual |
|---|---|---|---|---|
| C1 | 2530 | constructor returned NULL | **Unknown** | none acquired |
| C2 | 2541 | `up_force == -1` | **Discharged** | chunk **Released** |
| C3 | 2550 | header write `== -1` | **Discharged** | chunk **Released** |
| C4 | 2558 | `ic` calloc NULL | **Discharged** | chunk **Released** |
| C5 | 2587 | `routes_mask` calloc NULL | **Unknown** | chunk + `ic`, both unresolved |
| C6 | 2620 | success + publication | **Unknown** | chunk + `ic`, both unresolved |

All three discharged arms name the **same** acquisition site — the constructor
call at upstream 2525 — which is what makes this a constructor-to-caller
property rather than three unrelated observations.

C1, C5 and C6 stay Unknown for three separately named reasons:

* **C1** — `flb_error(…, storage->stream->name, name)` is an *uncontracted*
  callee whose actual is a **chained** pointer-field read. Chained accesses are
  refused by construction (§2 premise 3), so this arm is blocked by the
  admission's own boundary, not by an accident of contract coverage.
* **C5** — reached only through the `ic->…` aggregate stores and guarded by the
  aggregate branch condition `ic->routes_mask == NULL`. Branch conditions are
  out of scope for C02 (§7) and for this card.
* **C6** — O00 blockers B08/B09/B10: `ic->chunk = chunk`, the msgpack callback
  retaining `ic`, and one of four hash-table publications.

### 1.2 Constructor `cio_chunk_open` — six exits

| O00 exit | Upstream line | Measured | Residual |
|---|---|---|---|
| E1 | 40 (`!st`) | **Discharged** | none acquired |
| E2 | 45 (`!name`) | **Discharged** | none acquired |
| E3 | 51 (`strlen(name) == 0`) | **Discharged** | none acquired |
| E4 | 64 (`!ch`) | **Discharged** | none acquired |
| E5 | 92 (partial-failure rollback) | **Unknown** | chunk object unresolved |
| E6 | 108 (success) | **Unknown** | chunk object unresolved |

E3b (upstream 56) is compiled out, matching the pinned build, as O00 §2.1
recorded.

**This is the honest half of the result.** E5 and E6 are exactly the exits the
caller's declared contract talks about, and the constructor's own body does
**not** discharge that contract: `mk_list_add(&ch->_head, &st->chunks)`
publishes the chunk before it is returned (O00 B02/B03), `ch->name = strdup(…)`
and the other member stores are indirect, and both backend constructors are
uncontracted. The declared contract is therefore a **named premise**, not a
derived fact — see §3.

## 2. The capability added

### 2.1 C02-P — typed borrowed pointer-field actuals

`crates/qcore-c-gateway/src/borrowed_actual.rs`. C02 admitted direct *scalar*
field accesses and explicitly left pointer fields refused
([C02 §7](../remaining-c02/README.md)); O00 §7 blocker **B01** measured the
consequence — the constructor call statement was rejected by the lexical gate
*before* the declared contract was ever consulted, so `chunk` never bound and
every arm downstream reported an unresolved alias.

Two **read** shapes are now admitted, under a conjunctive whitelist:

1. a **borrowed actual** — a whole argument of a contracted call that is exactly
   `base -> field` or `base . field`, at an index the contract in force declares
   `Borrowed`;
2. an **opaque field read** — an assignment whose entire right-hand side is one
   such access and whose destination is a plain local name.

| # | Premise | Refused otherwise |
|---|---|---|
| 1 | `base` is a direct identifier whose declared type is a single-star pointer to the record (`->`) or the record itself (`.`) | computed or chained bases |
| 2 | the record is a `struct`, defined exactly once, every member declaration read; the member is declared with no array modifier, no `volatile`/`_Atomic`, and is a scalar or a pointer | union, ambiguous tag, incomplete record, aggregate member |
| 3 | the access does not continue into `->`, `.`, `[` or `(` | `a->b->c`, `a->b[i]` |
| 4 | the base carries no tracked identity (`untracked_bases`) | see below |
| 5 | the actual occupies a declared `Borrowed` position; a variadic tail never qualifies | consuming positions, uncontracted callees |

**The destination side of the lexical gate is untouched.** A store through
`->`, `.` or `[` keeps its existing rejection and its existing diagnostic —
which is what makes premise 4 hold by construction, because an admitted place
can then never be bound.

**Premise 4** is C02 §2's premise 8, *strengthened*: a borrowed actual survives
into the modeled call rather than disappearing into a no-op, so the base must
be shown to carry no tracked identity. `untracked_bases` is a monotone
syntactic over-approximation — a name is refused as soon as it is address-taken
whole, incremented, compound-assigned, passed at a non-`Borrowed` position of a
contracted call, passed at *any* position of an **uncontracted** call, or
assigned from anything but an admitted field read or a call-free, `&`-free
expression over names that already survive. Refusing too many names only
narrows the whitelist. This is what refuses `ic->chunk = chunk` (`ic` is bound
by `flb_calloc`) and admits `storage->cio` (`storage` is bound only by an
admitted field read).

`LegacyInstruction::OpaqueValue` binds exactly the unknown, possibly-null
pointer an alias from an unbound place would bind — so every downstream use
stays fail-closed — without the unresolved-alias diagnostic the typed premises
rule out.

### 2.2 Two smaller typed admissions the path needs

* **Constant array declarators.** A plain object declaration with constant array
  bounds and no initializer (`char name[64];`) allocates automatic storage and
  evaluates nothing. O00 §8 feature P2.
* **Scalar-only statements.** A statement over declared scalars, enumeration
  constants and numeric literals alone is custody-neutral, because this
  projection tracks pointer identities and a declared scalar is never bound to
  one. `is_scalar_initializer` covers the declaration form `int flag = FLAG_OFF;`.

  This rule is a **fallback**, consulted only where the importer would otherwise
  report an unresolved alias or refuse the statement. That ordering is not
  cosmetic: an earlier version placed it first and silently destroyed a pinned
  proof — `contextual_recursion::ordinary_proof_of_unreachable_recursion_is_preserved`
  depends on `flag = 0;` binding a **definitely-null** place so the recursive
  branch is refuted. The fallback ordering preserves that value fact and every
  variable-to-variable alias, and the pinned test passes unchanged.

### 2.3 The per-exit residual ledger

`LegacyExitResidual` / `ExitCustody` in `legacy_c.rs`, O00 §8 feature P4. Each
exit records whether every path reaching it is fully modeled, and each acquired
identity's disposition. `AbstractState` gains one `unmodeled` flag, set when a
node's own transfer emits a refusal or a custody defect, joined by disjunction.

* **Discharged** — no path reaching the exit carried anything unmodeled, no
  disposition joined, and no identity is live.
* **Residual** — fully modeled, and at least one identity is still live.
* **Unknown** — some path carried a refusal, an unresolved effect, a widened
  alias set or a joined disposition.

It is an **additional observation and never an input to a status**: no verdict,
diagnostic code or classification rule changed, and a discharged exit cannot
promote an imprecise function (Lean:
`a_discharged_exit_does_not_promote_an_imprecise_program`).

## 3. Every entry premise, named

Acceptance item 1 is "derive or name every entry premise". Nine premises are
**named, not derived**; they live in
[`o04_path.qv`](../../../crates/qcore-c-gateway/tests/fixtures/contracts/o04_path.qv),
one clause per declaration, deliberately **outside** `contract/qcore` so the
pinned corpus stays at 59 modules and nothing here is offered as a production
declaration.

| Premise | O00 row | Status |
|---|---|---|
| `cio_chunk_open` returns owned nullable custody, borrowing all else | R01 | **assumed** — E5/E6 do not discharge it (§1.2) |
| `cio_chunk_close` releases a non-null chunk | R04 | **assumed** |
| `chunk_is_live` / `chunk_is_not_busy` / `chunk_has_no_task_or_route_custody` | R07 | **not in force** — `chunk_destructor.qcore` carries no `resource_contract_v1` marker, so the loader rejects it, exactly as O00 §4.1 measured |
| `cio_chunk_is_up`, `cio_chunk_up_force`, `cio_chunk_down` borrow the chunk | — | **assumed** (O00 P3) |
| `generate_chunk_name`, `input_chunk_write_header`, `flb_routes_mask_set_by_tag` borrow their arguments | — | **assumed** (O00 P3) |
| `flb_calloc` returns owned nullable, `flb_free` releases non-null | R29 | **assumed** |
| `flb_errno` has no argument and no custody | — | **assumed** |
| `malloc`/`free`/`strdup`/`strlen` and the `cio_*` logging and counter helpers | R02, R05, R06 | **assumed** |
| `&base->member` does not reassign or release `base` | — | **assumed**, recorded in the `borrowed_actual` module header |

Two further premises are inherited unchanged from C02 and are **not** proved:
declaration completeness, and C effective types.

**No liveness is inferred from a success-looking integer or field.** `ret`,
`err`, `set_down` and `has_routes` carry no custody meaning at all; the chunk
stays live across every one of them and each arm must release it explicitly.
This is pinned in the *defect* direction by
`a_success_looking_integer_does_not_discharge_the_identity`, where an arm that
returns on a `== 0` "success" without releasing is reported as a **Residual**
with a `Leak`, not as a discharge.

## 4. Evidence

### 4.1 Held-out generic family

`crates/qcore-c-gateway/tests/o04_constructor_caller.rs`, a `g_*` API
deliberately unrelated to ChunkIO and Fluent Bit. Every rule is stated over
declared C types and contract modes, so no name in it can matter.

| Test | Result |
|---|---|
| `a_generic_constructor_caller_path_discharges_every_exit` | every exit **Discharged**: two arms Released, the success exit Escaped |
| `the_callee_summary_is_validated_and_independently_rederived` | the wrapper's W4 summary is published `InferredValidated` and re-derived by `check_legacy_safety_certificate` |
| `a_leak_on_one_arm_is_reported_as_a_residual` | **Unsafe**, `Leak`, and exactly one exit is a **Residual** with a Live identity |
| `an_invalid_transfer_is_reported_and_never_discharges` | **Unsafe**, `ReleaseAfterEscape`; the arms the transfer reaches drop to **Unknown**, the arms before it keep their discharge |
| `a_success_looking_integer_does_not_discharge_the_identity` | **Unsafe**, one **Residual** |
| `ambiguous_actual_shapes_keep_their_unknown` | five shapes — chained access, union base, consuming position, actual beyond the declared vector, bound base — each keeps **Unknown** *and* its existing `indirect memory access is outside the resource projection` diagnostic |
| `a_certificate_resting_on_the_admission_is_rederived_from_source` | four declaration substitutions (union, `volatile`, array member, unreadable function-pointer member) each refused; the unmodified certificate still rechecks at the end, so the refusals are caused by the substitutions |

### 4.2 Real-source recheck

| Test | Result |
|---|---|
| `the_extracted_regions_are_the_upstream_bytes` | four SHA-256 pins, no Clang needed |
| `the_upstream_record_declarations_are_readable_and_declare_the_read_members` | the **full** 239-member upstream `struct flb_input_instance` and the upstream `struct flb_storage_input` are fed through the pure-token entry point and admit `in->storage`, `storage->cio`, `storage->stream` |
| `substituting_a_dependency_identity_withdraws_the_discharged_arms` | four substitutions on the real fixture — an unreadable member, `volatile`, `_Atomic`, and passing the base at a consuming position — each drops all three arms to zero discharged |
| `a_borrowed_destructor_turns_every_cleanup_arm_into_a_residual` | weakening `cio_chunk_close` to `Borrowed` turns all three arms into **Residual** with a `Leak`, never a silent discharge |

The duplicate-tag substitution C02 pins at module level is not expressible here:
two definitions of one tag are invalid C and Clang rejects the unit before the
analyzer sees it. It remains covered by C02's own reader tests and by
`an_ambiguous_tag_never_admits_an_actual` in Lean.

### 4.3 The prelude concession, and why it is not a premise

`prelude.c` is **not** upstream. It is the smallest set of declarations that
makes the two verbatim bodies a translation unit Clang accepts, and it
**reduces** `struct flb_input_instance` and the two chunkio records to the
members the bodies mention, because the full declarations transitively require
roughly fifty other complete type definitions. The `#define`d constants are
modeled as enumeration constants; the admitting rule is insensitive to the
choice, since in the real preprocessed unit they are integer literals that the
same walker admits as numeric literals.

The reduction would be a hidden premise if the full declaration were *not*
readable — a single unreadable member refuses the whole record under C02's
all-or-nothing rule. That is why it is measured directly against the upstream
bytes rather than assumed (§4.2, row 2).

## 5. Verdict deltas

### 5.1 Canonical CMetrics audit — no change at all

Measured on this checkout, same generated configuration, once at the pre-O04
commit `846ffc6` in a separate worktree and once after:

```
cmetrics/lib/cmetrics/src/cmt_counter.c: Unsafe (7/7)
  resource diagnostics: leak=2, unknown_alias=2, unsupported=35
1 units: 0 Safe / 1 provisional Unsafe / 0 Unknown / 0 data-only / 0 gateway failures
```

[`cmetrics-baseline.json`](cmetrics-baseline.json) and
[`cmetrics-after.json`](cmetrics-after.json) are **byte-identical**. The raw
unit stays **provisional Unsafe**, unpromoted, and there is no per-function
delta to explain.

The `source_contracts` block (C02's 1 checked function / 6 Unknown) is absent
from **both** runs: `prepare_cmetrics_clang_audit.py` emits a
`source_revision_policy: checkout_head` configuration at a moved revision, and
that mode does not carry source-contract selections. Running the pinned
`contract/profiles/cmetrics-counter/audit.json` instead is refused by the pin
gate, measured:

```
audit rejected: pin `fluent-bit`: expected 66910c10a4d7eafa810b84229cf2dcf7b0f26f97,
                found    90a010910e5b3328f496cc20fadc937547133439
```

So the checked/Unknown split is **not re-measured** here and is **not claimed to
have changed**; it is the same pre-existing limitation C02 §6 recorded.

### 5.2 The pinned 185-unit corpus — not re-measured

This card touches shared analysis code, so the four archived artifacts *would*
change and cannot be re-measured in this checkout: the developer's Fluent Bit
checkout has moved off `66910c10…` and the pin gate correctly refuses to run.
The archived artifacts are **left untouched**. Re-measuring and deliberately
re-pinning them remains the open **CORPUS-REPIN** follow-up, now carrying C02,
G02 and O04.

### 5.3 One pinned expectation updated, deliberately

`c03_neutral_frame_recursion_probe::pointer_field_recursion_is_refused_before_the_recursion_rule_applies`
named the statement-level indirect-access gate as the blocker for
`seek(n->next, key)` and cited **C02-P** as the missing capability. This card
implements C02-P, so that call statement is now a modeled borrowed actual. The
load-bearing assertion is unchanged — `seek` stays **Unknown**, refused before
any recursion rule applies — but the gate that refuses it moved to the branch
condition `if (n->key == key)`, which both C02 (§7) and this card leave out of
scope. The expectation was updated with that reasoning recorded in the test, not
dropped or weakened.

No other pinned expectation changed. No diagnostic code was added, removed or
downgraded, no gate was weakened, and no `Unknown` became `Safe` without a
supplied typed premise covering the reason it was Unknown.

## 6. Lean parity

`proof/QCore/ExitResiduals.lean`, **37 theorems**, printed in `AuditW4.lean` and
checked by `check_lean_audits.py` (21 audits → **22**). No new axiom beyond
`propext`, no placeholder, no `sorry`.

* **The admission** — one refusal theorem per clause
  (`a_partial_argument_never_admits`, `a_non_borrowed_position_never_admits`,
  `an_actual_beyond_the_declared_vector_never_admits`,
  `an_uncontracted_callee_never_admits`, `a_union_never_admits_an_actual`,
  `an_incomplete_record_never_admits_an_actual`,
  `an_ambiguous_tag_never_admits_an_actual`,
  `an_aggregate_member_never_admits_an_actual`,
  `a_qualified_member_never_admits_an_actual`,
  `a_mismatched_base_never_admits_an_actual`,
  `a_bound_base_never_admits_an_actual`,
  `a_chained_access_never_admits_an_actual`,
  `unread_declarations_never_admit_an_actual`), plus two non-vacuity theorems —
  one for a pointer member and one for a scalar member, so the rule is shown to
  be a strict extension of C02 rather than a replacement — and
  `a_remaining_indirect_access_still_refuses`, which pins that the admission
  removes only the spans it justified.
* **The opaque read** — `an_admitted_field_read_does_not_discharge_a_live_identity`
  and `an_admitted_field_read_preserves_a_discharged_lifetime`: it can neither
  hide a discharge nor manufacture one.
* **The residual ledger** — `discharged_exit_is_sound`,
  `residual_implies_a_live_identity`, `a_joined_exit_is_discharged_only_if_both_paths_are`,
  and both fail-closed directions:
  `an_unmodeled_path_is_never_discharged` and
  `an_unmodeled_path_alleges_no_residual` (a refused construct is a reason to say
  nothing, never a reason to accuse).
* **Standing** — `a_discharged_exit_does_not_promote_an_imprecise_program`.

The model is of the **decisions**, not of C semantics. Declared types, record
facts, parameter modes and path flags are premises the reader and the transfer
function supply; nothing proves either extracts them faithfully, and nothing
here is a claim about arbitrary C memory safety.

## 7. Open blockers

O00's blockers, after this card:

| # | Blocker | Status |
|---|---|---|
| B01 | constructor call rejected before the contract is consulted | **repaired** (§2.1) |
| B02 | constructor publication into `st->chunks` unmodeled, in tension with the declared owned result | **open** — keeps E5/E6 and the caller's C6 Unknown |
| B03 | `mk_list_add` has no declaration in `contract/qcore` | **open** |
| B04 | `ch->backend` published before the child's failure arms | **open**, untouched |
| B05 | a successful construction may yield no descriptor and no mapping | **open** — ID-D/ID-M were out of scope by O00 §8 |
| B06 | `CIO_RETRY`/`CIO_CORRUPTED` reach the success path unenumerated | **open** — a source fact, not a defect claim |
| B07 | the success exit's `cio_chunk_down` releases unmodeled identities | **open** |
| B08 | `ic->chunk = chunk` pointer-field custody transfer | **open** — the admission covers reads only |
| B09 | `msgpack_packer_init(…, ic, …)` retains `ic` | **open** |
| B10 | `flb_hash_table_add(…, ic, 0)` under four event types | **open** |
| B11 | the lifecycle obligation inventory is derived over QCore source, not this C | **open**, architectural |

New boundaries this card records:

* **Branch conditions** remain outside every typed rule, and they are now the
  binding constraint on C5. C02 §7 already named this as its own increment.
* **Chained accesses** are refused by construction, which is what keeps C1
  Unknown. Admitting `a->b->c` needs a model of the intermediate aggregate, not
  a wider whitelist.
* **`untracked_bases` is flow-insensitive.** A name refused anywhere in the body
  is refused everywhere in it. Sound, and coarse.
* **The precision warning of O00 §8 stands unresolved.** Lifting the gate on the
  constructor call also unmasks `cio_scan_stream_files`, which would report a
  *discarded owned result* and stay Unknown. Resolving it properly needs B02.
  This card does not audit that unit; the corpus re-measurement follow-up must.

## 8. Scope and non-claims

- **No raw unit is promoted.** Both functions on the path keep status
  **Unknown**; the CMetrics unit remains **provisional Unsafe**, unpromoted.
- A per-exit **Discharged** is a claim about *this projection under the named
  premises of §3*, not a proof that the C is correct. Every declared contract it
  rests on is a trusted assumption, and the constructor's own body does not
  discharge its contract.
- A provisional `Unsafe` in the generic family is not an upstream defect claim;
  the `g_*` API is synthetic.
- **Unknown means Unknown.** An exit whose path carried a refusal alleges
  nothing in either direction.
- Projection coverage is not safety coverage; unknowns stay in the denominator.
- CT coverage is unchanged; the contract corpus remains 59 modules;
  `qcore-schemas` is unchanged; no plugin, protocol or evidence version moved.

## 9. Files changed

| Path | Change |
|---|---|
| `crates/qcore-c-gateway/src/borrowed_actual.rs` | new — the C02-P whitelist and the untracked-base closure |
| `crates/qcore-c-gateway/src/typed_field_access.rs` | borrowable (pointer) members, the shared walker, `is_scalar_only_noop`, `is_scalar_initializer` |
| `crates/qcore-c-gateway/src/legacy_c.rs` | `OpaqueValue`, the borrowed-actual gate mask, constant array declarators, the scalar fallback, `LegacyExitResidual`/`ExitCustody`/`SiteDisposition`, `AbstractState::unmodeled` |
| `crates/qcore-c-gateway/src/lib.rs` | declare the private module |
| `crates/qcore-c-gateway/tests/o04_constructor_caller.rs` | new — 14 tests plus one reporting aid |
| `crates/qcore-c-gateway/tests/fixtures/o04/` | new — four verbatim upstream regions, `provenance.json`, `prelude.c` |
| `crates/qcore-c-gateway/tests/fixtures/contracts/o04_{path,generic}.qcore` | new — the named premises and the held-out API |
| `crates/qcore-c-gateway/tests/c03_neutral_frame_recursion_probe.rs` | one pinned expectation updated deliberately (§5.3) |
| `proof/QCore/ExitResiduals.lean`, `proof/QCore.lean`, `proof/AuditW4.lean` | new module and its axiom audit |
| `docs/reports/remaining-o04/` | this record, the two CMetrics reports, the exit ledger |

## 10. Gate results

Run in a **clean temporary worktree** at the branch head `44d4528`, with the
Libevent submodule initialized, because this card shares its worktree with the
CT26–CT30 harness-integration lane and a gate result must not be mixed with
another lane's uncommitted edits.

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace` | **1,125 passed / 0 failed / 1 ignored** | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | ran | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | ran | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | **43 ran, OK** | 0 |
| Lean build | `cd proof && lake build` | 67 jobs, completed | 0 |
| Lean audits | `python3 -B scripts/check_lean_audits.py` | **23 audits passed** | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | **51 passed / 0 failed** | 0 |
| Provider fmt | `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | clean | 0 |
| Provider clippy | `cargo clippy --locked --offline --manifest-path providers/Cargo.toml --all-targets -- -D warnings` | clean | 0 |
| CMetrics audit | `qcore-audit <generated cmetrics config>` | byte-identical to the pre-O04 baseline (§5.1) | 0 |
| Pinned 185-unit corpus | `cycle19_fluent_bit_libraries_audit --pinned-evidence` | **refused: pin gate** (§5.2) | — |

The card contributes 15 tests: 14 assertions plus one `#[ignore]`d reporting
aid that regenerates [`logs/exit_ledger.txt`](logs/exit_ledger.txt). The 23 Lean
audits include one contributed by the CT26–CT30 lane already on this branch;
this card adds `AuditW4`'s 37 new `QCore.ExitResiduals` theorems to an existing
audit rather than a new audit file.
