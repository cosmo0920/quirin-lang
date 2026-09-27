# Unchecked linkage: `source.extern-call.unsafe.v1`

**Card:** `UNSAFE-LINKAGE` (wave 18). Design document, written before the
implementation and revised against it; the deviations found by measurement are
marked **[measured]** in place and reconciled in
[`docs/reports/wave18-unsafe-linkage/README.md`](../reports/wave18-unsafe-linkage/README.md).

## 0. The problem

`source.extern-call.v1` admits a call to a declared extern function **only**
against a trusted C04 contract that the resource-contract v1 loader admitted and
that an explicit `CompileOptions::extern_contracts` selection supplied. A callee
with no contract refuses at `Q023`, permanently.

That is the right default and it is not a realistic linking story. Real programs
link libraries nobody has written a contract for. The choice today is: write a
contract for every symbol you touch, or do not link. This card adds the third
option — **link it, and say so** — under one rule:

> The escape hatch never buys silence. Every result a program obtains through
> an uncontracted callee carries that callee, by name and by declared
> signature, as a printed premise.

This is not a relaxation of the verdict contract (hard invariant 2). It is the
verdict contract applied: `Safe` under a named assumption, with the assumption
in the certificate, is exactly what a declared trust boundary is supposed to
produce. What would be a violation is a `Safe` that stops mentioning it.

## 1. What the default is, and that it does not move

Without the opt-in surface of §2, every byte of behavior is what it is today:

* an extern call whose callee has no admitted contract refuses at `Q023`;
* an extern call in a file without `#import std.extern;` refuses at `Q022`;
* every existing fixture, contract module and diagnostic cascade is unchanged.

The new surface is **per declaration**. There is no compiler mode, no global
switch, no environment variable and no CompileOptions field that turns checking
off. A file that does not spell the opt-in cannot reach it, and a file that does
spell it opts in for exactly the declarations it marks.

**[measured] The builder-side alternative was rejected.** A
`CompileOptions::unchecked_extern: BTreeSet<String>` mirroring `extern_contracts`
would need no parser change at all, which is cheaper. It was rejected because it
makes the trust invisible where it matters: two source files, byte-identical,
would mean different things depending on how the builder invoked the compiler,
and a reader of the source could not tell which. The user's requirement is an
*explicit, recorded trust declaration*; a declaration that is not in the
declaration is not one.

## 1.1. Where this sits: the linkage spectrum

This card is not a standalone feature. It is the **uncontracted end of one
spectrum**, whose contracted end is designed in
[`NATIVE_LIBRARY_LINKAGE_V1.md`](NATIVE_LIBRARY_LINKAGE_V1.md)
(`source.native-library.v1`, adopted into this tree by this card — see §9). Read
the two together: linking is a single question asked at four decreasing levels
of knowledge, and each level carries a different trust artifact into the
certificate.

| Level | What the compiler knows | Selection | Trust artifact carried into the certificate | Callable surface | Status |
|---|---|---|---|---|---|
| **L3** Manifest-bound contracted | contract slice + exported header + artifact identity + ABI premises | `#import std.buffer;` — one name binds all four | artifact identity triple, the ABI premise fields, and each contract | every mode `source.extern-call.v1` calls, pointers included | **designed** (`NATIVE_LIBRARY_LINKAGE_V1.md`) |
| **L2** Declared contract | a C04 contract, from an ad-hoc selection | `CompileOptions::extern_contracts` | `DeclaredContract(name)` per callee, plus `NoExtentTransport`, `LinkageSuppliedByBuilder` | as L3 | **implemented** (`source.extern-call.v1` + `EXTERN-F1`/`-ERR-REL`/`-MODES`) |
| **L1** Manifest-resident unsafe | artifact identity + ABI premises + header types — **no modes, no effects** | a contract-less entry inside an L3 manifest | artifact identity, ABI premises, and `UncheckedLinkage` naming *which build* was waived | scalars only | **designed** (§4.2 below), follow-on `UNSAFE-LINKAGE-ARTIFACT` |
| **L0** Standalone unsafe | a declared ABI signature and nothing else | `unsafe extern fn` + `#import std.extern.unsafe;` | `UncheckedLinkage { callee, signature }` — and, by its silence about an artifact, the statement that **no build was named** | scalars only | **implemented — this card** |

Three properties of the table are the point of drawing it.

**The trust artifact shrinks monotonically as knowledge shrinks, and it is
always non-empty.** There is no level at which linking is free. L0 carries the
least, and what it carries is still a printed sentence naming the symbol and its
ABI.

**Pointers cross exactly where a contract exists to state their premise.** L3 and
L2 admit pointer modes because a C04 contract is the assumption text §3.1 says a
pointer needs; L1 and L0 refuse every pointer because they have none. This is one
rule, not two policies — and it is why the two documents' apparently opposite
tables do not conflict (§9).

**The ABI type gate never relaxes.** L3 checks the declaration against the
library's exported header, with `LOADER-ABI`-style per-unit withdrawal on
disagreement; L2 checks it against the selected contract module's own
declaration; L1 would check it against the manifest's header; L0 checks it
against the closed §C04.1 vocabulary and refuses a name a visible contract
environment already declares (§5). What is waived along the spectrum is the
**contract** — modes and effects — never the **ABI**.

## 2. The opt-in surface

Two independent gates, both required.

### 2.1 The module

```qcore
#import std.extern;          // calling a declared extern function at all
#import std.extern.unsafe;   // declaring one with no contract
```

`std.extern.unsafe` joins the closed bundled-module registry
(`qcore_core::module_imports`). It grants **only** the right to *declare* an
uncontracted callee; the call itself still needs `std.extern`, so an
unchecked-linkage module names both. Neither module implies the other, and
`std.extern` alone keeps refusing an uncontracted callee at `Q023` exactly as
today.

### 2.2 The declaration

```qcore
unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;
```

`unsafe extern fn` is the trust declaration. It says three things at once, in
the place a reader looks: this symbol is linked, this is its ABI signature, and
**nothing about its behavior is checked**.

**[measured] `unsafe` is already a reserved source word.** It is in
`is_reserved_source_word` and already introduces the `unsafe { … }` statement
form (`StatementKind::Unsafe`), so this card mints **no new reserved word** and
carries no golden-surface collision to resolve. What it does need is one line in
`is_declaration_start`, so an `unsafe extern fn` declaration may follow a
function body; that widening is pinned by a test that an `unsafe { … }`
statement inside a body still parses as it did.

The declaration keeps `DeclarationKind::ExternFunction`. Its unchecked-ness is
read from its own leading token, so no elaboration, name-resolution or typed-AST
path changes shape.

## 3. What may cross the boundary — and why nothing else may

The callee's effects are unknown. Not "declared and untrusted" — **unknown**.
The v1 admitted surface is therefore the largest surface over which "unknown
callee" is still a decidable statement about the caller's ledger:

| Position | Admitted | Rule |
|---|---|---|
| Parameter | a §C04.1 **scalar** ABI type, passed by value | `bool`, `i8`…`i64`, `isize`, `u8`…`u64`, `usize` |
| Result | a §C04.1 **scalar** ABI type, or `unit` | data only; binds nothing |
| Parameter | *pointer of any kind* | **refused**, `Q033` |
| Result | *pointer of any kind* | **refused**, `Q033` |
| Parameter | `unit` | **refused**, `Q033` |
| Anything | a spelling outside the closed §C04.1 vocabulary | **refused**, `Q033` |

### 3.1 Why pointers are refused, including read-only ones

A pointer argument hands unknown code a capability, and **no assumption makes
that capability's use checkable**. Enumerate the failure modes of the weakest
form — a `borrow<Buffer>` lent as `ptr<const, u8>`:

1. **Extent.** C04 pointers are extentless `u8` projections; the profile
   transports no length. A contracted callee's over-read is a premise a
   declaration at least *states*. An uncontracted callee's over-read is not even
   stated. Recording "assume `k` reads only what it should" is recording
   nothing: there is no *should*.
2. **Retention after return.** The callee may keep the pointer. The caller's
   borrow ends at the call; the callee's does not. `resource_borrowed` is a
   *lexical* loan and the C04 contract is what makes it lexical — remove the
   contract and the loan has no end. This is O00's B03/R13 publication
   non-model: a pointer that escapes into a store nobody models is exactly the
   residual the repository refuses to reason about.
3. **`const` is not a guarantee.** `ptr<const, u8>` constrains the *declaration*
   this file wrote, not the callee's implementation, which may cast it away.
   Under a contract, `const` is a declared premise about the implementation;
   with no contract there is no such declaration to premise.
4. **Aliasing and threads.** The callee may publish the pointer to another
   thread. Nothing in the caller's model would see it.

Failure modes 1–4 all have the same shape: the *assumption text there is no
declaration to write*. An assumption that cannot be stated cannot be carried,
and this card's whole discipline is that trust is carried. So v1 **refuses every
pointer**, in every mode, in both directions. `own<Buffer>` (a permanent
custody transfer to a callee whose release behavior is unknown), `resource_out`
slots, and every other C04 mode are refused a fortiori: they are pointer modes,
and each additionally needs a contract clause that does not exist here.

**This is a refusal, not an omission.** Each is a named constructor of
`UncheckedRefusal` with its own message and its own test row, so widening the
profile later means deleting a refusal on purpose.

### 3.2 The consequence for custody

The admitted surface carries **no custody at all**. Derived custody is the empty
vector, the derived caller-side flow is the zero flow, and the interprocedural
summary's parameter list is empty. An unchecked call cannot leak, cannot double
free and cannot invalidate an identity, because it is handed none. That is the
whole soundness argument for the v1 surface, and it is why the v1 surface is
scalars.

### 3.3 The derived effect row is maximal

A contracted callee derives `Io`, plus `Alloc` iff it acquires, plus `Mut` iff
some parameter is a mutable pointer. An **unchecked** callee derives the whole
modeled row — `{Io, Alloc, Mut}` — unconditionally, on the stated ground that
nothing is known about it: the row cannot honestly be narrower than everything
this profile can name. The practical effect is that an entry calling one must
declare `effects { io, alloc, mut }`, so the opt-in is visible
interprocedurally, in the entry's own signature, and not only at the
declaration.

## 4. Trust is carried, never laundered

Every certificate through this route carries, for each unchecked callee it
knows:

```
ExternAssumption::UncheckedLinkage { callee: "k_unchecked_add",
                                     signature: "(int32_t, int32_t) -> int32_t" }
```

**[measured]** the signature is rendered in the **C spellings** of the ABI
projection, not the QCore ones, because the premise is about what was linked.

The name and the declared signature together are the record: *this identity,
with this ABI, was linked and not checked.* It is derived inside
`check_extern_program` from the program alone, beside `DeclaredContract`, on the
same list, in the same sort order — the **existing** declared-assumption
channel, not a new parallel one. A contracted callee contributes
`DeclaredContract`; an unchecked one contributes `UncheckedLinkage`; the two are
the two arms of one total match on the callee's trust, so there is no third arm
that contributes nothing.

Three independent things then make omission impossible:

1. **Structural.** `ExternCallee` carries `trust`, so every callee lookup in the
   checker yields it. There is no representation of an unchecked callee that is
   not marked.
2. **Rederivation.** The driver publishes no certificate it did not rederive
   from the same program (`recheck_extern_certificate`, existing, `Q027`). An
   assumption list edited after the fact fails that recheck.
3. **A publication tripwire.** `certificate_carries_unchecked_linkage` is
   checked at the driver's publication boundary; a certificate missing a premise
   for a callee its program declares refuses at `Q035`. It is unreachable by
   construction, which is the point of asserting it.

And in Lean: `assumption_totality` — for every program and every unchecked
callee in it, the derived assumption list contains that callee's record.

### 4.1 Where a reader sees it

`CompiledModule::declared_assumptions()` returns the certificate's assumption
list for any compiled module, so a consumer that does not know the route still
sees the premises. `quirin check` prints them under an explicit
`under N declared assumption(s):` heading, so a `Safe` obtained through
unchecked linkage is never printed as a bare `Safe`.

### 4.2 The strongest form of the premise (L1), and what v1 records instead

The premise this card lands says *symbol X, unchecked, at ABI S*. That is not the
strongest honest version of it. The strongest version is:

> symbol X, unchecked, **from artifact Y at hash H, under ABI premises Z**

— and every one of those extra fields already has a design: they are
`NATIVE_LIBRARY_LINKAGE_V1.md` §4.3 (artifact identity: name, version, content
hash) and §4.4 (data model, no-unwinding, closed allocator custody, thread
posture). This section designs the unification; §10 stages it as
`UNSAFE-LINKAGE-ARTIFACT`.

**The design.** A library manifest may carry a **contract-less entry**: an entry
that names a symbol and binds it to the manifest's header declaration, with no
contract slice member and therefore no modes and no effects. Such an entry:

* **inherits** the manifest's artifact identity and ABI premise fields, so the
  waiver names which build it waived;
* **is still type-checked**, against the manifest's exported header, by the same
  §4.2-of-that-document header agreement with `LOADER-ABI`-style per-unit
  withdrawal — this is principle 4 at its strongest available setting, and it is
  a *stronger* check than L0's, which has only the closed ABI vocabulary to
  compare against;
* **keeps this card's admitted surface exactly**: scalars in and out, every
  pointer refused. A manifest cannot buy a pointer for a contract-less entry,
  because §3.1's argument does not turn on where the symbol came from — it turns
  on there being no modes to state. An artifact hash says *which code*, not
  *what it does with your pointer*;
* **must be marked contract-less in the manifest**, not inferred from the
  absence of a slice member. Total-or-reject means an unmatched symbol rejects
  the manifest (that document's §4.1); a contract-less entry is an explicit
  third kind of entry, so a *dropped* contract can never be silently read as a
  waiver;
* **satisfies that document's §4.4 closed-allocator-custody check vacuously**,
  because a contract-less entry cannot acquire.

**Landed (card `UNSAFE-LINKAGE-ARTIFACT`, wave 19).** All six bullets above are
implemented, with one deviation recorded in that card's report: the header
agreement is a **refusal** rather than a `LOADER-ABI`-style per-unit withdrawal,
because the only weaker admissible reading here is the bare L0 waiver, and
withdrawing to it would reward a contradiction with a premise that says *less*
about the artifact. The premise is a separate `ExternAssumption` variant rather
than an optional field on this card's, so the paragraph below stays literally
true. See
[`docs/reports/wave19-unsafe-linkage-artifact/README.md`](../reports/wave19-unsafe-linkage-artifact/README.md).

**Why v1 stopped at L0 anyway.** The manifest did not exist yet — its format,
loader, header reader, artifact verification and withdrawal semantics are stage
(a) of that document and are a card of their own. Landing L1 first would mean
landing the whole manifest to carry one field.

**What v1 records instead, and it is not nothing.** `UncheckedLinkage { callee,
signature }` carries no artifact field, and that absence is itself part of the
trust statement rather than an omission: **no build was named.** A reader of the
certificate can tell an L0 waiver from a future L1 one by exactly that — the L1
premise will carry the identity triple, the L0 premise will not, and neither can
be mistaken for the other. When `UNSAFE-LINKAGE-ARTIFACT` lands, an L0 premise
stays a valid, weaker statement rather than becoming a malformed L1 one.

## 5. Type honesty at the boundary stays

What is waived is the **contract** (modes and effects). The **ABI** is not
waived:

* the declared spellings must be in the closed §C04.1 vocabulary (§3, `Q033`);
* **[measured]** a *selected* contract environment that does not load refuses
  the module even when every callee is waived (`Q023`). Loading it leniently
  was the first implementation and it is wrong: the identity gate below would
  be silently skipped, so a deliberately broken contract module would be a way
  to hide that a callee has a contract;
* an `unsafe extern fn` that carries an inline `contract { … }` block refuses
  (`Q034`) — an unenforced contract beside a waiver is a false comfort;
* an `unsafe extern fn` whose name **also has a declaration in the selected
  contract environment** refuses (`Q034`), whether or not the signatures agree.
  This is LOADER-ABI's rule applied to this surface: where a declaration is
  visible, it is compared, and the contracted route wins. You may not launder
  away a contract you already have.

## 6. Diagnostics

A new semantic pass `UncheckedLinkage` owning the free band **`Q032`–`Q035`**
(`Q022`–`Q027` are `ExternCall`, `Q028`–`Q031` are `StandardIo`). Spec §L21 and
`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` are updated in the same change; no
existing code changes meaning.

| Code | Refusal |
|---|---|
| `Q032` | an `unsafe extern fn` declaration in a file without `#import std.extern.unsafe;` |
| `Q033` | the declared signature is outside the closed unchecked table: a pointer parameter, a pointer result, a `unit` parameter, or a spelling the §C04.1 vocabulary does not contain |
| `Q034` | a declared input and a visible checked declaration disagree: an inline `contract { … }` block on the unchecked declaration, or a name that also has a declaration in the selected contract environment |
| `Q035` | the published certificate does not carry an unchecked-linkage premise for a callee its own program declares (publication tripwire) |

## 7. Emission and execution

**[measured] Neither the backend nor the reference interpreter needed a line.**
An unchecked callee's parameters are all `CallParameter::Data` and its result is
`CallResult::Data`, so `acquires()` is false and the call takes the existing
plain-call arm in both `emit_extern_c11` and `interpret_extern_program`. The
emitted C prints the prototype from the ABI table and calls the symbol directly;
linkage stays the builder's premise, recorded as the existing
`LinkageSuppliedByBuilder`.

Native evidence: a fixed ASan/UBSan/LSan run linking a real C file that supplies
uncontracted callees, with the native observation compared to the reference
observation line for line.

## 8. Lean obligations

A new `QCore.UncheckedLinkage` namespace with a registered `Audit*.lean`, no new
axiom:

* **assumption-carrying totality** — no program with an unchecked callee derives
  an assumption list without that callee's record (§4);
* **scalar-only admission, fail-closed** — every pointer parameter, pointer
  result and `unit` parameter refuses, with the admitted table shown inhabited
  so the refusals are not vacuous;
* **the default is unchanged** — a program with no unchecked callee derives
  exactly the assumptions it derived before, and carries no unchecked premise;
* **custody-freedom** — an unchecked call's derived custody is empty and its
  ledger delta is the identity;
* **the maximal derived row** — an unchecked callee derives `{Io, Alloc, Mut}`,
  and an entry declaring less refuses;
* **the import gate** — the unchecked capability requires its own module, and
  `std.extern` alone does not grant it.

## 9. The adopted document, and every deviation from it

`source.native-library.v1` was designed in a prior workflow on branch
`claude/qcore-c-abi-linking-c8c443` (`e48786c`, ledger framing at `ecede86`).
This card **adopts that document into the tree** at
[`NATIVE_LIBRARY_LINKAGE_V1.md`](NATIVE_LIBRARY_LINKAGE_V1.md), provenance-noted
and with four recorded adjustments (its §0), rather than citing it from a
branch. Its ledger edits are deliberately not carried — acceptance and status
files are integrator surface.

It is **not** this document's competitor, and neither supersedes the other: read
side by side, they are the two ends of one spectrum (§1.1) and the two halves of
one rule.

| | `NATIVE-LIBRARY-LINKAGE` (`source.native-library.v1`) | This document (`source.extern-call.unsafe.v1`) |
|---|---|---|
| Precondition | a contract **exists** | a contract **does not exist** |
| What it does | turns "linkage is the builder's premise" into a declared, checkable unit: a manifest binding a contract slice, the exported header, an artifact identity and the ABI premises | permits linking with no contract, against a recorded waiver |
| Admitted call surface | every mode `source.extern-call.v1` already calls, pointers included | scalars only; **every pointer refused** |
| Direction of movement | strictly *tightens* the contracted route | strictly *widens* the uncontracted one, from "impossible" to "possible and printed" |

The apparent disagreement — that one admits pointer modes and the other refuses
them — is the reconciliation, not a conflict. §3.1's argument is that a pointer
needs an assumption text, and an uncontracted callee has none to write. A
manifest-bound library *does*: its contract slice is exactly that text, its
header agreement checks the projection, and its closed-allocator-custody field
is the premise about who releases what. **Pointers cross where a contract exists
to state the premise, and nowhere else.** Both documents say that; they differ
only in which side of it they are standing on.

Everything else checks out against this card's principles. Its opt-in is a
per-module registry import in the same bundled-module precedent (principle 1/5); its
ABI premises are named fields the certificate carries rather than unstated
reader assumptions (principle 2); and its §4.2 header agreement is
LOADER-ABI's constraint-8 withdrawal, which is principle 4 applied where a
second declaration is visible — the same rule §5 above applies, in a stronger
form because it has a header to compare against.

### 9.1 Deviations and reconciliations, by name

Four, and only the first is a genuine conflict.

1. **Diagnostic band — a real collision, resolved here.** That document's §7
   reserves "the first free band after `Q031`" for its new pass. This card has
   now taken `Q032`–`Q035`, so `NATIVE-LIBRARY-LINKAGE`'s band is **`Q036`
   onward**. Its §7 text is otherwise unaffected; nothing else about it moves.
2. **Its closed-allocator-custody check is vacuous on this route, structurally.**
   Its §4.4 rejects a slice that can acquire but cannot discharge. An unchecked
   callee cannot acquire at all (§3.2), so the unchecked route satisfies that
   condition by having no custody rather than by checking one. That is a
   consequence of §3, not an exemption from §4.4.
3. **The two compose, and the composition is designed rather than waved at.**
   §4.2 above designs the contract-less manifest entry (level L1): a symbol that
   lives inside an L3 manifest, inherits its artifact identity and ABI premises,
   is still type-checked against the manifest's header, and carries no modes and
   no effects. That is the strongest honest form of this card's premise. It is
   **not implemented in v1** because the manifest it lives in does not exist
   yet, and is staged as `UNSAFE-LINKAGE-ARTIFACT` (§10), aligned with that
   document's stage (a).
4. **One deviation where this card's principles win, recorded as required.**
   That document's §4.4 admits `single-threaded-caller` as a stated-but-vacuous
   premise, "ready for the day the concurrency refusal is lifted". This card
   states no thread posture at all for a waived callee: an uncontracted callee
   has no declaration in which to state one, and a premise nobody wrote is not a
   premise this repository will record on its behalf. If the concurrency refusal
   is ever lifted for the unchecked route, that is a new card and it must add
   the field explicitly. Naming it here so the omission is deliberate rather
   than inherited.

Its specifics predate the wave-17 `EXTERN-MODES`/`STD-MODULES`/`LOADER-ABI-MODES`
surface this card builds on, so they were treated as a starting proposal rather
than settled design — nothing in this card's implementation is derived from it,
and nothing is based on it. Its four adoption adjustments are recorded in its own
§0; the four points above are the only places where this card's landing
constrains it.

## 10. Out of scope (named follow-ons, not omissions)

* **`UNSAFE-LINKAGE-PTR`** — any pointer at the unchecked boundary. §3.1 is the
  argument for why this needs new model capability (an extent transport and a
  retention model), not a wider table.
* **`UNSAFE-LINKAGE-ARGS`** — argument forms beyond integer literals. The v1
  argument grammar is the existing one; a `bool` parameter is declarable but
  callable only with an integer literal, which is inherited, not introduced.
* **`UNSAFE-LINKAGE-CTX`** — unchecked calls in task workers, `with lock`
  bodies and error regions, which keep every existing refusal.
* **`UNSAFE-LINKAGE-CLI`** — a `quirin` flag selecting a contract environment.
  The CLI selects none, so today an all-unchecked module is exactly what it can
  compile end to end.
* **`UNSAFE-LINKAGE-ARTIFACT`** — **landed in wave 19.** A waived declaration
  binds to a library artifact identity in the manifest sense
  `NATIVE-LIBRARY-LINKAGE` §4.3 scopes, so the premise names *which build* was
  waived; see §4.2's landed note and
  [that card's record](../reports/wave19-unsafe-linkage-artifact/README.md).
  Its own residuals — a contract slice, a real header reader, and a builder-side
  hash observation — are named there.
* Variadics, function pointers, structs, floats and verifying the link step,
  which are out of scope for the contracted route too.
