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

//! Declared-contract / C-declaration ABI agreement (card **LOADER-ABI**).
//!
//! # The fail-open this closes
//!
//! A resource-contract v1 declaration writes an opaque C ABI projection for
//! every parameter and for the result:
//!
//! ```text
//! extern fn flb_sds_create_len(str: ptr<const, u8>, len: usize) -> ptr<mut, u8>
//! ```
//!
//! Those spellings are checked by `qcore_contract::contract_loader` against its
//! own closed table — and against **nothing else**. They are not carried on
//! `ResourceFunctionContract`, which holds custody modes only, and until this
//! card nothing anywhere compared them against the C declaration of the same
//! function in the translation unit being analysed. A declaration could
//! therefore say `-> unit` for a function that really returns `void *`, or
//! `(p: ptr<mut, u8>, n: i32)` for a C function declared `(int n, void *p)`,
//! and the analyzer would apply its custody modes positionally to actuals whose
//! declared types cannot carry them. Card NEIGHBOURS measured the fixture half
//! of this (a gateway fixture declared `-> u64` beside a C prelude declaring
//! `double`) and named the gap; this module is the gate.
//!
//! The types are not decoration: the loader's admissibility rules are keyed on
//! them. `resource_released(p)` is admitted only when `p` is a pointer
//! projection, `resource_out(p)` only at `ptr<mut, ptr<mut, u8>>`, and a
//! non-`None` result only on a pointer return type. An unfaithful type is
//! therefore exactly what buys a custody mode the real C type could not carry,
//! and a `-> unit` declaration of an allocating function hides an acquisition
//! the projection would otherwise have to account for. Both directions are
//! reproduced in `crates/qcore-c-gateway/tests/loader_abi.rs`.
//!
//! # Where the comparison happens, and why here
//!
//! At **bind time**, per translation unit — not at load time. The loader has no
//! translation unit: the same contract module is loaded for 185 units whose
//! declarations of a callee may differ, and one unit's disagreement must not
//! reject the whole environment for the others. The comparison also has to be
//! reproducible by W4 summary derivation, W4 validation and
//! `check_legacy_safety_certificate`, which hold the preprocessed token stream
//! and the environment's own retained trusted texts and nothing else. Both
//! sides of the comparison are read from exactly those two: the C declarations
//! from the same token stream the importer is already projecting (card C02's
//! rule), and the declared spellings from the environment's retained module
//! texts (`contract_loader::declared_abi_signatures_of`, the same rederivation
//! `declared_variadic_data_tails` and `literal_argument_premises` use).
//!
//! # What a disagreement does
//!
//! It **withdraws the declaration for this translation unit**, in both
//! directions that matter:
//!
//! * every call site is rewritten to a marked callee, so the contract is not
//!   applied at any of them and the statement refuses with a named key — the
//!   technique card HASH-CLAUSE uses for an unmet literal premise, chosen for
//!   the same reason: the withdrawal then holds uniformly at every consumer of
//!   the token stream rather than at whichever contract lookup remembered it;
//! * the name is removed from the environment the derivation seeds, so a
//!   function *defined* in this unit is no longer checked against, and its
//!   callers no longer trust, an interface its own C declaration contradicts.
//!
//! The decision is taken once per environment, at each entry that owns one:
//! `analyze_legacy_c_translation_unit_selected_partial`,
//! `recursive_component_census` and `analyze_legacy_c_tokens`. It is
//! deliberately **not** repeated inside `build_function_cfg`, whose callers all
//! sit below one of those and are handed the tokens and the environment the
//! withdrawal already applied to; re-deciding per function would re-read every
//! declared contract module once per analysed body.
//!
//! Withdrawal is monotone in the refusing direction: an uncontracted call is
//! `Unknown` and poisons the enclosing function's state, so no verdict can move
//! towards `Safe` because of it. An inferred W4 summary may still be derived for
//! a defined function whose declared contract was withdrawn — that is sound
//! because an inferred summary is published only after re-analysis validates
//! it, and it never rests on the withdrawn declaration.
//!
//! # Fail-closed on anything unreadable
//!
//! A source set the loader cannot re-read **poisons** the whole comparison and
//! withdraws every contracted callee, exactly as an unreadable premise set
//! does. That is deliberately not the empty map: an empty map compares nothing
//! and applies every declaration, which is the fail-open direction.
//!
//! # The one boundary this version keeps, recorded rather than hidden
//!
//! A contracted name this reader finds **no** declaration for in the unit is
//! not compared, and its contract still applies. That is the status quo for
//! that name, and it is where the reader's own coverage — not the declaration's
//! faithfulness — decides. It is named **`LOADER-ABI-UNDECLARED`** and measured
//! per unit rather than assumed away; requiring a declaration would refuse
//! every callee whose prototype this reader cannot see, which is a different
//! and larger gate.
//!
//! # What card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF widened, and what it did not
//!
//! Card CONSTRUCTOR-RESIDUALS (§4.2) measured the whole remaining residual of
//! production `flb_input_chunk_create` down to **one family**: three callees
//! this reader would not commit to a parameter type of, and therefore withdrew
//! at every call site with `abi-unreadable-declaration`. Both shapes were
//! reader boundaries rather than disagreements:
//!
//! * a `struct`/`union`/`enum` **defined by the typedef itself** —
//!   `typedef union { … } pthread_mutex_t;` — which the earlier `read_typedefs`
//!   abandoned at the aggregate's own opening brace and never recorded;
//! * a **function-pointer typedef** —
//!   `typedef int (*msgpack_packer_write)(void *, const char *, size_t);` —
//!   whose declarator the earlier reader did not parse.
//!
//! Both are now read. Neither is a guess:
//!
//! * an aggregate definition is an **object type**, and the pointer projection
//!   of a pointer to one reads no tag and no member — `union tag *` was already
//!   readable, and an untagged definition differs from it in nothing this
//!   vocabulary can spell. A *tagged* definition is recorded as the specifier
//!   pair `["struct", tag]` it is exactly equal to, so it composes with a
//!   separate `typedef struct tag tag;` instead of colliding with it. As a
//!   **value** an aggregate still refuses: this vocabulary has no spelling for
//!   one, exactly as it has none for a `struct tag` value.
//! * a pointer to function gets its own projection,
//!   [`FUNCTION_POINTER_ABI_TYPE`], which is not an entry of `POINTER_ABI_TYPES`
//!   and is not spelled `ptr<…>`. The loader's two custody gates key on the
//!   leading word `ptr`, so **no custody mode and no owned result can be written
//!   on it** — the admission is in the refusing direction for exactly the thing
//!   card LOADER-ABI exists to stop, a declared type buying a mode the C type
//!   cannot carry.
//!
//! Two boundaries stay, narrowed rather than deleted, and are named:
//!
//! * **`ABI-FNPTR-OPAQUE`** — `fnptr` does not distinguish two function
//!   pointers of different signatures, exactly as `ptr<mut, u8>` does not
//!   distinguish two object pointers of different pointee types. The opacity is
//!   the vocabulary's, not this reader's, and nothing can be bought with it
//!   because the spelling carries no custody.
//! * **`ABI-FNPTR-DEPTH1`** — a pointer *to* a function pointer refuses. It is
//!   an object pointer and would project into a custody-bearing spelling; no
//!   declaration in scope needs it, so it stays refused rather than admitted
//!   untested.
//!
//! An *inline* function-pointer parameter — `int (*cb)(void *)` written out at
//! the parameter position rather than behind a typedef — is read since card
//! CG22-INDIRECT-CALLS-AND-CALLBACKS (absorbing
//! LOADER-ABI-INLINE-FNPTR-DECLARATOR) as the same `fnptr`, in exactly the
//! typedef reader's shape; every other nested declarator still refuses.
//!
//! # What card W43-LIBC-RESULT-ALIAS widened
//!
//! A qualifier after a **parameter** declarator's last `*` — glibc's
//! `const char *__restrict __format`, `void *const p` — qualifies the parameter
//! object, and C11 6.7.6.3p15 drops it for type compatibility. The reader used
//! to count only trailing `*` words, so every such parameter projected to
//! nothing and its declaration was withdrawn with `abi-unreadable-declaration`
//! (card W43-LIBC-CONTRACTS measured 2,577 call-site leaves across fourteen
//! drafted libc declarations). `drop_parameter_object_qualifiers` now reads
//! it as the unqualified type. A qualifier *between* two `*` still refuses, a
//! result declarator is not widened, and the measured effect on the 114
//! declarations selected at the card's base was nil: every one of the 8,292
//! (unit, name) census rows kept its outcome. `QCore.PointerQualifierReader`
//! is the Lean model of this step.
//!
//! # What card W43-LIBC-RESTRICT-PROTOTYPES widened
//!
//! The same parameter-object qualifier, spelled inside an **array bound**:
//! C11 6.7.6.3p7 adjusts `T a[restrict n]` to `T *restrict a`, and
//! 6.7.6.3p15 then drops the `restrict`. `array_bound_is_read` reads a bound
//! that is a run of qualifiers followed by at most one size word (a decimal
//! constant or an identifier); `static`, `[*]` and multi-token sizes still
//! refuse. glibc's `regexec` (`regmatch_t __pmatch[__restrict __nmatch]` after
//! preprocessing) was the measured witness. `QCore.PointerQualifierReader`
//! models this step too.
//!
//! # What card W44-LIBC-RESIDUALS widened
//!
//! `__builtin_va_list`, the compiler's variable-argument list type that
//! glibc's `__gnuc_va_list` names, projects to the loader's parameter-only
//! `va_list` spelling (see [`BUILTIN_VA_LIST`]). `QCore.GnuBuiltinReader`
//! models the step.
//!
//! # MSVC's default C convention and explicit 64-bit integer
//!
//! One `__cdecl` in a function's specifier prefix selects the default C
//! convention and does not change this opaque parameter/result projection.
//! It is removed only there, before counting result pointer levels; other
//! conventions and a repeated annotation remain unreadable. `__int64` with
//! at most one `signed` or `unsigned` is the explicit 64-bit integer reading.
//! It shares `long long`'s projection, including the existing 64-bit-pointer
//! premise for `isize` / `usize`. A newly readable cdecl position involving
//! ordinary `long` refuses, including typedef chains. `QCore.MsvcAbiReader`
//! models both steps; faithful extraction and correspondence remain
//! boundary-28 premises.
//!
//! # What card W43-ABI-ENUM-AND-CONST-PP widened
//!
//! Two shapes the reader refused, on the user's decisions of 2026-10-05
//! (contract specification C04.1):
//!
//! * **an enumeration** (rule 12u) — `enum TAG`, or a typedef chain ending in
//!   one or in an untagged `typedef enum { … } NAME;`
//!   ([`TypedefTarget::UntaggedEnum`]) — projects, at a parameter or result,
//!   to the one integer spelling of the width and signedness Clang states
//!   for it in the analysed unit ([`super::enum_projection`]). The reading is
//!   two-phase inside [`AbiAgreement::read`]: a first pass records every
//!   enumeration a contracted declaration names, one appended Clang probe
//!   answers them all, and the declarations are read again with the answers.
//!   Outside that scope (and whenever Clang does not answer) an enumeration
//!   still refuses, never read as `int`;
//! * **a pointer to a pointer to const** (rule 12v) — `const char **` —
//!   projects to `ptr<mut, ptr<const, u8>>` ([`pointer_projection`]). A
//!   qualifier on the slot (written between the `*`, on a typedef name that
//!   stands for a pointer, or on a pointer link of a typedef chain) refuses.
//!
//! `QCore.AbiEnumConstPointer` is the Lean model.
//!
//! Card CG20-NUMERIC-DOMAIN (rule 11q) removes the reader's LP64 premise for
//! ordinary `long` everywhere else: [`project_arithmetic`] reads one `long`
//! word as the 64-bit projection only when the analysing Clang's default
//! target reports a 64-bit `long` and pointer
//! ([`crate::c_numeric::plain_long_is_lp64`]); under any other target, or if
//! the query fails, the declaration is unreadable. A per-unit `-arch` flag is
//! not seen by this process-wide query (`CG20-ABI-READER-ARCH-FLAG`).

use std::collections::{BTreeMap, BTreeSet};

use qcore_contract::contract_loader::{
    self, DeclaredAbiSignature, FUNCTION_POINTER_ABI_TYPE, VA_LIST_ABI_TYPE,
};
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::reason::ReasonCategory;

use super::Token;

/// The prefix a withdrawn callee's call sites are rewritten with.
///
/// The rewritten name carries both the refusal and the original callee, so the
/// statement that refuses can name them without a second lookup — the parser
/// reads a token stream, not the environment the withdrawal already left.
pub(crate) const WITHDRAWN_PREFIX: &str = "qcore_abi_withdrawn__";

/// The separator between the refusal token and the original callee name.
const WITHDRAWN_SEPARATOR: &str = "__";

/// The refusal and callee a rewritten name stands for, for diagnostics.
pub(crate) fn withdrawn_callee(function: &str) -> Option<(AbiRefusal, &str)> {
    let rest = function.strip_prefix(WITHDRAWN_PREFIX)?;
    let (token, callee) = rest.split_once(WITHDRAWN_SEPARATOR)?;
    Some((AbiRefusal::from_token(token)?, callee))
}

/// What the comparison decided for one contracted name in one unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AbiOutcome {
    /// The unit declares the function and every projection agrees.
    Agreed,
    /// The unit declares the function and the declaration was withdrawn.
    Withdrawn(AbiRefusal),
    /// The unit carries no declaration this reader recognises
    /// (`LOADER-ABI-UNDECLARED`). Nothing is compared and the contract applies.
    Undeclared,
}

/// One census row: what the comparison decided about one contracted name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AbiAgreementRow {
    /// The contracted function name.
    pub function: String,
    /// The decision.
    pub outcome: AbiOutcome,
}

/// Decide ABI agreement for every contracted name over one preprocessed
/// translation unit, without analysing anything.
///
/// This is the same decision the analyzer applies, exposed for measurement:
/// the census, the tests and the report read it rather than a second
/// implementation.
#[must_use]
pub fn declared_abi_census(
    preprocessed: &str,
    contracts: &ResourceContracts,
) -> Vec<AbiAgreementRow> {
    let tokens = super::tokenize(preprocessed);
    // Card W43-ABI-ENUM-AND-CONST-PP (rule 12u): the census reads the unit
    // the analysis would, so an enumeration is asked of Clang here too.
    let _scope =
        crate::typed_access_path::enter_source(preprocessed, super::CCompilerDialect::C11, None);
    let agreement = AbiAgreement::read(&tokens, contracts);
    let mut rows = Vec::new();
    for (function, refusal) in agreement.withdrawn() {
        rows.push(AbiAgreementRow {
            function: function.clone(),
            outcome: AbiOutcome::Withdrawn(*refusal),
        });
    }
    for function in agreement.agreed() {
        rows.push(AbiAgreementRow {
            function: function.clone(),
            outcome: AbiOutcome::Agreed,
        });
    }
    for function in agreement.undeclared() {
        rows.push(AbiAgreementRow {
            function: function.clone(),
            outcome: AbiOutcome::Undeclared,
        });
    }
    rows.sort();
    rows
}

/// Why one declared contract was withdrawn for one translation unit.
///
/// Every variant carries a stable key, a full prose reason and a closed
/// [`ReasonCategory`], the shape `EnumerationRefusal` established. A
/// *disagreement* is `StaleEvidence`: the declaration is evidence bound to a
/// different C declaration than the one under check. A declaration this reader
/// cannot commit to is `UnsupportedSyntax`, a source-side statement; a source
/// set the loader cannot re-read is `MissingPremise`, an analyser-side one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AbiRefusal {
    /// The declared result spelling is not an admissible projection of the C
    /// return type.
    ReturnTypeDisagreement,
    /// A declared parameter spelling is not an admissible projection of the C
    /// parameter type at the same position.
    ParameterTypeDisagreement,
    /// The declaration names a different number of parameters than the C
    /// declaration does.
    ArityDisagreement,
    /// The declaration carries `resource_variadic_data_tail()` for a C function
    /// that is not variadic, so the tail would admit actuals C cannot accept.
    VariadicDisagreement,
    /// The C declaration's parameter list is empty (`f()`), which specifies no
    /// parameters at all in C11, so the declared arity and modes cannot be
    /// checked against anything.
    UnspecifiedParameters,
    /// The C declaration is present but this reader will not commit to one of
    /// its types.
    UnreadableDeclaration,
    /// Two declarations of the same name in this unit project differently.
    AmbiguousDeclaration,
    /// The environment's own retained trusted texts could not be re-read, so no
    /// declared spelling is available for any name.
    UnreadableDeclaredSources,
}

impl AbiRefusal {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReturnTypeDisagreement => "abi-return-type-disagreement",
            Self::ParameterTypeDisagreement => "abi-parameter-type-disagreement",
            Self::ArityDisagreement => "abi-arity-disagreement",
            Self::VariadicDisagreement => "abi-variadic-disagreement",
            Self::UnspecifiedParameters => "abi-unspecified-parameters",
            Self::UnreadableDeclaration => "abi-unreadable-declaration",
            Self::AmbiguousDeclaration => "abi-ambiguous-declaration",
            Self::UnreadableDeclaredSources => "abi-unreadable-declared-sources",
        }
    }

    /// The identifier spelling carried inside a rewritten call token.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::ReturnTypeDisagreement => "return_type",
            Self::ParameterTypeDisagreement => "parameter_type",
            Self::ArityDisagreement => "arity",
            Self::VariadicDisagreement => "variadic",
            Self::UnspecifiedParameters => "unspecified_parameters",
            Self::UnreadableDeclaration => "unreadable_declaration",
            Self::AmbiguousDeclaration => "ambiguous_declaration",
            Self::UnreadableDeclaredSources => "unreadable_declared_sources",
        }
    }

    /// The refusal a rewritten call token names.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|kind| kind.token() == token)
    }

    /// Every refusal, in declaration order. The vocabulary is closed.
    pub const ALL: [Self; 8] = [
        Self::ReturnTypeDisagreement,
        Self::ParameterTypeDisagreement,
        Self::ArityDisagreement,
        Self::VariadicDisagreement,
        Self::UnspecifiedParameters,
        Self::UnreadableDeclaration,
        Self::AmbiguousDeclaration,
        Self::UnreadableDeclaredSources,
    ];

    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::ReturnTypeDisagreement => {
                "the declared contract's result type is not an admissible projection of the \
                 function's C return type in this translation unit"
            }
            Self::ParameterTypeDisagreement => {
                "a declared contract parameter type is not an admissible projection of the C \
                 parameter type at the same position in this translation unit"
            }
            Self::ArityDisagreement => {
                "the declared contract names a different number of parameters than the C \
                 declaration in this translation unit"
            }
            Self::VariadicDisagreement => {
                "the declared contract claims a variadic data tail for a function this \
                 translation unit does not declare variadic"
            }
            Self::UnspecifiedParameters => {
                "the C declaration in this translation unit specifies no parameters, so the \
                 declared parameter vector cannot be checked against it"
            }
            Self::UnreadableDeclaration => {
                "the C declaration in this translation unit carries a type this reader does not \
                 commit to"
            }
            Self::AmbiguousDeclaration => {
                "this translation unit declares the function more than once with projections \
                 that disagree"
            }
            Self::UnreadableDeclaredSources => {
                "the declared contract sources could not be re-read, so no declared ABI \
                 projection is available to compare"
            }
        }
    }

    #[must_use]
    pub const fn category(self) -> ReasonCategory {
        match self {
            Self::ReturnTypeDisagreement
            | Self::ParameterTypeDisagreement
            | Self::ArityDisagreement
            | Self::VariadicDisagreement
            | Self::AmbiguousDeclaration => ReasonCategory::StaleEvidence,
            Self::UnspecifiedParameters | Self::UnreadableDeclaration => {
                ReasonCategory::UnsupportedSyntax
            }
            Self::UnreadableDeclaredSources => ReasonCategory::MissingPremise,
        }
    }
}

/// The ABI agreement decision for one translation unit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AbiAgreement {
    /// Contracted names withdrawn for this unit, with the reason.
    withdrawn: BTreeMap<String, AbiRefusal>,
    /// Contracted names this unit declares and that agree.
    agreed: BTreeSet<String>,
    /// Contracted names this unit does not declare (`LOADER-ABI-UNDECLARED`).
    undeclared: BTreeSet<String>,
}

impl AbiAgreement {
    /// The withdrawn names with their reasons.
    pub(crate) fn withdrawn(&self) -> &BTreeMap<String, AbiRefusal> {
        &self.withdrawn
    }

    /// The compared-and-agreeing names.
    pub(crate) fn agreed(&self) -> &BTreeSet<String> {
        &self.agreed
    }

    /// The names carrying a contract that this unit does not declare.
    pub(crate) fn undeclared(&self) -> &BTreeSet<String> {
        &self.undeclared
    }

    /// Decide agreement for every contracted name, over one preprocessed
    /// translation unit and one contract environment.
    pub(crate) fn read(tokens: &[Token], contracts: &ResourceContracts) -> Self {
        // Card CALLEE-RETENTION-CLASS1: a return-code disposition and a
        // retaining declaration are compared with the unit's C declaration
        // like every other declaration.
        let names = contracts
            .functions()
            .keys()
            .chain(contracts.relational().keys())
            .chain(contracts.retaining().keys())
            .chain(contracts.guarded_retaining().keys())
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if names.is_empty() {
            return Self::default();
        }
        // An environment with no retained texts declares no spellings: an
        // inferred W4 summary or a directly built table never had one, so
        // there is nothing to compare and nothing is withdrawn.
        if contracts.source_assumptions().is_empty() {
            return Self {
                undeclared: names.iter().map(|name| (*name).to_owned()).collect(),
                ..Self::default()
            };
        }
        let Ok(declared) = contract_loader::declared_abi_signatures_of(contracts) else {
            // Poisoned: refuse every contracted callee rather than compare none.
            return Self {
                withdrawn: names
                    .iter()
                    .map(|name| ((*name).to_owned(), AbiRefusal::UnreadableDeclaredSources))
                    .collect(),
                ..Self::default()
            };
        };
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        // Card W43-ABI-ENUM-AND-CONST-PP (rule 12u): a first reading records
        // every enumeration type a contracted declaration names; Clang is
        // asked about them once, over the analysed unit, and the reading is
        // repeated with its answers. No enumeration, no launch.
        let (read, enumerations) =
            super::enum_projection::collecting(|| read_c_declarations(&texts, &names));
        let read = if enumerations.is_empty() {
            read
        } else {
            let answers = super::enum_projection::query(&enumerations);
            super::enum_projection::answering(answers, || read_c_declarations(&texts, &names))
        };
        let mut agreement = Self::default();
        for name in names {
            let Some(signature) = declared.get(name) else {
                // The environment carries a contract the retained texts do not
                // declare. It cannot have come from these sources, so it is not
                // a declared contract at all; nothing here withdraws it.
                agreement.undeclared.insert(name.to_owned());
                continue;
            };
            match read.get(name) {
                None => {
                    agreement.undeclared.insert(name.to_owned());
                }
                Some(Err(refusal)) => {
                    agreement.withdrawn.insert(name.to_owned(), *refusal);
                }
                Some(Ok(declaration)) => match compare(signature, declaration) {
                    Some(refusal) => {
                        agreement.withdrawn.insert(name.to_owned(), refusal);
                    }
                    None => {
                        agreement.agreed.insert(name.to_owned());
                    }
                },
            }
        }
        agreement
    }

    /// The environment with every withdrawn declaration removed.
    ///
    /// `None` means the environment is unchanged. The retained trusted sources
    /// are carried over untouched: they are the *declared texts*, and a unit
    /// that cannot use one of them does not make it undeclared.
    pub(crate) fn withdraw_environment(
        &self,
        contracts: &ResourceContracts,
    ) -> Option<ResourceContracts> {
        if self.withdrawn.is_empty() {
            return None;
        }
        let mut reduced = ResourceContracts::default();
        for (name, contract) in contracts.functions() {
            if !self.withdrawn.contains_key(name) {
                reduced.insert_contract(name, contract.clone());
            }
        }
        // Card CALLEE-RETENTION-CLASS1: the dispositions and retaining
        // declarations this unit does not withdraw stay in force.
        reduced.carry_side_declarations(contracts, |name| self.withdrawn.contains_key(name));
        reduced.set_source_assumptions(contracts.source_assumptions().clone());
        reduced.carry_cross_unit_imports(contracts);
        Some(reduced)
    }

    /// Rewrite every call site of a withdrawn callee so that the rest of the
    /// analyzer sees an uncontracted, *named* callee.
    ///
    /// `None` means the stream is unchanged.
    pub(crate) fn withdraw_call_sites(&self, tokens: &[Token]) -> Option<Vec<Token>> {
        if self.withdrawn.is_empty() {
            return None;
        }
        let mut rewritten: Option<Vec<Token>> = None;
        for index in 0..tokens.len() {
            if tokens.get(index + 1).is_none_or(|token| token.text != "(") {
                continue;
            }
            let Some(refusal) = self.withdrawn.get(&tokens[index].text).copied() else {
                continue;
            };
            if !is_call_position(tokens, index) {
                continue;
            }
            let target = rewritten.get_or_insert_with(|| tokens.to_vec());
            target[index].text = format!(
                "{WITHDRAWN_PREFIX}{}{WITHDRAWN_SEPARATOR}{}",
                refusal.token(),
                tokens[index].text
            );
        }
        rewritten
    }
}

/// Whether the name at `index` occurs where a *call* can occur.
///
/// A declaration or definition of the same name is left alone: rewriting it
/// would rename a function this unit declares, and the declaration is exactly
/// what the comparison read. The test is the one card HASH-CLAUSE established,
/// conservative in the refusing direction.
///
/// Card **LOADER-ABI-MODES** withdraws by the same mechanism and reuses this
/// rule rather than restating it, so the two withdrawals cannot drift apart in
/// what they consider a call site.
pub(crate) fn is_call_position(tokens: &[Token], index: usize) -> bool {
    match index.checked_sub(1).map(|previous| &tokens[previous].text) {
        None => false,
        Some(previous) if previous == "*" => false,
        Some(previous) if previous == "return" => true,
        Some(previous) => !is_c_identifier(previous),
    }
}

fn is_c_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// One C function declaration, projected into the loader's ABI vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CDeclaration {
    /// `None` for a type this reader will not commit to.
    result: Option<BTreeSet<&'static str>>,
    parameters: Vec<Option<BTreeSet<&'static str>>>,
    variadic: bool,
    /// The C parameter list was empty, which specifies no parameters (C11
    /// 6.7.6.3p14 for a declaration that is not a definition).
    unspecified: bool,
}

/// Compare one declared contract signature against one C declaration.
fn compare(signature: &DeclaredAbiSignature, declaration: &CDeclaration) -> Option<AbiRefusal> {
    let Some(result) = &declaration.result else {
        return Some(AbiRefusal::UnreadableDeclaration);
    };
    if !result.contains(signature.result.as_str()) {
        return Some(AbiRefusal::ReturnTypeDisagreement);
    }
    if declaration.unspecified {
        // An empty parameter list in a declaration that is not a definition
        // specifies *no* information about the parameters (C11 6.7.6.3p14), so
        // a declared parameter vector cannot be checked against it and the
        // contract is withdrawn.
        //
        // One case is admitted, on a stated ground rather than by exception: a
        // contract that names **no** parameters and claims **no** variadic tail
        // asserts nothing about parameters at all, and the analyzer's existing
        // call-site arity gate already refuses every call that passes an actual
        // to it (`arguments.len() > contract.parameters.len()` with no tail).
        // There is therefore nothing an unspecified list could contradict.
        // Measured instance: `double flb_time_now();` (Fluent Bit
        // `include/fluent-bit/flb_lib.h:94`), whose definition in `flb_lib.c`
        // does specify the empty list and agrees on its own.
        if !signature.parameters.is_empty() || signature.variadic_data_tail {
            return Some(AbiRefusal::UnspecifiedParameters);
        }
        return None;
    }
    if signature.parameters.len() != declaration.parameters.len() {
        return Some(AbiRefusal::ArityDisagreement);
    }
    for (declared, projected) in signature.parameters.iter().zip(&declaration.parameters) {
        let Some(projected) = projected else {
            return Some(AbiRefusal::UnreadableDeclaration);
        };
        if !projected.contains(declared.as_str()) {
            return Some(AbiRefusal::ParameterTypeDisagreement);
        }
    }
    // A variadic C function whose declaration carries no tail clause is not a
    // disagreement: the clause only ever *widens* what the arity gate admits,
    // so its absence is the refusing direction. The other direction is a
    // disagreement, because a tail clause would admit actuals the C
    // declaration cannot accept at all.
    if signature.variadic_data_tail && !declaration.variadic {
        return Some(AbiRefusal::VariadicDisagreement);
    }
    None
}

/// Read every declaration of the named functions in one preprocessed unit.
///
/// A name declared more than once composes: an unspecified parameter list
/// composes with a specified one to the specified signature (C11 6.2.7p3), and
/// two specified declarations that project differently are ambiguous and refuse.
fn read_c_declarations<'a>(
    texts: &[&'a str],
    names: &BTreeSet<&'a str>,
) -> BTreeMap<String, Result<CDeclaration, AbiRefusal>> {
    let typedefs = read_typedefs(texts);
    let mut found: BTreeMap<String, Result<CDeclaration, AbiRefusal>> = BTreeMap::new();
    for index in 0..texts.len() {
        if texts.get(index + 1) != Some(&"(") || !names.contains(texts[index]) {
            continue;
        }
        let Some(declaration) = read_declaration(texts, index, &typedefs) else {
            continue;
        };
        let name = texts[index].to_owned();
        match found.remove(&name) {
            None => {
                found.insert(name, declaration);
            }
            Some(existing) => {
                found.insert(name, compose(existing, declaration));
            }
        }
    }
    found
}

/// Card **W44-C11-GENERIC-SELECTION**: does this unit's own C declaration of
/// `name` read, unambiguously, as a function returning an object pointer?
///
/// The generic-selection reader removes a pointer cast around a call only
/// when the call yields a pointer, so that the cast converts a pointer to a
/// pointer and leaves its value unchanged. The answer comes from the same
/// declaration reader and the same token stream as the ABI gate, so W4
/// derivation, W4 validation and certificate rechecking reproduce it. An
/// unreadable, ambiguous or absent declaration, a `void` or scalar result and
/// a function pointer (`fnptr`) all answer `false`, the refusing direction.
pub(crate) fn declares_object_pointer_result(texts: &[&str], name: &str) -> bool {
    let names = BTreeSet::from([name]);
    matches!(
        read_c_declarations(texts, &names).get(name),
        Some(Ok(CDeclaration { result: Some(result), .. }))
            if !result.is_empty() && result.iter().all(|spelling| spelling.starts_with("ptr<"))
    )
}

/// Rule 36: positively matching depth-one object-pointer result/formal types.
/// Card RULE36-PILOT: the decision is the package's
/// (`rule_36_exact_formal_return::source::pointer_formals`); this reads the
/// declaration for it.
pub(super) fn exact_pointer_formals(texts: &[&str], name: &str) -> BTreeSet<usize> {
    let declarations = read_c_declarations(texts, &BTreeSet::from([name]));
    let Some(Ok(declaration)) = declarations.get(name) else {
        return BTreeSet::new();
    };
    rule_36_exact_formal_return::source::pointer_formals(
        declaration.unspecified,
        declaration.result.as_ref(),
        &declaration.parameters,
    )
}

/// Compose two readings of the same name in one unit.
fn compose(
    left: Result<CDeclaration, AbiRefusal>,
    right: Result<CDeclaration, AbiRefusal>,
) -> Result<CDeclaration, AbiRefusal> {
    let (left, right) = match (left, right) {
        // A reading this reader refused wins: it is the fail-closed direction,
        // and a second readable declaration does not make the first one read.
        (Err(refusal), _) | (_, Err(refusal)) => return Err(refusal),
        (Ok(left), Ok(right)) => (left, right),
    };
    if left.result != right.result {
        return Err(AbiRefusal::AmbiguousDeclaration);
    }
    match (left.unspecified, right.unspecified) {
        (true, true) => Ok(left),
        (true, false) => Ok(right),
        (false, true) => Ok(left),
        (false, false) => {
            if left.parameters == right.parameters && left.variadic == right.variadic {
                Ok(left)
            } else {
                Err(AbiRefusal::AmbiguousDeclaration)
            }
        }
    }
}

/// Storage-class, function and inline specifiers that carry no type.
const DISCARDED_SPECIFIERS: &[&str] = &[
    "static",
    "extern",
    "inline",
    "__inline",
    "__inline__",
    "register",
    "auto",
    "_Noreturn",
    "typedef",
];

/// Qualifiers that do not change the ABI projection of the type they qualify.
const QUALIFIERS: &[&str] = &[
    "const",
    "volatile",
    "restrict",
    "__restrict",
    "__restrict__",
];

/// Declaration suffixes this reader steps over: they carry no type.
const SUFFIX_INTRODUCERS: &[&str] = &["__attribute__", "__attribute", "__asm__", "asm", "__asm"];

/// How deep a typedef chain may be followed before the reader gives up.
const TYPEDEF_DEPTH_LIMIT: usize = 8;

/// What one `typedef` name stands for, as far as this reader will commit.
///
/// Card **CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF** replaced the earlier
/// `Option<(Vec<String>, usize)>` with this enum so that two shapes the earlier
/// reader recorded as *unreadable* can be committed to: a `struct`/`union`/
/// `enum` **defined by the typedef itself**, and a pointer to function. The
/// unreadable class is narrowed, never deleted: [`TypedefTarget::Unresolved`]
/// keeps every other shape fail-closed exactly as before.
#[derive(Clone, Debug, PartialEq, Eq)]
enum TypedefTarget {
    /// Specifier words plus pointer depth — `typedef struct cio_chunk *handle;`.
    Declarator {
        specifiers: Vec<String>,
        stars: usize,
    },
    /// A `struct`/`union`/`enum` defined by this typedef and given **no tag**,
    /// plus pointer depth — `typedef union { … } pthread_mutex_t;`.
    ///
    /// A *tagged* definition is not recorded here: it is recorded as the
    /// [`TypedefTarget::Declarator`] `["struct", tag]` it is exactly equal to,
    /// so that `typedef struct msgpack_packer { … } msgpack_packer;` and a
    /// second `typedef struct msgpack_packer msgpack_packer;` in the same unit
    /// compose to one reading instead of colliding.
    Aggregate { stars: usize },
    /// Card **W43-ABI-ENUM-AND-CONST-PP** (rule 12u): an `enum` defined by
    /// this typedef with **no tag** and no pointer level —
    /// `typedef enum { … } rd_async_t;`. As a value it is the integer
    /// projection Clang states for it ([`super::enum_projection`]); under a
    /// pointer it is the object type [`TypedefTarget::Aggregate`] reads.
    UntaggedEnum,
    /// A pointer to function, at exactly one pointer level —
    /// `typedef int (*msgpack_packer_write)(void *, const char *, size_t);`.
    FunctionPointer,
    /// Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES**: a function *type* —
    /// `typedef rd_kafka_op_res_t(rd_kafka_q_serve_cb_t)(rd_kafka_t *, …);`.
    /// A pointer to it, at exactly one level, is the same `fnptr` the
    /// pointer typedef and the inline declarator read; the type itself, and
    /// a deeper pointer, stay unread.
    Function,
    /// This reader does not commit to the target.
    Unresolved,
}

/// Read every `typedef` this reader can commit to.
///
/// A typedef is *transparent* in C — the declared type is the target type — so
/// resolving one is exact rather than an approximation, which is why the
/// comparison admits through it. A name typedef'd twice to different targets
/// records [`TypedefTarget::Unresolved`], which makes every declaration using it
/// unreadable, and so does every declarator shape below that this reader does
/// not parse.
///
/// Three shapes are committed to:
///
/// 1. a plain declarator — specifier words plus trailing `*`s;
/// 2. an aggregate **definition**, `typedef struct [TAG] { … } *^k NAME;`;
/// 3. a function pointer, `typedef RET (*NAME)(…);`.
///
/// The terminating `;` is now found at brace/paren/bracket depth zero rather
/// than as the first of `;`/`{`/`}`. That is what lets shapes 2 and 3 be seen
/// at all — the earlier scan stopped at the aggregate's own opening brace and
/// abandoned the declaration.
fn read_typedefs(texts: &[&str]) -> BTreeMap<String, TypedefTarget> {
    let mut typedefs: BTreeMap<String, TypedefTarget> = BTreeMap::new();
    for index in 0..texts.len() {
        if texts[index] != "typedef" {
            continue;
        }
        let Some(end) = typedef_end(texts, index + 1) else {
            continue;
        };
        let slice = &texts[index + 1..end];
        let Some((name, resolved)) = read_typedef_declarator(slice) else {
            continue;
        };
        match typedefs.get(name) {
            Some(existing) if *existing == resolved => {}
            Some(_) => {
                typedefs.insert(name.to_owned(), TypedefTarget::Unresolved);
            }
            None => {
                typedefs.insert(name.to_owned(), resolved);
            }
        }
    }
    typedefs
}

/// The index of the `;` ending a `typedef` that starts at `start`, at
/// brace/paren/bracket depth zero.
fn typedef_end(texts: &[&str], start: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(start) {
        match *text {
            "{" | "(" | "[" => depth += 1,
            "}" | ")" | "]" => depth = depth.checked_sub(1)?,
            ";" if depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

/// Read one `typedef` body — the tokens between `typedef` and its `;`.
///
/// `None` means "not a shape this reader recognises at all", which leaves the
/// name absent from the map and therefore unreadable, exactly as before.
fn read_typedef_declarator<'a>(slice: &[&'a str]) -> Option<(&'a str, TypedefTarget)> {
    if let Some(read) = read_function_pointer_typedef(slice) {
        return Some(read);
    }
    if let Some(read) = read_function_typedef(slice) {
        return Some(read);
    }
    if let Some(read) = read_aggregate_typedef(slice) {
        return Some(read);
    }
    let last = slice.last().copied().filter(|last| is_c_identifier(last))?;
    let body = &slice[..slice.len() - 1];
    let resolved = if body
        .iter()
        .any(|text| matches!(*text, "(" | ")" | "[" | "]" | "," | "{" | "}"))
    {
        TypedefTarget::Unresolved
    } else {
        let stars = body.iter().rev().take_while(|text| **text == "*").count();
        let specifiers = &body[..body.len() - stars];
        if specifiers.contains(&"*") {
            TypedefTarget::Unresolved
        } else {
            TypedefTarget::Declarator {
                specifiers: specifiers.iter().map(|text| (*text).to_owned()).collect(),
                stars,
            }
        }
    };
    Some((last, resolved))
}

/// `typedef RET (*NAME)(PARAMETERS)` — nothing else.
///
/// Only a single pointer level is committed to. A `typedef RET (**NAME)(…)`,
/// a parenthesised declarator that is not exactly `* NAME`, and a return type
/// carrying its own declarator punctuation are all left to the caller's
/// fall-through, which records them unreadable.
fn read_function_pointer_typedef<'a>(slice: &[&'a str]) -> Option<(&'a str, TypedefTarget)> {
    if slice.last() != Some(&")") {
        return None;
    }
    let parameters_open = matching_open(slice, slice.len() - 1)?;
    let declarator_close = parameters_open.checked_sub(1)?;
    if slice[declarator_close] != ")" {
        return None;
    }
    let declarator_open = matching_open(slice, declarator_close)?;
    let ["*", name] = &slice[declarator_open + 1..declarator_close] else {
        return None;
    };
    if !is_c_identifier(name) {
        return None;
    }
    let head = &slice[..declarator_open];
    if head.is_empty()
        || head
            .iter()
            .any(|text| matches!(*text, "(" | ")" | "[" | "]" | "," | "{" | "}"))
    {
        return None;
    }
    Some((name, TypedefTarget::FunctionPointer))
}

/// Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES**: `typedef RET (NAME)(PARAMETERS)`
/// or `typedef RET NAME(PARAMETERS)` — a function type, nothing else (a return
/// type carrying its own declarator punctuation falls through, unread).
fn read_function_typedef<'a>(mut slice: &[&'a str]) -> Option<(&'a str, TypedefTarget)> {
    // A trailing GNU attribute qualifies the function type, not its shape
    // (`RD_WARN_UNUSED_RESULT`, `__attribute__((warn_unused_result))`).
    while slice.last() == Some(&")") {
        let open = matching_open(slice, slice.len() - 1)?;
        match open.checked_sub(1).map(|at| slice[at]) {
            Some("__attribute__") => slice = &slice[..open - 1],
            _ => break,
        }
    }
    if slice.last() != Some(&")") {
        return None;
    }
    let parameters_open = matching_open(slice, slice.len() - 1)?;
    let before = parameters_open.checked_sub(1)?;
    let (name, head_end) = if slice[before] == ")" {
        let declarator_open = matching_open(slice, before)?;
        let [name] = &slice[declarator_open + 1..before] else {
            return None;
        };
        (*name, declarator_open)
    } else {
        (slice[before], before)
    };
    if !is_c_identifier(name) {
        return None;
    }
    let head = &slice[..head_end];
    if head.is_empty()
        || head
            .iter()
            .any(|text| matches!(*text, "(" | ")" | "[" | "]" | "," | "{" | "}"))
    {
        return None;
    }
    Some((name, TypedefTarget::Function))
}

/// `typedef struct|union|enum [TAG] { … } *^k NAME` — nothing else.
///
/// A tagged definition is recorded as the specifier pair it *is*, so that it
/// composes with a separate `typedef struct TAG TAG;` instead of colliding with
/// it. An untagged one has no specifier spelling and is recorded as
/// [`TypedefTarget::Aggregate`].
fn read_aggregate_typedef<'a>(slice: &[&'a str]) -> Option<(&'a str, TypedefTarget)> {
    let kind = *slice.first()?;
    if !matches!(kind, "struct" | "union" | "enum") {
        return None;
    }
    let (tag, brace) = match (slice.get(1), slice.get(2)) {
        (Some(&"{"), _) => (None, 1),
        (Some(tag), Some(&"{")) if is_c_identifier(tag) => (Some(*tag), 2),
        _ => return None,
    };
    let close = matching_close(slice, brace)?;
    let rest = &slice[close + 1..];
    let stars = rest.iter().take_while(|text| **text == "*").count();
    let [name] = &rest[stars..] else {
        return None;
    };
    if !is_c_identifier(name) {
        return None;
    }
    let target = match tag {
        Some(tag) => TypedefTarget::Declarator {
            specifiers: vec![kind.to_owned(), tag.to_owned()],
            stars,
        },
        // Card W43-ABI-ENUM-AND-CONST-PP (rule 12u): an untagged enumeration
        // is recorded as one, so its value can be projected; a pointer to it
        // is an object pointer like any other aggregate's.
        None if kind == "enum" && stars == 0 => TypedefTarget::UntaggedEnum,
        None => TypedefTarget::Aggregate { stars },
    };
    Some((name, target))
}

/// Read the declaration whose declarator name stands at `index`, if the tokens
/// there really are a function declaration or definition.
///
/// `None` means "not a declaration of this name" — a call, a cast, an
/// initializer — and leaves the name to any other occurrence. `Some(Err(_))`
/// means "a declaration this reader will not commit to", which refuses.
fn read_declaration(
    texts: &[&str],
    index: usize,
    typedefs: &BTreeMap<String, TypedefTarget>,
) -> Option<Result<CDeclaration, AbiRefusal>> {
    let close = matching_close(texts, index + 1)?;
    // A declarator's parameter list is followed by `;` or by the body, with
    // only type-free suffixes in between.
    let mut after = close + 1;
    while let Some(introducer) = texts.get(after) {
        if !SUFFIX_INTRODUCERS.contains(introducer) || texts.get(after + 1) != Some(&"(") {
            break;
        }
        after = matching_close(texts, after + 1)? + 1;
    }
    if !matches!(texts.get(after), Some(&";") | Some(&"{")) {
        return None;
    }
    // The specifier prefix. A call statement has none, which is what separates
    // `f(x);` from `int f(int);`.
    let mut start = index;
    while start > 0 {
        let previous = texts[start - 1];
        if previous == "*"
            || QUALIFIERS.contains(&previous)
            || DISCARDED_SPECIFIERS.contains(&previous)
            || matches!(previous, "struct" | "union" | "enum")
            || (is_c_identifier(previous) && !RESERVED_NON_SPECIFIERS.contains(&previous))
        {
            start -= 1;
            continue;
        }
        break;
    }
    if start == index {
        return None;
    }
    let prefix = &texts[start..index];
    if prefix.contains(&"typedef") {
        return None;
    }
    let cdecl_count = prefix.iter().filter(|word| **word == "__cdecl").count();
    if cdecl_count > 1 {
        return Some(Err(AbiRefusal::UnreadableDeclaration));
    }
    let prefix = prefix
        .iter()
        .copied()
        .filter(|word| *word != "__cdecl")
        .collect::<Vec<_>>();
    let result = if cdecl_count == 1 && has_target_dependent_long(&prefix, typedefs) {
        None
    } else {
        project_declarator(&prefix, typedefs, true)
    };
    let parameters = &texts[index + 2..close];
    if parameters.is_empty() {
        return Some(Ok(CDeclaration {
            result,
            parameters: Vec::new(),
            variadic: false,
            // A definition's empty list specifies *no* parameters, and a
            // non-definition's specifies nothing at all (C11 6.7.6.3p14).
            unspecified: texts.get(after) != Some(&"{"),
        }));
    }
    if parameters == ["void"] {
        return Some(Ok(CDeclaration {
            result,
            parameters: Vec::new(),
            variadic: false,
            unspecified: false,
        }));
    }
    let mut groups = Vec::new();
    let mut depth = 0usize;
    let mut current: Vec<&str> = Vec::new();
    for text in parameters {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.saturating_sub(1),
            "," if depth == 0 => {
                groups.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(text);
    }
    groups.push(current);
    let mut projected = Vec::new();
    let mut variadic = false;
    for (position, group) in groups.iter().enumerate() {
        if group.as_slice() == [".", ".", "."] {
            if position + 1 != groups.len() {
                return Some(Err(AbiRefusal::UnreadableDeclaration));
            }
            variadic = true;
            continue;
        }
        // A K&R identifier list declares no types at all: every group is a bare
        // name. It is not a prototype and cannot be compared.
        projected.push(
            if cdecl_count == 1 && has_target_dependent_long(group, typedefs) {
                None
            } else {
                project_parameter(group, typedefs)
            },
        );
    }
    if projected.is_empty() {
        return Some(Err(AbiRefusal::UnreadableDeclaration));
    }
    Some(Ok(CDeclaration {
        result,
        parameters: projected,
        variadic,
        unspecified: false,
    }))
}

/// Ordinary `long` differs between LP64 and MSVC LLP64. A newly readable
/// cdecl declaration must not inherit the old LP64 reading for it. Refuse
/// that ambiguity, including typedef chains, until the reader carries a
/// target. `long long` and `__int64` have the stated 64-bit reading.
fn has_target_dependent_long(words: &[&str], typedefs: &BTreeMap<String, TypedefTarget>) -> bool {
    words.iter().filter(|word| **word == "long").count() == 1
        || words.iter().any(|word| {
            matches!(resolve_typedef(word, typedefs, 0),
                Some(TypedefTarget::Declarator { specifiers, .. })
                    if specifiers.iter().filter(|specifier| *specifier == "long").count() == 1)
        })
}

/// Identifiers that are reserved words and can never be a type specifier or a
/// typedef name, so meeting one ends the specifier prefix.
const RESERVED_NON_SPECIFIERS: &[&str] = &[
    "return",
    "if",
    "else",
    "while",
    "for",
    "do",
    "switch",
    "case",
    "default",
    "break",
    "continue",
    "goto",
    "sizeof",
    "_Alignof",
    "alignof",
    "typeof",
    "__typeof",
    "__typeof__",
];

/// `RET (*NAME)(PARAMETERS)` or the abstract `RET (*)(PARAMETERS)` at a
/// parameter position, exactly the declarator shape
/// [`read_function_pointer_typedef`] commits to: one pointer level, a return
/// type with no declarator punctuation of its own. Anything else (`(**f)`,
/// `(*const f)`, `(*f[2])`, a function returning a function pointer) is left
/// to the caller's refusal (`ABI-FNPTR-DEPTH1` and the nested-declarator
/// boundary stay).
fn inline_function_pointer_parameter(group: &[&str]) -> Option<BTreeSet<&'static str>> {
    if group.last() != Some(&")") {
        return None;
    }
    let parameters_open = matching_open(group, group.len() - 1)?;
    let declarator_close = parameters_open.checked_sub(1)?;
    if group[declarator_close] != ")" {
        return None;
    }
    let declarator_open = matching_open(group, declarator_close)?;
    match &group[declarator_open + 1..declarator_close] {
        ["*"] => {}
        ["*", name] if is_c_identifier(name) => {}
        _ => return None,
    }
    let head = &group[..declarator_open];
    if head.is_empty()
        || head
            .iter()
            .any(|text| matches!(*text, "(" | ")" | "[" | "]" | "," | "{" | "}"))
    {
        return None;
    }
    Some(BTreeSet::from([FUNCTION_POINTER_ABI_TYPE]))
}

/// Project one parameter declaration group.
fn project_parameter(
    group: &[&str],
    typedefs: &BTreeMap<String, TypedefTarget>,
) -> Option<BTreeSet<&'static str>> {
    // Card CG22-INDIRECT-CALLS-AND-CALLBACKS (absorbing
    // LOADER-ABI-INLINE-FNPTR-DECLARATOR): an inline function-pointer
    // parameter `RET (*NAME)(PARAMETERS)` is the same type as the typedef
    // CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF reads, so it projects to the same
    // opaque `fnptr`, on which no custody mode can be written.
    if let Some(projection) = inline_function_pointer_parameter(group) {
        return Some(projection);
    }
    // Any other nested declarator (a pointer to a function pointer, a
    // parenthesised declarator) is not read.
    if group.iter().any(|text| *text == "(" || *text == ")") {
        return None;
    }
    let mut rest = group.to_vec();
    // C adjusts an array parameter to a pointer to its element type (C11
    // 6.7.6.3p7), so a declarator suffix `[]` or `[N]` is exactly one more
    // pointer level over the same element type.
    let mut array_levels = 0usize;
    while rest.last() == Some(&"]") {
        let open = rest.iter().rposition(|text| *text == "[")?;
        let bounds = &rest[open + 1..rest.len() - 1];
        if !array_bound_is_read(bounds) {
            return None;
        }
        rest.truncate(open);
        array_levels += 1;
    }
    if array_levels > 1 {
        return None;
    }
    // Drop the declared parameter name, if it has one. `int` alone is an
    // abstract declarator; `int x` names one. A trailing word that is itself a
    // type word — an arithmetic keyword, a qualifier, a typedef name, or the
    // tag of a `struct`/`union`/`enum` specifier — is never a parameter name
    // here, and refusing to drop it makes the group unprojectable rather than
    // misread.
    if let Some(last) = rest.last().copied()
        && rest.len() > 1
        && is_c_identifier(last)
        && !ARITHMETIC_WORDS.contains(&last)
        && !QUALIFIERS.contains(&last)
        && !typedefs.contains_key(last)
        && !matches!(rest[rest.len() - 2], "struct" | "union" | "enum")
    {
        rest.pop();
    }
    if array_levels == 1 {
        // The adjusted pointer's own qualifiers are the bound's, and
        // `array_bound_is_read` has already dropped them (C11 6.7.6.3p7,
        // p15). A qualifier left at the end of the *element* declarator —
        // `char *const p[]`, an array of const pointers — qualifies the inner
        // pointer of `char *const *`, which this vocabulary cannot spell, so it
        // is not dropped here and `project_declarator` refuses it, exactly as
        // it refuses the pointer spelling (card W43-LIBC-RESTRICT-PROTOTYPES).
        rest.push("*");
    } else {
        drop_parameter_object_qualifiers(&mut rest);
    }
    project_declarator(&rest, typedefs, false)
}

/// Whether one array parameter's bound — the words between its `[` and `]` —
/// is read.
///
/// C adjusts a parameter declared "array of T" to "qualified pointer to T,
/// where the type qualifiers (if any) are those specified within the `[` and
/// `]`" (C11 6.7.6.3p7), and then takes the parameter as having the
/// unqualified version of that type (6.7.6.3p15). So `T a[]`, `T a[8]`,
/// `T a[restrict]` and `T a[restrict n]` are one parameter type, `T *`: the
/// bound's qualifiers qualify the parameter object exactly as a qualifier after
/// the last `*` does, and a size — constant or, at function prototype scope,
/// variable (6.7.6.2p5 treats it as `*`) — is not part of the adjusted type.
///
/// Before card W43-LIBC-RESTRICT-PROTOTYPES only an empty or plain decimal
/// bound was read. glibc's `regexec` spells `regmatch_t
/// __pmatch[_Restrict_arr_ _REGEX_NELTS (__nmatch)]`, which Clang preprocesses
/// to `[__restrict __nmatch]`, so it was withdrawn with
/// `abi-unreadable-declaration`. A bound is now read when it is a run of
/// qualifiers followed by at most one size word, a decimal constant or an
/// identifier. `static`, `[*]` and a size of more than one token still refuse:
/// no measured prototype needs them, so they stay refused rather than admitted
/// untested. `QCore.PointerQualifierReader`'s array-bound section is the Lean
/// model.
fn array_bound_is_read(bounds: &[&str]) -> bool {
    let qualifiers = bounds
        .iter()
        .take_while(|word| QUALIFIERS.contains(word))
        .count();
    match &bounds[qualifiers..] {
        [] => true,
        [size] => {
            size.bytes().all(|byte| byte.is_ascii_digit())
                || (is_c_identifier(size)
                    && *size != "static"
                    && !RESERVED_NON_SPECIFIERS.contains(size)
                    && !ARITHMETIC_WORDS.contains(size))
        }
        _ => false,
    }
}

/// Card W43-LIBC-RESULT-ALIAS (`W43-LIBC-POINTER-QUALIFIER-READER`): drop the
/// qualifiers that follow a parameter declarator's **last** `*`.
///
/// Those qualifiers qualify the parameter object itself — the pointer — not
/// anything it points to, and C drops them for type compatibility: "each
/// parameter declared with qualified type is taken as having the unqualified
/// version of its declared type" (C11 6.7.6.3p15). So `const char *__restrict
/// __format` and `const char *__format` are one parameter type, and glibc
/// spells `__restrict` on `memcpy`, `strcpy`, `strncpy`, the printf family and
/// others. Before this card the reader counted only *trailing* `*` words, so
/// such a parameter projected to nothing and its declaration was withdrawn
/// with `abi-unreadable-declaration` at every call site.
///
/// Exactly one position is widened. A qualifier *between* two `*` qualifies an
/// inner pointer, which this vocabulary cannot spell, and still refuses in
/// [`project_declarator`]; a qualifier with no `*` before it qualifies the
/// pointee or a scalar and is read there exactly as before; and a result type
/// is not a parameter, so this is applied to parameters only.
fn drop_parameter_object_qualifiers(words: &mut Vec<&str>) {
    let Some(last_star) = words.iter().rposition(|word| *word == "*") else {
        return;
    };
    if words[last_star + 1..]
        .iter()
        .all(|word| QUALIFIERS.contains(word))
    {
        words.truncate(last_star + 1);
    }
}

const ARITHMETIC_WORDS: &[&str] = &[
    "__int64", "void", "char", "short", "int", "long", "float", "double", "signed", "unsigned",
    "_Bool", "bool", "_Complex",
];

/// The pointer projection of `stars` levels over a pointee that is or is not
/// `const`-qualified.
fn pointer_projection(stars: usize, const_pointee: bool) -> BTreeSet<&'static str> {
    match (stars, const_pointee) {
        (1, false) => BTreeSet::from(["ptr<mut, u8>"]),
        (1, true) => BTreeSet::from(["ptr<const, u8>"]),
        (2, false) => BTreeSet::from(["ptr<mut, ptr<mut, u8>>"]),
        // Card W43-ABI-ENUM-AND-CONST-PP (rule 12v): a pointer to a pointer
        // to const — `const char **`, `const void **`. The `const` is on the
        // innermost pointee; the pointer in between (the slot) is not
        // qualified. Every caller reaches this only with that reading: a
        // qualifier written between the two `*` and a qualifier on a typedef
        // name that stands for a pointer both refuse before it.
        (2, true) => BTreeSet::from([CONST_POINTEE_SLOT]),
        _ => BTreeSet::new(),
    }
}

/// The spelling of a pointer to a pointer to const (rule 12v):
/// `ptr<mut, ptr<const, u8>>`. The outer `mut` says the slot the outer pointer
/// designates is not `const`; the inner `const` is the innermost pointee's.
const CONST_POINTEE_SLOT: &str = "ptr<mut, ptr<const, u8>>";

/// Whether the typedef `name` stands for a pointer (object or function).
fn typedef_is_pointer(name: &str, typedefs: &BTreeMap<String, TypedefTarget>) -> bool {
    matches!(
        resolve_typedef(name, typedefs, 0),
        Some(
            TypedefTarget::Declarator { stars: 1.., .. }
                | TypedefTarget::Aggregate { stars: 1.. }
                | TypedefTarget::FunctionPointer
        )
    )
}

/// Rule 12v: whether some link of the typedef chain from `name` writes a
/// qualifier on a name that stands for a pointer — `typedef const str
/// cstr_t;` for `typedef char *str;` — which qualifies a pointer level, not
/// the innermost pointee. [`resolve_typedef`] folds such a link away, which
/// the depth-two readings before this card never had to care about; an over-
/// deep chain answers `true`.
fn chain_qualifies_a_pointer(
    name: &str,
    typedefs: &BTreeMap<String, TypedefTarget>,
    depth: usize,
) -> bool {
    if depth >= TYPEDEF_DEPTH_LIMIT {
        return true;
    }
    let Some(TypedefTarget::Declarator { specifiers, .. }) = typedefs.get(name) else {
        return false;
    };
    let qualified = specifiers
        .iter()
        .any(|word| QUALIFIERS.contains(&word.as_str()));
    let core = specifiers
        .iter()
        .map(String::as_str)
        .filter(|word| !QUALIFIERS.contains(word) && !DISCARDED_SPECIFIERS.contains(word))
        .collect::<Vec<_>>();
    match core.as_slice() {
        [inner]
            if !ARITHMETIC_WORDS.contains(inner)
                && !matches!(*inner, "struct" | "union" | "enum") =>
        {
            (qualified && typedef_is_pointer(inner, typedefs))
                || chain_qualifies_a_pointer(inner, typedefs, depth + 1)
        }
        _ => false,
    }
}

/// Rule 12v: a depth-two pointer-to-const reading reached through the
/// typedef `name` stands only when no link of its chain qualifies a pointer.
fn refuse_qualified_chain(
    projection: Option<BTreeSet<&'static str>>,
    name: &str,
    typedefs: &BTreeMap<String, TypedefTarget>,
) -> Option<BTreeSet<&'static str>> {
    match projection {
        Some(spellings)
            if spellings.contains(CONST_POINTEE_SLOT)
                && chain_qualifies_a_pointer(name, typedefs, 0) =>
        {
            None
        }
        other => other,
    }
}

/// Project one declarator's specifier words plus its pointer stars.
///
/// `result` admits `void` as `unit`; a parameter never may (the loader's
/// vocabulary has no `unit` parameter, and `(void)` is handled as an empty
/// parameter vector before this is reached).
fn project_declarator(
    words: &[&str],
    typedefs: &BTreeMap<String, TypedefTarget>,
    result: bool,
) -> Option<BTreeSet<&'static str>> {
    let stars = words.iter().rev().take_while(|word| **word == "*").count();
    let head = &words[..words.len() - stars];
    // A qualifier standing between two pointer levels qualifies the inner
    // pointer, which this vocabulary cannot spell. Refuse rather than guess.
    if stars > 0 && head.contains(&"*") {
        return None;
    }
    let specifiers = head
        .iter()
        .copied()
        .filter(|word| !DISCARDED_SPECIFIERS.contains(word))
        .collect::<Vec<_>>();
    let const_pointee = specifiers.contains(&"const");
    let core = specifiers
        .iter()
        .copied()
        .filter(|word| !QUALIFIERS.contains(word))
        .collect::<Vec<_>>();
    if stars > 0 {
        // Every object pointer projects to the same opaque spelling, so the
        // pointee type only has to be *readable*, not identified. A pointee
        // this reader cannot read at all — a nested declarator, an unresolved
        // typedef standing for a pointer — is refused below by the same
        // recursion.
        let pointee_readable = match core.as_slice() {
            [] => false,
            ["struct", tag] | ["union", tag] | ["enum", tag] => is_c_identifier(tag),
            ["struct"] | ["union"] | ["enum"] => false,
            [name] if !ARITHMETIC_WORDS.contains(name) => {
                // Card W43-ABI-ENUM-AND-CONST-PP (rule 12v): a qualifier
                // written on a typedef name that stands for a pointer
                // qualifies that pointer — `const str *` is `char *const *`
                // — never the innermost pointee. That is the inner-pointer
                // qualifier this vocabulary cannot spell, so it refuses, as
                // it did before the card (no depth-two projection carried a
                // `const`); and so does a chain whose own links qualify a
                // pointer, for a reading that would carry one now.
                if const_pointee && typedef_is_pointer(name, typedefs) {
                    return None;
                }
                match resolve_typedef(name, typedefs, 0) {
                    Some(TypedefTarget::Declarator {
                        specifiers,
                        stars: target_stars,
                    }) => {
                        let target = specifiers.iter().map(String::as_str).collect::<Vec<_>>();
                        return refuse_qualified_chain(
                            project_declarator_resolved(
                                &target,
                                target_stars + stars,
                                const_pointee,
                                typedefs,
                                result,
                            ),
                            name,
                            typedefs,
                        );
                    }
                    // An aggregate defined by its own typedef is an object type
                    // exactly as a tagged one is, and the pointer projection
                    // reads no tag, so a pointer to it projects identically.
                    Some(TypedefTarget::Aggregate {
                        stars: target_stars,
                    }) => {
                        let projection = pointer_projection(target_stars + stars, const_pointee);
                        return (!projection.is_empty()).then_some(projection);
                    }
                    // Card W43-ABI-ENUM-AND-CONST-PP: so does a pointer to an
                    // untagged enumeration.
                    Some(TypedefTarget::UntaggedEnum) => {
                        let projection = pointer_projection(stars, const_pointee);
                        return (!projection.is_empty()).then_some(projection);
                    }
                    // `ABI-FNPTR-DEPTH1`: a pointer *to* a function pointer is
                    // an object pointer, and projecting it would put a function
                    // pointer's storage into a custody-bearing spelling. No
                    // declaration in scope needs it, so it stays refused.
                    Some(TypedefTarget::FunctionPointer) => return None,
                    // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES: one pointer to
                    // a function type is a function pointer
                    // (`rd_kafka_q_serve_cb_t *callback`); deeper stays refused.
                    Some(TypedefTarget::Function) if stars == 1 => {
                        return Some(BTreeSet::from([FUNCTION_POINTER_ABI_TYPE]));
                    }
                    Some(TypedefTarget::Function) => return None,
                    Some(TypedefTarget::Unresolved) | None => false,
                }
            }
            rest => rest.iter().all(|word| ARITHMETIC_WORDS.contains(word)),
        };
        if !pointee_readable {
            return None;
        }
        let projection = pointer_projection(stars, const_pointee);
        return (!projection.is_empty()).then_some(projection);
    }
    project_scalar(&core, typedefs, result, 0)
}

/// Project a declarator whose specifiers came from a resolved typedef.
fn project_declarator_resolved(
    target: &[&str],
    stars: usize,
    const_pointee: bool,
    typedefs: &BTreeMap<String, TypedefTarget>,
    result: bool,
) -> Option<BTreeSet<&'static str>> {
    if stars == 0 {
        let core = target
            .iter()
            .copied()
            .filter(|word| !QUALIFIERS.contains(word) && !DISCARDED_SPECIFIERS.contains(word))
            .collect::<Vec<_>>();
        return project_scalar(&core, typedefs, result, 0);
    }
    let const_pointee = const_pointee || target.contains(&"const");
    let core = target
        .iter()
        .copied()
        .filter(|word| !QUALIFIERS.contains(word) && !DISCARDED_SPECIFIERS.contains(word))
        .collect::<Vec<_>>();
    let readable = match core.as_slice() {
        [] => false,
        ["struct", tag] | ["union", tag] | ["enum", tag] => is_c_identifier(tag),
        ["struct"] | ["union"] | ["enum"] => false,
        // A pointee that is itself an aggregate definition or a function
        // pointer is readable: the projection of *this* pointer reads neither
        // the tag nor the pointee's own shape.
        [name] if !ARITHMETIC_WORDS.contains(name) => !matches!(
            resolve_typedef(name, typedefs, 0),
            None | Some(TypedefTarget::Unresolved)
        ),
        rest => rest.iter().all(|word| ARITHMETIC_WORDS.contains(word)),
    };
    if !readable {
        return None;
    }
    let projection = pointer_projection(stars, const_pointee);
    (!projection.is_empty()).then_some(projection)
}

/// Follow one typedef name to the target this reader commits to, folding the
/// pointer depth of every link in the chain.
///
/// `None` is "this reader does not commit", which is what every unreadable
/// shape and every over-deep chain collapses to.
fn resolve_typedef(
    name: &str,
    typedefs: &BTreeMap<String, TypedefTarget>,
    depth: usize,
) -> Option<TypedefTarget> {
    if depth >= TYPEDEF_DEPTH_LIMIT {
        return None;
    }
    let (specifiers, stars) = match typedefs.get(name)? {
        TypedefTarget::Unresolved => return None,
        TypedefTarget::FunctionPointer => return Some(TypedefTarget::FunctionPointer),
        TypedefTarget::Function => return Some(TypedefTarget::Function),
        TypedefTarget::Aggregate { stars } => {
            return Some(TypedefTarget::Aggregate { stars: *stars });
        }
        TypedefTarget::UntaggedEnum => return Some(TypedefTarget::UntaggedEnum),
        TypedefTarget::Declarator { specifiers, stars } => (specifiers, *stars),
    };
    let core = specifiers
        .iter()
        .map(String::as_str)
        .filter(|word| !QUALIFIERS.contains(word) && !DISCARDED_SPECIFIERS.contains(word))
        .collect::<Vec<_>>();
    if let [inner] = core.as_slice()
        && !ARITHMETIC_WORDS.contains(inner)
        && !matches!(*inner, "struct" | "union" | "enum")
    {
        match resolve_typedef(inner, typedefs, depth + 1) {
            Some(TypedefTarget::Declarator {
                specifiers: deeper,
                stars: deeper_stars,
            }) => {
                return Some(TypedefTarget::Declarator {
                    specifiers: deeper,
                    stars: deeper_stars + stars,
                });
            }
            Some(TypedefTarget::Aggregate {
                stars: deeper_stars,
            }) => {
                return Some(TypedefTarget::Aggregate {
                    stars: deeper_stars + stars,
                });
            }
            // Card W43-ABI-ENUM-AND-CONST-PP: a name for an untagged
            // enumeration stays one; a pointer to it is an object pointer.
            Some(TypedefTarget::UntaggedEnum) if stars == 0 => {
                return Some(TypedefTarget::UntaggedEnum);
            }
            Some(TypedefTarget::UntaggedEnum) => {
                return Some(TypedefTarget::Aggregate { stars });
            }
            // A name for a function pointer keeps its one pointer level; a name
            // for a *pointer to* one is `ABI-FNPTR-DEPTH1` again.
            Some(TypedefTarget::FunctionPointer) if stars == 0 => {
                return Some(TypedefTarget::FunctionPointer);
            }
            Some(TypedefTarget::FunctionPointer) => return None,
            // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES: a name for a function
            // type stays one; one pointer to it is a function pointer.
            Some(TypedefTarget::Function) if stars == 0 => {
                return Some(TypedefTarget::Function);
            }
            Some(TypedefTarget::Function) if stars == 1 => {
                return Some(TypedefTarget::FunctionPointer);
            }
            Some(TypedefTarget::Function) => return None,
            Some(TypedefTarget::Unresolved) | None => {}
        }
    }
    Some(TypedefTarget::Declarator {
        specifiers: specifiers.clone(),
        stars,
    })
}

/// Project a non-pointer type into the admissible scalar spellings.
///
/// The set is the closed reading of one *opaque C ABI projection*, on the
/// LP64 data model this analyzer's audited targets use: `long` and a pointer
/// are eight bytes, so `u64` and `usize` are two spellings of the same
/// eight-byte unsigned reading and both are admissible for `unsigned long`.
/// That is why the projection is a set rather than a single spelling; the
/// alternative would report a faithful `size_t` declaration as a disagreement.
/// Project a multiset of arithmetic type specifiers.
///
/// C11 6.7.2p2 lists the admissible specifier sets *as multisets*: `long
/// unsigned int` and `unsigned long` are the same type, and a preprocessed
/// header uses whichever spelling its author chose. Counting rather than
/// sequence-matching is therefore the faithful reading, and it is what stops a
/// perfectly faithful `size_t` declaration — `typedef long unsigned int size_t`
/// after preprocessing — being reported as unreadable.
///
/// `long double` is deliberately absent: this vocabulary has no spelling for
/// it, so it stays unreadable rather than being folded onto `f64`.
fn project_arithmetic(core: &[&str], result: bool) -> Option<BTreeSet<&'static str>> {
    if core.contains(&"__int64") {
        // A sized keyword cannot be mixed with `int`, `long`, another sized
        // keyword or conflicting/repeated signs, even if a width coincides.
        if core.iter().filter(|word| **word == "__int64").count() != 1
            || core
                .iter()
                .any(|word| !matches!(*word, "__int64" | "signed" | "unsigned"))
            || core.len() > 2
        {
            return None;
        }
        let mut equivalent = vec!["long", "long"];
        equivalent.extend(core.iter().copied().filter(|word| *word != "__int64"));
        return project_arithmetic(&equivalent, result);
    }
    let mut signed = 0usize;
    let mut unsigned = 0usize;
    let mut char_words = 0usize;
    let mut short_words = 0usize;
    let mut long_words = 0usize;
    let mut int_words = 0usize;
    let mut float_words = 0usize;
    let mut double_words = 0usize;
    let mut bool_words = 0usize;
    let mut void_words = 0usize;
    for word in core {
        match *word {
            "signed" => signed += 1,
            "unsigned" => unsigned += 1,
            "char" => char_words += 1,
            "short" => short_words += 1,
            "long" => long_words += 1,
            "int" => int_words += 1,
            "float" => float_words += 1,
            "double" => double_words += 1,
            "_Bool" | "bool" => bool_words += 1,
            "void" => void_words += 1,
            _ => return None,
        }
    }
    let set = |entries: &[&'static str]| Some(entries.iter().copied().collect::<BTreeSet<_>>());
    let integer = |signed_spelling: &[&'static str], unsigned_spelling: &[&'static str]| {
        if unsigned == 1 && signed == 0 {
            set(unsigned_spelling)
        } else if unsigned == 0 && signed <= 1 {
            set(signed_spelling)
        } else {
            None
        }
    };
    match (
        void_words,
        bool_words,
        float_words,
        double_words,
        char_words,
        short_words,
        long_words,
        int_words,
    ) {
        (1, 0, 0, 0, 0, 0, 0, 0) if result && signed == 0 && unsigned == 0 => set(&["unit"]),
        (0, 1, 0, 0, 0, 0, 0, 0) if signed == 0 && unsigned == 0 => set(&["bool"]),
        (0, 0, 1, 0, 0, 0, 0, 0) if signed == 0 && unsigned == 0 => set(&["f32"]),
        (0, 0, 0, 1, 0, 0, 0, 0) if signed == 0 && unsigned == 0 => set(&["f64"]),
        // Plain `char` has an implementation-defined signedness (C11
        // 6.2.5p15), so both byte-width spellings are admissible readings of
        // it; `signed char` and `unsigned char` fix it and admit only one.
        (0, 0, 0, 0, 1, 0, 0, 0) => {
            if signed == 1 && unsigned == 0 {
                set(&["i8"])
            } else if unsigned == 1 && signed == 0 {
                set(&["u8"])
            } else if signed == 0 && unsigned == 0 {
                set(&["i8", "u8"])
            } else {
                None
            }
        }
        (0, 0, 0, 0, 0, 1, 0, 0 | 1) => integer(&["i16"], &["u16"]),
        // Card CG20-NUMERIC-DOMAIN (rule 11q): plain `long` is read as the
        // 64-bit, pointer-width projection only when the analysing Clang's
        // target is LP64; under LLP64 (`x86_64-pc-windows-msvc`), ILP32 or a
        // failed query it is unreadable, never the LP64 reading.
        (0, 0, 0, 0, 0, 0, 1, 0 | 1) if !crate::c_numeric::plain_long_is_lp64() => None,
        (0, 0, 0, 0, 0, 0, 1 | 2, 0 | 1) => integer(&["i64", "isize"], &["u64", "usize"]),
        (0, 0, 0, 0, 0, 0, 0, 1) => integer(&["i32"], &["u32"]),
        (0, 0, 0, 0, 0, 0, 0, 0) if signed + unsigned == 1 => integer(&["i32"], &["u32"]),
        _ => None,
    }
}

/// The compiler's variable-argument list type (card **W44-LIBC-RESIDUALS**,
/// `LIBC-PRINTF-VA-LIST`).
///
/// glibc's `vsnprintf`, `vprintf`, `vfprintf` and `vsprintf` end in
/// `__gnuc_va_list __arg`, and `typedef __builtin_va_list __gnuc_va_list;` is
/// the only definition the unit carries: the builtin itself is the compiler's,
/// never a `typedef` in the token stream, so [`resolve_typedef`] stopped at it
/// and every such declaration was withdrawn with `abi-unreadable-declaration`
/// in all 168 units that declare one.
///
/// It is now read as [`VA_LIST_ABI_TYPE`], exactly and only:
///
/// * as a **parameter** of no pointer level, reached directly or through any
///   typedef chain the reader already follows. The spelling names the type,
///   not its target-dependent representation (`__va_list_tag[1]`, an object
///   pointer after adjustment, on x86-64; a structure on AArch64), so no
///   target is assumed beyond the LP64 caveat the reader already carries;
/// * never as a result, and never under a pointer (`va_list *`) or as an array
///   element, all of which still refuse;
/// * never where the unit itself `typedef`s the name: a recorded typedef is
///   followed as every other one is, so the builtin reading cannot shadow a
///   definition the token stream contains.
///
/// `QCore.GnuBuiltinReader` is the Lean model.
const BUILTIN_VA_LIST: &str = "__builtin_va_list";

fn project_scalar(
    core: &[&str],
    typedefs: &BTreeMap<String, TypedefTarget>,
    result: bool,
    depth: usize,
) -> Option<BTreeSet<&'static str>> {
    if depth >= TYPEDEF_DEPTH_LIMIT {
        return None;
    }
    if let Some(projection) = project_arithmetic(core, result) {
        return Some(projection);
    }
    match core {
        // Card W44-LIBC-RESIDUALS: the compiler's variable-argument list type
        // has its own spelling, at a parameter only. See [`BUILTIN_VA_LIST`].
        [name] if *name == BUILTIN_VA_LIST && !typedefs.contains_key(*name) => {
            (!result).then(|| BTreeSet::from([VA_LIST_ABI_TYPE]))
        }
        [name]
            if !ARITHMETIC_WORDS.contains(name)
                && !matches!(*name, "struct" | "union" | "enum") =>
        {
            match resolve_typedef(name, typedefs, 0)? {
                TypedefTarget::Declarator {
                    specifiers,
                    stars: 0,
                } => {
                    let target = specifiers
                        .iter()
                        .map(String::as_str)
                        .filter(|word| {
                            !QUALIFIERS.contains(word) && !DISCARDED_SPECIFIERS.contains(word)
                        })
                        .collect::<Vec<_>>();
                    project_scalar(&target, typedefs, result, depth + 1)
                }
                TypedefTarget::Declarator { specifiers, stars } => {
                    let target = specifiers.iter().map(String::as_str).collect::<Vec<_>>();
                    refuse_qualified_chain(
                        project_declarator_resolved(&target, stars, false, typedefs, result),
                        name,
                        typedefs,
                    )
                }
                // An aggregate *value* has no spelling in this vocabulary, for
                // the same reason a `struct tag` value has none. Refused, not
                // guessed at some integer.
                TypedefTarget::Aggregate { stars: 0 } => None,
                // Card W43-ABI-ENUM-AND-CONST-PP (rule 12u): an untagged
                // enumeration's value is the integer projection Clang states
                // for it, asked by the outermost name (a typedef is
                // transparent, so every name of the chain denotes it).
                TypedefTarget::UntaggedEnum => {
                    super::enum_projection::project(name).map(|spelling| BTreeSet::from([spelling]))
                }
                TypedefTarget::Aggregate { stars } => {
                    let projection = pointer_projection(stars, false);
                    (!projection.is_empty()).then_some(projection)
                }
                // The payoff of this card: a pointer to function has its own
                // projection, which no custody mode can be written on.
                TypedefTarget::FunctionPointer => Some(BTreeSet::from([FUNCTION_POINTER_ABI_TYPE])),
                // A function type's value is no parameter spelling.
                TypedefTarget::Function | TypedefTarget::Unresolved => None,
            }
        }
        // An enumeration's compatible integer type is implementation-defined
        // (C11 6.7.2.2p4), so no spelling of this vocabulary is established by
        // the declaration alone. Card W43-ABI-ENUM-AND-CONST-PP (rule 12u):
        // Clang states it for the analysed unit; without that answer it is
        // refused, never guessed at `int`.
        ["enum", tag] if is_c_identifier(tag) => {
            super::enum_projection::project(&format!("enum {tag}"))
                .map(|spelling| BTreeSet::from([spelling]))
        }
        _ => None,
    }
}

/// The index of the token closing the group opened at `open`.
fn matching_close(texts: &[&str], open: usize) -> Option<usize> {
    let (opener, closer) = match *texts.get(open)? {
        "(" => ("(", ")"),
        "[" => ("[", "]"),
        "{" => ("{", "}"),
        _ => return None,
    };
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        if *text == opener {
            depth += 1;
        } else if *text == closer {
            if depth == 0 {
                return None;
            }
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

/// The index of the token opening the group closed at `close`.
fn matching_open(texts: &[&str], close: usize) -> Option<usize> {
    let (opener, closer) = match *texts.get(close)? {
        ")" => ("(", ")"),
        "]" => ("[", "]"),
        "}" => ("{", "}"),
        _ => return None,
    };
    let mut depth = 0usize;
    for index in (0..=close).rev() {
        let text = texts[index];
        if text == closer {
            depth += 1;
        } else if text == opener {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

/// Card **CG30-DIALECTS-FLAGS-TYPES** (gateway rule 11y): the canonical type
/// identity ([`crate::c_frontend_identity::CTypeId`]) of a declaration's
/// specifier words under `stars` pointer levels, resolved through the unit's
/// typedefs exactly as this reader resolves them. Additive: the agreement
/// above does not consult it. `None` for a `volatile` type, an unresolved
/// typedef, an aggregate or function-pointer typedef and every specifier set
/// outside the identity's arithmetic and tagged vocabulary.
pub fn declared_type_id(
    texts: &[&str],
    specifiers: &[&str],
    stars: usize,
) -> Option<crate::c_frontend_identity::CTypeId> {
    resolved_type_id(specifiers, stars, &read_typedefs(texts), 0)
}

fn resolved_type_id(
    specifiers: &[&str],
    stars: usize,
    typedefs: &BTreeMap<String, TypedefTarget>,
    depth: usize,
) -> Option<crate::c_frontend_identity::CTypeId> {
    if specifiers.contains(&"volatile") || depth >= TYPEDEF_DEPTH_LIMIT {
        return None;
    }
    let core = specifiers
        .iter()
        .copied()
        .filter(|word| {
            *word == "const" || (!QUALIFIERS.contains(word) && !DISCARDED_SPECIFIERS.contains(word))
        })
        .collect::<Vec<_>>();
    let unqualified = core
        .iter()
        .copied()
        .filter(|word| *word != "const")
        .collect::<Vec<_>>();
    if let [name] = unqualified.as_slice()
        && !ARITHMETIC_WORDS.contains(name)
    {
        let TypedefTarget::Declarator {
            specifiers: resolved,
            stars: levels,
        } = resolve_typedef(name, typedefs, 0)?
        else {
            return None;
        };
        let mut words = resolved.iter().map(String::as_str).collect::<Vec<_>>();
        if core.contains(&"const") && stars == 0 && levels == 0 {
            words.push("const");
        }
        return resolved_type_id(&words, levels + stars, typedefs, depth + 1);
    }
    crate::c_frontend_identity::CTypeId::from_specifiers(&core, stars)
}
