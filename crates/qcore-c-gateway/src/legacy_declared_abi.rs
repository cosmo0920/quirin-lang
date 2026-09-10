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

use std::collections::{BTreeMap, BTreeSet};

use qcore_contract::contract_loader::{self, DeclaredAbiSignature};
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
        let names = contracts
            .functions()
            .keys()
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
        let read = read_c_declarations(&texts, &names);
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
        reduced.set_source_assumptions(contracts.source_assumptions().clone());
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

/// One resolved typedef: the specifier words of its target and its pointer depth.
type TypedefTarget = Option<(Vec<String>, usize)>;

/// Read every `typedef` this reader can commit to.
///
/// A typedef is *transparent* in C — the declared type is the target type — so
/// resolving one is exact rather than an approximation, which is why the
/// comparison admits through it. A typedef whose declarator this reader does
/// not read (a function pointer, an array, several declarators at once) and a
/// name typedef'd twice to different targets both record `None`, which makes
/// every declaration using them unreadable.
fn read_typedefs(texts: &[&str]) -> BTreeMap<String, TypedefTarget> {
    let mut typedefs: BTreeMap<String, TypedefTarget> = BTreeMap::new();
    for index in 0..texts.len() {
        if texts[index] != "typedef" {
            continue;
        }
        let Some(end) = texts
            .iter()
            .enumerate()
            .skip(index + 1)
            .find(|(_, text)| **text == ";" || **text == "{" || **text == "}")
            .filter(|(_, text)| **text == ";")
            .map(|(offset, _)| offset)
        else {
            continue;
        };
        let slice = &texts[index + 1..end];
        let Some(last) = slice.last().copied().filter(|last| is_c_identifier(last)) else {
            continue;
        };
        let body = &slice[..slice.len() - 1];
        let resolved = if body
            .iter()
            .any(|text| matches!(*text, "(" | ")" | "[" | "]" | "," | "{" | "}"))
        {
            None
        } else {
            let stars = body.iter().rev().take_while(|text| **text == "*").count();
            let specifiers = &body[..body.len() - stars];
            if specifiers.contains(&"*") {
                None
            } else {
                Some((
                    specifiers.iter().map(|text| (*text).to_owned()).collect(),
                    stars,
                ))
            }
        };
        match typedefs.get(last) {
            Some(existing) if *existing == resolved => {}
            Some(_) => {
                typedefs.insert(last.to_owned(), None);
            }
            None => {
                typedefs.insert(last.to_owned(), resolved);
            }
        }
    }
    typedefs
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
    let result = project_declarator(prefix, typedefs, true);
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
        projected.push(project_parameter(group, typedefs));
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

/// Project one parameter declaration group.
fn project_parameter(
    group: &[&str],
    typedefs: &BTreeMap<String, TypedefTarget>,
) -> Option<BTreeSet<&'static str>> {
    // A nested declarator (a function pointer, a parenthesised declarator) is
    // not read.
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
        // Only an empty or plainly constant bound: `[static 4]`, `[const 4]`
        // and `[*]` carry qualifiers and a variable length this reader does
        // not read.
        if bounds.len() > 1
            || bounds
                .iter()
                .any(|text| !text.bytes().all(|byte| byte.is_ascii_digit()))
        {
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
        rest.push("*");
    }
    project_declarator(&rest, typedefs, false)
}

const ARITHMETIC_WORDS: &[&str] = &[
    "void", "char", "short", "int", "long", "float", "double", "signed", "unsigned", "_Bool",
    "bool", "_Complex",
];

/// The pointer projection of `stars` levels over a pointee that is or is not
/// `const`-qualified.
fn pointer_projection(stars: usize, const_pointee: bool) -> BTreeSet<&'static str> {
    match (stars, const_pointee) {
        (1, false) => BTreeSet::from(["ptr<mut, u8>"]),
        (1, true) => BTreeSet::from(["ptr<const, u8>"]),
        (2, false) => BTreeSet::from(["ptr<mut, ptr<mut, u8>>"]),
        _ => BTreeSet::new(),
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
                match resolve_typedef(name, typedefs, 0) {
                    Some((target, target_stars)) => {
                        let target = target.iter().map(String::as_str).collect::<Vec<_>>();
                        return project_declarator_resolved(
                            &target,
                            target_stars + stars,
                            const_pointee,
                            typedefs,
                            result,
                        );
                    }
                    None => false,
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
        [name] if !ARITHMETIC_WORDS.contains(name) => resolve_typedef(name, typedefs, 0).is_some(),
        rest => rest.iter().all(|word| ARITHMETIC_WORDS.contains(word)),
    };
    if !readable {
        return None;
    }
    let projection = pointer_projection(stars, const_pointee);
    (!projection.is_empty()).then_some(projection)
}

fn resolve_typedef(
    name: &str,
    typedefs: &BTreeMap<String, TypedefTarget>,
    depth: usize,
) -> Option<(Vec<String>, usize)> {
    if depth >= TYPEDEF_DEPTH_LIMIT {
        return None;
    }
    let (target, stars) = typedefs.get(name)?.clone()?;
    let core = target
        .iter()
        .map(String::as_str)
        .filter(|word| !QUALIFIERS.contains(word) && !DISCARDED_SPECIFIERS.contains(word))
        .collect::<Vec<_>>();
    if let [inner] = core.as_slice()
        && !ARITHMETIC_WORDS.contains(inner)
        && !matches!(*inner, "struct" | "union" | "enum")
        && let Some((deeper, deeper_stars)) = resolve_typedef(inner, typedefs, depth + 1)
    {
        return Some((deeper, deeper_stars + stars));
    }
    Some((target, stars))
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
        (0, 0, 0, 0, 0, 0, 1 | 2, 0 | 1) => integer(&["i64", "isize"], &["u64", "usize"]),
        (0, 0, 0, 0, 0, 0, 0, 1) => integer(&["i32"], &["u32"]),
        (0, 0, 0, 0, 0, 0, 0, 0) if signed + unsigned == 1 => integer(&["i32"], &["u32"]),
        _ => None,
    }
}

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
        [name]
            if !ARITHMETIC_WORDS.contains(name)
                && !matches!(*name, "struct" | "union" | "enum") =>
        {
            let (target, stars) = resolve_typedef(name, typedefs, 0)?;
            if stars > 0 {
                let target = target.iter().map(String::as_str).collect::<Vec<_>>();
                return project_declarator_resolved(&target, stars, false, typedefs, result);
            }
            let target = target
                .iter()
                .map(String::as_str)
                .filter(|word| !QUALIFIERS.contains(word) && !DISCARDED_SPECIFIERS.contains(word))
                .collect::<Vec<_>>();
            project_scalar(&target, typedefs, result, depth + 1)
        }
        // An enumeration's compatible integer type is implementation-defined
        // (C11 6.7.2.2p4), so no spelling of this vocabulary is established by
        // the declaration. Refused rather than guessed at `int`.
        _ => None,
    }
}

/// The index of the token closing the group opened at `open`.
fn matching_close(texts: &[&str], open: usize) -> Option<usize> {
    let (opener, closer) = match *texts.get(open)? {
        "(" => ("(", ")"),
        "[" => ("[", "]"),
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
