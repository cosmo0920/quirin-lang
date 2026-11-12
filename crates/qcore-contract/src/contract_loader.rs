// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Total-or-reject bridge from native contract modules to legacy-C custody.
//!
//! # Interpreter selection
//!
//! `the canonical contract corpus` is shared by three consumers with disjoint vocabularies.
//! The **`requires resource_contract_v1` marker is the selector**: a declaration
//! that carries it must map completely into [`ResourceFunctionContract`], and a
//! declaration that does not carry it is rejected here rather than skipped, so
//! asking this loader for a `ResourceContracts` environment over an
//! observation/lifecycle module fails closed instead of silently producing a
//! weaker table. `compiler/tests/contract_loader.rs::
//! every_corpus_module_outside_the_v1_vocabulary_is_rejected_not_skipped` pins
//! that split over the whole corpus.
//!
//! | Clause family | Owner |
//! | --- | --- |
//! | `requires resource_contract_v1` + the `resource_*` predicates below | this loader |
//! | integer/Boolean ghost predicates (`result == 0`, `field_NAME`, `busy`, `lock`, `allocations`, `releases`, `old(...)` arithmetic) | `contract_observation` and the ChunkIO/source interpreters |
//! | named atomic lifecycle propositions (`chunk_is_live`, `arena_is_discharged`, `descriptor_custody_is_discharged_once`, ...) | the mechanism/lifecycle interpreters |
//! | `own<T>` / `borrow<'a, _, T>` parameters, `use` annotations, `effects { .. }` rows, `quant` clauses, `ensures any`, `ensures error(E)`, `result<T, E>` returns, `resource type` / `error` / `axis` declarations | the QCore language front end and checker |
//!
//! These families are not approximated into custody modes. A resource
//! projection of a lifecycle module is written as its own v1 module (as
//! `fluent_bit/resource_memory.qv` projects `fluent_bit/memory.qv`);
//! it is never inferred by dropping predicates this loader cannot check.
//!
//! # Version 1 vocabulary
//!
//! Every parameter and the result must occur exactly once, and all clauses
//! other than the marker must be `ensures normal` calls:
//!
//! - `resource_borrowed(p)` -> Borrowed (no custody requirement).
//! - `resource_{owned,consumed,released,out}(p, nullable|nonnull)` -> that mode.
//! - `resource_{stored,removed}(p, owner, nullable|nonnull)` -> indexed custody.
//! - `resource_return_none()` -> None; `resource_return_owned(nullability)` -> Owned.
//! - `resource_return_reallocated(p, nullability)` -> Reallocated, requiring
//!   an Owned parameter; null failure preserves it, success replaces it once.
//! - `resource_return_alias(p, nullability)` and
//!   `resource_return_interior(p, nullability)` -> Derived (card
//!   W43-LIBC-RESULT-ALIAS): the result is `p`'s own value, or a pointer into
//!   the object `p` points to, and owns nothing. See below.
//! - `resource_variadic_data_tail()` -> the declaration additionally covers
//!   calls that pass *more* actuals than it names. See below.
//! - `resource_variadic_borrowed_tail()` -> the data tail, strengthened: every
//!   trailing argument is `Borrowed` (card CG26-VARIADICS). See below.
//! - `resource_kind(p, KIND)` and `resource_return_kind(KIND)` -> the non-heap
//!   kind of a custody position (card W43-DESCRIPTOR-RESOURCE-TRACKING). See
//!   below.
//! - `resource_no_return()` -> the call never returns and no frame of the
//!   caller's thread resumes (card CG21-CALL-EXITS-AND-BOUNDARIES). See
//!   below.
//!
//! and one further **precondition** beside the marker:
//!
//! - `requires resource_literal_argument(p, LITERAL);` -> the declaration
//!   applies only at call sites whose actual at `p` is exactly `LITERAL`. See
//!   below.
//!
//! # The variadic data tail (card O04-TRACE)
//!
//! A C variadic function (`void flb_trace(const char *, ...)`) has no
//! per-argument declaration for its tail, so before this clause the vocabulary
//! could only describe one arity at a time — card O04-ARMS declared `flb_error`
//! at its single call site's arity and recorded the gap as
//! **`O04-ARMS-VARIADIC`**.
//!
//! `ensures normal resource_variadic_data_tail();` is that missing form. It
//! declares, for **every** argument beyond the named parameter vector:
//!
//! * the callee takes **no custody** of it, and
//! * the callee **retains no pointer** to it past the call.
//!
//! **What it does not say.** It is a trusted assumption about the callee's
//! contract, exactly like every other clause here; it verifies nothing about
//! the callee's body, and it is not a claim that the callee is a `printf`-style
//! formatter, that the tail is type-correct against a format string, or that
//! the tail arguments are read at all. It also does not admit anything on its
//! own: a consumer that honours the tail must still establish, per call site,
//! that each tail actual really is data. The C gateway does exactly that
//! (`qcore_c_gateway::variadic_nested`), and an actual that carries a tracked
//! resource identity is refused there even under this declaration — the
//! declaration covers what it covers.
//!
//! **The borrowed tail (card CG26-VARIADICS).**
//! `ensures normal resource_variadic_borrowed_tail();` states the same two
//! facts and one more: every trailing argument is a `Borrowed` position. A
//! consumer may then read a tracked identity there as a borrow (its liveness
//! required, nothing acquired, released or transferred) instead of refusing
//! it. It is a strengthening of the data tail, not a second clause beside it:
//! a declaration carries at most one of the two, and the borrowed tail sets
//! the data-tail flag as well, so every check keyed on the data tail (the C
//! declaration must be variadic) applies to it unchanged.
//!
//! The tail flag is **not** carried on [`ResourceFunctionContract`]. It is
//! rederived from the retained trusted source texts by
//! [`variadic_data_tails`], so a contract environment that has no declared
//! sources — an inferred W4 summary, a provider proposal, a directly
//! constructed table — has no tails at all, which is the fail-closed direction.
//!
//! # The no-return exit (card CG21-CALL-EXITS-AND-BOUNDARIES)
//!
//! `ensures normal resource_no_return();` states that the call has **no
//! normal exit**: its normal postcondition is false, so every other
//! `ensures normal` clause of the declaration holds vacuously and states only
//! what the call requires of its actuals. It also states the **kind** of the
//! exit the call takes instead: the program terminates (or the behaviour is
//! undefined), and no frame of the calling thread resumes. A non-local exit
//! (`longjmp`, `pthread_exit`) is not this exit and must never be declared
//! with it: a frame's live resources are abandoned there, not discharged.
//! The C attribute `noreturn` alone cannot tell the two apart, which is why
//! the kind is a declaration and not an inference.
//!
//! The loader admits it only on a `unit` result with `resource_return_none()`
//! and `resource_borrowed` on every parameter (the declaration's whole custody
//! statement is then its preconditions). Like the variadic tail it is **not**
//! carried on [`ResourceFunctionContract`]; it is rederived from the retained
//! trusted texts by [`no_return_callees`]. A consumer that does not read it
//! applies the declaration as an ordinary borrowing call with a continuation
//! C does not have, which over-approximates the paths and is never the unsound
//! direction; the C gateway reads it where it lowers the call as a path end
//! (rule 11o), and only where the call's own Clang declaration is `noreturn`.
//!
//! # Derived results (card W43-LIBC-RESULT-ALIAS)
//!
//! `memcpy` returns its first argument and `strchr` returns a pointer into its
//! first argument or NULL. Neither result is fresh custody, and
//! `resource_return_none()` would drop the relation between the result and the
//! base, so neither was declarable. Two result predicates now state it:
//!
//! * `resource_return_alias(p, n)` — the result is the value of `p` itself
//!   ([`DerivedExtent::Exact`]); with `nullable` the call may return NULL
//!   instead (`gmtime_r`), with `nonnull` it never substitutes NULL (`memcpy`).
//! * `resource_return_interior(p, n)` — the result points into the object `p`
//!   points to ([`DerivedExtent::Interior`]); with `nullable` it may be NULL.
//!
//! Both load to [`ResourceReturnMode::Derived`]. The result is **non-owning**:
//! it acquires, discharges and transfers nothing. `p` must be a
//! `resource_borrowed` parameter of type `ptr<mut, u8>` or `ptr<const, u8>`,
//! and the result type must be one of those two; a base declared
//! `ptr<const, u8>` under a `ptr<mut, u8>` result records `const_dropped`.
//! Every other placement rejects the module. What a consumer must do with the
//! relation — a release of the result is not a release of the base, a release
//! of the base invalidates the result — is contract spec §C04.3; this loader
//! only states it.
//!
//! **Version.** This is an addition to resource vocabulary version 1, as the
//! variadic tail and the literal premise were: the marker is unchanged, every
//! module that loaded before loads to the same functions, and a loader that
//! predates the predicates rejects a module using them (unknown predicate)
//! rather than misreading it. The exact module texts travel with every
//! verdict in `source_assumptions`, so the evidence identity already names
//! which declarations — and therefore which vocabulary — a verdict rests on.
//!
//! # Resource kinds (card W43-DESCRIPTOR-RESOURCE-TRACKING)
//!
//! A custody position carries a [`ResourceKind`]: `heap` (spelled by the
//! **absence** of a kind clause, so every earlier module loads to exactly the
//! contract it loaded to before), `descriptor`, `stream`, `directory`,
//! `dynamic_library` or the in-place `regex` (card CG25-RESOURCE-KINDS, whose
//! placement rules are in `contract_in_place.rs`, contract spec §C04.6.1).
//! A consumer discharges an obligation only
//! with a release of its own kind (contract spec §C04.6). The clauses are
//! `ensures normal resource_kind(p, KIND);` on an owned, consumed, released or
//! out parameter and `ensures normal resource_return_kind(KIND);` on an owned
//! result; a borrowed, stored, removed or reallocated position, a derived or
//! `none` result, an unknown kind, the spelled-out `heap` and a duplicate each
//! reject the whole module.
//!
//! `descriptor` is the one kind carried by an **integer**. Its position must be
//! typed `i32`, and its validity is spelled `sentinel` (may be the failure value
//! `-1`) or `valid`, never `nullable`/`nonnull`; conversely `sentinel`/`valid`
//! on any other position rejects the module. That is the only way a scalar ever
//! carries custody here: an `i32` position without the `descriptor` kind keeps
//! the borrowed-only rule and its verbatim refusals. A descriptor has no out
//! slot in this vocabulary.
//!
//! **Version.** An addition to vocabulary version 1, as the derived results
//! were: the marker is unchanged, kinds serialize only when non-heap, and a
//! loader that predates the clauses rejects them (unsupported predicate or
//! nullability) rather than misreading them.
//!
//! # The literal-argument premise (card HASH-CLAUSE)
//!
//! A v1 declaration names one custody mode per parameter and says nothing about
//! argument *values* (§C04.2 has no value-dependent mode). Some real C entry
//! points switch behaviour on a scalar argument:
//! `flb_hash_table_add(ht, key, key_len, val, val_size)` stores the caller's
//! pointer uncopied and never frees it at `val_size == 0`, and allocates its own
//! copy at a positive size. A declaration read off one arm must not be believed
//! on the other.
//!
//! `requires resource_literal_argument(p, LITERAL);` is that restriction, as a
//! first-class clause. It declares that the contract applies **only** at call
//! sites whose actual at parameter `p` is exactly the token sequence a C lexer
//! produces for `LITERAL`; everywhere else a consumer MUST treat the callee as
//! undeclared. The clause can therefore only ever **withdraw** call sites the
//! declaration would otherwise have covered — it can never admit one, and it
//! never alters the modes.
//!
//! It is a `requires` clause because it is a precondition on *applicability*,
//! and it names its parameter by name because every other predicate in this
//! vocabulary does. `LITERAL` is a decimal integer with an optional leading
//! minus and no suffix, alternate base, leading zero or second spelling of zero;
//! a spelling this loader cannot compare exactly against a C token vector is
//! rejected rather than approximated. The declared spelling is therefore
//! **canonical**: one spelling per value. The *call site* may spell the same
//! value several ways, and which of those a consumer admits is decided by the
//! consumer against this canonical form — see
//! `qcore_c_gateway::legacy_literal_premise` and §C04.5.
//!
//! ## Any parameter, deliberately (card HASH-REFINE)
//!
//! `p` may name a parameter of **any** declared type in this vocabulary. The
//! decision is closed rather than open-ended, because the vocabulary is: a
//! parameter is either a [`SCALAR_ABI_TYPES`] projection or a
//! [`POINTER_ABI_TYPES`] one, so restricting the clause could only mean
//! "scalars only". Three measured reasons against that restriction:
//!
//! 1. A literal premise on a pointer parameter is **meaningful**: an integer
//!    constant expression with value `0` is C's null pointer constant
//!    (C11 6.3.2.3p3), so `requires resource_literal_argument(p, 0);` declares
//!    a contract read off the arm where `p` is null. That is a legitimate C04
//!    use, exercised end to end by the gateway's null-sentinel fixture.
//! 2. A premise on any parameter can only ever **withdraw** call sites, so a
//!    premise that no call site can satisfy — a nonzero literal at a pointer
//!    parameter, which is not even valid C without a cast — is vacuous in the
//!    *refusing* direction. Restricting buys no soundness.
//! 3. The retired comment directive imposed no such restriction either, so
//!    adding one would move the migration baseline in the refusing direction
//!    with no measured need.
//!
//! What the loader does still refuse is unchanged: the name must be a parameter
//! of this declaration, and at most one premise may name each parameter.
//!
//! Total-or-reject applies in full: an unknown parameter name, a malformed
//! literal, a second premise on the same parameter, and any other `requires`
//! spelling each reject the **whole module**, so a typo can never leave a
//! declaration silently unrestricted.
//!
//! Like the variadic tail, the premise is **not** carried on
//! [`ResourceFunctionContract`]; it is rederived from the retained trusted
//! source texts by [`literal_argument_premises`], so an environment with no
//! declared sources — an inferred W4 summary, a provider proposal, a directly
//! constructed table — declares no premises. That direction is fail-closed for
//! the *loader* (no premise is claimed) and consumers must treat a rederivation
//! failure over sources that mention the clause as a refusal, never as an empty
//! premise set: `qcore_c_gateway::legacy_literal_premise` does exactly that.
//!
//! The mechanism this clause replaces was the machine-parsed
//! `// qcore-premise: literal_argument(...)` comment directive of card
//! HASH-CONTRACTS. That carrier is **retired**, and a leftover occurrence of it
//! must fail loudly rather than rot: a module whose text still contains the
//! directive introducer is rejected by name — see
//! [`RETIRED_PREMISE_DIRECTIVE`]. Silently ignoring one would drop a declared
//! restriction, which is exactly the fail-open direction.
//!
//! Signatures are opaque C ABI projections, not checked C layouts. The type
//! vocabulary is likewise an explicit table, not a wildcard:
//!
//! - [`SCALAR_ABI_TYPES`] — sizes, counts, flags, descriptors, offsets and
//!   real-valued readings. Borrowed only, unless the position declares the
//!   integer-carried `descriptor` kind on an `i32` (see *Resource kinds*):
//!   every other custody mode on a scalar is rejected. Card NEIGHBOURS admitted `f32` and
//!   `f64` under exactly the condition this note stated for them — a resource
//!   API in scope that passes one: Fluent Bit's `double flb_time_now()`
//!   (`src/flb_lib.c:936`), whose reading `flb_input_chunk_create` stores in
//!   `ic->create_time`. Before that there was no use, and the table had none.
//! - [`POINTER_ABI_TYPES`] — the custody-bearing projections. `resource_out`
//!   additionally requires `ptr<mut, ptr<mut, u8>>`, and any non-`None` result
//!   requires a pointer return type.
//! - [`FUNCTION_POINTER_ABI_TYPE`] (`fnptr`) — the projection of a pointer to
//!   function. Borrowed only, by the same two gates that hold every scalar
//!   borrowed-only: both key on the leading word `ptr`, which `fnptr` is not.
//! - [`VA_LIST_ABI_TYPE`] (`va_list`) — the projection of the compiler's
//!   `__builtin_va_list` at a parameter. Borrowed only, by the same two gates,
//!   and a parameter type only (card W44-LIBC-RESIDUALS).
//! - `unit` is a result type only, never a parameter type.
//!
//! Acquire/Release/Escape/Neutral compatibility kinds map respectively to Owned
//! results, Released/Consumed parameters, and borrowed-only/None contracts.
//! A pointer error sentinel (a resource returned as `MAP_FAILED` rather than
//! null) remains unrepresentable and is rejected. The integer sentinel `-1` of
//! a file descriptor is representable only through the `descriptor` kind and
//! its `sentinel`/`valid` words (card W43-DESCRIPTOR-RESOURCE-TRACKING); no
//! other scalar sentinel is, and nullability is never reinterpreted as one. Conditional effects, effects
//! rows, quantitative clauses, arbitrary predicates and other exits are
//! likewise rejected.
//!
//! Files are trusted assumptions, not proofs. The exact input texts are retained
//! in ResourceContracts and independently reloaded by the certificate checker.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_inclusion::{
    ContractInclusionError, ContractLoadPath, Elaborated, Elaborator, InclusionCode,
    LoadPathResolver, module_key, reject_unresolved_inclusion, replay_evidence,
};
use qcore_frontend::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceContractExitSelector, SourceDeclaration,
    SourceModuleMode, parse_source_program,
};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};

// Card W45-FB-RETRY-LIFETIME: counted-user and container clauses.
#[path = "contract_counted_users.rs"]
mod counted_users;
#[path = "contract_in_place.rs"]
mod in_place;
// Card CG23-RELATIONAL-POSTCONDITIONS: return-code dispositions (§C04.13).
#[path = "contract_relational.rs"]
mod relational;

// Card CALLEE-RETENTION-CLASS1: the retention clauses (§C04.14).
#[path = "contract_retained_by.rs"]
mod retained_by;
// Card SLOT-MOVE-AND-INTERIOR-BORROW: the slot clauses (§C04.15, §C04.16) and
// the borrowed result (§C04.3.4).
#[path = "contract_slot_move.rs"]
pub mod slot_move;
// Card REFCOUNT-RESOURCE-KIND: the reference clauses (§C04.17).
#[path = "contract_reference_count.rs"]
mod reference_count;
// Card CG22-CALLBACK-REGISTRY-CONTRACT: the callback clauses (§C04.19).
#[path = "contract_callback.rs"]
mod callback;
// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES: the type-switched release
// (§C04.19.2).
#[path = "contract_arm_release.rs"]
mod arm_release;
// Card OP-CONSTRUCTOR-DECLARATIONS: a constructor's result state (§C04.19.4).
#[path = "contract_constructed_state.rs"]
mod constructed_state;
// Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: a formal the callback takes
// depending on its return code, and the module's aliases (§C04.19.5).
#[path = "contract_descriptor_cell.rs"]
mod descriptor_cell;
#[path = "contract_handoff.rs"]
mod handoff;
#[path = "contract_member_path.rs"]
mod member_path;
use qcore_schemas::contract::{
    ContractTrust, DerivedExtent, PendingHumanReview, ResourceContracts, ResourceFunctionContract,
    ResourceKind, ResourceParameterMode, ResourceReturnMode, ReviewDeferral, ReviewDisposition,
    ReviewScope,
};

/// Opaque C ABI scalar projections. Borrowed only: they carry sizes, counts,
/// flags, descriptors, offsets and real-valued readings, never custody.
/// Widening this table changes which signatures can be *written*, never what a
/// loaded contract means.
///
/// `f32`/`f64` were added by card NEIGHBOURS so that Fluent Bit's
/// `double flb_time_now()` could be declared with its OWN return type rather
/// than projected onto an integer it does not have. Nothing else moved with
/// them: a float is refused in every custody mode exactly as every other scalar
/// is, and a non-`None` result still requires a pointer return type, so no
/// float can occupy a custody position. `unit` and every pointer spelling
/// outside [`POINTER_ABI_TYPES`] stay refused as parameter types.
pub const SCALAR_ABI_TYPES: &[&str] = &[
    "bool", "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize", "f32", "f64",
];

/// Opaque C ABI pointer projections. These are the custody-bearing types.
///
/// Card **W43-ABI-ENUM-AND-CONST-PP** (gateway rule 12v) added
/// `ptr<mut, ptr<const, u8>>`, a pointer to a pointer to const (`const char
/// **`, `const void **`): the outer `mut` says the slot the outer pointer
/// designates is not `const`, the inner `const` is the innermost pointee's.
/// It carries custody exactly as `ptr<mut, ptr<mut, u8>>` does, except that
/// `resource_out` still requires that one spelling (a fresh object delivered
/// through a slot of pointers to const has no measured need). A qualifier on
/// the slot (`char *const *`) has no spelling and stays refused.
pub const POINTER_ABI_TYPES: &[&str] = &[
    "ptr<mut, u8>",
    "ptr<const, u8>",
    "ptr<mut, ptr<mut, u8>>",
    "ptr<mut, ptr<const, u8>>",
];

/// The opaque C ABI projection of a **pointer to function** (card
/// **CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF**).
///
/// It is deliberately *not* an entry of [`POINTER_ABI_TYPES`] and deliberately
/// not spelled `ptr<…>`. A function designator is never a resource identity in
/// this model, so a function pointer can never carry custody, and the two gates
/// that decide custody both key on the leading word `ptr`:
/// `custody mode requires a pointer parameter` and
/// `owned result requires a pointer return type`. Spelling this projection
/// `fnptr` therefore makes every custody mode on it — and an owned result of it
/// — a **load-time rejection**, by the rules that were already there.
///
/// It is not folded onto `u64`/`usize` either. That would be the same
/// eight-byte reading on the LP64 targets audited here, but it would also make
/// a declaration of an *integer* parameter and a declaration of a *callback*
/// parameter indistinguishable, and card LOADER-ABI's whole point is that the
/// declared spelling is compared against the C declaration. A separate spelling
/// keeps the comparison exact in both directions: a `fnptr` declaration is
/// refused for every C type that is not a pointer to function, and every other
/// spelling is refused for one that is.
///
/// Like [`SCALAR_ABI_TYPES`], widening the vocabulary changes which signatures
/// can be *written*, never what a loaded contract means. The condition card
/// NEIGHBOURS stated for `f32`/`f64` is met here too: a resource API in scope
/// passes one — msgpack's
/// `msgpack_packer_init(msgpack_packer *, void *, msgpack_packer_write)`,
/// whose third parameter is `typedef int (*msgpack_packer_write)(void *, const
/// char *, size_t)` and which the production `flb_input_chunk_create` calls.
pub const FUNCTION_POINTER_ABI_TYPE: &str = "fnptr";

/// The opaque C ABI projection of the compiler's variable-argument list type,
/// `__builtin_va_list` (card **W44-LIBC-RESIDUALS**, `LIBC-PRINTF-VA-LIST`).
///
/// glibc spells the last parameter of `vsnprintf`, `vprintf`, `vfprintf` and
/// `vsprintf` `__gnuc_va_list __arg`, a typedef of the builtin. What the
/// builtin *is* depends on the target — `__va_list_tag[1]`, adjusted to an
/// object pointer at a parameter, on x86-64; a structure passed by value on
/// AArch64 — so no existing spelling reads it without binding the reader to a
/// target. This spelling names the type, not its representation, and so it is
/// target-independent.
///
/// It is deliberately **not** spelled `ptr<…>`: the two custody gates key on
/// the leading word `ptr`, exactly as they do for [`FUNCTION_POINTER_ABI_TYPE`],
/// so `resource_borrowed` is the only mode a declaration can write on it. A
/// variable-argument list is read as data by the callee; it is never a resource
/// identity. It is a **parameter** type only: a function cannot return the
/// array type the builtin is on x86-64, and no declaration in scope needs a
/// result of it, so it is left out of the result vocabulary.
///
/// Like [`SCALAR_ABI_TYPES`], widening the vocabulary changes which signatures
/// can be *written*, never what a loaded contract means. The condition card
/// NEIGHBOURS stated is met: a resource API in scope passes one — glibc's
/// `vsnprintf`, called in 25 functions of the pinned corpus.
pub const VA_LIST_ABI_TYPE: &str = "va_list";

/// The introducer of the retired `qcore-premise` comment directive (card
/// HASH-CONTRACTS, superseded by HASH-CLAUSE's `resource_literal_argument`
/// clause).
///
/// A contract module whose text still contains it is **rejected**, anywhere in
/// the text and in any comment shape. Ignoring a leftover directive would drop
/// the restriction it declares and silently widen the declaration back to every
/// call site, so the refusal is deliberately broader than the shape the
/// directive parser recognised: prose that names the retired introducer belongs
/// in this loader's own documentation, never in a contract module.
pub const RETIRED_PREMISE_DIRECTIVE: &str = "qcore-premise:";

/// Reject a module carrying the retired directive.
fn reject_retired_premise_directive(path: &str, source: &str) -> Result<(), String> {
    if source.contains(RETIRED_PREMISE_DIRECTIVE) {
        return Err(format!(
            "{path}: the `{RETIRED_PREMISE_DIRECTIVE}` comment directive is retired; \
             declare `requires resource_literal_argument(PARAMETER, LITERAL);` instead"
        ));
    }
    Ok(())
}

/// One declared literal-argument premise (card HASH-CLAUSE): the contract
/// applies only where the actual at `parameter` is exactly `literal`.
///
/// `literal` is the token *texts* a C lexer produces for the declared spelling,
/// so a consumer comparing against a C token stream compares vectors and never
/// re-parses a number: `0` is `["0"]` and `-1` is `["-", "1"]`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclaredLiteralArgument {
    /// Index into the declaration's parameter list.
    pub parameter: usize,
    /// The declared literal, as C token texts.
    pub literal: Vec<String>,
}

/// What a declared loop-owner obligation says the operation does to the loop
/// its instance's collectors attach to (card **W47-LOOP-OWNER-CONTRACTS**,
/// §C04.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoopOwnerOperation {
    /// Reads loop-owned state (a collector's `running` flag).
    Read,
    /// Mutates the loop's links or a collector's state.
    Mutate,
}

/// `requires resource_loop_owner(p, read|mutate);` (card
/// **W47-LOOP-OWNER-CONTRACTS**, §C04.9): the declaration applies at a call
/// site only where the call's derived thread is exactly the owner of the loop
/// that the instance at `p` attaches its collectors to. No per-unit resource
/// route derives a thread, so there the declaration is never applied; only the
/// thread-entry census route can discharge it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclaredLoopOwner {
    /// Index into the declaration's parameter list: the input instance.
    pub parameter: usize,
    pub operation: LoopOwnerOperation,
}

/// The loop-owner precondition's name.
pub const LOOP_OWNER_CLAUSE: &str = "resource_loop_owner";

/// What a declared thread-synchronization obligation says the call is (card
/// **W47-CONDVAR-JOIN-PUBLICATION**, §C04.10, gateway rule 10q).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreadSyncOperation {
    /// `pthread_cond_wait`: releases and reacquires the named mutex.
    CondWait,
    /// `pthread_cond_timedwait`: as a wait, and may return on a deadline.
    CondTimedWait,
    /// `pthread_cond_signal`.
    CondSignal,
    /// `pthread_cond_init`: only in a quiescent phase.
    CondInit,
    /// `pthread_cond_destroy`: only in a quiescent phase.
    CondDestroy,
    /// `pthread_join`: ends a thread; only a checked zero result joins.
    Join,
}

impl ThreadSyncOperation {
    /// The clause's spelling of the operation.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::CondWait => "cond_wait",
            Self::CondTimedWait => "cond_timedwait",
            Self::CondSignal => "cond_signal",
            Self::CondInit => "cond_init",
            Self::CondDestroy => "cond_destroy",
            Self::Join => "join",
        }
    }

    /// Whether the clause names a mutex after the condvar.
    #[must_use]
    pub const fn names_mutex(self) -> bool {
        matches!(self, Self::CondWait | Self::CondTimedWait)
    }
}

/// `requires resource_thread_sync(cond, mutex, cond_wait|cond_timedwait);`,
/// `requires resource_thread_sync(cond, cond_signal|cond_init|cond_destroy);`
/// or `requires resource_thread_sync(thread, join);` (card
/// **W47-CONDVAR-JOIN-PUBLICATION**, §C04.10): the declaration applies at a
/// call site only where the census discharges the obligation (a wait in a
/// predicate loop under its mutex, a signal under the paired mutex, an
/// `init`/`destroy` in a quiescent phase, a join whose zero result is
/// checked). No per-unit resource route derives any of that, so there the
/// declaration is never applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclaredThreadSync {
    /// The condvar (a pointer parameter) or the thread (a scalar one).
    pub parameter: usize,
    /// The mutex a wait releases and reacquires (a pointer parameter).
    pub mutex: Option<usize>,
    pub operation: ThreadSyncOperation,
}

/// The thread-synchronization precondition's name.
pub const THREAD_SYNC_CLAUSE: &str = "resource_thread_sync";

/// Which transition of a strictly paired start/stop lifecycle a declared
/// call is (cards **W47-FLB-LIB-LIFECYCLE** and
/// **W47-FLB-LIB-STRICT-PAIRING**, §C04.11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LifecycleOperation {
    /// Starts the context's worker: permitted only from created (a restart is
    /// not derived).
    Start,
    /// Stops it: permitted only after a start that returned 0, once; the
    /// first stop ends what the start began. A later stop, a stop after a
    /// failed start and a stop of a never-started context are caller
    /// violations.
    Stop,
    /// Destroys the context: permitted from created, after a failed start,
    /// or after the stop.
    Destroy,
}

impl LifecycleOperation {
    /// The clause's spelling of the operation.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Destroy => "destroy",
        }
    }
}

/// `requires resource_lifecycle(ctx, start|stop|destroy);` (card
/// **W47-FLB-LIB-LIFECYCLE**, §C04.11): the call is that transition of the
/// strictly paired lifecycle of the context at `parameter`. The clauses the
/// state machine implies are *specifications to check*: the census holds each
/// caller to the preconditions and checks the body clauses against the
/// declared bodies, applying only those in force. No per-unit resource route
/// derives a context's lifecycle state, so there the declaration is never
/// applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclaredLifecycle {
    /// Index into the declaration's parameter list: the context pointer.
    pub parameter: usize,
    pub operation: LifecycleOperation,
}

/// The lifecycle obligation's name.
pub const LIFECYCLE_CLAUSE: &str = "resource_lifecycle";

/// The declared C ABI projection of one admitted declaration (card
/// **LOADER-ABI**).
///
/// The custody modes on [`ResourceFunctionContract`] are what the analyzer
/// applies; these are the *types* the declaration claims those modes sit on.
/// They are discarded by [`ResourceFunctionContract`] on purpose — nothing in
/// the model reads a type — which is exactly why nothing used to compare them
/// against the analyzed translation unit's own C declaration. A consumer that
/// wants that comparison rederives this from the retained trusted texts, the
/// same way [`variadic_data_tails`] and [`literal_argument_premises`] are
/// rederived, so W4 derivation, W4 validation and certificate rechecking all
/// reproduce it from the bytes they already hold.
///
/// Every spelling here is an entry of [`SCALAR_ABI_TYPES`],
/// [`POINTER_ABI_TYPES`], [`FUNCTION_POINTER_ABI_TYPE`], for a parameter only
/// [`VA_LIST_ABI_TYPE`], or, for `result` only, `unit`: the loader admits no
/// other spelling, so a consumer compares against a closed table.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclaredAbiSignature {
    /// The declared parameter type spellings, in declaration order.
    pub parameters: Vec<String>,
    /// The declared result type spelling, `unit` included.
    pub result: String,
    /// Whether the declaration carries `resource_variadic_data_tail()`.
    pub variadic_data_tail: bool,
}

/// Everything one admitted declaration carries beyond its custody modes.
struct LoadedDeclaration {
    contract: ResourceFunctionContract,
    variadic_data_tail: bool,
    /// Card CG26-VARIADICS: `resource_variadic_borrowed_tail()` was declared
    /// (it implies `variadic_data_tail`).
    variadic_borrowed_tail: bool,
    /// Card CG21-CALL-EXITS-AND-BOUNDARIES: the declaration carries
    /// `resource_no_return()`.
    no_return: bool,
    literal_arguments: Vec<DeclaredLiteralArgument>,
    /// Card VERDICT-CONTRACT-DEPENDENCIES: every named premise the
    /// declaration states (`requires resource_premise(NAME, "STATEMENT");`),
    /// in clause order, followed by its literal-argument premises rendered as
    /// premises. This metadata list decides no verdict; C04.21's scoped
    /// atoms/recognized snapshot clause are separately parsed structured data.
    premises: Vec<DeclaredPremise>,
    hypotheses: qcore_schemas::hypothesis::Graph,
    /// Card W47-LOOP-OWNER-CONTRACTS: the declared loop-owner obligation.
    loop_owner: Option<DeclaredLoopOwner>,
    /// Card W47-CONDVAR-JOIN-PUBLICATION: the declared thread-synchronization
    /// obligation.
    thread_sync: Option<DeclaredThreadSync>,
    /// Card W47-FLB-LIB-LIFECYCLE: the declared lifecycle obligation.
    lifecycle: Option<DeclaredLifecycle>,
    abi: DeclaredAbiSignature,
    /// Card W45-CONTRACT-DEFERRED-REVIEW: every review marker, in clause
    /// order. Nonempty withholds the whole contract.
    review: Vec<ReviewDeferral>,
    /// Card W45-FB-BACKLOG-CUSTODY: the declaration carries
    /// `requires resource_body_checked;` (§C06.3): it is a specification of a
    /// body the audit analyses, applied only where that body establishes it.
    body_checked: bool,
    /// Card W45-CONTRACT-REVISION-MODE: the declaration's revision markers,
    /// validated as a whole; `None` when it carries none (pinned).
    revision: Option<qcore_schemas::contract_revision::DeclaredRevision>,
    /// Card W48-QR-C-STRUCT-IMPORT: the declaration carries
    /// `[| quant #c_ffi |]`. It is withheld from the function map.
    ffi: Option<DeclaredFfiFunction>,
    /// Card CG23-RELATIONAL-POSTCONDITIONS: the declaration's return-code
    /// dispositions (§C04.13). When set, `contract` is its first class and the
    /// declaration is installed only as a relational contract.
    relational: Option<qcore_schemas::relational_post::RelationalContract>,
    /// Card CALLEE-RETENTION-CLASS1: the declaration's retention clauses
    /// (§C04.14). When set, the declaration is installed only as a retaining
    /// contract, never in the function map.
    retaining: Option<qcore_schemas::retained_by::RetainingContract>,
}

/// The parameter vocabulary: every scalar, every pointer, `fnptr` and
/// `va_list`.
fn parameter_abi_types() -> impl Iterator<Item = &'static str> {
    SCALAR_ABI_TYPES
        .iter()
        .copied()
        .chain(POINTER_ABI_TYPES.iter().copied())
        .chain(std::iter::once(FUNCTION_POINTER_ABI_TYPE))
        .chain(std::iter::once(VA_LIST_ABI_TYPE))
}

/// The result vocabulary: `unit` plus every parameter type except `va_list`,
/// which is a parameter type only.
fn result_abi_types() -> impl Iterator<Item = &'static str> {
    std::iter::once("unit").chain(parameter_abi_types().filter(|ty| *ty != VA_LIST_ABI_TYPE))
}

/// Load all selected modules atomically. No partial environment escapes on error.
pub fn load_resource_contract_files(paths: &[PathBuf]) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&read_resource_contract_files(paths)?)
}

/// Versioned selection of resource modules; paths are relative to the manifest.
///
/// **Schema 1** selects files: every entry is a path relative to the manifest.
///
/// **Schema 2** (card **CONTRACT-INHERIT-ADOPTION**) adds a required
/// `load_path` (a load-path manifest, relative to this one; see
/// [`crate::load_path_manifest`]) and admits a second kind of entry: a
/// `crate:module` reference, spelled exactly like an `#include` target, which
/// selects a module owned by a crate on that load path and is loaded through
/// [`load_published_resource_contracts`]'s elaboration and v1 agreement gate.
/// An entry is a reference when it contains `:`; a path entry may not. Path
/// entries resolve **identically** to schema 1: the same file, read the same
/// way, retained under the same key. So a schema-2 manifest whose entries are
/// all paths loads exactly the environment its schema-1 twin loads (test
/// `schema_two_path_entries_load_exactly_as_schema_one`). Everything is
/// total-or-reject: a stale or invalid load path, a malformed reference, a
/// module selected twice, or two declarations of one function refuse the
/// whole selection.
pub fn load_resource_contract_manifest(path: &Path) -> Result<ResourceContracts, String> {
    let source = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    load_resource_contract_manifest_text(path, &source)
}

/// [`load_resource_contract_manifest`] over manifest text already in hand
/// (card **W44-QUIRIN-AUDIT-PROVIDERS**): `path` names where the manifest
/// sits, and every relative entry resolves against its parent exactly as if
/// the text had been read from that file. A provider selection is lowered to
/// such a text and loaded here, so it and an equal hand-written manifest are
/// one load, not two implementations of it.
pub fn load_resource_contract_manifest_text(
    path: &Path,
    source: &str,
) -> Result<ResourceContracts, String> {
    load_resource_contract_manifest_text_stamped(
        path,
        source,
        &crate::contract_supersession::Unstamped,
    )
}

/// [`load_resource_contract_manifest`] with pin stamps (card
/// **CONTRACT-SUPERSESSION**).
pub fn load_resource_contract_manifest_stamped(
    path: &Path,
    stamps: &dyn crate::contract_supersession::DeclarationStamps,
) -> Result<ResourceContracts, String> {
    let source = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    load_resource_contract_manifest_text_stamped(path, &source, stamps)
}

/// [`load_resource_contract_manifest_text`] with pin stamps (card
/// **CONTRACT-SUPERSESSION**): the audit passes the stamps of its locks, so a
/// function two selected modules declare is decided by them.
pub fn load_resource_contract_manifest_text_stamped(
    path: &Path,
    source: &str,
    stamps: &dyn crate::contract_supersession::DeclarationStamps,
) -> Result<ResourceContracts, String> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Manifest {
        schema_version: u32,
        #[serde(default)]
        load_path: Option<String>,
        modules: Vec<String>,
    }
    let manifest: Manifest =
        serde_json::from_str(source).map_err(|e| format!("{}: {e}", path.display()))?;
    let supported = match manifest.schema_version {
        1 => manifest.load_path.is_none(),
        2 => manifest.load_path.is_some(),
        _ => false,
    };
    if !supported || manifest.modules.is_empty() {
        return Err(format!(
            "{}: unsupported version or empty module selection",
            path.display()
        ));
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let relative = |entry: &str| -> Result<PathBuf, String> {
        let module = Path::new(entry);
        if entry.is_empty()
            || module
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err("contract module paths must be relative without traversal".to_owned());
        }
        Ok(parent.join(module))
    };
    let mut paths = Vec::new();
    let mut references = Vec::new();
    for entry in &manifest.modules {
        if manifest.schema_version == 2 && entry.contains(':') {
            references.push(
                published_module_reference(entry)
                    .map_err(|e| format!("{}: {e}", path.display()))?,
            );
        } else {
            paths.push(relative(entry)?);
        }
    }
    let Some(load_path) = manifest.load_path else {
        return load_resource_contract_sources_stamped(
            &read_resource_contract_files(&paths)?,
            stamps,
        );
    };
    let load_path = relative(&load_path)?;
    // Validated even when every entry is a path: a schema-2 manifest names a
    // load path, so a stale one is refused rather than ignored.
    let load_path = crate::load_path_manifest::load_contract_load_path_manifest(&load_path)?;
    let path_sources = read_resource_contract_files(&paths)?;
    if references.is_empty() {
        return load_resource_contract_sources_stamped(&path_sources, stamps);
    }
    load_resource_contract_selection_stamped(
        &load_path.load_path,
        path_sources,
        &references,
        stamps,
    )
}

/// Parse a manifest's `crate:module` entry, with the `#include` target's
/// admitted spelling (card **CONTRACT-INHERIT-ADOPTION**).
fn published_module_reference(entry: &str) -> Result<PublishedModuleRef, String> {
    use qcore_frontend::contract_inclusion::{
        valid_contract_crate_name, valid_contract_module_path,
    };
    match entry.split_once(':') {
        Some((crate_name, module))
            if valid_contract_crate_name(crate_name) && valid_contract_module_path(module) =>
        {
            Ok(PublishedModuleRef::new(crate_name, module))
        }
        _ => Err(format!(
            "`{entry}` is not a `crate:module` reference (a lowercase crate name, `:`, and a \
             relative `[a-z0-9_/-]+.qcore` module path)"
        )),
    }
}

/// Read selected files into the snapshot [`load_resource_contract_files`]
/// loads, keyed by the path as spelled.
fn read_resource_contract_files(paths: &[PathBuf]) -> Result<BTreeMap<String, String>, String> {
    let names = paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    crate::extension_compatibility::reject_alias_collisions(names.iter().map(String::as_str))?;
    let mut sources = BTreeMap::new();
    for path in paths {
        let name = path.to_string_lossy().into_owned();
        let source =
            fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
        if sources.insert(name.clone(), source).is_some() {
            return Err(format!("duplicate contract input {name}"));
        }
    }
    Ok(sources)
}

/// Load an immutable source snapshot, also used for certificate replay.
///
/// Card **CONTRACT-SUPERSESSION** (contract spec §C06.6): a function declared
/// by more than one module is no longer refused. With no stamps every such
/// function is withheld as a contract conflict
/// ([`crate::contract_supersession`]); an audit passes the stamps of its
/// locks through [`load_resource_contract_sources_stamped`].
pub fn load_resource_contract_sources(
    sources: &BTreeMap<String, String>,
) -> Result<ResourceContracts, String> {
    load_resource_contract_sources_stamped(sources, &crate::contract_supersession::Unstamped)
}

/// [`load_resource_contract_sources`] with the pin stamps of the selected
/// declarations (card **CONTRACT-SUPERSESSION**). A snapshot in which no
/// function is declared twice loads exactly as before, without reading a
/// stamp. Otherwise the duplicates are decided from `stamps`
/// ([`crate::contract_supersession::supersede_sources`]), the retained
/// sources are the transformed ones, and the environment records every
/// superseded and every conflicted declaration.
pub fn load_resource_contract_sources_stamped(
    sources: &BTreeMap<String, String>,
    stamps: &dyn crate::contract_supersession::DeclarationStamps,
) -> Result<ResourceContracts, String> {
    match load_resource_contract_sources_strict(sources)? {
        Ok(contracts) => Ok(contracts),
        Err(_) => {
            let supersession = crate::contract_supersession::supersede_sources(sources, stamps)?;
            // Every selected module may lose its every declaration (one
            // conflicted function declared alone in each): the environment
            // is then empty but for the record.
            let mut contracts = if supersession.sources.is_empty() {
                ResourceContracts::default()
            } else {
                load_resource_contract_sources_strict(&supersession.sources)?.map_err(
                    |context| format!("{context}: duplicate function contract after supersession"),
                )?
            };
            contracts.set_supersession(supersession.superseded, supersession.conflicts);
            Ok(contracts)
        }
    }
}

/// The loader proper: `Ok(Err(CONTEXT))` when a function is declared twice
/// (the first repeated declaration's context), so the caller decides it.
fn load_resource_contract_sources_strict(
    sources: &BTreeMap<String, String>,
) -> Result<Result<ResourceContracts, String>, String> {
    if sources.is_empty() {
        return Err("no resource contract inputs selected".into());
    }
    crate::extension_compatibility::reject_alias_collisions(sources.keys().map(String::as_str))?;
    let mut contracts = ResourceContracts::default();
    let mut pending = BTreeMap::new();
    // Card W45-CONTRACT-REVISION-MODE: the markers of every declaration that
    // carries one, pending declarations included.
    let mut revisions = BTreeMap::new();
    for (path, source) in sources {
        reject_retired_premise_directive(path, source)?;
        reject_unresolved_inclusion(path, source)?;
        let parsed = parse_source_program(source).map_err(|e| format!("{path}: {e}"))?;
        qcore_frontend::source_extension::validate_source_extension(path, parsed.mode)
            .map_err(|error| format!("{path}: {error}"))?;
        if parsed.mode != SourceModuleMode::Contract || parsed.declarations.is_empty() {
            return Err(format!("{path}: expected a nonempty module contract"));
        }
        if let Some(warning) =
            qcore_frontend::source_extension::legacy_source_warning(path, parsed.mode)
        {
            eprintln!("{warning}");
        }
        // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: the module's aliases,
        // module-local, each colliding with no keyword or binder.
        let aliases = handoff::module_aliases(&parsed).map_err(|e| format!("{path}: {e}"))?;
        for declaration in parsed.declarations {
            let decl = &declaration.value;
            let context = format!("{path}: {} at byte {}", decl.name, declaration.span.start);
            // Card VERDICT-CONTRACT-DEPENDENCIES: a duplicate check of the
            // loader is not a consultation, so it reads the maps directly.
            if contracts.functions().contains_key(&decl.name)
                || pending.contains_key(&decl.name)
                || contracts.relational().contains_key(&decl.name)
                || contracts.retaining().contains_key(&decl.name)
                || contracts.guarded_retaining().contains_key(&decl.name)
            {
                return Ok(Err(context));
            }
            let loaded = load_function_in(decl, &aliases).map_err(|e| format!("{context}: {e}"))?;
            if let Some(revision) = loaded.revision {
                revisions.insert(decl.name.clone(), revision);
            }
            // Card W45-CONTRACT-DEFERRED-REVIEW: a declaration carrying a
            // review marker is validated in full above and then withheld. It
            // is never an entry of the function map, so a consumer that knows
            // nothing of the marker sees an uncontracted callee.
            if loaded.ffi.is_some() {
                // Card W48-QR-C-STRUCT-IMPORT: a `#c_ffi` declaration is
                // validated in full above and withheld, like a pending review:
                // it is never an entry of the function map, so every consumer
                // but `source.ffi.v1` (which rederives it through
                // `ffi_declarations`) sees an uncontracted callee.
            } else if let Some(relational) = loaded.relational {
                // Card CG23-RELATIONAL-POSTCONDITIONS: never an entry of the
                // function map; a consumer that knows nothing of return-code
                // dispositions sees an uncontracted callee.
                contracts.insert_relational(&decl.name, relational);
            } else if let Some(retaining) = loaded.retaining {
                // Card CALLEE-RETENTION-CLASS1: never an entry of the function
                // map; a consumer that knows nothing of retention sees an
                // uncontracted callee, so a retained pointer is never read as
                // borrowed and forgotten.
                contracts.insert_retaining(&decl.name, retaining);
            } else if loaded.review.is_empty() {
                contracts.insert_contract(&decl.name, loaded.contract);
            } else {
                pending.insert(
                    decl.name.clone(),
                    PendingHumanReview {
                        trust: ContractTrust::PendingHumanReview,
                        module: path.clone(),
                        contract: loaded.contract,
                        deferrals: loaded.review,
                    },
                );
            }
        }
    }
    contracts.set_source_assumptions(sources.clone());
    contracts.set_pending_review(pending);
    contracts.set_declared_revisions(revisions);
    Ok(Ok(contracts))
}

/// Card **GATEWAY-ANALYSIS-PATH-OPTIMIZATION**: one declared module text as
/// the environment readers below read it: the same rejections, the same
/// parse, the same mode check and every declaration's [`load_function`]
/// result, in source order, up to and including the first declaration that
/// fails to load (a reader stops there).
///
/// Each reader used to re-parse and re-lex every declared module of the
/// environment on every call, and the gateway asks them per CFG build: the
/// measured arbiter spent 43 % of its worker time there. The reading is a
/// pure function of the (path, text) pair -- every step above reads only
/// them -- so it is computed once per pair and shared; each reader's own
/// decisions (which field it collects, its duplicate check) and every error
/// string, in the order a reader meets them, are unchanged.
struct ModuleReading {
    declarations: Vec<DeclarationReading>,
}

struct DeclarationReading {
    name: String,
    /// `PATH: NAME at byte N`, the prefix of every error about it.
    context: String,
    loaded: Result<LoadedDeclaration, String>,
}

/// Readings by path, then by exact text. Bounded: past
/// [`READING_CAPACITY`] texts a reading is computed and not kept.
type ModuleReadings =
    BTreeMap<String, Vec<(String, std::sync::Arc<Result<ModuleReading, String>>)>>;

static MODULE_READINGS: std::sync::Mutex<ModuleReadings> = std::sync::Mutex::new(BTreeMap::new());

const READING_CAPACITY: usize = 4096;

fn read_module(path: &str, source: &str) -> Result<ModuleReading, String> {
    reject_retired_premise_directive(path, source)?;
    reject_unresolved_inclusion(path, source)?;
    let parsed = parse_source_program(source).map_err(|e| format!("{path}: {e}"))?;
    if parsed.mode != SourceModuleMode::Contract || parsed.declarations.is_empty() {
        return Err(format!("{path}: expected a nonempty module contract"));
    }
    let aliases = handoff::module_aliases(&parsed).map_err(|e| format!("{path}: {e}"))?;
    let mut declarations = Vec::new();
    for declaration in parsed.declarations {
        let decl = &declaration.value;
        let loaded = load_function_in(decl, &aliases);
        let failed = loaded.is_err();
        declarations.push(DeclarationReading {
            name: decl.name.clone(),
            context: format!("{path}: {} at byte {}", decl.name, declaration.span.start),
            loaded,
        });
        if failed {
            break;
        }
    }
    Ok(ModuleReading { declarations })
}

fn module_reading(path: &str, source: &str) -> std::sync::Arc<Result<ModuleReading, String>> {
    let cached = |readings: &ModuleReadings| {
        readings.get(path).and_then(|texts| {
            texts
                .iter()
                .find(|(text, _)| text == source)
                .map(|(_, reading)| std::sync::Arc::clone(reading))
        })
    };
    if let Some(reading) = cached(&MODULE_READINGS.lock().unwrap_or_else(|e| e.into_inner())) {
        return reading;
    }
    let reading = std::sync::Arc::new(read_module(path, source));
    let mut readings = MODULE_READINGS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(raced) = cached(&readings) {
        return raced;
    }
    if readings.values().map(Vec::len).sum::<usize>() < READING_CAPACITY {
        readings
            .entry(path.to_owned())
            .or_default()
            .push((source.to_owned(), std::sync::Arc::clone(&reading)));
    }
    reading
}

/// Visit every declaration of `sources` as the readers below did: module by
/// module in path order, a module's rejection first, then each declaration
/// in source order, a load failure ending the walk with `CONTEXT: ERROR`.
/// `visit` gets the name, the context and the loaded declaration, and may end
/// the walk with its own error.
fn each_loaded_declaration(
    sources: &BTreeMap<String, String>,
    mut visit: impl FnMut(&str, &str, &LoadedDeclaration) -> Result<(), String>,
) -> Result<(), String> {
    for (path, source) in sources {
        let reading = module_reading(path, source);
        let reading = reading.as_ref().as_ref().map_err(Clone::clone)?;
        for declaration in &reading.declarations {
            let loaded = declaration
                .loaded
                .as_ref()
                .map_err(|e| format!("{}: {e}", declaration.context))?;
            visit(&declaration.name, &declaration.context, loaded)?;
        }
    }
    Ok(())
}

/// The names declaring a `resource_variadic_data_tail()` in `sources`.
///
/// Derived from the same trusted texts [`load_resource_contract_sources`]
/// reads, and total-or-reject in the same way: a source set this loader would
/// reject yields an error here too, never a partial set.
pub fn variadic_data_tails(sources: &BTreeMap<String, String>) -> Result<BTreeSet<String>, String> {
    let mut tails = BTreeSet::new();
    each_loaded_declaration(sources, |name, _, loaded| {
        if loaded.variadic_data_tail {
            tails.insert(name.to_owned());
        }
        Ok(())
    })?;
    Ok(tails)
}

/// The names declaring a `resource_variadic_borrowed_tail()` in `sources`
/// (card **CG26-VARIADICS**): a subset of [`variadic_data_tails`], derived and
/// total-or-reject in the same way.
pub fn variadic_borrowed_tails(
    sources: &BTreeMap<String, String>,
) -> Result<BTreeSet<String>, String> {
    let mut tails = BTreeSet::new();
    each_loaded_declaration(sources, |name, _, loaded| {
        if loaded.variadic_borrowed_tail {
            tails.insert(name.to_owned());
        }
        Ok(())
    })?;
    Ok(tails)
}

/// The names declaring a `resource_no_return()` in `sources` (card
/// CG21-CALL-EXITS-AND-BOUNDARIES).
///
/// Derived from the same trusted texts [`load_resource_contract_sources`]
/// reads, and total-or-reject in the same way as [`variadic_data_tails`].
pub fn no_return_callees(sources: &BTreeMap<String, String>) -> Result<BTreeSet<String>, String> {
    let mut callees = BTreeSet::new();
    each_loaded_declaration(sources, |name, _, loaded| {
        if loaded.no_return {
            callees.insert(name.to_owned());
        }
        Ok(())
    })?;
    Ok(callees)
}

/// The declared C ABI projections in `sources`, keyed by function name (card
/// **LOADER-ABI**).
///
/// Derived from the same trusted texts [`load_resource_contract_sources`]
/// reads, and total-or-reject in the same way: a source set this loader would
/// reject yields an error here too, never a partial map. A consumer must
/// therefore treat an error as a refusal of every comparison, never as an
/// empty map — an empty map compares nothing, which is the fail-open
/// direction.
pub fn declared_abi_signatures(
    sources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, DeclaredAbiSignature>, String> {
    let mut signatures = BTreeMap::new();
    each_loaded_declaration(sources, |name, context, loaded| {
        if signatures
            .insert(name.to_owned(), loaded.abi.clone())
            .is_some()
        {
            return Err(format!("{context}: duplicate function contract"));
        }
        Ok(())
    })?;
    Ok(signatures)
}

/// The declared C ABI projections of an environment, rederived from its
/// **retained trusted sources**.
///
/// An environment with no declared sources — an inferred W4 summary, a
/// provider proposal, a directly constructed table — declares no signatures,
/// so nothing is compared. That is the honest reading: this loader claims no
/// declared type it cannot read, and an inferred summary never had one.
pub fn declared_abi_signatures_of(
    contracts: &ResourceContracts,
) -> Result<BTreeMap<String, DeclaredAbiSignature>, String> {
    if contracts.source_assumptions().is_empty() {
        return Ok(BTreeMap::new());
    }
    declared_abi_signatures(contracts.source_assumptions())
}

/// The literal-argument premises `sources` declares, keyed by function name
/// (card HASH-CLAUSE).
///
/// Derived from the same trusted texts [`load_resource_contract_sources`] reads
/// and total-or-reject in the same way: a source set this loader would reject
/// yields an error here too, never a partial set. A function with no premise is
/// absent from the map, which is *unrestricted* — so a consumer must treat an
/// error as a refusal rather than as an empty map.
pub fn literal_argument_premises(
    sources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, Vec<DeclaredLiteralArgument>>, String> {
    let mut premises = BTreeMap::new();
    each_loaded_declaration(sources, |name, context, loaded| {
        if !loaded.literal_arguments.is_empty() {
            // Function names are unique across a selection, so a name that
            // is already present is a duplicate declaration, which
            // `load_resource_contract_sources` rejects; refuse here too
            // rather than silently keeping one of them.
            if premises
                .insert(name.to_owned(), loaded.literal_arguments.clone())
                .is_some()
            {
                return Err(format!("{context}: duplicate function contract"));
            }
        }
        Ok(())
    })?;
    Ok(premises)
}

/// The literal-argument premises of an environment, rederived from its
/// **retained trusted sources**.
///
/// An environment with no declared sources declares no premises. That is the
/// honest reading — the loader claims no restriction it cannot read — and it is
/// why a consumer that acts on premises must refuse, not proceed, when
/// rederivation over sources that mention the clause fails.
pub fn declared_literal_argument_premises(
    contracts: &ResourceContracts,
) -> Result<BTreeMap<String, Vec<DeclaredLiteralArgument>>, String> {
    if contracts.source_assumptions().is_empty() {
        return Ok(BTreeMap::new());
    }
    literal_argument_premises(contracts.source_assumptions())
}

/// The declarations of `sources` that carry `requires resource_body_checked;`
/// (card W45-FB-BACKLOG-CUSTODY, §C06.3), keyed by function name.
///
/// Derived from the same trusted texts [`load_resource_contract_sources`]
/// reads and total-or-reject in the same way. A consumer must treat an error
/// as a refusal of every declaration it cannot classify, never as an empty
/// set: an empty set would trust a specification as a declaration, which is
/// the fail-open direction.
pub fn checked_specifications(
    sources: &BTreeMap<String, String>,
) -> Result<BTreeSet<String>, String> {
    let mut checked = BTreeSet::new();
    each_loaded_declaration(sources, |name, _, loaded| {
        if loaded.body_checked {
            checked.insert(name.to_owned());
        }
        Ok(())
    })?;
    Ok(checked)
}

/// The checked specifications of an environment, rederived from its
/// **retained trusted sources**. An environment with no declared sources has
/// none: an inferred summary is never a specification.
pub fn declared_checked_specifications(
    contracts: &ResourceContracts,
) -> Result<BTreeSet<String>, String> {
    if contracts.source_assumptions().is_empty() {
        return Ok(BTreeSet::new());
    }
    checked_specifications(contracts.source_assumptions())
}

/// The variadic data tails of an environment, rederived from its **retained
/// trusted sources**.
///
/// An environment with no declared sources — an inferred W4 summary, a provider
/// proposal, a directly constructed table — has no tails, which is the
/// fail-closed direction: the tail is a declaration, never an inference.
pub fn declared_variadic_data_tails(
    contracts: &ResourceContracts,
) -> Result<BTreeSet<String>, String> {
    if contracts.source_assumptions().is_empty() {
        return Ok(BTreeSet::new());
    }
    variadic_data_tails(contracts.source_assumptions())
}

/// The borrowed variadic tails of an environment (card **CG26-VARIADICS**),
/// rederived from its retained trusted sources exactly as
/// [`declared_variadic_data_tails`]; none without declared sources.
pub fn declared_variadic_borrowed_tails(
    contracts: &ResourceContracts,
) -> Result<BTreeSet<String>, String> {
    if contracts.source_assumptions().is_empty() {
        return Ok(BTreeSet::new());
    }
    variadic_borrowed_tails(contracts.source_assumptions())
}

/// The no-return callees of an environment, rederived from its **retained
/// trusted sources** (card CG21-CALL-EXITS-AND-BOUNDARIES). An environment
/// with no declared sources has none: the exit is a declaration, never an
/// inference.
pub fn declared_no_return_callees(
    contracts: &ResourceContracts,
) -> Result<BTreeSet<String>, String> {
    if contracts.source_assumptions().is_empty() {
        return Ok(BTreeSet::new());
    }
    no_return_callees(contracts.source_assumptions())
}

/// Recheck a declarative environment against its declared trusted sources
/// without consulting mutable files. Lives here rather than on
/// `ResourceContracts` because the recheck needs the front-end parser, which
/// the schema crate must not depend on.
///
/// Card CONTRACT-INHERIT: an environment that carries inclusion evidence is
/// also rechecked by **replaying** every elaboration from the retained texts
/// alone — no load path, no file — and requiring the replay to reproduce the
/// retained elaborated source byte for byte, the recorded chains, and the
/// consumer agreement between every inherited declaration and its parent.
/// Evidence for a module the environment does not retain is refused.
pub fn check_source_assumptions(contracts: &ResourceContracts) -> Result<(), String> {
    contracts.validate_lifetime_partition()?;
    // Card CONTRACT-SUPERSESSION: a function withheld as a contract conflict
    // is in no table; a superseded function's declaration in force is the
    // retained text's (the reload below).
    for name in contracts.contract_conflict().keys() {
        if contracts.functions().contains_key(name)
            || contracts.relational().contains_key(name)
            || contracts.retaining().contains_key(name)
            || contracts.guarded_retaining().contains_key(name)
            || contracts.pending_review().contains_key(name)
            || contracts.pin_mismatch().contains_key(name)
            || contracts.superseded().contains_key(name)
        {
            return Err(format!(
                "`{name}` is both withheld as a contract conflict and declared"
            ));
        }
    }
    for name in contracts.pin_mismatch().keys() {
        if contracts.functions().contains_key(name)
            || contracts.relational().contains_key(name)
            || contracts.retaining().contains_key(name)
            || contracts.guarded_retaining().contains_key(name)
        {
            return Err(format!(
                "`{name}` is both in force and withheld for a pin mismatch"
            ));
        }
    }
    if !contracts.inferred_lifetimes().is_empty() {
        return Err("inferred lifetime effects require independent same-unit derivation, not contract input assumptions".into());
    }
    if !contracts.source_assumptions().is_empty() {
        let loaded = load_resource_contract_sources(contracts.source_assumptions())?;
        if loaded.functions() != &crate::contract_revision::in_force_before_pins(contracts)?
            || loaded.pending_review() != contracts.pending_review()
            || loaded.declared_revisions() != contracts.declared_revisions()
            || loaded.relational() != &declared_relational(contracts)
            || loaded.retaining() != &declared_retaining(contracts, false)
            || loaded.guarded_retaining() != &declared_retaining(contracts, true)
            || loaded.declared_guarded() != contracts.declared_guarded()
        {
            return Err("resource contracts differ from their declared source assumptions".into());
        }
    }
    for (key, evidence) in contracts.inclusion_evidence() {
        let Some(elaborated_source) = contracts.source_assumptions().get(key) else {
            return Err(ContractInclusionError {
                code: InclusionCode::Qi016,
                module: key.clone(),
                message: "inclusion evidence for a module the environment does not retain".into(),
            }
            .into());
        };
        let elaborated = replay_evidence(key, evidence, elaborated_source)?;
        inherited_declarations_agree(&elaborated)?;
    }
    Ok(())
}

/// Card CG23-RELATIONAL-POSTCONDITIONS: the declared return-code
/// dispositions of an environment as their texts load them — an inferred one
/// is not a declaration, and a withdrawal is the audit's finding about a body,
/// not part of the text.
fn declared_relational(
    contracts: &ResourceContracts,
) -> BTreeMap<String, qcore_schemas::relational_post::RelationalContract> {
    contracts
        .relational()
        .iter()
        .filter(|(_, contract)| {
            contract.origin == qcore_schemas::relational_post::RelationalOrigin::Declared
        })
        .map(|(name, contract)| {
            let mut contract = contract.clone();
            contract.withdrawn = None;
            (name.clone(), contract)
        })
        // Card CALLEE-RETENTION-CLASS1: a disposition withheld for its pin is
        // still declared.
        .chain(
            contracts
                .pin_mismatch()
                .iter()
                .filter_map(|(name, withheld)| {
                    withheld
                        .relational
                        .clone()
                        .map(|relational| (name.clone(), relational))
                }),
        )
        .collect()
}

/// Card CALLEE-RETENTION-CLASS1: the declared retaining contracts, those
/// withheld for their pin included.
fn declared_retaining(
    contracts: &ResourceContracts,
    guarded: bool,
) -> BTreeMap<String, qcore_schemas::retained_by::RetainingContract> {
    contracts
        .retaining()
        .iter()
        .chain(contracts.guarded_retaining())
        .filter(|(_, contract)| contract.is_condition_bearing() == guarded)
        .map(|(name, contract)| (name.clone(), contract.clone()))
        .chain(
            contracts
                .pin_mismatch()
                .iter()
                .filter_map(|(name, withheld)| {
                    withheld
                        .retaining
                        .clone()
                        .filter(|retaining| retaining.is_condition_bearing() == guarded)
                        .map(|retaining| (name.clone(), retaining))
                }),
        )
        .collect()
}

/// The published provenance of an environment's inherited contracts (card
/// **CONTRACT-INHERIT**), for a report's assumptions block: each selected
/// module's identity, its text as written, every included module's identity
/// and text, and each inheritance chain. `None` for an environment that used
/// no inclusion, so a report without it is unchanged.
#[must_use]
pub fn inclusion_provenance(contracts: &ResourceContracts) -> Option<serde_json::Value> {
    if contracts.inclusion_evidence().is_empty() {
        return None;
    }
    Some(serde_json::json!({
        "trust": "inherited contracts are declared assumptions of the same strength as hand-written ones; each chain records where the assumption was written, not evidence that it holds",
        "modules": contracts.inclusion_evidence(),
    }))
}

/// A module selected from a published load path, by crate and logical path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PublishedModuleRef {
    pub crate_name: String,
    pub module: String,
}

impl PublishedModuleRef {
    #[must_use]
    pub fn new(crate_name: impl Into<String>, module: impl Into<String>) -> Self {
        Self {
            crate_name: crate_name.into(),
            module: module.into(),
        }
    }
}

/// Load resource-v1 contracts from modules selected on a published load path
/// (card **CONTRACT-INHERIT**).
///
/// Each selected module is elaborated — its `#include` directives resolved
/// under the publication rule, every `contract inherit PARENT;` replaced by
/// the parent's clauses under the positional renaming — and the elaborated
/// texts are loaded by the unchanged total-or-reject v1 loader, keyed
/// `crate:module`. Only the selected modules' declarations enter the
/// environment; an included module contributes parents, not functions.
///
/// Then, for every inherited declaration, the elaborated child and its
/// immediate parent are both loaded as v1 declarations and must be **equal**:
/// custody modes, result mode, literal-argument premises, variadic tail and
/// ABI projection (`QI012` otherwise). That is the check that the renaming
/// means the same thing to this consumer, which reads some positions
/// structurally rather than by parameter lookup. Equality makes every
/// inherited contract exactly as strong as its parent: never weaker.
///
/// The environment carries [`ContractInclusionEvidence`] for every selected
/// module that used inclusion, and is rechecked by
/// [`check_source_assumptions`] before it is returned.
pub fn load_published_resource_contracts(
    load_path: &ContractLoadPath,
    selection: &[PublishedModuleRef],
) -> Result<ResourceContracts, String> {
    load_resource_contract_selection(load_path, BTreeMap::new(), selection)
}

/// Load a mixed selection (card **CONTRACT-INHERIT-ADOPTION**): modules
/// already read from files, retained under their path keys exactly as
/// [`load_resource_contract_sources`] retains them, together with modules
/// selected on a published load path, elaborated and gated exactly as
/// [`load_published_resource_contracts`] does and retained under their
/// `crate:module` keys. One total-or-reject load over the union: a key
/// selected twice, or a function declared twice across the two kinds, refuses
/// the whole selection.
pub fn load_resource_contract_selection(
    load_path: &ContractLoadPath,
    path_sources: BTreeMap<String, String>,
    selection: &[PublishedModuleRef],
) -> Result<ResourceContracts, String> {
    load_resource_contract_selection_stamped(
        load_path,
        path_sources,
        selection,
        &crate::contract_supersession::Unstamped,
    )
}

/// [`load_resource_contract_selection`] with pin stamps (card
/// **CONTRACT-SUPERSESSION**). A supersession that would cut a declaration
/// out of a module elaborated through inclusion refuses the whole selection:
/// the module's inclusion evidence replays the elaborated text byte for byte,
/// and a cut text would no longer replay.
pub fn load_resource_contract_selection_stamped(
    load_path: &ContractLoadPath,
    path_sources: BTreeMap<String, String>,
    selection: &[PublishedModuleRef],
    stamps: &dyn crate::contract_supersession::DeclarationStamps,
) -> Result<ResourceContracts, String> {
    if selection.is_empty() && path_sources.is_empty() {
        return Err("no resource contract inputs selected".into());
    }
    let resolver = LoadPathResolver(load_path);
    let mut elaborator = Elaborator::new(&resolver);
    let mut sources = path_sources;
    let mut evidence = BTreeMap::new();
    let mut elaborated_modules = Vec::new();
    for reference in selection {
        let (identity, text) = load_path.owned_module(&reference.crate_name, &reference.module)?;
        let key = module_key(&identity);
        let elaborated = elaborator.elaborate(identity, &text)?;
        if sources
            .insert(key.clone(), elaborated.text.clone())
            .is_some()
        {
            return Err(format!("duplicate contract input {key}"));
        }
        if elaborated.uses_inclusion {
            evidence.insert(key, elaborated.evidence());
        }
        elaborated_modules.push(elaborated);
    }
    // Agreement first, so a renaming the v1 reading cannot accept is reported
    // as the inheritance refusal it is rather than as an anonymous v1 error.
    for elaborated in &elaborated_modules {
        inherited_declarations_agree(elaborated)?;
    }
    let mut contracts = load_resource_contract_sources_stamped(&sources, stamps)?;
    if let Some(key) = evidence
        .keys()
        .find(|key| contracts.source_assumptions().get(*key) != sources.get(*key))
    {
        return Err(format!(
            "{key}: a superseded or conflicting declaration lies in a module elaborated \
             through inclusion, whose evidence would no longer replay"
        ));
    }
    contracts.set_inclusion_evidence(evidence);
    check_source_assumptions(&contracts)?;
    Ok(contracts)
}

/// The v1 consumer's agreement check for every inherited declaration of one
/// elaborated module: the loaded child equals the loaded immediate parent.
fn inherited_declarations_agree(elaborated: &Elaborated) -> Result<(), String> {
    for child_name in elaborated.chains.keys() {
        let refuse = |message: String| -> String {
            ContractInclusionError {
                code: InclusionCode::Qi012,
                module: module_key(&elaborated.identity),
                message,
            }
            .into()
        };
        let child = elaborated
            .declaration(child_name)
            .ok_or_else(|| refuse(format!("`{child_name}` is missing after elaboration")))?;
        let parent = elaborated
            .parent_declaration(child_name)
            .ok_or_else(|| refuse(format!("the parent of `{child_name}` is missing")))?;
        let child_loaded = load_function(child)
            .map_err(|e| refuse(format!("`{child_name}` does not load as resource v1: {e}")))?;
        let parent_loaded = load_function(parent).map_err(|e| {
            refuse(format!(
                "the parent `{}` of `{child_name}` does not load as resource v1: {e}",
                parent.name
            ))
        })?;
        if child_loaded.contract != parent_loaded.contract
            || child_loaded.variadic_data_tail != parent_loaded.variadic_data_tail
            || child_loaded.variadic_borrowed_tail != parent_loaded.variadic_borrowed_tail
            || child_loaded.no_return != parent_loaded.no_return
            || child_loaded.literal_arguments != parent_loaded.literal_arguments
            // A literal-argument premise is compared above by parameter
            // index; its rendering names the child's own parameter.
            || declared_premises_of(&child_loaded) != declared_premises_of(&parent_loaded)
            || child_loaded.hypotheses.atoms.iter().filter(|(id,_)| !id.starts_with("legacy_")).collect::<Vec<_>>() != parent_loaded.hypotheses.atoms.iter().filter(|(id,_)| !id.starts_with("legacy_")).collect::<Vec<_>>()
            || child_loaded.hypotheses.refinements != parent_loaded.hypotheses.refinements
            || child_loaded.loop_owner != parent_loaded.loop_owner
            || child_loaded.thread_sync != parent_loaded.thread_sync
            || child_loaded.lifecycle != parent_loaded.lifecycle
            || child_loaded.abi != parent_loaded.abi
            || !review_inherited(&child_loaded.review, &parent_loaded.review)
            || child_loaded.body_checked != parent_loaded.body_checked
            || child_loaded.relational != parent_loaded.relational
            || child_loaded.retaining != parent_loaded.retaining
        {
            return Err(refuse(format!(
                "`{child_name}` would not mean what its parent `{}` means under the resource-v1 \
                 reading: the renamed contract loads differently",
                parent.name
            )));
        }
    }
    Ok(())
}

/// Card VERDICT-CONTRACT-DEPENDENCIES: a declaration's named premises
/// (C04.18), without the literal-argument premises rendered beside them.
fn declared_premises_of(loaded: &LoadedDeclaration) -> Vec<&DeclaredPremise> {
    loaded
        .premises
        .iter()
        .filter(|premise| premise.origin == PremiseOrigin::Declared)
        .collect()
}

fn token_kinds(source: &str) -> Result<Vec<TokenKind>, String> {
    Ok(lex(source)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| *kind != TokenKind::Eof)
        .collect())
}

/// Card **CG21-CALL-EXITS-AND-BOUNDARIES** (gateway rule 11o): C names no
/// resource v1 declaration may carry, with the reason. Each is either a
/// Clang extension form whose spelling looks like a call but which Clang
/// parses as another AST node (an ordinary function contract is never applied
/// to a non-call node), or a C/POSIX facility whose exit no v1 mode states: a
/// second return (`setjmp`) or a non-local exit (`longjmp`,
/// `pthread_exit`). Declaring one would give the call the continuation of an
/// ordinary call and hide the edge or the abandoned frame. The C gateway
/// names its refusal of an undeclared call with the same reason.
pub const UNDECLARABLE_C_NAMES: &[(&str, &str)] = &[
    (
        "__builtin_choose_expr",
        "the Clang extension node ChooseExpr",
    ),
    (
        "__builtin_bit_cast",
        "the Clang extension node BuiltinBitCastExpr",
    ),
    (
        "__builtin_offsetof",
        "the Clang extension node OffsetOfExpr",
    ),
    (
        "__builtin_types_compatible_p",
        "the Clang extension node TypeTraitExpr",
    ),
    (
        "__builtin_convertvector",
        "the Clang extension node ConvertVectorExpr",
    ),
    (
        "__builtin_shufflevector",
        "the Clang extension node ShuffleVectorExpr",
    ),
    ("__builtin_FILE", "the Clang extension node SourceLocExpr"),
    (
        "__builtin_FUNCTION",
        "the Clang extension node SourceLocExpr",
    ),
    ("__builtin_LINE", "the Clang extension node SourceLocExpr"),
    ("__builtin_COLUMN", "the Clang extension node SourceLocExpr"),
    ("setjmp", "a returns-twice call"),
    ("_setjmp", "a returns-twice call"),
    ("__setjmp", "a returns-twice call"),
    ("sigsetjmp", "a returns-twice call"),
    ("__sigsetjmp", "a returns-twice call"),
    ("savectx", "a returns-twice call"),
    ("vfork", "a returns-twice call"),
    ("getcontext", "a returns-twice call"),
    ("longjmp", "a non-local exit"),
    ("_longjmp", "a non-local exit"),
    ("siglongjmp", "a non-local exit"),
    ("__longjmp_chk", "a non-local exit"),
    ("setcontext", "a non-local exit"),
    ("swapcontext", "a non-local exit"),
    ("pthread_exit", "a non-local exit"),
    ("thrd_exit", "a non-local exit"),
];

/// The reason `name` may carry no resource v1 declaration, if it is one of
/// [`UNDECLARABLE_C_NAMES`].
pub fn undeclarable_c_name(name: &str) -> Option<&'static str> {
    UNDECLARABLE_C_NAMES
        .iter()
        .find(|(listed, _)| *listed == name)
        .map(|(_, reason)| *reason)
}

fn load_function(decl: &SourceDeclaration) -> Result<LoadedDeclaration, String> {
    load_function_in(decl, &BTreeMap::new())
}

/// [`load_function`] with the module's aliases (card
/// SUPPLIED-OWNED-RETURN-CODE-HANDOFF): a hand-off guard's `#NAME` resolves
/// against them; without them (an inheritance comparison) it rejects.
fn load_function_in(
    decl: &SourceDeclaration,
    aliases: &BTreeMap<String, String>,
) -> Result<LoadedDeclaration, String> {
    if decl.kind != DeclarationKind::ExternFunction {
        return Err("resource modules accept only extern functions".into());
    }
    // Card CG21-CALL-EXITS-AND-BOUNDARIES: no declaration for a name whose
    // exit or node no v1 mode states.
    if let Some(reason) = undeclarable_c_name(&decl.name) {
        return Err(format!(
            "`{}` is {reason}, which no resource v1 declaration states",
            decl.name
        ));
    }
    // Card W48-QR-C-STRUCT-IMPORT: a marked declaration is read by its own,
    // narrower vocabulary, and only a marked one may name a C struct.
    if decl.c_ffi.is_some() {
        return load_ffi_function(decl);
    }
    let lifetime = crate::local_condition::read(decl)?;
    if !lifetime.bindings.is_empty()
        || !lifetime.registered.is_empty()
        || !lifetime.blocks.is_empty()
        || !lifetime.cells.is_empty()
    {
        if decl.parameters.iter().any(|p| p.usage.is_some()) {
            return Err(
                "local-condition-signature: usage annotations are not a resource-v1 ABI".into(),
            );
        }
        let base = crate::local_condition::without_local(decl, None)?;
        let mut loaded = load_function_in(&base, aliases)?;
        if loaded.relational.is_some()
            || loaded.ffi.is_some()
            || matches!(loaded.contract.result, ResourceReturnMode::Removed { .. })
        {
            return Err(
                "local-condition-contract: incompatible conditional ABI or return disposition"
                    .into(),
            );
        }
        for block in &lifetime.blocks {
            for clause in &block.clauses {
                if let qcore_schemas::loop_lifetime::LocalClause::Prerequisite(text) = clause {
                    // Existing obligation readers remain the authority for inner
                    // syntax. This validation grants no premise to the caller.
                    let checked = crate::local_condition::without_local(decl, Some(text))?;
                    load_function_in(&checked, aliases)?;
                }
            }
        }
        let mut retaining = loaded.retaining.take().unwrap_or_else(|| {
            qcore_schemas::retained_by::RetainingContract::custody_only(loaded.contract.clone())
        });
        retaining.lifetime = Some(lifetime);
        loaded.retaining = Some(retaining);
        return Ok(loaded);
    }
    let clauses = &decl
        .function_contract
        .as_ref()
        .ok_or("missing resource contract")?
        .clauses;
    // Reconstruct the entire declaration, allowing only plain C ABI types and
    // the exact parsed contract block. No ignored signature/tail syntax.
    // `unit` is a result type only, and usage annotations belong to the
    // language-level lifecycle vocabulary, not to this projection.
    let mut parameter_types = Vec::with_capacity(decl.parameters.len());
    for parameter in &decl.parameters {
        let tokens = token_kinds(&parameter.type_source)?;
        let matched =
            parameter_abi_types().find(|ty| token_kinds(ty).ok().as_ref() == Some(&tokens));
        let Some(matched) = matched.filter(|_| parameter.usage.is_none()) else {
            // Card W48-QR-C-STRUCT-IMPORT: an unmarked declaration naming a C
            // struct is refused by name — the struct table is the marked
            // vocabulary's alone (user decision 4).
            if tokens
                .iter()
                .any(|kind| matches!(kind, TokenKind::StringLiteral(_)))
            {
                return Err(
                    "a C struct type is admitted only on a declaration marked `[| quant #c_ffi |]`"
                        .into(),
                );
            }
            return Err("unsupported parameter type or usage annotation".into());
        };
        // Card LOADER-ABI: the canonical table spelling, not the module's own
        // text, so a consumer comparing against a C declaration compares one
        // closed vocabulary rather than incidental whitespace.
        parameter_types.push(matched.to_owned());
    }
    let parameter_text = decl
        .parameters
        .iter()
        .map(|p| format!("{}: {}", p.name.value, p.type_source))
        .collect::<Vec<_>>()
        .join(", ");
    let clause_text = clauses
        .iter()
        .map(|c| {
            let prefix = match &c.value.kind {
                SourceContractClauseKind::Requires => "requires",
                SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal) => {
                    "ensures normal"
                }
                // Card OP-CONSTRUCTOR-DECLARATIONS (§C04.19.4): a result
                // state, read below; every other `quant` form still rejects.
                SourceContractClauseKind::QuantLive => "quant live",
                SourceContractClauseKind::QuantProposition => "quant proposition",
                _ => return Err("unsupported error, any-exit, or quantitative clause"),
            };
            Ok(format!("{prefix} {};", c.value.expression_source))
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(" ");
    let return_type = result_abi_types()
        .find(|ty| {
            let expected = format!(
                "extern fn {}({parameter_text}) -> {ty} contract {{ {clause_text} }};",
                decl.name
            );
            token_kinds(&expected).ok().as_ref() == Some(&decl.tokens)
        })
        .ok_or("unsupported signature, effects, or declaration suffix")?;

    let mut marker = false;
    let mut body_checked = false;
    let mut variadic_data_tail = false;
    let mut variadic_borrowed_tail = false;
    // Card CG21-CALL-EXITS-AND-BOUNDARIES.
    let mut no_return = false;
    let mut literal_arguments: Vec<DeclaredLiteralArgument> = Vec::new();
    // Card VERDICT-CONTRACT-DEPENDENCIES: the named premises, in clause order.
    let mut premises: Vec<DeclaredPremise> = Vec::new();
    // Validate the whole graph before accepting any of its clauses.
    let graph_premises = decl
        .function_contract
        .iter()
        .flat_map(|c| &c.clauses)
        .map(|c| premise_clause(&c.value.expression_source))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let graph = crate::hypothesis::declaration(decl, &graph_premises)?;
    if !graph.requirements.is_empty() {
        return Err("formal entry obligations have no interpreted source valuation; no summary is installed".into());
    }

    crate::premise_flow::declaration(decl, &graph)?;
    crate::contract_lifecycle::declaration(decl, &graph)?;
    crate::guarded_decomposition::declaration(decl, &graph)?;
    let mut loop_owner: Option<DeclaredLoopOwner> = None;
    let mut thread_sync: Option<DeclaredThreadSync> = None;
    let mut lifecycle: Option<DeclaredLifecycle> = None;
    let mut parameters = vec![None; decl.parameters.len()];
    let mut result = None;
    // Card W43-DESCRIPTOR-RESOURCE-TRACKING: which validity vocabulary each
    // position's nullability word was spelled in, and each declared kind.
    let mut parameter_words: Vec<Option<ValidityWord>> = vec![None; decl.parameters.len()];
    let mut result_word: Option<ValidityWord> = None;
    let mut parameter_kinds: Vec<Option<ResourceKind>> = vec![None; decl.parameters.len()];
    let mut result_kind: Option<ResourceKind> = None;
    let mut names = BTreeMap::new();
    for (index, parameter) in decl.parameters.iter().enumerate() {
        if names.insert(parameter.name.value.as_str(), index).is_some() {
            return Err("duplicate parameter name".into());
        }
    }
    let index = |name: &str| {
        names
            .get(name)
            .copied()
            .ok_or_else(|| format!("unknown parameter {name}"))
    };
    let mut review: Vec<ReviewDeferral> = Vec::new();
    let mut revision: Vec<qcore_schemas::contract_revision::RevisionMarker> = Vec::new();
    let mut collected = counted_users::Collected::default();
    // Card CG23-RELATIONAL-POSTCONDITIONS: every `resource_on_return` clause,
    // in clause order.
    let mut conditional: Vec<(qcore_schemas::relational_post::ReturnRelation, String)> = Vec::new();
    // Card CALLEE-RETENTION-CLASS1: every `resource_retained_by(p, h)` and
    // `resource_holder_reset(h)`, as written, in clause order.
    let mut retentions: Vec<(String, String, Option<retained_by::WrittenGuard>)> = Vec::new();
    let mut resets: Vec<String> = Vec::new();
    // Card SLOT-MOVE-AND-INTERIOR-BORROW: every `resource_slot_moved(s, n)` and
    // `resource_out_derived(s, p, n)`, as written, in clause order.
    let mut slot_clauses: Vec<slot_move::WrittenSlot> = Vec::new();
    // Card REFCOUNT-RESOURCE-KIND: every `resource_reference_kept(p)` and
    // `resource_reference_dropped(p)`, as written, in clause order.
    let mut reference_clauses: Vec<reference_count::WrittenReference> = Vec::new();
    // Card CG22-CALLBACK-REGISTRY-CONTRACT: every callback clause, as
    // written, in clause order.
    let mut callback_clauses: Vec<callback::WrittenCallback> = Vec::new();
    // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES: the discriminant and the arm
    // clauses, as written, in clause order.
    let mut discriminants: Vec<arm_release::WrittenDiscriminant> = Vec::new();
    let mut arm_clauses: Vec<arm_release::WrittenArm> = Vec::new();
    // Card OP-CONSTRUCTOR-DECLARATIONS: the result-state clauses, as written,
    // in clause order (both levels of rule 14e's shorthand).
    let mut constructed_clauses: Vec<constructed_state::Written> = Vec::new();
    // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: the hand-off clauses, as
    // written, their constants canonical.
    let mut handoff_clauses: Vec<handoff::Written> = Vec::new();
    // Card OP-CALLBACK-MEMBER-PATH-RELEASE: the member-path states, as
    // written, their guards' constants canonical.
    let mut member_path_clauses: Vec<member_path::Written> = Vec::new();
    // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (§C04.14): holder-path
    // retentions and removals of one registration, as written.
    let mut path_retentions: Vec<retained_by::WrittenPath> = Vec::new();
    let mut removals: Vec<(String, String)> = Vec::new();
    // Card MK-EVENT-DESCRIPTOR-QUANT (§C04.6.2, rule 38): the descriptor
    // cell clauses (both levels), the guarded releases and the removal
    // guards, as written.
    let mut cell_clauses: Vec<descriptor_cell::Written> = Vec::new();
    let mut cell_releases: Vec<(String, descriptor_cell::WrittenEquality)> = Vec::new();
    let mut removal_guards: Vec<descriptor_cell::WrittenEquality> = Vec::new();
    for clause in clauses {
        if clause.value.kind == SourceContractClauseKind::QuantProposition {
            // The whole graph was type checked above; this is an explicit
            // hypothesis representation, never an interpreted C effect.
            continue;
        }
        if clause.value.kind == SourceContractClauseKind::QuantLive {
            if descriptor_cell::is_guarded_removal(&clause.value.expression_source) {
                let (holder, retained, guard) =
                    descriptor_cell::guarded_removal_clause(&clause.value.expression_source)?;
                removals.push((holder, retained));
                removal_guards.push(guard);
            } else if descriptor_cell::is_cell_clause(&clause.value.expression_source) {
                cell_clauses.push(descriptor_cell::quant_cell_clause(
                    &clause.value.expression_source,
                )?);
            } else if retained_by::is_removal(&clause.value.expression_source) {
                removals.push(retained_by::removal_clause(
                    &clause.value.expression_source,
                )?);
            } else if member_path::is_member_path(&clause.value.expression_source) {
                member_path_clauses.push(member_path::quant_live_clause(
                    &clause.value.expression_source,
                    aliases,
                )?);
            } else if handoff::binds_a_formal(&clause.value.expression_source) {
                handoff_clauses.push(handoff::quant_live_clause(
                    &clause.value.expression_source,
                    aliases,
                )?);
            } else {
                constructed_clauses.extend(constructed_state::quant_live_clause(
                    &clause.value.expression_source,
                )?);
            }
            continue;
        }
        let requires = clause.value.kind == SourceContractClauseKind::Requires;
        // Card W45-CONTRACT-REVISION-MODE: a revision marker, contract-level (a
        // requirement) or clause-level (wrapping one `ensures normal` clause,
        // which is then read exactly as if written bare).
        let revision_read =
            crate::contract_revision::revision_marker(requires, &clause.value.expression_source)?;
        let written: &str = match &revision_read {
            None => &clause.value.expression_source,
            Some(crate::contract_revision::RevisionMarkerRead::Contract(mode)) => {
                revision.push(qcore_schemas::contract_revision::RevisionMarker {
                    scope: qcore_schemas::contract_revision::RevisionScope::Contract,
                    mode: *mode,
                });
                continue;
            }
            Some(crate::contract_revision::RevisionMarkerRead::Clause { inner, mode }) => {
                revision.push(qcore_schemas::contract_revision::RevisionMarker {
                    scope: qcore_schemas::contract_revision::RevisionScope::Clause {
                        clause: format!("ensures normal {inner}"),
                    },
                    mode: *mode,
                });
                inner
            }
        };
        // Card W45-CONTRACT-DEFERRED-REVIEW: a review marker, contract-level
        // (a requirement) or clause-level (wrapping one `ensures normal`
        // clause, which is then read exactly as if written bare).
        let unwrapped = review_marker(requires, written)?;
        if let Some(ReviewMarker::Clause { inner, .. }) = &unwrapped
            && crate::contract_revision::starts_with_revision_marker(inner)
        {
            return Err(format!(
                "unsupported review marker `{written}`: a marker cannot wrap another marker"
            ));
        }
        let expression: &str = match &unwrapped {
            None => written,
            Some(ReviewMarker::Contract(deferral)) => {
                if !review.is_empty() {
                    return Err(DUPLICATE_REVIEW_MARKER.into());
                }
                review.push(deferral.clone());
                continue;
            }
            Some(ReviewMarker::Clause { inner, deferral }) => {
                if review
                    .iter()
                    .any(|earlier| earlier.scope == ReviewScope::Contract)
                {
                    return Err(DUPLICATE_REVIEW_MARKER.into());
                }
                review.push(deferral.clone());
                inner
            }
        };
        if clause.value.kind == SourceContractClauseKind::Requires {
            if token_kinds(expression)? == token_kinds("resource_contract_v1")? {
                if marker {
                    return Err(format!(
                        "unsupported or duplicate requirement `{expression}`"
                    ));
                }
                marker = true;
                continue;
            }
            // Card W45-FB-BACKLOG-CUSTODY: the checked-specification marker, a
            // contract-level requirement with no argument, at most once.
            if token_kinds(expression)? == token_kinds(BODY_CHECKED_MARKER)? {
                if body_checked {
                    return Err(format!("duplicate requirement `{expression}`"));
                }
                body_checked = true;
                continue;
            }
            // Card W47-LOOP-OWNER-CONTRACTS: the loop-owner obligation, at
            // most once, naming a pointer parameter.
            if let Some((parameter, operation)) = loop_owner_clause(expression)? {
                if loop_owner.is_some() {
                    return Err(format!("duplicate requirement `{expression}`"));
                }
                let parameter = index(&parameter)?;
                if !POINTER_ABI_TYPES.contains(&parameter_types[parameter].as_str()) {
                    return Err(format!(
                        "a loop-owner obligation names an instance pointer, not `{}`",
                        decl.parameters[parameter].name.value
                    ));
                }
                loop_owner = Some(DeclaredLoopOwner {
                    parameter,
                    operation,
                });
                continue;
            }
            // Card W47-CONDVAR-JOIN-PUBLICATION: the thread-synchronization
            // obligation, at most once; a condvar and a mutex are pointer
            // parameters, a joined thread a scalar one.
            if let Some((objects, operation)) = thread_sync_clause(expression)? {
                if thread_sync.is_some() {
                    return Err(format!("duplicate requirement `{expression}`"));
                }
                thread_sync = Some(thread_sync_declaration(
                    decl,
                    &parameter_types,
                    &objects,
                    operation,
                    |name| index(name),
                )?);
                continue;
            }
            // Card W47-FLB-LIB-LIFECYCLE: the lifecycle obligation, at most
            // once, naming the context pointer.
            if let Some((parameter, operation)) = lifecycle_clause(expression)? {
                if lifecycle.is_some() {
                    return Err(format!("duplicate requirement `{expression}`"));
                }
                let parameter = index(&parameter)?;
                if parameter_types[parameter] != "ptr<mut, u8>" {
                    return Err(format!(
                        "a lifecycle obligation names a context pointer, not `{}`",
                        decl.parameters[parameter].name.value
                    ));
                }
                lifecycle = Some(DeclaredLifecycle {
                    parameter,
                    operation,
                });
                continue;
            }
            // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (§C04.19.2): the
            // discriminant of a type-switched release; validated below with
            // its arms.
            if let Some(written) = arm_release::discriminant_clause(expression)? {
                discriminants.push(written);
                continue;
            }
            // Card VERDICT-CONTRACT-DEPENDENCIES: a named premise, at most
            // once per name. It narrows nothing and widens nothing: it names
            // an assumption the declaration rests on, for the report.
            if crate::hypothesis::is_clause(expression)
                || crate::premise_flow::is_clause(expression)
                || crate::contract_lifecycle::is_clause(expression)
                || crate::guarded_decomposition::is_source_clause(expression)
            {
                continue;
            }
            if let Some(premise) = premise_clause(expression)? {
                if premises
                    .iter()
                    .any(|declared: &DeclaredPremise| declared.name == premise.name)
                {
                    return Err(format!(
                        "duplicate premise `{}`: a declaration names each premise once",
                        premise.name
                    ));
                }
                premises.push(premise);
                continue;
            }
            // Card HASH-CLAUSE. The only other admitted precondition: the
            // declaration applies at a call site only if the actual at the
            // named parameter is exactly the named literal. Anything else is
            // still an unsupported requirement, so the whole module rejects.
            let (parameter, literal) = literal_argument_clause(expression)?;
            let parameter = index(&parameter)?;
            if literal_arguments
                .iter()
                .any(|declared| declared.parameter == parameter)
            {
                return Err(format!(
                    "duplicate literal-argument premise for {}",
                    decl.parameters[parameter].name.value
                ));
            }
            literal_arguments.push(DeclaredLiteralArgument { parameter, literal });
            continue;
        }
        // Card MK-EVENT-DESCRIPTOR-QUANT (§C04.6.2, rule 38): a release
        // guarded by a member equality, and the member-cell words of the lower
        // level, read before every other guarded or dotted clause.
        if let Some(release) = descriptor_cell::guarded_release(expression)? {
            cell_releases.push(release);
            continue;
        }
        if let Some(written) = descriptor_cell::lower_clause(expression)? {
            cell_clauses.push(written);
            continue;
        }
        // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (§C04.19.2): an arm
        // clause, guarded by `when FIELD == VALUE` or `otherwise`, read before
        // the retention guard (which refuses every other guarded clause).
        if let Some(written) = arm_release::arm_clause(expression)? {
            arm_clauses.push(written);
            continue;
        }
        // Card OP-CONSTRUCTOR-DECLARATIONS (§C04.19.4): the discriminant set
        // at construction and a member registration, whose dotted member the
        // identifier-only predicate reader does not read.
        if let Some(written) = constructed_state::ensures_clause(expression)? {
            constructed_clauses.push(written);
            continue;
        }
        // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (§C04.19.3): a member
        // invocation, whose dotted member the identifier-only predicate reader
        // does not read.
        if let Some(written) = callback::member_clause(expression)? {
            callback_clauses.push(written);
            continue;
        }
        // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (§C04.14, rule 12t):
        // a retention whose holder is a path below a formal.
        if let Some(written) = retained_by::path_clause(expression)? {
            path_retentions.push(written);
            continue;
        }
        // Card W43-ABI-ENUM-AND-CONST-PP (§C04.14.1, rule 12w): a guarded
        // clause, `CLAUSE when X OP K`. Only a retention takes a guard; any
        // other guarded clause, and any other use of `when`, rejects.
        if let Some((guarded, guard)) = retained_by::split_guard(expression)? {
            match predicate_call(guarded)?.as_slice() {
                [name, p, h] if name == retained_by::RETAINED_BY => {
                    retentions.push((p.clone(), h.clone(), Some(guard)));
                    continue;
                }
                _ => {
                    return Err(format!(
                        "a guard stands only on `{}`: `{expression}`",
                        retained_by::RETAINED_BY
                    ));
                }
            }
        }
        // Card CG23-RELATIONAL-POSTCONDITIONS: a return-code disposition
        // (§C04.13), read before the identifier-only predicate reader.
        if let Some(disposition) = relational::clause(expression)? {
            conditional.push(disposition);
            continue;
        }
        // Card W45-FB-RETRY-LIFETIME: the counted-user and container clauses
        // (§C04.7, §C04.8). The exit clause carries a value, so it is read
        // before the identifier-only predicate reader.
        if let Some(clause) = counted_users::parse(None, expression)? {
            counted_users::record(&mut collected, clause, &index)?;
            continue;
        }
        let words = predicate_call(expression)?;
        let words = words.iter().map(String::as_str).collect::<Vec<_>>();
        if let Some(clause) = counted_users::parse(Some(&words), expression)? {
            counted_users::record(&mut collected, clause, &index)?;
            continue;
        }
        // Card CALLEE-RETENTION-CLASS1 (§C04.14): the retention clauses, read
        // before the three-word parameter predicates; validated below once
        // every mode is read.
        match words.as_slice() {
            [retained_by::RETAINED_BY, p, h] => {
                retentions.push(((*p).to_owned(), (*h).to_owned(), None));
                continue;
            }
            [retained_by::HOLDER_RESET, h] => {
                resets.push((*h).to_owned());
                continue;
            }
            _ => {}
        }
        // Card SLOT-MOVE-AND-INTERIOR-BORROW (§C04.15, §C04.16): the slot
        // clauses, read before the parameter predicates; validated below once
        // every mode is read.
        if let Some(written) = slot_move::clause(&words) {
            slot_clauses.push(written);
            continue;
        }
        // Card REFCOUNT-RESOURCE-KIND (§C04.17): the reference clauses, read
        // before the parameter predicates; validated below once every mode is
        // read.
        if let Some(written) = reference_count::clause(&words) {
            reference_clauses.push(written);
            continue;
        }
        // Card CG22-CALLBACK-REGISTRY-CONTRACT (§C04.19): the callback
        // clauses, read before the parameter predicates; validated below once
        // every mode is read.
        if let Some(written) = callback::clause(&words) {
            callback_clauses.push(written);
            continue;
        }
        // The variadic data tail is a *whole-declaration* fact, not a parameter
        // or result mode: it says what the arguments the declaration does not
        // name carry. It takes no argument (a tail has no index to point at)
        // and, like every other clause, may occur at most once.
        if words.as_slice() == ["resource_variadic_data_tail"] {
            if variadic_data_tail {
                return Err("duplicate variadic data tail".into());
            }
            variadic_data_tail = true;
            continue;
        }
        // Card CG26-VARIADICS: the borrowed tail states that every trailing
        // argument is `Borrowed` (no custody taken, no pointer retained), so a
        // consumer may read a tracked identity there as a borrow. It is the
        // data tail strengthened, never beside it: one spelling per meaning.
        if words.as_slice() == ["resource_variadic_borrowed_tail"] {
            if variadic_data_tail {
                return Err("duplicate variadic data tail".into());
            }
            variadic_data_tail = true;
            variadic_borrowed_tail = true;
            continue;
        }
        // Card CG21-CALL-EXITS-AND-BOUNDARIES: the no-return exit, a
        // whole-declaration fact like the tail; its premises are checked once
        // every mode is read, below.
        if words.as_slice() == ["resource_no_return"] {
            if no_return {
                return Err("duplicate no-return exit".into());
            }
            no_return = true;
            continue;
        }
        // Card W43-DESCRIPTOR-RESOURCE-TRACKING: the two kind clauses. Heap is
        // spelled by the absence of a clause, so `heap` itself is refused: one
        // spelling per meaning.
        match words.as_slice() {
            ["resource_kind", p, kind] => {
                let kind = declared_kind(kind)?;
                if parameter_kinds[index(p)?].replace(kind).is_some() {
                    return Err(format!("duplicate kind for {p}"));
                }
                continue;
            }
            ["resource_return_kind", kind] => {
                let kind = declared_result_kind(kind)?;
                if result_kind.replace(kind).is_some() {
                    return Err("duplicate result kind".into());
                }
                continue;
            }
            _ => {}
        }
        let return_mode = match words.as_slice() {
            ["resource_return_none"] => Some((ResourceReturnMode::None, None)),
            ["resource_return_owned", null] => {
                let (nullable, word) = validity(null)?;
                Some((ResourceReturnMode::Owned { nullable }, Some(word)))
            }
            ["resource_return_removed", parameter, null] => {
                let (nullable, word) = validity(null)?;
                Some((
                    ResourceReturnMode::Removed {
                        parameter: index(parameter)?,
                        nullable,
                    },
                    Some(word),
                ))
            }
            ["resource_return_reallocated", parameter, null] => {
                let (nullable, word) = validity(null)?;
                Some((
                    ResourceReturnMode::Reallocated {
                        parameter: index(parameter)?,
                        nullable,
                    },
                    Some(word),
                ))
            }
            // Card W43-LIBC-RESULT-ALIAS. `const_dropped` is decided below,
            // from the declared types, once the whole signature is known.
            // Card SLOT-MOVE-AND-INTERIOR-BORROW (§C04.3.4): the borrowed
            // result of class 8 is the same mode at the `Borrowed` extent.
            [
                name @ ("resource_return_alias"
                | "resource_return_interior"
                | slot_move::RETURN_BORROWED_FROM),
                parameter,
                null,
            ] => {
                let (nullable, word) = validity(null)?;
                Some((
                    ResourceReturnMode::Derived {
                        parameter: index(parameter)?,
                        extent: match *name {
                            "resource_return_alias" => DerivedExtent::Exact,
                            "resource_return_interior" => DerivedExtent::Interior,
                            _ => DerivedExtent::Borrowed,
                        },
                        nullable,
                        const_dropped: false,
                    },
                    Some(word),
                ))
            }
            _ => None,
        };
        if let Some((mode, word)) = return_mode {
            if result.replace(mode).is_some() {
                return Err("duplicate result mode".into());
            }
            result_word = word;
            continue;
        }
        let (parameter, mode, word) = match words.as_slice() {
            ["resource_borrowed", _] | [_, _, _] => plain_parameter_predicate(&words, expression)?,
            [
                name @ ("resource_stored" | "resource_removed"),
                p,
                owner,
                null,
            ] => {
                let owner_parameter = index(owner)?;
                let (nullable, word) = validity(null)?;
                (
                    *p,
                    if *name == "resource_stored" {
                        ResourceParameterMode::Stored {
                            owner_parameter,
                            nullable,
                        }
                    } else {
                        ResourceParameterMode::Removed {
                            owner_parameter,
                            nullable,
                        }
                    },
                    Some(word),
                )
            }
            _ => return Err(format!("unsupported predicate `{expression}`")),
        };
        let position = index(parameter)?;
        if parameters[position].replace(mode).is_some() {
            return Err(format!("duplicate mode for {parameter}"));
        }
        parameter_words[position] = word;
    }
    if !marker {
        return Err("missing resource_contract_v1 requirement".into());
    }
    let result = result.ok_or("missing result mode")?;
    // Card MK-EVENT-DESCRIPTOR-QUANT (§C04.6.2, rule 38): a slot cell's lower
    // level (its kind and its two return-code dispositions), a slot binder's
    // kind, and a guarded release's kind are the cell's, not a position's:
    // they move into the cell clauses, and the formal's custody mode is
    // `resource_borrowed` (the route applies the cell's effect).
    {
        let slots = descriptor_cell::slot_binders(&cell_clauses);
        for (position, parameter) in decl.parameters.iter().enumerate() {
            let name = parameter.name.value.as_str();
            let Some(kind) = parameter_kinds[position] else {
                continue;
            };
            if cell_releases.iter().any(|(released, _)| released == name) {
                if kind != ResourceKind::Descriptor || parameters[position].is_some() {
                    return Err(format!(
                        "quant-live-cell-equality: a guarded release of `{name}` states its kind \
                         `descriptor` and no other mode"
                    ));
                }
                parameter_kinds[position] = None;
                parameters[position] = Some(ResourceParameterMode::Borrowed);
                continue;
            }
            if slots.iter().any(|slot| slot == name) {
                cell_clauses.push(descriptor_cell::Written::Kind {
                    binder: name.to_owned(),
                    place: descriptor_cell::WrittenPlace::Pointee,
                    kind,
                    text: format!("resource_kind({name}, {kind:?})"),
                });
                parameter_kinds[position] = None;
                continue;
            }
            if parameter_types[position] != "ptr<mut, u8>" || parameters[position].is_some() {
                continue;
            }
            let mut dispositions = Vec::new();
            for (at, (relation, inner)) in conditional.iter().enumerate() {
                let words = predicate_call(inner)?;
                if words.get(1).map(String::as_str) == Some(name) {
                    dispositions.push((at, *relation, inner.clone()));
                }
            }
            if let Some(written) = descriptor_cell::slot_lower_level(name, kind, &dispositions) {
                for (at, ..) in dispositions.iter().rev() {
                    conditional.remove(*at);
                }
                cell_clauses.extend(written);
                parameter_kinds[position] = None;
                parameters[position] = Some(ResourceParameterMode::Borrowed);
            }
        }
        for (released, _) in &cell_releases {
            let position = index(released)?;
            if parameters[position].is_none() {
                return Err(format!(
                    "quant-live-cell-equality: a guarded release of `{released}` states its kind \
                     `descriptor`"
                ));
            }
        }
    }
    // Card CG23-RELATIONAL-POSTCONDITIONS: one parameter vector per exit
    // class, each finished by the same checks as a plain declaration.
    let relational_classes = if conditional.is_empty() {
        None
    } else {
        if !review.is_empty()
            || body_checked
            || variadic_data_tail
            || !literal_arguments.is_empty()
            || loop_owner.is_some()
            || thread_sync.is_some()
            || lifecycle.is_some()
            || !collected.users.is_empty()
            || !collected.container_effects.is_empty()
        {
            return Err(
                "a return-code disposition stands only beside plain parameter and result \
                 predicates and kinds"
                    .into(),
            );
        }
        // Card CG25-RESOURCE-KINDS composes: an in-place initialiser's status
        // is already its kind's own witness (rule 11v), so a disposition never
        // stands beside an in-place kind, which would read one status twice.
        if parameter_kinds
            .iter()
            .flatten()
            .any(|kind| kind.is_in_place())
        {
            return Err(
                "a return-code disposition does not stand beside an in-place kind, whose \
                 initialiser status is its own witness (rule 11v)"
                    .into(),
            );
        }
        if result != ResourceReturnMode::None || !relational::SIGNED_RESULTS.contains(&return_type)
        {
            return Err(
                "a return-code disposition requires `resource_return_none()` over a signed \
                 integer result"
                    .into(),
            );
        }
        let mut relations: Vec<qcore_schemas::relational_post::ReturnRelation> = Vec::new();
        for (relation, _) in &conditional {
            if !relations.contains(relation) {
                relations.push(*relation);
            }
        }
        let mut classes = Vec::with_capacity(relations.len());
        for relation in &relations {
            let mut modes = parameters.clone();
            let mut words = parameter_words.clone();
            for (_, inner) in conditional.iter().filter(|(r, _)| r == relation) {
                let predicate = predicate_call(inner)?;
                let predicate = predicate.iter().map(String::as_str).collect::<Vec<_>>();
                let (parameter, mode, word) = plain_parameter_predicate(&predicate, inner)?;
                let position = index(parameter)?;
                if parameters[position].is_some() {
                    return Err(format!(
                        "{parameter} has an unconditional mode and a return-code disposition"
                    ));
                }
                if modes[position].replace(mode).is_some() {
                    return Err(format!(
                        "duplicate mode for {parameter} in one return-code class"
                    ));
                }
                words[position] = word;
            }
            let modes = modes
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .ok_or("a return-code class leaves a parameter without a mode")?;
            classes.push((*relation, modes, words));
        }
        for position in (0..parameters.len()).filter(|position| parameters[*position].is_none()) {
            let requirements = classes
                .iter()
                .map(|(_, modes, _)| relational::entry_requirement(modes[position]))
                .collect::<Vec<_>>();
            if requirements.iter().any(Option::is_none)
                || requirements.windows(2).any(|pair| pair[0] != pair[1])
            {
                return Err(format!(
                    "the return-code classes disagree on what {} requires at entry",
                    decl.parameters[position].name.value
                ));
            }
        }
        Some(classes)
    };
    let classes = match relational_classes {
        Some(classes) => classes,
        None => vec![(
            qcore_schemas::relational_post::ReturnRelation::new(
                qcore_schemas::relational_post::RelationOp::Ne,
                0,
            ),
            parameters
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .ok_or("missing parameter mode")?,
            parameter_words.clone(),
        )],
    };
    let is_relational = !conditional.is_empty();
    let mut finished = Vec::with_capacity(classes.len());
    for (relation, parameters, parameter_words) in classes {
        let mut result = result;
        // Card W43-DESCRIPTOR-RESOURCE-TRACKING: where a kind may stand, and which
        // validity words and ABI types it takes. Checked before the pointer-only
        // rules below, to which an integer-carried kind is the one exception.
        let kinds = check_kinds(
            &KindPositions {
                parameters: &parameters,
                parameter_types: &parameter_types,
                parameter_words: &parameter_words,
                parameter_kinds: &parameter_kinds,
            },
            result,
            return_type,
            result_word,
            result_kind,
        )?;
        let integer_parameter = |i: usize| kinds.parameter_kind(i).is_integer();
        if result != ResourceReturnMode::None
            && !return_type.starts_with("ptr<")
            && !kinds.result_kind.is_integer()
        {
            return Err("owned result requires a pointer return type".into());
        }
        // Card W43-LIBC-RESULT-ALIAS. A derived result names a borrowed base of
        // depth one and is itself a pointer of depth one: the base passes no
        // custody through the call, so the relation is the only thing the result
        // carries, and a pointer-to-pointer or function pointer has no object for
        // it to point into.
        if let ResourceReturnMode::Derived {
            parameter,
            extent,
            ref mut const_dropped,
            ..
        } = result
        {
            const DEPTH_ONE: [&str; 2] = ["ptr<mut, u8>", "ptr<const, u8>"];
            if parameters[parameter] != ResourceParameterMode::Borrowed {
                return Err("a derived result requires a borrowed base parameter".into());
            }
            // Card SLOT-MOVE-AND-INTERIOR-BORROW (§C04.3.4): a borrowed result
            // may also be a pointer to the parent's array of child pointers.
            let results: &[&str] = if extent == DerivedExtent::Borrowed {
                &slot_move::BORROWED_RESULTS
            } else {
                &DEPTH_ONE
            };
            if !DEPTH_ONE.contains(&parameter_types[parameter].as_str())
                || !results.contains(&return_type)
            {
                return Err("a derived result requires a depth-one pointer base and result".into());
            }
            *const_dropped = parameter_types[parameter] == "ptr<const, u8>"
                && return_type.starts_with("ptr<mut,");
        }
        if let ResourceReturnMode::Removed {
            parameter,
            nullable,
        } = result
        {
            let ResourceParameterMode::Removed {
                owner_parameter,
                nullable: input_nullable,
            } = parameters[parameter]
            else {
                return Err("a removed result requires the same removed parameter".into());
            };
            if !kinds.parameter_kind(owner_parameter).is_heap()
                || !kinds.parameter_kind(parameter).is_heap()
                || !collected.container_effects.is_empty()
                || !collected.users.is_empty()
                || input_nullable != nullable
                || parameters[owner_parameter] != (ResourceParameterMode::Owned { nullable: false })
                || parameter_types[parameter] != "ptr<mut, u8>"
                || parameter_types[owner_parameter] != "ptr<mut, u8>"
                || return_type != "ptr<mut, u8>"
                || parameters.iter().enumerate().any(|(i, mode)| {
                    i != parameter
                        && i != owner_parameter
                        && *mode != ResourceParameterMode::Borrowed
                })
            {
                return Err("a removed result requires matching nullability, mutable depth-one pointers, a distinct nonnull owned parent and no other custody effects".into());
            }
        }
        for (i, mode) in parameters.iter().enumerate() {
            let ty = token_kinds(&decl.parameters[i].type_source)?;
            if *mode != ResourceParameterMode::Borrowed
                && !matches!(ty.first(), Some(TokenKind::Word(word)) if word == "ptr")
                && !integer_parameter(i)
            {
                return Err("custody mode requires a pointer parameter".into());
            }
            // Card CG25-RESOURCE-KINDS: an in-place kind's out position names the
            // object it initialises (`regex_t *`), not a slot a pointer is written
            // to; `check_kinds` already required `ptr<mut, u8>` there.
            if matches!(mode, ResourceParameterMode::Out { .. })
                && !kinds.parameter_kind(i).is_in_place()
                && ty != token_kinds("ptr<mut, ptr<mut, u8>>")?
            {
                return Err("out mode requires a pointer-to-pointer parameter".into());
            }
            if let ResourceParameterMode::Stored {
                owner_parameter, ..
            }
            | ResourceParameterMode::Removed {
                owner_parameter, ..
            } = mode
                && (*owner_parameter == i
                    || !matches!(
                        parameters[*owner_parameter],
                        ResourceParameterMode::Owned { .. }
                    ))
            {
                return Err("aggregate owner must be a distinct owned parameter".into());
            }
        }
        if let ResourceReturnMode::Reallocated { parameter, .. } = result
            && !matches!(parameters[parameter], ResourceParameterMode::Owned { .. })
        {
            return Err("reallocation requires an owned input parameter".into());
        }
        counted_users::validate(&collected, &parameters, &parameter_types, return_type)?;
        // Card CG21-CALL-EXITS-AND-BOUNDARIES: a no-return declaration states
        // preconditions only. It returns nothing and takes no custody, so no
        // mode it carries could describe an effect a caller ever observes.
        if no_return
            && (return_type != "unit"
                || result != ResourceReturnMode::None
                || parameters
                    .iter()
                    .any(|mode| *mode != ResourceParameterMode::Borrowed)
                || !collected.container_effects.is_empty()
                || !collected.users.is_empty()
                || loop_owner.is_some()
                || thread_sync.is_some()
                || lifecycle.is_some())
        {
            return Err(
                "a no-return declaration returns `unit` with `resource_return_none()` and borrows every parameter"
                    .into(),
            );
        }
        let mut contract = ResourceFunctionContract::new(parameters, result);
        contract.parameter_kinds = kinds.parameter_kinds;
        contract.result_kind = kinds.result_kind;
        contract.container_effects = collected.container_effects.clone();
        contract.counted_users = collected.users.clone();
        finished.push(qcore_schemas::relational_post::ExitClass { relation, contract });
    }
    let relational = if is_relational {
        let relational = qcore_schemas::relational_post::RelationalContract {
            origin: qcore_schemas::relational_post::RelationalOrigin::Declared,
            classes: finished.clone(),
            withdrawn: None,
        };
        if !relational.disjoint() {
            return Err("the return-code classes overlap".into());
        }
        if !relational.total() {
            return Err("the return-code classes do not cover every returned value".into());
        }
        if relational.join().is_some() {
            return Err(
                "every return-code class states the same contract: write it as a plain \
                 declaration"
                    .into(),
            );
        }
        Some(relational)
    } else {
        None
    };
    let contract = finished
        .into_iter()
        .next()
        .map(|class| class.contract)
        .ok_or("missing parameter mode")?;
    // Card CALLEE-RETENTION-CLASS1 (§C04.14).
    let retaining = if retentions.is_empty()
        && resets.is_empty()
        && path_retentions.is_empty()
        && removals.is_empty()
        && slot_clauses.is_empty()
        && reference_clauses.is_empty()
        && callback_clauses.is_empty()
        && discriminants.is_empty()
        && arm_clauses.is_empty()
        && constructed_clauses.is_empty()
        && handoff_clauses.is_empty()
        && member_path_clauses.is_empty()
        && cell_clauses.is_empty()
        && cell_releases.is_empty()
        && removal_guards.is_empty()
    {
        None
    } else {
        // Card LIST-ENTRY-RELEASE-IDENTITY (gateway rule 43, §C04.8's element
        // destroyer): a declaration whose only retaining clauses are rule 30's
        // member-path states may also state the element destroyer's unlink of
        // the object it releases. The call is classified under its custody
        // contract, so rule 34 orders the unlink before the release exactly as
        // for a plain destroyer; every other container clause still rejects.
        let member_path_destroyer = retentions.is_empty()
            && resets.is_empty()
            && path_retentions.is_empty()
            && removals.is_empty()
            && slot_clauses.is_empty()
            && reference_clauses.is_empty()
            && callback_clauses.is_empty()
            && discriminants.is_empty()
            && arm_clauses.is_empty()
            && constructed_clauses.is_empty()
            && handoff_clauses.is_empty()
            && cell_clauses.is_empty()
            && cell_releases.is_empty()
            && removal_guards.is_empty()
            && collected.container_effects.iter().all(|(index, effect)| {
                *effect == qcore_schemas::contract::ContainerEffect::Unpublished
                    && matches!(
                        contract.parameters.get(*index),
                        Some(ResourceParameterMode::Released { .. })
                    )
            });
        if is_relational
            || !review.is_empty()
            || body_checked
            || variadic_data_tail
            || no_return
            || !literal_arguments.is_empty()
            || loop_owner.is_some()
            || thread_sync.is_some()
            || lifecycle.is_some()
            || !collected.users.is_empty()
            || (!collected.container_effects.is_empty() && !member_path_destroyer)
        {
            return Err(
                "a retention clause stands only beside plain parameter and result predicates"
                    .into(),
            );
        }
        let mut declared = retained_by::declaration(
            &retentions,
            &resets,
            &contract,
            &parameter_types,
            &parameter_kinds,
            &names,
        )?;
        // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (§C04.14, rule 12t).
        let (path_declared, removals_declared) = retained_by::paths_and_removals(
            &path_retentions,
            &removals,
            &declared,
            &contract,
            &parameter_types,
            &parameter_kinds,
            &names,
        )?;
        declared.path_retentions = path_declared;
        declared.removals = removals_declared;
        if !declared.removals.is_empty()
            && let Some(name) = [retained_by::HOLDS, "absent"]
                .into_iter()
                .find(|name| names.contains_key(*name))
        {
            return Err(format!(
                "`#{name}` is reserved in a `#` form: rename the parameter `{name}`"
            ));
        }
        // Card SLOT-MOVE-AND-INTERIOR-BORROW (§C04.15, §C04.16).
        let (slot_moves, derived_outs) = slot_move::declaration(
            &slot_clauses,
            &contract,
            &parameter_types,
            &parameter_kinds,
            &names,
        )?;
        if (!declared.path_retentions.is_empty() || !declared.removals.is_empty())
            && (!slot_moves.is_empty() || !derived_outs.is_empty())
        {
            return Err(
                "a holder path or a removal stands only beside retentions and plain predicates"
                    .into(),
            );
        }
        if let Some(retention) = declared.retentions.iter().find(|retention| {
            slot_moves
                .iter()
                .any(|moved| moved.slot == retention.retained)
                || derived_outs
                    .iter()
                    .any(|out| out.slot == retention.retained)
        }) {
            return Err(format!(
                "a slot is not also a retained pointer (parameter {})",
                retention.retained
            ));
        }
        declared.slot_moves = slot_moves;
        declared.derived_outs = derived_outs;
        // Card REFCOUNT-RESOURCE-KIND (§C04.17).
        let references = reference_count::declaration(
            &reference_clauses,
            &contract,
            &parameter_types,
            &parameter_kinds,
            &names,
        )?;
        if let Some(reference) = references.iter().find(|reference| {
            declared
                .retentions
                .iter()
                .any(|retention| retention.retained == reference.parameter)
                || declared
                    .slot_moves
                    .iter()
                    .any(|moved| moved.slot == reference.parameter)
                || declared
                    .derived_outs
                    .iter()
                    .any(|out| out.slot == reference.parameter || out.base == reference.parameter)
        }) {
            return Err(format!(
                "a referenced object is not also retained or a slot (parameter {})",
                reference.parameter
            ));
        }
        declared.references = references;
        // Card CG22-CALLBACK-REGISTRY-CONTRACT (§C04.19): callback clauses
        // stand beside retentions and plain predicates, never beside a slot
        // or a reference clause, and an element is never also retained.
        let callbacks = callback::declaration(
            &callback_clauses,
            &contract,
            &parameter_types,
            &parameter_kinds,
            &names,
        )?;
        if !callbacks.is_empty() {
            if !declared.slot_moves.is_empty()
                || !declared.derived_outs.is_empty()
                || !declared.references.is_empty()
            {
                return Err(
                    "a callback clause stands only beside retentions and plain predicates".into(),
                );
            }
            if callbacks.iter().any(|clause| {
                matches!(clause, qcore_schemas::callback::CallbackClause::ElementAdded { element, .. }
                    if declared.retentions.iter().any(|retention| retention.retained == *element))
            }) {
                return Err("a registry element is not also a retained pointer".into());
            }
        }
        // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (§C04.19.3): `#id =
        // #result` binds an integer result the declaration states nothing of.
        if callbacks.iter().any(|clause| {
            matches!(
                clause,
                qcore_schemas::callback::CallbackClause::MemberRegistered { by_id: Some(_), .. }
            )
        }) && (retained_by::guard_range(return_type).is_none()
            || contract.result != ResourceReturnMode::None)
        {
            return Err(
                "`#result` in an id binding is an integer result with `resource_return_none()`"
                    .into(),
            );
        }
        if (!declared.path_retentions.is_empty() || !declared.removals.is_empty())
            && (!declared.references.is_empty())
        {
            return Err(
                "a holder path or a removal stands only beside retentions and plain predicates"
                    .into(),
            );
        }
        declared.callbacks = callbacks;
        // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (§C04.19.2): the
        // type-switched release stands where a callback clause may.
        let (discriminant, arms) = arm_release::declaration(
            &discriminants,
            &arm_clauses,
            &contract,
            &parameter_types,
            &parameter_kinds,
            &names,
        )?;
        if discriminant.is_some()
            && (!declared.slot_moves.is_empty()
                || !declared.derived_outs.is_empty()
                || !declared.references.is_empty())
        {
            return Err("an arm clause stands only beside retentions and plain predicates".into());
        }
        declared.discriminant = discriminant;
        declared.arms = arms;
        // Card OP-CONSTRUCTOR-DECLARATIONS (§C04.19.4): the result state
        // stands where a callback clause may; a registered callback is
        // registered once and invoked by no clause of the declaration.
        let constructed = constructed_state::declaration(
            &constructed_clauses,
            &contract,
            &parameter_types,
            &names,
        )?;
        if !constructed.is_empty()
            && (!declared.slot_moves.is_empty()
                || !declared.derived_outs.is_empty()
                || !declared.references.is_empty())
        {
            return Err("a result state stands only beside retentions and plain predicates".into());
        }
        for state in &constructed.members {
            if let qcore_schemas::constructed_state::MemberValue::Callback { callback } =
                state.value
                && declared.callbacks.iter().any(|clause| match clause {
                    qcore_schemas::callback::CallbackClause::Invoked {
                        callback: named, ..
                    }
                    | qcore_schemas::callback::CallbackClause::Registered {
                        callback: named, ..
                    } => *named == callback,
                    _ => false,
                })
            {
                return Err("a callback is invoked or registered by one clause".into());
            }
        }
        declared.constructed = constructed;
        // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (§C04.19.5): the hand-off
        // stands where a callback clause may; a declaration carrying a `#`
        // form rejects a parameter named like a reserved keyword (its binder
        // would read as the keyword); a member invocation's object may be the
        // handed formal (`resource_owned`), and no other owned formal.
        let handed = handoff::declaration(
            &handoff_clauses,
            &contract,
            &parameter_types,
            &names,
            return_type,
        )?;
        if handed.is_some()
            && (!declared.slot_moves.is_empty()
                || !declared.derived_outs.is_empty()
                || !declared.references.is_empty())
        {
            return Err("a hand-off stands only beside retentions and plain predicates".into());
        }
        // Card OP-CALLBACK-MEMBER-PATH-RELEASE (§C04.19.6): the member-path
        // states stand where a hand-off may, beside it or alone.
        let member_paths = member_path::declaration(
            &member_path_clauses,
            &contract,
            &parameter_types,
            &names,
            handed.as_ref(),
        )?;
        if !member_paths.is_empty()
            && (!declared.slot_moves.is_empty()
                || !declared.derived_outs.is_empty()
                || !declared.references.is_empty()
                || !declared.callbacks.is_empty()
                || declared.discriminant.is_some()
                || !declared.arms.is_empty()
                || !declared.constructed.is_empty()
                || !declared.retentions.is_empty()
                || !declared.resets.is_empty())
        {
            return Err(
                "a member-path state stands only beside a hand-off and plain predicates".into(),
            );
        }
        if (handed.is_some() || !declared.constructed.is_empty() || !member_paths.is_empty())
            && let Some(name) = qcore_schemas::handoff::RESERVED_KEYWORDS
                .iter()
                .find(|name| names.contains_key(**name))
        {
            return Err(format!(
                "`#{name}` is reserved in a `#` form: rename the parameter `{name}`"
            ));
        }
        for clause in &declared.callbacks {
            if let qcore_schemas::callback::CallbackClause::MemberInvoked {
                object: qcore_schemas::callback::MemberObject::Parameter { index },
                ..
            } = clause
                && matches!(
                    contract.parameters[*index],
                    ResourceParameterMode::Owned { .. }
                )
                && handed.as_ref().map(|handed| handed.parameter) != Some(*index)
            {
                return Err(
                    "a member invocation's object is `resource_borrowed` or `resource_released`, \
                     or the formal a hand-off hands"
                        .into(),
                );
            }
        }
        declared.handoff = handed;
        declared.member_paths = member_paths;
        // Card MK-EVENT-DESCRIPTOR-QUANT (§C04.6.2, rule 38): the cells stand
        // beside retentions, removals and plain predicates only.
        let cells = descriptor_cell::declaration(
            &cell_clauses,
            &cell_releases,
            &removal_guards,
            declared.removals.len(),
            &contract,
            &parameter_types,
            &names,
            return_type,
        )?;
        if !cells.is_empty() {
            if !declared.slot_moves.is_empty()
                || !declared.derived_outs.is_empty()
                || !declared.references.is_empty()
                || !declared.callbacks.is_empty()
                || declared.discriminant.is_some()
                || !declared.arms.is_empty()
                || !declared.constructed.is_empty()
                || declared.handoff.is_some()
                || !declared.member_paths.is_empty()
                || !declared.path_retentions.is_empty()
            {
                return Err(
                    "a descriptor cell stands only beside retentions, removals and plain \
                     predicates"
                        .into(),
                );
            }
            if let Some(name) = qcore_schemas::handoff::RESERVED_KEYWORDS
                .iter()
                .find(|name| names.contains_key(**name))
            {
                return Err(format!(
                    "`#{name}` is reserved in a `#` form: rename the parameter `{name}`"
                ));
            }
        }
        declared.descriptor_cells = cells;
        Some(declared)
    };
    if matches!(contract.result, ResourceReturnMode::Removed { .. })
        && (retaining.is_some()
            || loop_owner.is_some()
            || thread_sync.is_some()
            || lifecycle.is_some()
            || variadic_data_tail
            || variadic_borrowed_tail)
    {
        return Err(
            "a removed result stands only beside plain custody predicates and named premises"
                .into(),
        );
    }
    let mut premises: Vec<DeclaredPremise> = premises
        .into_iter()
        .chain(literal_arguments.iter().map(|premise| DeclaredPremise {
            name: LITERAL_PREMISE_NAME.to_owned(),
            statement: format!(
                "{} = {}",
                decl.parameters[premise.parameter].name.value,
                premise.literal.concat()
            ),
            origin: PremiseOrigin::LiteralArgument,
        }))
        .collect();

    let hypotheses = crate::hypothesis::declaration(decl, &premises)?;
    for p in hypotheses
        .premises
        .values()
        .filter(|p| !p.id.starts_with("legacy_"))
    {
        premises.push(DeclaredPremise {
            name: p.erasure.name.clone(),
            statement: p.erasure.statement.clone(),
            origin: PremiseOrigin::Declared,
        });
    }
    Ok(LoadedDeclaration {
        relational,
        retaining,
        contract,
        variadic_data_tail,
        variadic_borrowed_tail,
        no_return,
        premises,
        hypotheses,
        literal_arguments,
        loop_owner,
        thread_sync,
        lifecycle,
        abi: DeclaredAbiSignature {
            parameters: parameter_types,
            result: return_type.to_owned(),
            variadic_data_tail,
        },
        review,
        body_checked,
        revision: crate::contract_revision::declared_revision(revision)?,
        ffi: None,
    })
}

/// One `[| quant #c_ffi |]`-marked declaration (card
/// `W48-QR-C-STRUCT-IMPORT`, user decision 4; contract spec §C04.9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredFfiFunction {
    pub name: String,
    /// The module the declaration was loaded from.
    pub module: String,
    pub parameters: Vec<(String, qcore_core::ffi::CAbiType)>,
    pub result: qcore_core::ffi::CAbiType,
}

/// Read a `#c_ffi` signature type: a §G02t scalar, `c"struct TAG"`,
/// `ptr<const, c"struct TAG">`, or (`result` only) `unit`.
fn ffi_abi_type(source: &str, result: bool) -> Result<qcore_core::ffi::CAbiType, String> {
    use qcore_core::ffi::{CAbiType, parse_c_struct_spelling};
    use qcore_core::scalar_types::ScalarType;
    let tokens = lex(source).map_err(|error| error.to_string())?;
    let tokens: Vec<_> = tokens
        .into_iter()
        .filter(|token| token.kind != TokenKind::Eof)
        .collect();
    let refuse = || {
        format!(
            "`{source}` is not in the `#c_ffi` type table: a §G02t scalar, `c\"struct TAG\"` or `ptr<const, c\"struct TAG\">`{}",
            if result {
                ", or `unit` as a result"
            } else {
                ""
            }
        )
    };
    let c_struct = |at: usize| -> Result<String, String> {
        match (tokens.get(at), tokens.get(at + 1)) {
            (Some(c), Some(text)) if c.kind == TokenKind::Word("c".into()) => {
                let TokenKind::StringLiteral(text_value) = &text.kind else {
                    return Err(refuse());
                };
                if c.span.end != text.span.start {
                    return Err(
                        "`c\"…\"` is one spelling: no space between `c` and the quote".into(),
                    );
                }
                parse_c_struct_spelling(text_value)
                    .map(str::to_owned)
                    .ok_or_else(|| format!("`c\"{text_value}\"` names no struct tag: the spelling is exactly `struct TAG`"))
            }
            _ => Err(refuse()),
        }
    };
    let kinds: Vec<&TokenKind> = tokens.iter().map(|token| &token.kind).collect();
    match kinds.as_slice() {
        [TokenKind::Word(word)] if result && word == "unit" => Ok(CAbiType::Unit),
        // Card `W48-QR-FLOATING-POINT`: `f32`/`f64` are in the `#c_ffi` type
        // table too, a C `float`/`double`.
        [TokenKind::Word(word)] => ScalarType::parse_admitting_floats(word)
            .map(CAbiType::Scalar)
            .ok_or_else(refuse),
        [TokenKind::Word(_), TokenKind::StringLiteral(_)] => c_struct(0).map(CAbiType::Struct),
        [
            TokenKind::Word(ptr),
            TokenKind::Symbol(Symbol::Less),
            TokenKind::Word(constness),
            TokenKind::Symbol(Symbol::Comma),
            TokenKind::Word(_),
            TokenKind::StringLiteral(_),
            TokenKind::Symbol(Symbol::Greater),
        ] if ptr == "ptr" && constness == "const" => c_struct(4).map(CAbiType::ConstStructPointer),
        _ => Err(refuse()),
    }
}

/// Load one `[| quant #c_ffi |]`-marked declaration, total or reject.
///
/// The marked vocabulary is the custody-free subset of v1 over the `#c_ffi`
/// type table: `requires resource_contract_v1;`, `resource_borrowed(p)` for
/// every parameter and `resource_return_none()`, each exactly once, and no
/// other clause, marker or suffix. A struct is data here: a by-value struct
/// is a copy, a `ptr<const, c"struct TAG">` is a borrowed pointer the callee
/// reads and does not retain, and a returned struct is a copy the caller owns
/// outright.
fn load_ffi_function(decl: &SourceDeclaration) -> Result<LoadedDeclaration, String> {
    use qcore_core::ffi::CAbiType;
    if decl.tokens.first() != Some(&TokenKind::Word("extern".into())) {
        return Err("`[| quant #c_ffi |]` marks an `extern fn` declaration".into());
    }
    let clauses = &decl
        .function_contract
        .as_ref()
        .ok_or("a `#c_ffi` declaration carries its resource contract")?
        .clauses;
    let mut parameters = Vec::with_capacity(decl.parameters.len());
    let mut names = BTreeSet::new();
    for parameter in &decl.parameters {
        if parameter.usage.is_some() {
            return Err("a `#c_ffi` parameter carries no usage annotation".into());
        }
        if !names.insert(parameter.name.value.clone()) {
            return Err("duplicate parameter name".into());
        }
        parameters.push((
            parameter.name.value.clone(),
            ffi_abi_type(&parameter.type_source, false)?,
        ));
    }
    let result_source = decl
        .result_type
        .as_ref()
        .map(|result| result.source.clone())
        .ok_or("a `#c_ffi` declaration states its result")?;
    let result = ffi_abi_type(&result_source, true)?;
    // The whole declaration, reconstructed: no effects row, no suffix.
    let parameter_text = decl
        .parameters
        .iter()
        .map(|p| format!("{}: {}", p.name.value, p.type_source))
        .collect::<Vec<_>>()
        .join(", ");
    let clause_text = clauses
        .iter()
        .map(|c| match &c.value.kind {
            SourceContractClauseKind::Requires => {
                Ok(format!("requires {};", c.value.expression_source))
            }
            SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal) => {
                Ok(format!("ensures normal {};", c.value.expression_source))
            }
            _ => Err(
                "a `#c_ffi` declaration admits `requires` and `ensures normal` clauses only"
                    .to_owned(),
            ),
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(" ");
    let expected = format!(
        "extern fn {}({parameter_text}) -> {result_source} contract {{ {clause_text} }};",
        decl.name
    );
    if token_kinds(&expected)? != decl.tokens {
        return Err(
            "unsupported signature, effects, or declaration suffix on a `#c_ffi` declaration"
                .into(),
        );
    }
    let mut marker = false;
    let mut borrowed = BTreeSet::new();
    let mut none = false;
    for clause in clauses {
        let expression = &clause.value.expression_source;
        if clause.value.kind == SourceContractClauseKind::Requires {
            if token_kinds(expression)? == token_kinds("resource_contract_v1")? && !marker {
                marker = true;
                continue;
            }
            return Err(format!(
                "`requires {expression};` is not admitted on a `#c_ffi` declaration, whose only requirement is `resource_contract_v1`"
            ));
        }
        let call = predicate_call(expression)?;
        match call.as_slice() {
            [predicate, parameter] if predicate == "resource_borrowed" => {
                if !names.contains(parameter) || !borrowed.insert(parameter.clone()) {
                    return Err(format!(
                        "`resource_borrowed({parameter})` names no parameter, or names one twice"
                    ));
                }
            }
            [predicate] if predicate == "resource_return_none" && !none => none = true,
            _ => {
                return Err(format!(
                    "`ensures normal {expression};` is not admitted on a `#c_ffi` declaration: every parameter is `resource_borrowed` and the result `resource_return_none()` — a struct crosses the boundary as data"
                ));
            }
        }
    }
    if !marker {
        return Err("missing requires resource_contract_v1".into());
    }
    if borrowed.len() != names.len() || !none {
        return Err("a `#c_ffi` declaration states `resource_borrowed` for every parameter and `resource_return_none()` once".into());
    }
    // A marked declaration that carries no struct still loads: the marker is
    // the auditor's handle on every FFI function, carried or not.
    let _: &CAbiType = &result;
    let contract = ResourceFunctionContract::new(
        vec![ResourceParameterMode::Borrowed; decl.parameters.len()],
        ResourceReturnMode::None,
    );
    Ok(LoadedDeclaration {
        relational: None,
        retaining: None,
        contract,
        variadic_data_tail: false,
        variadic_borrowed_tail: false,
        no_return: false,
        literal_arguments: Vec::new(),
        premises: Vec::new(),
        hypotheses: crate::hypothesis::declaration(decl, &[])?,
        loop_owner: None,
        thread_sync: None,
        lifecycle: None,
        abi: DeclaredAbiSignature {
            parameters: parameters.iter().map(|(_, ty)| ty.to_string()).collect(),
            result: result.to_string(),
            variadic_data_tail: false,
        },
        review: Vec::new(),
        body_checked: false,
        revision: None,
        ffi: Some(DeclaredFfiFunction {
            name: decl.name.clone(),
            module: String::new(),
            parameters,
            result,
        }),
    })
}

/// Every `#c_ffi`-marked declaration of a selection, rederived from its
/// retained texts (the variadic-tail precedent): total or reject, with the
/// module each came from. The ordinary loader withholds these declarations
/// from the function map, so no route but `source.ffi.v1` can reach one.
pub fn ffi_declarations(
    sources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, DeclaredFfiFunction>, String> {
    let mut declared = BTreeMap::new();
    for (path, source) in sources {
        reject_retired_premise_directive(path, source)?;
        reject_unresolved_inclusion(path, source)?;
        let parsed = parse_source_program(source).map_err(|e| format!("{path}: {e}"))?;
        for declaration in parsed.declarations {
            if declaration.value.c_ffi.is_none() {
                continue;
            }
            let decl = &declaration.value;
            let context = format!("{path}: {} at byte {}", decl.name, declaration.span.start);
            let loaded = load_function(decl).map_err(|e| format!("{context}: {e}"))?;
            let mut ffi = loaded
                .ffi
                .ok_or_else(|| format!("{context}: not a `#c_ffi` declaration"))?;
            ffi.module = path.clone();
            if declared.insert(decl.name.clone(), ffi).is_some() {
                return Err(format!("{context}: duplicate function contract"));
            }
        }
    }
    Ok(declared)
}

/// The checked-specification marker (card W45-FB-BACKLOG-CUSTODY, §C06.3).
pub const BODY_CHECKED_MARKER: &str = "resource_body_checked";

/// The clause introducer of a review marker (card W45-CONTRACT-DEFERRED-REVIEW).
pub const REVIEW_MARKER: &str = "resource_review_pending";

/// The optional last argument of a review marker stating that the text must
/// be rewritten before it can be accepted (card
/// W47-REVIEW-HOLD-CONCRETE-REASON, contract spec §C06.1).
pub const REWRITE_BEFORE_ACCEPTANCE: &str = "rewrite_before_acceptance";

/// A second contract-level marker, or a contract-level marker beside a
/// clause-level one: the contract-level marker already defers every clause.
const DUPLICATE_REVIEW_MARKER: &str =
    "duplicate review marker: a contract-level marker defers every clause, so it stands alone";

/// One review marker, parsed.
enum ReviewMarker {
    /// `requires resource_review_pending("REASON"[, "OWNER"][, rewrite_before_acceptance]);`
    Contract(ReviewDeferral),
    /// `ensures normal resource_review_pending(CLAUSE, "REASON"[, "OWNER"][,
    /// rewrite_before_acceptance]);`
    /// with `inner` the wrapped clause's expression text.
    Clause {
        inner: String,
        deferral: ReviewDeferral,
    },
}

/// Read one clause expression as a review marker, if it is one.
///
/// `None` means the clause is not a marker and is read as written. Every
/// malformed marker is an error, so total-or-reject refuses the whole module:
/// a missing or empty reason, a reason or owner that is not one plain string
/// literal, a contract-level marker under `ensures`, a clause-level marker
/// under `requires` (neither the resource-v1 selector nor a literal-argument
/// premise can be deferred on its own — the whole contract can), and a marker
/// wrapping a marker.
fn review_marker(requires: bool, expression: &str) -> Result<Option<ReviewMarker>, String> {
    let lexed = lex(expression).map_err(|e| e.to_string())?;
    let tokens = lexed
        .iter()
        .filter(|token| token.kind != TokenKind::Eof)
        .collect::<Vec<_>>();
    if !matches!(
        tokens.first().map(|t| &t.kind),
        Some(TokenKind::Word(word)) if word == REVIEW_MARKER
    ) {
        return Ok(None);
    }
    let malformed = || {
        format!(
            "malformed review marker `{expression}`: write `requires {REVIEW_MARKER}(\"REASON\"[, \
             \"OWNER\"][, {REWRITE_BEFORE_ACCEPTANCE}]);` or `ensures normal \
             {REVIEW_MARKER}(CLAUSE, \"REASON\"[, \"OWNER\"][, {REWRITE_BEFORE_ACCEPTANCE}]);`"
        )
    };
    let missing_reason =
        || format!("review marker `{expression}` gives no reason; a reason is mandatory");
    if tokens.len() < 3
        || tokens[1].kind != TokenKind::Symbol(Symbol::LeftParen)
        || tokens[tokens.len() - 1].kind != TokenKind::Symbol(Symbol::RightParen)
    {
        return Err(malformed());
    }
    let arguments = &tokens[2..tokens.len() - 1];
    // A wrapped clause is a balanced `NAME(...)` prefix of the arguments.
    let (inner, strings) = match arguments.first().map(|t| &t.kind) {
        Some(TokenKind::Word(name)) => {
            if name == REVIEW_MARKER {
                return Err(format!(
                    "a review marker cannot wrap another review marker: `{expression}`"
                ));
            }
            let mut depth = 0usize;
            let mut close = None;
            for (index, token) in arguments.iter().enumerate() {
                match token.kind {
                    TokenKind::Symbol(Symbol::LeftParen) => depth += 1,
                    TokenKind::Symbol(Symbol::RightParen) => {
                        depth = depth.checked_sub(1).ok_or_else(malformed)?;
                        if depth == 0 {
                            close = Some(index);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let close = close.ok_or_else(malformed)?;
            let text = &expression[arguments[0].span.start..arguments[close].span.end];
            let Some((first, rest)) = arguments[close + 1..].split_first() else {
                return Err(missing_reason());
            };
            if first.kind != TokenKind::Symbol(Symbol::Comma) {
                return Err(malformed());
            }
            (
                Some(text.split_whitespace().collect::<Vec<_>>().join(" ")),
                rest,
            )
        }
        _ => (None, arguments),
    };
    // Card W47-REVIEW-HOLD-CONCRETE-REASON: one optional trailing word, after
    // the reason and the owner, states that the text must be rewritten before
    // it can be accepted. Any other word, or this one anywhere else, is
    // malformed (the literal loop below refuses it).
    let (strings, disposition) = match strings {
        [rest @ .., comma, last]
            if comma.kind == TokenKind::Symbol(Symbol::Comma)
                && matches!(&last.kind, TokenKind::Word(word) if word == REWRITE_BEFORE_ACCEPTANCE) =>
        {
            (rest, ReviewDisposition::MustBeRewrittenBeforeAcceptance)
        }
        _ => (strings, ReviewDisposition::AcceptableAsWrittenAfterReview),
    };
    let mut literals = Vec::new();
    for (index, token) in strings.iter().enumerate() {
        match (index % 2, &token.kind) {
            (0, TokenKind::StringLiteral(text)) => literals.push(text.clone()),
            (1, TokenKind::Symbol(Symbol::Comma)) => {}
            _ => return Err(malformed()),
        }
    }
    if !strings.is_empty() && strings.len() % 2 == 0 {
        return Err(malformed());
    }
    let (reason, owner) = match literals.as_slice() {
        [] => return Err(missing_reason()),
        [reason] => (reason.clone(), None),
        [reason, owner] => (reason.clone(), Some(owner.clone())),
        _ => return Err(malformed()),
    };
    let plain = |text: &str, what: &str| -> Result<(), String> {
        if text.trim().is_empty() {
            return Err(format!("review marker `{expression}` has an empty {what}"));
        }
        if text.contains('\\') || text.chars().any(char::is_control) {
            return Err(format!(
                "review marker `{expression}`: the {what} is one plain line with no escapes"
            ));
        }
        Ok(())
    };
    plain(&reason, "reason")?;
    if let Some(owner) = &owner {
        plain(owner, "owner")?;
    }
    match (inner, requires) {
        (None, true) => Ok(Some(ReviewMarker::Contract(ReviewDeferral {
            scope: ReviewScope::Contract,
            reason,
            owner,
            disposition,
        }))),
        (None, false) => Err(format!(
            "a contract-level review marker is a requirement: write `requires {expression};`"
        )),
        (Some(_), true) => Err(format!(
            "unsupported review marker `{expression}`: a clause-level marker wraps an `ensures \
             normal` clause; the resource-v1 selector and a literal-argument premise cannot be \
             deferred on their own, so defer the whole contract instead"
        )),
        (Some(inner), false) => {
            let name = inner.split('(').next().unwrap_or_default().trim();
            if name == "resource_contract_v1"
                || name == "resource_literal_argument"
                || name == LOOP_OWNER_CLAUSE
                || name == THREAD_SYNC_CLAUSE
            {
                return Err(format!(
                    "unsupported review marker `{expression}`: only an `ensures normal` \
                     resource-v1 clause can be deferred"
                ));
            }
            Ok(Some(ReviewMarker::Clause {
                deferral: ReviewDeferral {
                    scope: ReviewScope::Clause {
                        clause: format!("ensures normal {inner}"),
                    },
                    reason,
                    owner,
                    disposition,
                },
                inner,
            }))
        }
    }
}

/// Card W45-CONTRACT-DEFERRED-REVIEW, contract spec §C15: an inherited
/// declaration carries exactly its parent's review markers. Elaboration copies
/// the parent's clauses with each parameter renamed by position, so a
/// clause-level marker's recorded clause text is compared after that renaming
/// rather than byte for byte; the scope kind, reason, owner and disposition must be equal.
fn review_inherited(child: &[ReviewDeferral], parent: &[ReviewDeferral]) -> bool {
    child.len() == parent.len()
        && child.iter().zip(parent).all(|(child, parent)| {
            child.reason == parent.reason
                && child.owner == parent.owner
                && child.disposition == parent.disposition
                && matches!(
                    (&child.scope, &parent.scope),
                    (ReviewScope::Contract, ReviewScope::Contract)
                        | (ReviewScope::Clause { .. }, ReviewScope::Clause { .. })
                )
        })
}

/// `resource_loop_owner(PARAM, read|mutate)`, exactly (card
/// W47-LOOP-OWNER-CONTRACTS). `None` when the expression is not this clause at
/// all; an expression that names it and is malformed rejects the module.
fn loop_owner_clause(source: &str) -> Result<Option<(String, LoopOwnerOperation)>, String> {
    let tokens = token_kinds(source)?;
    let Some(TokenKind::Word(name)) = tokens.first() else {
        return Ok(None);
    };
    if name != LOOP_OWNER_CLAUSE {
        return Ok(None);
    }
    let malformed = || format!("unsupported loop-owner requirement `{source}`");
    let [
        _,
        TokenKind::Symbol(Symbol::LeftParen),
        TokenKind::Word(parameter),
        TokenKind::Symbol(Symbol::Comma),
        TokenKind::Word(operation),
        TokenKind::Symbol(Symbol::RightParen),
    ] = tokens.as_slice()
    else {
        return Err(malformed());
    };
    let operation = match operation.as_str() {
        "read" => LoopOwnerOperation::Read,
        "mutate" => LoopOwnerOperation::Mutate,
        _ => return Err(malformed()),
    };
    Ok(Some((parameter.clone(), operation)))
}

/// Card **VERDICT-CONTRACT-DEPENDENCIES**: the clause introducer of a named
/// premise, `requires resource_premise(NAME, "STATEMENT");` (contract spec
/// §C04.18).
pub const PREMISE_CLAUSE: &str = "resource_premise";

/// The name a literal-argument premise (§C04.5) is reported under.
pub const LITERAL_PREMISE_NAME: &str = "literal_argument";

/// Where a declaration's premise comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PremiseOrigin {
    /// `requires resource_premise(NAME, "STATEMENT");`.
    Declared,
    /// `requires resource_literal_argument(p, LITERAL);`, read as the premise
    /// `p = LITERAL`.
    LiteralArgument,
}

impl PremiseOrigin {
    /// The report spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::LiteralArgument => "literal-argument",
        }
    }
}

/// One premise a declaration rests on (card **VERDICT-CONTRACT-DEPENDENCIES**).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclaredPremise {
    /// The premise's name: a lowercase identifier for a declared premise,
    /// [`LITERAL_PREMISE_NAME`] for a literal-argument one.
    pub name: String,
    /// What it states.
    pub statement: String,
    pub origin: PremiseOrigin,
}

/// `resource_premise(NAME, "STATEMENT")`, exactly. `None` when the expression
/// is not this clause; one that names it and is malformed rejects the module:
/// the name is a lowercase identifier (`[a-z][a-z0-9_]*`) and the statement one
/// plain, non-blank string literal with no escape and no control character.
pub(crate) fn premise_clause(source: &str) -> Result<Option<DeclaredPremise>, String> {
    let tokens = token_kinds(source)?;
    let Some(TokenKind::Word(head)) = tokens.first() else {
        return Ok(None);
    };
    if head != PREMISE_CLAUSE {
        return Ok(None);
    }
    let malformed = || {
        format!(
            "malformed premise `{source}`: write `requires {PREMISE_CLAUSE}(NAME, \"STATEMENT\");` \
             with a lowercase identifier and one plain string"
        )
    };
    let [
        _,
        TokenKind::Symbol(Symbol::LeftParen),
        TokenKind::Word(name),
        TokenKind::Symbol(Symbol::Comma),
        TokenKind::StringLiteral(statement),
        TokenKind::Symbol(Symbol::RightParen),
    ] = tokens.as_slice()
    else {
        return Err(malformed());
    };
    let identifier = name
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !identifier
        || statement.trim().is_empty()
        || statement.contains('\\')
        || statement.chars().any(char::is_control)
    {
        return Err(malformed());
    }
    Ok(Some(DeclaredPremise {
        name: name.clone(),
        statement: statement.clone(),
        origin: PremiseOrigin::Declared,
    }))
}

/// Card **VERDICT-CONTRACT-DEPENDENCIES**: every premise `sources` declares,
/// keyed by function name: the named premises in clause order, then the
/// literal-argument premises. A declaration with none is absent.
/// Total-or-reject exactly as [`literal_argument_premises`]; read by the
/// audit's dependency report only, never by a verdict.
pub fn declaration_premises(
    sources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, Vec<DeclaredPremise>>, String> {
    let mut premises = BTreeMap::new();
    each_loaded_declaration(sources, |name, context, loaded| {
        if !loaded.premises.is_empty()
            && premises
                .insert(name.to_owned(), loaded.premises.clone())
                .is_some()
        {
            return Err(format!("{context}: duplicate function contract"));
        }
        Ok(())
    })?;
    Ok(premises)
}

/// Card **LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER** (USER DECISION 2026-10-15,
/// "Add premise support"): the named premises a contract module states, by
/// function, in any vocabulary, read from the contract format alone
/// (`requires resource_premise(NAME, "STATEMENT");`). This is how a Quirin
/// model contract outside every resource view (a `resource type` module such
/// as librdkafka's `queue_destructor.qv`) states and publishes a premise.
/// Total-or-reject: a module that does not parse in contract mode, or a
/// malformed or repeated premise (the parser's check), rejects the whole
/// module. Read by no verdict.
pub fn module_premises(source: &str) -> Result<BTreeMap<String, Vec<DeclaredPremise>>, String> {
    let parsed = parse_source_program(source).map_err(|e| e.to_string())?;
    if parsed.mode != SourceModuleMode::Contract {
        return Err("a premise is read only from a `module contract;` module".into());
    }
    let mut out: BTreeMap<String, Vec<DeclaredPremise>> = BTreeMap::new();
    for declaration in &parsed.declarations {
        let Some(contract) = declaration.value.function_contract.as_ref() else {
            continue;
        };
        let mut premises = Vec::new();
        for clause in &contract.clauses {
            if let Some(premise) = premise_clause(&clause.value.expression_source)? {
                premises.push(premise);
            }
        }
        let graph = crate::hypothesis::declaration(&declaration.value, &premises)?;
        crate::premise_flow::declaration(&declaration.value, &graph)?;
        crate::contract_lifecycle::declaration(&declaration.value, &graph)?;
        crate::guarded_decomposition::declaration(&declaration.value, &graph)?;
        for node in graph
            .premises
            .values()
            .filter(|p| !p.id.starts_with("legacy_"))
        {
            premises.push(DeclaredPremise {
                name: node.erasure.name.clone(),
                statement: node.erasure.statement.clone(),
                origin: PremiseOrigin::Declared,
            });
        }
        if !premises.is_empty()
            && out
                .insert(declaration.value.name.clone(), premises)
                .is_some()
        {
            return Err(format!(
                "{}: duplicate function contract",
                declaration.value.name
            ));
        }
    }
    Ok(out)
}

/// The premises of an environment, rederived from its retained trusted
/// sources ([`declaration_premises`]); none without declared sources.
pub fn declared_premises(
    contracts: &ResourceContracts,
) -> Result<BTreeMap<String, Vec<DeclaredPremise>>, String> {
    if contracts.source_assumptions().is_empty() {
        return Ok(BTreeMap::new());
    }
    declaration_premises(contracts.source_assumptions())
}

/// The loop-owner obligations `sources` declares, keyed by function name
/// (card W47-LOOP-OWNER-CONTRACTS). Total-or-reject exactly as
/// [`literal_argument_premises`]: a consumer must treat an error as a refusal
/// of every call it cannot classify, never as an empty map.
pub fn loop_owner_obligations(
    sources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, DeclaredLoopOwner>, String> {
    let mut obligations = BTreeMap::new();
    each_loaded_declaration(sources, |name, context, loaded| {
        if let Some(obligation) = loaded.loop_owner
            && obligations.insert(name.to_owned(), obligation).is_some()
        {
            return Err(format!("{context}: duplicate function contract"));
        }
        Ok(())
    })?;
    Ok(obligations)
}

/// The loop-owner obligations of an environment, rederived from its retained
/// trusted sources (card W47-LOOP-OWNER-CONTRACTS).
pub fn declared_loop_owner_obligations(
    contracts: &ResourceContracts,
) -> Result<BTreeMap<String, DeclaredLoopOwner>, String> {
    if contracts.source_assumptions().is_empty() {
        return Ok(BTreeMap::new());
    }
    loop_owner_obligations(contracts.source_assumptions())
}

/// `resource_thread_sync(OBJECT[, MUTEX], OPERATION)`, exactly (card
/// W47-CONDVAR-JOIN-PUBLICATION). `None` when the expression is not this clause
/// at all; an expression that names it and is malformed rejects the module.
fn thread_sync_clause(source: &str) -> Result<Option<(Vec<String>, ThreadSyncOperation)>, String> {
    let tokens = token_kinds(source)?;
    let Some(TokenKind::Word(name)) = tokens.first() else {
        return Ok(None);
    };
    if name != THREAD_SYNC_CLAUSE {
        return Ok(None);
    }
    let malformed = || format!("unsupported thread-synchronization requirement `{source}`");
    let (words, operation) = match tokens.as_slice() {
        [
            _,
            TokenKind::Symbol(Symbol::LeftParen),
            TokenKind::Word(object),
            TokenKind::Symbol(Symbol::Comma),
            TokenKind::Word(operation),
            TokenKind::Symbol(Symbol::RightParen),
        ] => (vec![object.clone()], operation),
        [
            _,
            TokenKind::Symbol(Symbol::LeftParen),
            TokenKind::Word(object),
            TokenKind::Symbol(Symbol::Comma),
            TokenKind::Word(mutex),
            TokenKind::Symbol(Symbol::Comma),
            TokenKind::Word(operation),
            TokenKind::Symbol(Symbol::RightParen),
        ] => (vec![object.clone(), mutex.clone()], operation),
        _ => return Err(malformed()),
    };
    let operation = match operation.as_str() {
        "cond_wait" => ThreadSyncOperation::CondWait,
        "cond_timedwait" => ThreadSyncOperation::CondTimedWait,
        "cond_signal" => ThreadSyncOperation::CondSignal,
        "cond_init" => ThreadSyncOperation::CondInit,
        "cond_destroy" => ThreadSyncOperation::CondDestroy,
        "join" => ThreadSyncOperation::Join,
        _ => return Err(malformed()),
    };
    // A wait names its condvar and mutex; every other operation one object.
    if words.len() != if operation.names_mutex() { 2 } else { 1 } {
        return Err(malformed());
    }
    Ok(Some((words, operation)))
}

/// Check one thread-synchronization clause's parameters against the
/// declaration's ABI projection (card W47-CONDVAR-JOIN-PUBLICATION): a condvar
/// and a mutex are object pointers (`ptr<mut, u8>`), distinct; a joined
/// thread is an integer scalar (`pthread_t`).
fn thread_sync_declaration(
    decl: &SourceDeclaration,
    parameter_types: &[String],
    objects: &[String],
    operation: ThreadSyncOperation,
    index: impl Fn(&str) -> Result<usize, String>,
) -> Result<DeclaredThreadSync, String> {
    let parameter = index(&objects[0])?;
    let mutex = objects.get(1).map(|name| index(name)).transpose()?;
    let object_pointer = |position: usize| parameter_types[position] == "ptr<mut, u8>";
    let name = |position: usize| decl.parameters[position].name.value.clone();
    match operation {
        ThreadSyncOperation::Join => {
            if !matches!(parameter_types[parameter].as_str(), "u64" | "usize") {
                return Err(format!(
                    "a join obligation names a thread identifier, not `{}`",
                    name(parameter)
                ));
            }
        }
        _ => {
            if !object_pointer(parameter) {
                return Err(format!(
                    "a condvar obligation names a condvar pointer, not `{}`",
                    name(parameter)
                ));
            }
        }
    }
    if let Some(mutex) = mutex
        && (mutex == parameter || !object_pointer(mutex))
    {
        return Err(format!(
            "a wait obligation names a distinct mutex pointer, not `{}`",
            name(mutex)
        ));
    }
    Ok(DeclaredThreadSync {
        parameter,
        mutex,
        operation,
    })
}

/// The thread-synchronization obligations `sources` declares, keyed by
/// function name (card W47-CONDVAR-JOIN-PUBLICATION). Total-or-reject exactly
/// as [`loop_owner_obligations`].
pub fn thread_sync_obligations(
    sources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, DeclaredThreadSync>, String> {
    let mut obligations = BTreeMap::new();
    each_loaded_declaration(sources, |name, context, loaded| {
        if let Some(obligation) = loaded.thread_sync
            && obligations.insert(name.to_owned(), obligation).is_some()
        {
            return Err(format!("{context}: duplicate function contract"));
        }
        Ok(())
    })?;
    Ok(obligations)
}

/// The thread-synchronization obligations of an environment, rederived from
/// its retained trusted sources (card W47-CONDVAR-JOIN-PUBLICATION).
pub fn declared_thread_sync_obligations(
    contracts: &ResourceContracts,
) -> Result<BTreeMap<String, DeclaredThreadSync>, String> {
    if contracts.source_assumptions().is_empty() {
        return Ok(BTreeMap::new());
    }
    thread_sync_obligations(contracts.source_assumptions())
}

/// `resource_lifecycle(PARAM, start|stop|destroy)`, exactly (card
/// W47-FLB-LIB-LIFECYCLE). `None` when the expression is not this clause at
/// all; an expression that names it and is malformed rejects the module.
fn lifecycle_clause(source: &str) -> Result<Option<(String, LifecycleOperation)>, String> {
    let tokens = token_kinds(source)?;
    let Some(TokenKind::Word(name)) = tokens.first() else {
        return Ok(None);
    };
    if name != LIFECYCLE_CLAUSE {
        return Ok(None);
    }
    let malformed = || format!("unsupported lifecycle requirement `{source}`");
    let [
        _,
        TokenKind::Symbol(Symbol::LeftParen),
        TokenKind::Word(parameter),
        TokenKind::Symbol(Symbol::Comma),
        TokenKind::Word(operation),
        TokenKind::Symbol(Symbol::RightParen),
    ] = tokens.as_slice()
    else {
        return Err(malformed());
    };
    let operation = match operation.as_str() {
        "start" => LifecycleOperation::Start,
        "stop" => LifecycleOperation::Stop,
        "destroy" => LifecycleOperation::Destroy,
        _ => return Err(malformed()),
    };
    Ok(Some((parameter.clone(), operation)))
}

/// The lifecycle obligations `sources` declares, keyed by function name (card
/// W47-FLB-LIB-LIFECYCLE). Total-or-reject exactly as
/// [`loop_owner_obligations`].
pub fn lifecycle_obligations(
    sources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, DeclaredLifecycle>, String> {
    let mut obligations = BTreeMap::new();
    each_loaded_declaration(sources, |name, context, loaded| {
        if let Some(obligation) = loaded.lifecycle
            && obligations.insert(name.to_owned(), obligation).is_some()
        {
            return Err(format!("{context}: duplicate function contract"));
        }
        Ok(())
    })?;
    Ok(obligations)
}

/// The lifecycle obligations of an environment, rederived from its retained
/// trusted sources (card W47-FLB-LIB-LIFECYCLE).
pub fn declared_lifecycle_obligations(
    contracts: &ResourceContracts,
) -> Result<BTreeMap<String, DeclaredLifecycle>, String> {
    if contracts.source_assumptions().is_empty() {
        return Ok(BTreeMap::new());
    }
    lifecycle_obligations(contracts.source_assumptions())
}

/// `resource_literal_argument(PARAM, LITERAL)`, exactly (card HASH-CLAUSE).
///
/// This clause is the one place in the v1 vocabulary where an argument is a
/// *value* rather than a name, so it is parsed here rather than through
/// [`predicate_call`], whose identifier-only strictness is load-bearing for
/// every other predicate. `LITERAL` is a decimal integer with an optional
/// leading minus: the returned vector is the C token texts, which is what a
/// consumer compares an actual against. Alternate bases and suffixes lex as an
/// extra token and so fail this match rather than being approximated.
///
/// Card **HASH-REFINE** narrows the *declared* spelling to one canonical form
/// per value, so that the consumer-side spelling equivalence it admits (§C04.5)
/// has a unique target. Two spellings are refused here that the digits-only test
/// used to admit:
///
/// * a **leading zero** (`010`) — that is an octal constant in C, and this
///   clause decides no base; and
/// * a **negative zero** (`-0`) — one value with two declared spellings, which
///   would make the declared literal ambiguous where the call-site relation is
///   not.
///
/// Neither occurs in the corpus, and both refusals are strictly refusing: a
/// module using either no longer loads at all.
fn literal_argument_clause(source: &str) -> Result<(String, Vec<String>), String> {
    let unsupported = || format!("unsupported or duplicate requirement `{source}`");
    let tokens = token_kinds(source)?;
    let [
        TokenKind::Word(name),
        TokenKind::Symbol(Symbol::LeftParen),
        TokenKind::Word(parameter),
        TokenKind::Symbol(Symbol::Comma),
        literal @ ..,
    ] = tokens.as_slice()
    else {
        return Err(unsupported());
    };
    if name != "resource_literal_argument" {
        return Err(unsupported());
    }
    let (sign, digits) = match literal {
        [
            TokenKind::Symbol(Symbol::Minus),
            TokenKind::Integer(digits),
            TokenKind::Symbol(Symbol::RightParen),
        ] => (Some("-".to_owned()), digits),
        [
            TokenKind::Integer(digits),
            TokenKind::Symbol(Symbol::RightParen),
        ] => (None, digits),
        _ => return Err(unsupported()),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(unsupported());
    }
    // One canonical spelling per declared value (card HASH-REFINE): no octal
    // leading zero, and no second spelling of zero.
    if (digits.len() > 1 && digits.starts_with('0')) || (sign.is_some() && digits == "0") {
        return Err(unsupported());
    }
    Ok((
        parameter.clone(),
        sign.into_iter().chain([digits.clone()]).collect(),
    ))
}

/// Card W43-DESCRIPTOR-RESOURCE-TRACKING: which validity vocabulary a
/// position's nullability word belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValidityWord {
    /// `nullable` / `nonnull`: a pointer-carried identity whose failure value
    /// is NULL.
    Pointer,
    /// `sentinel` / `valid`: an integer-carried identity whose failure value is
    /// the sentinel `-1`.
    Integer,
}

/// A position's nullability word: whether the position may carry the failure
/// value, and in which vocabulary that was said. `nullable`/`nonnull` keep
/// their v1 meaning exactly; `sentinel`/`valid` are their integer-carried
/// counterparts and are admitted only where the position's kind is carried by
/// an integer (checked in [`check_kinds`]).
fn validity(word: &str) -> Result<(bool, ValidityWord), String> {
    match word {
        "nullable" => Ok((true, ValidityWord::Pointer)),
        "nonnull" => Ok((false, ValidityWord::Pointer)),
        "sentinel" => Ok((true, ValidityWord::Integer)),
        "valid" => Ok((false, ValidityWord::Integer)),
        _ => Err(format!("unsupported nullability {word}")),
    }
}

/// A declared non-heap kind. `heap` is spelled by absence and refused here, so
/// every meaning has exactly one spelling.
fn declared_kind(word: &str) -> Result<ResourceKind, String> {
    match ResourceKind::from_spelling(word) {
        Some(ResourceKind::Heap) => {
            Err("the heap kind is spelled by omitting the kind clause, not by `heap`".into())
        }
        // Card REFCOUNT-RESOURCE-KIND (§C04.17): a reference is taken and
        // dropped only by the reference clauses, counted per object; no
        // parameter position spells it.
        Some(ResourceKind::Reference) => Err(
            "the reference kind is taken by `resource_reference_kept` and dropped by \
             `resource_reference_dropped`, not spelled on a parameter"
                .into(),
        ),
        Some(kind) => Ok(kind),
        None => Err(format!("unsupported resource kind {word}")),
    }
}

/// A declared non-heap result kind. Card REFCOUNT-RESOURCE-KIND (§C04.17):
/// the reference kind is admitted here, on an owned result only (a call that
/// returns one reference its caller owes: a reference-returning constructor
/// or lookup); `check_kinds` requires the owned result.
fn declared_result_kind(word: &str) -> Result<ResourceKind, String> {
    match ResourceKind::from_spelling(word) {
        Some(ResourceKind::Reference) => Ok(ResourceKind::Reference),
        _ => declared_kind(word),
    }
}

/// The parameter half of one declaration, as [`check_kinds`] reads it.
struct KindPositions<'a> {
    parameters: &'a [ResourceParameterMode],
    parameter_types: &'a [String],
    parameter_words: &'a [Option<ValidityWord>],
    parameter_kinds: &'a [Option<ResourceKind>],
}

/// The kinds a declaration's positions carry, once every placement rule holds.
struct DeclaredKinds {
    parameter_kinds: BTreeMap<usize, ResourceKind>,
    result_kind: ResourceKind,
}

impl DeclaredKinds {
    fn parameter_kind(&self, index: usize) -> ResourceKind {
        self.parameter_kinds
            .get(&index)
            .copied()
            .unwrap_or(ResourceKind::Heap)
    }
}

/// One position's validity word and ABI type against its kind.
fn kind_agrees(kind: ResourceKind, word: Option<ValidityWord>, ty: &str) -> Result<(), String> {
    let expected = if kind.is_integer() {
        ValidityWord::Integer
    } else {
        ValidityWord::Pointer
    };
    if word.is_some_and(|word| word != expected) {
        return Err(if kind.is_integer() {
            format!(
                "a {} position spells its validity `sentinel` or `valid`",
                kind.spelling()
            )
        } else {
            format!(
                "a {} position spells its nullability `nullable` or `nonnull`",
                kind.spelling()
            )
        });
    }
    if kind.is_integer() && ty != "i32" {
        return Err(format!("a {} position requires `i32`", kind.spelling()));
    }
    if !kind.is_heap() && !kind.is_integer() && !ty.starts_with("ptr<") {
        return Err(format!(
            "a {} position requires a pointer type",
            kind.spelling()
        ));
    }
    Ok(())
}

/// Card W43-DESCRIPTOR-RESOURCE-TRACKING: every placement rule of a kind.
///
/// * A kind clause names a custody-bearing position: an owned, consumed,
///   released or out parameter, or an owned result. A borrowed position
///   touches no obligation, and a stored, removed or reallocated position is
///   heap only, because the owner cascade and the replacement relation were
///   specified for allocations and a descriptor has neither.
/// * An integer-carried kind (`descriptor`) sits on an `i32`, spells its
///   validity `sentinel`/`valid`, and has no out slot here.
/// * Every other kind sits on an admitted pointer and spells its nullability
///   `nullable`/`nonnull`, exactly as a heap position does; so `sentinel` or
///   `valid` on a heap or pointer-kind position is refused.
fn check_kinds(
    positions: &KindPositions<'_>,
    result: ResourceReturnMode,
    return_type: &str,
    result_word: Option<ValidityWord>,
    result_kind: Option<ResourceKind>,
) -> Result<DeclaredKinds, String> {
    let mut declared = DeclaredKinds {
        parameter_kinds: BTreeMap::new(),
        result_kind: ResourceKind::Heap,
    };
    for (index, mode) in positions.parameters.iter().enumerate() {
        let kind = positions.parameter_kinds[index].unwrap_or(ResourceKind::Heap);
        if kind.is_in_place() {
            in_place::check_parameter(
                kind,
                *mode,
                positions.parameter_words[index],
                &positions.parameter_types[index],
            )?;
            declared.parameter_kinds.insert(index, kind);
            continue;
        }
        if !kind.is_heap() {
            match mode {
                ResourceParameterMode::Owned { .. }
                | ResourceParameterMode::Consumed { .. }
                | ResourceParameterMode::Released { .. } => {}
                ResourceParameterMode::Out { .. } if !kind.is_integer() => {}
                ResourceParameterMode::Out { .. } => {
                    return Err(format!(
                        "a {} has no out slot in this vocabulary",
                        kind.spelling()
                    ));
                }
                ResourceParameterMode::Borrowed => {
                    return Err(
                        "a kind names a custody-bearing parameter, not a borrowed one".into(),
                    );
                }
                ResourceParameterMode::Stored { .. } | ResourceParameterMode::Removed { .. } => {
                    return Err("a stored or removed parameter is heap only".into());
                }
            }
            declared.parameter_kinds.insert(index, kind);
        }
        kind_agrees(
            kind,
            positions.parameter_words[index],
            &positions.parameter_types[index],
        )?;
    }
    in_place::check_declaration(&declared, positions.parameters, result, return_type)?;
    let kind = result_kind.unwrap_or(ResourceKind::Heap);
    if kind.is_in_place() {
        return Err(format!(
            "a {} object is initialised in caller storage, never returned",
            kind.spelling()
        ));
    }
    if !kind.is_heap() {
        if !matches!(result, ResourceReturnMode::Owned { .. }) {
            return Err("a result kind requires an owned result".into());
        }
        declared.result_kind = kind;
    }
    kind_agrees(kind, result_word, return_type)?;
    Ok(declared)
}

/// One plain parameter predicate: `resource_borrowed(p)` or
/// `resource_{owned,consumed,released,out}(p, nullable|nonnull)` (and their
/// integer-carried validity words), as `words` from [`predicate_call`].
fn plain_parameter_predicate<'w>(
    words: &[&'w str],
    expression: &str,
) -> Result<(&'w str, ResourceParameterMode, Option<ValidityWord>), String> {
    match words {
        ["resource_borrowed", p] => Ok((*p, ResourceParameterMode::Borrowed, None)),
        [name, p, null] => {
            let (nullable, word) = validity(null)?;
            let mode = match *name {
                "resource_owned" => ResourceParameterMode::Owned { nullable },
                "resource_consumed" => ResourceParameterMode::Consumed { nullable },
                "resource_released" => ResourceParameterMode::Released { nullable },
                "resource_out" => ResourceParameterMode::Out { nullable },
                _ => return Err(format!("unsupported predicate `{expression}`")),
            };
            Ok((*p, mode, Some(word)))
        }
        _ => Err(format!("unsupported predicate `{expression}`")),
    }
}

fn predicate_call(source: &str) -> Result<Vec<String>, String> {
    let tokens = token_kinds(source)?;
    let [
        TokenKind::Word(name),
        TokenKind::Symbol(Symbol::LeftParen),
        args @ ..,
        TokenKind::Symbol(Symbol::RightParen),
    ] = tokens.as_slice()
    else {
        return Err(format!("unsupported predicate `{source}`"));
    };
    let mut words = vec![name.clone()];
    for (i, token) in args.iter().enumerate() {
        match (i % 2, token) {
            (0, TokenKind::Word(word)) => words.push(word.clone()),
            (1, TokenKind::Symbol(Symbol::Comma)) => {}
            _ => return Err(format!("unsupported predicate argument `{source}`")),
        }
    }
    if !args.is_empty() && args.len() % 2 == 0 {
        return Err("trailing predicate comma".into());
    }
    Ok(words)
}

/// Declaration graphs freshly reconstructed through the same loader as custody.
pub fn declaration_hypotheses(
    sources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, qcore_schemas::hypothesis::Graph>, String> {
    let mut graphs = BTreeMap::new();
    // Independent graph replay deliberately bypasses the memoized module
    // readings used by hot-path metadata queries.
    for (path, source) in sources {
        let reading = read_module(path, source)?;
        for declaration in reading.declarations {
            let loaded = declaration
                .loaded
                .map_err(|e| format!("{}: {e}", declaration.context))?;
            if graphs.insert(declaration.name, loaded.hypotheses).is_some() {
                return Err("hypothesis: duplicate declaration".into());
            }
        }
    }
    Ok(graphs)
}
