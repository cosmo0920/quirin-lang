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

//! LOADER-ABI — the declared contract's C ABI projections are compared against
//! the analysed translation unit's own C declaration
//! (`docs/reports/remaining-loader-abi/README.md`).
//!
//! Card NEIGHBOURS recorded the gap this closes: the loader checks a
//! declaration's parameter and result spellings against its own closed table
//! and against nothing else, so an unfaithful spelling passed silently. The
//! spellings are not decoration — the loader's admissibility rules are keyed on
//! them, so an unfaithful type is exactly what buys a custody mode the real C
//! type could not carry.
//!
//! **The family is held out.** A `k_*` API unrelated to Fluent Bit, ChunkIO,
//! Monkey and every earlier card's fixtures.
//!
//! Nothing here promotes anything: every `Safe` below is `Safe` for a synthetic
//! declared custody projection under the premises the fixture names.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AbiOutcome, AbiRefusal, AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind,
    LegacyFunctionSummary, LegacyProgramSummary, ResourceContracts, ResourceFunctionContract,
    ResourceParameterMode, ResourceReturnMode, UnrestrictedLegacyResult,
    analyze_legacy_c_translation_unit, analyze_unrestricted_c11, declared_abi_census,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[cfg(all(windows, target_pointer_width = "64"))]
const HOST_SIZE_C: &str = "unsigned long long";
#[cfg(all(not(windows), target_pointer_width = "64"))]
const HOST_SIZE_C: &str = "unsigned long";
#[cfg(target_pointer_width = "32")]
const HOST_SIZE_C: &str = "unsigned int";

#[cfg(all(windows, target_pointer_width = "64"))]
const HOST_SIZE_TYPEDEF_C: &str =
    "typedef unsigned long long k_size;\nvoid k_use(void *p, k_size n);";
#[cfg(all(not(windows), target_pointer_width = "64"))]
const HOST_SIZE_TYPEDEF_C: &str = "typedef unsigned long k_size;\nvoid k_use(void *p, k_size n);";
#[cfg(target_pointer_width = "32")]
const HOST_SIZE_TYPEDEF_C: &str = "typedef unsigned int k_size;\nvoid k_use(void *p, k_size n);";

#[cfg(all(windows, target_pointer_width = "64"))]
const HOST_SIZE_TYPEDEF_CHAIN_C: &str = "typedef unsigned long long k_word;\n\
typedef k_word k_size;\nvoid k_use(void *p, k_size n);";
#[cfg(all(not(windows), target_pointer_width = "64"))]
const HOST_SIZE_TYPEDEF_CHAIN_C: &str = "typedef unsigned long k_word;\n\
typedef k_word k_size;\nvoid k_use(void *p, k_size n);";
#[cfg(target_pointer_width = "32")]
const HOST_SIZE_TYPEDEF_CHAIN_C: &str = "typedef unsigned int k_word;\n\
typedef k_word k_size;\nvoid k_use(void *p, k_size n);";

#[cfg(windows)]
const HOST_ENUM_TYPE: &str = "p: ptr<mut, u8>, n: i32";
#[cfg(not(windows))]
const HOST_ENUM_TYPE: &str = "p: ptr<mut, u8>, n: u32";
#[cfg(windows)]
const OTHER_ENUM_TYPE: &str = "p: ptr<mut, u8>, n: u32";
#[cfg(not(windows))]
const OTHER_ENUM_TYPE: &str = "p: ptr<mut, u8>, n: i32";
use qcore_schemas::reason::{ReasonCategory, ReasonSubject};

fn load(source: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "k_api.qcore".to_owned(),
        format!("module contract;\n\n{source}"),
    )]))
    .expect("the fixture module loads")
}

fn function<'a>(summary: &'a LegacyProgramSummary, name: &str) -> &'a LegacyFunctionSummary {
    summary
        .functions
        .get(name)
        .expect("the function is analysed")
}

fn unsupported(summary: &LegacyFunctionSummary) -> Vec<String> {
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
            _ => None,
        })
        .collect()
}

/// Analyse one whole translation unit through the pure-token entry point.
///
/// The ABI comparison reads the same token stream the importer projects, so it
/// needs no Clang; the two Clang-gated tests below check the same decision on
/// the production entry point and at certificate issuance.
fn analyse(source: &str, contracts: &ResourceContracts) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit(source, "w", contracts).expect("the fixture tokenizes")
}

fn clang_available() -> bool {
    qcore_c_gateway::clang_command().is_ok()
}

/// The refusal a withdrawn callee's call carries, by key.
fn withdrawal(callee: &str, refusal: AbiRefusal) -> String {
    format!(
        "call to `{callee}` whose declared contract was withdrawn against its C declaration in \
         this translation unit ({})",
        refusal.as_str()
    )
}

/// The contract every reproduction below shares: an allocator declared to
/// return **nothing**, so its acquisition is invisible to the projection.
const UNIT_RESULT: &str = "\
extern fn k_alloc(size: usize) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(size);
    ensures normal resource_return_none();
};
";

/// The body: one call whose result is discarded. Under `UNIT_RESULT` there is
/// nothing to account for; under the real C return type there is.
const DISCARDING_BODY: &str = "void w(void) { k_alloc(8); }";

// ---------------------------------------------------------------------------
// 1. The fail-open, reproduced — and the boundary that still carries it
// ---------------------------------------------------------------------------

/// **The reproduction.** With no C declaration of the callee in the unit there
/// is nothing to compare, the unfaithful `-> unit` declaration applies, and the
/// allocation it hides never enters the projection: the unit is `Safe`.
///
/// This is deliberately still true after this card: it is the recorded boundary
/// `LOADER-ABI-UNDECLARED`, and pinning it here is what keeps the boundary
/// honest instead of implied.
#[test]
fn without_a_c_declaration_the_unfaithful_result_type_still_applies() {
    let summary = analyse(DISCARDING_BODY, &load(UNIT_RESULT));
    assert_eq!(function(&summary, "w").status, AnalysisStatus::Safe);
    assert!(unsupported(function(&summary, "w")).is_empty());
    assert_eq!(
        declared_abi_census(DISCARDING_BODY, &load(UNIT_RESULT))
            .into_iter()
            .map(|row| (row.function, row.outcome))
            .collect::<Vec<_>>(),
        vec![("k_alloc".to_owned(), AbiOutcome::Undeclared)]
    );
}

/// **The repair.** Declare the callee as C really declares it — returning a
/// pointer — and the same unfaithful contract is withdrawn: the call refuses
/// with the named key and the unit is `Unknown`.
#[test]
fn a_result_type_the_c_declaration_contradicts_withdraws_the_contract() {
    let source = format!("void *k_alloc({HOST_SIZE_C} size);\n{DISCARDING_BODY}");
    let contracts = load(UNIT_RESULT);
    let summary = analyse(&source, &contracts);
    assert_eq!(function(&summary, "w").status, AnalysisStatus::Unknown);
    assert_eq!(
        unsupported(function(&summary, "w")),
        vec![withdrawal("k_alloc", AbiRefusal::ReturnTypeDisagreement)]
    );
    assert_eq!(
        declared_abi_census(&source, &contracts)
            .into_iter()
            .map(|row| (row.function, row.outcome))
            .collect::<Vec<_>>(),
        vec![(
            "k_alloc".to_owned(),
            AbiOutcome::Withdrawn(AbiRefusal::ReturnTypeDisagreement)
        )]
    );
}

/// The faithful declaration of the same callee is admitted, and then the
/// projection sees the acquisition it was hiding: the discarded owned result is
/// reported. Without this row the test above could be passing for the wrong
/// reason — a comparison that refuses everything.
#[test]
fn the_faithful_result_type_is_admitted_and_the_acquisition_becomes_visible() {
    let contracts = load(
        "\
extern fn k_alloc(size: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(size);
    ensures normal resource_return_owned(nullable);
};
",
    );
    let source = format!("void *k_alloc({HOST_SIZE_C} size);\n{DISCARDING_BODY}");
    let summary = analyse(&source, &contracts);
    let w = function(&summary, "w");
    assert_eq!(w.status, AnalysisStatus::Unknown);
    assert!(
        unsupported(w)
            .iter()
            .all(|text| !text.contains("withdrawn against its C declaration")),
        "the faithful declaration must be admitted: {:?}",
        unsupported(w)
    );
    assert_eq!(
        declared_abi_census(&source, &contracts)
            .into_iter()
            .map(|row| row.outcome)
            .collect::<Vec<_>>(),
        vec![AbiOutcome::Agreed]
    );
}

/// The second direction the loader's type-keyed rules make reachable: a
/// declaration that names its parameters in the *other* order applies a
/// `Released` mode to the argument at the wrong position. The C declaration
/// contradicts it at both positions, so it is withdrawn.
#[test]
fn a_swapped_parameter_vector_is_withdrawn() {
    let contracts = load(
        "\
extern fn k_open() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
};

extern fn k_close(handle: ptr<mut, u8>, flags: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(handle, nonnull);
    ensures normal resource_borrowed(flags);
    ensures normal resource_return_none();
};
",
    );
    let source = "void k_close(int flags, void *handle);\n\
                  void *k_open(void);\n\
                  void w(void) { void *h; h = k_open(); k_close(0, h); }";
    let summary = analyse(source, &contracts);
    assert_eq!(
        unsupported(function(&summary, "w")),
        vec![withdrawal("k_close", AbiRefusal::ParameterTypeDisagreement)]
    );
}

// ---------------------------------------------------------------------------
// 2. The admission and refusal shapes, one row each
// ---------------------------------------------------------------------------

/// One contracted callee, one C declaration, one expected outcome.
struct Row {
    label: &'static str,
    declared: &'static str,
    c: &'static str,
    outcome: Option<AbiRefusal>,
}

const ROWS: &[Row] = &[
    // -- exact agreement, the base case ------------------------------------
    Row {
        label: "the exact shape agrees",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(void *p, int n);",
        outcome: None,
    },
    // -- integer widths -----------------------------------------------------
    #[cfg(all(windows, target_pointer_width = "64"))]
    Row {
        label: "the target's size type admits `usize`",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "void k_use(void *p, unsigned long long n);",
        outcome: None,
    },
    #[cfg(all(not(windows), target_pointer_width = "64"))]
    Row {
        label: "the target's size type admits `usize`",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "void k_use(void *p, unsigned long n);",
        outcome: None,
    },
    #[cfg(target_pointer_width = "32")]
    Row {
        label: "the target's size type admits `usize`",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "void k_use(void *p, unsigned int n);",
        outcome: None,
    },
    Row {
        label: "the target's 64-bit unsigned type admits `u64`",
        declared: "p: ptr<mut, u8>, n: u64",
        c: "void k_use(void *p, unsigned long long n);",
        outcome: None,
    },
    #[cfg(all(windows, target_pointer_width = "64"))]
    Row {
        label: "specifier order is a multiset, so `long unsigned int` is the same type",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "void k_use(void *p, long long unsigned int n);",
        outcome: None,
    },
    #[cfg(all(not(windows), target_pointer_width = "64"))]
    Row {
        label: "specifier order is a multiset, so `long unsigned int` is the same type",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "void k_use(void *p, long unsigned int n);",
        outcome: None,
    },
    #[cfg(target_pointer_width = "32")]
    Row {
        label: "specifier order is a multiset, so `unsigned int` is the same type",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "void k_use(void *p, unsigned int n);",
        outcome: None,
    },
    #[cfg(all(windows, target_pointer_width = "64"))]
    Row {
        label: "signedness is not admitted across",
        declared: "p: ptr<mut, u8>, n: i64",
        c: "void k_use(void *p, unsigned long n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    #[cfg(not(all(windows, target_pointer_width = "64")))]
    Row {
        label: "signedness is not admitted across",
        declared: "p: ptr<mut, u8>, n: i64",
        c: "void k_use(void *p, unsigned long n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "width is not admitted across",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "void k_use(void *p, int n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "plain `char` has an implementation-defined signedness: both byte spellings",
        declared: "p: ptr<mut, u8>, n: u8",
        c: "void k_use(void *p, char n);",
        outcome: None,
    },
    Row {
        label: "`signed char` fixes it, and refuses the other spelling",
        declared: "p: ptr<mut, u8>, n: u8",
        c: "void k_use(void *p, signed char n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    // -- MSVC default C convention and exact-width spelling ----------------
    Row {
        label: "MSVC default C convention",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void __cdecl k_use(void *p, int n);",
        outcome: None,
    },
    Row {
        label: "MSVC explicit signed 64-bit",
        declared: "p: ptr<mut, u8>, n: i64",
        c: "void __cdecl k_use(void *p, __int64 n);",
        outcome: None,
    },
    Row {
        label: "MSVC unsigned size typedef chain",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "typedef unsigned __int64 KSize; typedef KSize KAlias; void __cdecl k_use(void *p, KAlias n);",
        outcome: None,
    },
    Row {
        label: "MSVC unsigned reordered",
        declared: "p: ptr<mut, u8>, n: u64",
        c: "void k_use(void *p, __int64 unsigned n);",
        outcome: None,
    },
    Row {
        label: "MSVC signed reordered",
        declared: "p: ptr<mut, u8>, n: isize",
        c: "void k_use(void *p, __int64 signed n);",
        outcome: None,
    },
    Row {
        label: "MSVC abstract sized parameter retains its type",
        declared: "p: ptr<mut, u8>, n: u64",
        c: "void k_use(void *, unsigned __int64);",
        outcome: None,
    },
    Row {
        label: "MSVC signedness still disagrees",
        declared: "p: ptr<mut, u8>, n: u64",
        c: "void k_use(void *p, signed __int64 n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "MSVC width still disagrees",
        declared: "p: ptr<mut, u8>, n: u32",
        c: "void k_use(void *p, unsigned __int64 n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "MSVC mixed arithmetic words refuse",
        declared: "p: ptr<mut, u8>, n: u64",
        c: "void k_use(void *p, unsigned __int64 int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "MSVC repeated signs refuse",
        declared: "p: ptr<mut, u8>, n: u64",
        c: "void k_use(void *p, unsigned unsigned __int64 n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "MSVC conflicting signs refuse",
        declared: "p: ptr<mut, u8>, n: u64",
        c: "void k_use(void *p, signed unsigned __int64 n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "MSVC repeated sized keyword refuses",
        declared: "p: ptr<mut, u8>, n: u64",
        c: "void k_use(void *p, __int64 __int64 n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "MSVC repeated convention refuses",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void __cdecl __cdecl k_use(void *p, int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "another convention stays refused",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void __stdcall k_use(void *p, int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "vector convention stays refused",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void __vectorcall k_use(void *p, int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "cdecl ordinary long cannot inherit LP64",
        declared: "p: ptr<mut, u8>, n: u64",
        c: "void __cdecl k_use(void *p, unsigned long n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "cdecl ordinary long typedef chain stays refused",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "typedef unsigned long KLong; typedef KLong KAlias; void __cdecl k_use(void *p, KAlias n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "cdecl explicit long long is still 64-bit",
        declared: "p: ptr<mut, u8>, n: u64",
        c: "void __cdecl k_use(void *p, unsigned long long n);",
        outcome: None,
    },
    // -- pointers and qualifiers -------------------------------------------
    Row {
        label: "a const pointee is `ptr<const, u8>`, never `ptr<mut, u8>`",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(const char *p, int n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "and the faithful const spelling agrees",
        declared: "p: ptr<const, u8>, n: i32",
        c: "void k_use(const char *p, int n);",
        outcome: None,
    },
    Row {
        label: "a mutable pointee refuses the const spelling",
        declared: "p: ptr<const, u8>, n: i32",
        c: "void k_use(char *p, int n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "an opaque `struct` pointer is the same opaque projection",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "struct k_opaque;\nvoid k_use(struct k_opaque *p, int n);",
        outcome: None,
    },
    Row {
        label: "`volatile` on the pointee does not change the pointer's projection",
        declared: "p: ptr<const, u8>, n: i32",
        c: "void k_use(const volatile void *p, int n);",
        outcome: None,
    },
    Row {
        label: "a pointer to pointer is the out-slot projection",
        declared: "p: ptr<mut, ptr<mut, u8>>, n: i32",
        c: "void k_use(struct k_item **p, int n);",
        outcome: None,
    },
    // Card W43-ABI-ENUM-AND-CONST-PP (rule 12v): `const char **` is read now,
    // as `ptr<mut, ptr<const, u8>>`, so a declared pointer to a mutable
    // pointer disagrees (it was unreadable before; withdrawn either way).
    Row {
        label: "a pointer to a pointer to const is not a pointer to a mutable pointer",
        declared: "p: ptr<mut, ptr<mut, u8>>, n: i32",
        c: "void k_use(const char **p, int n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "a qualifier between two pointer levels has no spelling here",
        declared: "p: ptr<mut, ptr<mut, u8>>, n: i32",
        c: "void k_use(char *const *p, int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "a three-level pointer has no spelling here",
        declared: "p: ptr<mut, ptr<mut, u8>>, n: i32",
        c: "void k_use(char ***p, int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    // -- C's own declarator adjustments ------------------------------------
    Row {
        label: "an array parameter is adjusted to a pointer (C11 6.7.6.3p7)",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(char p[], int n);",
        outcome: None,
    },
    Row {
        label: "a sized array parameter is the same adjustment",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(char p[8], int n);",
        outcome: None,
    },
    // -- typedefs ----------------------------------------------------------
    Row {
        label: "a typedef is transparent, so it is resolved rather than refused",
        declared: "p: ptr<mut, u8>, n: usize",
        c: HOST_SIZE_TYPEDEF_C,
        outcome: None,
    },
    Row {
        label: "a typedef chain resolves too",
        declared: "p: ptr<mut, u8>, n: usize",
        c: HOST_SIZE_TYPEDEF_CHAIN_C,
        outcome: None,
    },
    Row {
        label: "a typedef to a pointer resolves to the pointer projection",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "typedef char *k_string;\nvoid k_use(k_string p, int n);",
        outcome: None,
    },
    Row {
        label: "and resolving it does not lose the disagreement underneath",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "typedef int k_count;\nvoid k_use(void *p, k_count n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "a typedef declared twice to different targets is not resolved",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "typedef unsigned long k_size;\ntypedef int k_size;\nvoid k_use(void *p, k_size n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    // -- typedefs that DEFINE their target (card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF)
    Row {
        label: "an untagged `union` definition is an object type: a pointer to it is \
                the same opaque projection (`pthread_mutex_t *`)",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "typedef union { int a; long b; } k_mutex;\nvoid k_use(k_mutex *p, int n);",
        outcome: None,
    },
    Row {
        label: "an untagged `struct` definition behind `const` is the const projection",
        declared: "p: ptr<const, u8>, n: i32",
        c: "typedef struct { int a; } k_rec;\nvoid k_use(const k_rec *p, int n);",
        outcome: None,
    },
    Row {
        label: "a *tagged* definition is recorded as the specifier pair it equals \
                (`msgpack_packer`)",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "typedef struct k_packer { int a; } k_packer;\nvoid k_use(k_packer *p, int n);",
        outcome: None,
    },
    Row {
        label: "so it composes with a separate plain typedef of the same name \
                instead of colliding with it",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "typedef struct k_packer { int a; } k_packer;\n\
            typedef struct k_packer k_packer;\n\
            void k_use(k_packer *p, int n);",
        outcome: None,
    },
    Row {
        label: "an untagged definition dereferenced twice is the out-slot projection",
        declared: "p: ptr<mut, ptr<mut, u8>>, n: i32",
        c: "typedef struct { int a; } k_rec;\nvoid k_use(k_rec **p, int n);",
        outcome: None,
    },
    Row {
        label: "an aggregate *value* still has no spelling in this vocabulary",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "typedef struct { int a; } k_rec;\nvoid k_use(void *p, k_rec n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    // Card W43-ABI-ENUM-AND-CONST-PP (rule 12u): an enumeration is read now,
    // as the implementation-defined integer type Clang states for the target.
    Row {
        label: "an untagged `enum` value is its Clang integer type, never assumed `int`",
        declared: OTHER_ENUM_TYPE,
        c: "typedef enum { K_A, K_B } k_kind;\nvoid k_use(void *p, k_kind n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "and agrees spelled as that type",
        declared: HOST_ENUM_TYPE,
        c: "typedef enum { K_A, K_B } k_kind;\nvoid k_use(void *p, k_kind n);",
        outcome: None,
    },
    Row {
        label: "but a pointer to that enumeration is readable, as `enum tag *` already was",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "typedef enum { K_A, K_B } k_kind;\nvoid k_use(k_kind *p, int n);",
        outcome: None,
    },
    Row {
        label: "a typedef declaring several declarators at once is still not read",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "typedef struct { int a; } k_rec, *k_recp;\nvoid k_use(k_recp p, int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    // -- function-pointer typedefs (card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF) --
    Row {
        label: "a function-pointer typedef projects to `fnptr` (`msgpack_packer_write`)",
        declared: "p: ptr<mut, u8>, n: fnptr",
        c: "typedef int (*k_write)(void *, const char *, unsigned long);\n\
            void k_use(void *p, k_write n);",
        outcome: None,
    },
    Row {
        label: "`ABI-FNPTR-OPAQUE`: a different function signature agrees with the same \
                spelling, exactly as two object pointers share `ptr<mut, u8>`",
        declared: "p: ptr<mut, u8>, n: fnptr",
        c: "typedef void (*k_write)(int);\nvoid k_use(void *p, k_write n);",
        outcome: None,
    },
    Row {
        label: "a chain ending at a function pointer keeps its one pointer level",
        declared: "p: ptr<mut, u8>, n: fnptr",
        c: "typedef int (*k_write)(void *);\n\
            typedef k_write k_writer;\n\
            void k_use(void *p, k_writer n);",
        outcome: None,
    },
    Row {
        label: "a function pointer is not an integer: the LP64 size coincidence buys nothing",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "typedef int (*k_write)(void *);\nvoid k_use(void *p, k_write n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "and it is not an object pointer either",
        declared: "p: ptr<mut, u8>, n: ptr<mut, u8>",
        c: "typedef int (*k_write)(void *);\nvoid k_use(void *p, k_write n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "the refusal runs the other way too: `fnptr` is refused for an object pointer",
        declared: "p: ptr<mut, u8>, n: fnptr",
        c: "void k_use(void *p, char *n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "and for an integer",
        declared: "p: ptr<mut, u8>, n: fnptr",
        c: "void k_use(void *p, unsigned int n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "`ABI-FNPTR-DEPTH1`: a pointer *to* a function pointer stays refused",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "typedef int (*k_write)(void *);\nvoid k_use(k_write *p, int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "a name typedef'd once to a function pointer and once to an integer refuses",
        declared: "p: ptr<mut, u8>, n: fnptr",
        c: "typedef int (*k_write)(void *);\ntypedef int k_write;\nvoid k_use(void *p, k_write n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    // -- a pointer to a function-type typedef (card
    //    CG22-OP-CALLBACKS-AND-SERVED-QUEUES, `rd_kafka_q_serve_cb_t *`) -----
    Row {
        label: "one pointer to a function-type typedef is `fnptr`",
        declared: "p: ptr<mut, u8>, n: fnptr",
        c: "typedef int(k_cb_t)(void *, int);\nvoid k_use(void *p, k_cb_t *n);",
        outcome: None,
    },
    Row {
        label: "so is one written without parentheses around the name",
        declared: "p: ptr<mut, u8>, n: fnptr",
        c: "typedef int k_cb_t(void *, int);\nvoid k_use(void *p, k_cb_t *n);",
        outcome: None,
    },
    Row {
        label: "an object-pointer spelling of it disagrees",
        declared: "p: ptr<mut, u8>, n: ptr<mut, u8>",
        c: "typedef int(k_cb_t)(void *, int);\nvoid k_use(void *p, k_cb_t *n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "the served-queue shape: an enumeration-returning function type behind \
                typedefs of its own parameter types",
        declared: "p: ptr<mut, u8>, n: fnptr",
        c: "typedef enum { K_A, K_B } k_kind_t;\n\
            typedef enum { K_R1, K_R2 } k_res_t;\n\
            typedef struct k_q_s k_q_t; typedef struct k_op_s k_op_t;\n\
            typedef k_res_t(k_serve_cb_t)(void *rk, k_q_t *rkq, struct k_op_s *rko,\n\
                                          k_kind_t cb_type, void *opaque)\n\
                __attribute__((warn_unused_result));\n\
            void k_use(k_q_t *p, k_serve_cb_t *n);",
        outcome: None,
    },
    Row {
        label: "a pointer to a pointer to a function type stays refused",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "typedef int(k_cb_t)(void *, int);\nvoid k_use(k_cb_t **p, int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    // -- types with no established spelling --------------------------------
    // Card W43-ABI-ENUM-AND-CONST-PP (rule 12u): implementation-defined, so
    // read from Clang, never assumed from the source spelling alone.
    Row {
        label: "an enumeration's compatible integer type is implementation-defined",
        declared: HOST_ENUM_TYPE,
        c: "enum k_kind { K_A, K_B };\nvoid k_use(void *p, enum k_kind n);",
        outcome: None,
    },
    Row {
        label: "an incomplete enumeration has no type Clang states",
        declared: "p: ptr<mut, u8>, n: u32",
        c: "enum k_kind;\nvoid k_use(void *p, enum k_kind n);",
        outcome: if cfg!(windows) {
            Some(AbiRefusal::ParameterTypeDisagreement)
        } else {
            Some(AbiRefusal::UnreadableDeclaration)
        },
    },
    Row {
        label: "`long double` has no spelling in this vocabulary",
        declared: "p: ptr<mut, u8>, n: f64",
        c: "void k_use(void *p, long double n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    // -- an inline function-pointer parameter (card CG22, absorbing
    //    LOADER-ABI-INLINE-FNPTR-DECLARATOR) --------------------------------
    Row {
        label: "an *inline* function-pointer parameter is read as `fnptr`, so an object-pointer \
                spelling of it disagrees",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(void (*p)(int), int n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "and `fnptr` agrees with it, as with the typedef'd spelling",
        declared: "p: fnptr, n: i32",
        c: "void k_use(void (*p)(int), int n);",
        outcome: None,
    },
    Row {
        label: "the measured librdkafka shape: commas inside the declarator's own parameters",
        declared: "p: ptr<mut, u8>, n: fnptr",
        c: "void k_use(void *p, int (*n)(const void *a, const void *b, void *o));",
        outcome: None,
    },
    Row {
        label: "an abstract inline declarator reads the same way",
        declared: "p: fnptr, n: i32",
        c: "void k_use(int (*)(void *), int n);",
        outcome: None,
    },
    Row {
        label: "`ABI-FNPTR-DEPTH1`: an inline pointer *to* a function pointer stays refused",
        declared: "p: fnptr, n: i32",
        c: "void k_use(void (**p)(int), int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "a qualified inline declarator is not committed to",
        declared: "p: fnptr, n: i32",
        c: "void k_use(void (*const p)(int), int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "nor is a function returning a function pointer",
        declared: "p: fnptr, n: i32",
        c: "void k_use(void (*(*p)(int))(void), int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    // -- a qualifier after the last `*` (card W43-LIBC-RESULT-ALIAS) -------
    Row {
        label: "`__restrict` after the last `*` qualifies the parameter object (C11 6.7.6.3p15)",
        declared: "p: ptr<const, u8>, n: i32",
        c: "void k_use(const void *__restrict p, int n);",
        outcome: None,
    },
    Row {
        label: "so does `const` after the last `*`: a const pointer to mutable bytes",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(void *const p, int n);",
        outcome: None,
    },
    Row {
        label: "an unnamed parameter ending in a qualifier reads the same way",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(void *restrict, int n);",
        outcome: None,
    },
    Row {
        label: "several trailing qualifiers are all the parameter object's",
        declared: "p: ptr<const, u8>, n: i32",
        c: "void k_use(const char *const volatile __restrict__ p, int n);",
        outcome: None,
    },
    Row {
        label: "dropping the object's qualifier keeps the pointee's constness",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(const void *__restrict p, int n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "a qualifier between two `*` still qualifies an inner pointer and refuses",
        declared: "p: ptr<mut, ptr<mut, u8>>, n: i32",
        c: "void k_use(void *const *p, int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "and so does one between two `*` with a trailing one after them",
        declared: "p: ptr<mut, ptr<mut, u8>>, n: i32",
        c: "void k_use(void *__restrict *__restrict p, int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    // -- a qualifier inside an array bound (card W43-LIBC-RESTRICT-PROTOTYPES)
    Row {
        label: "`[__restrict]` is `*__restrict`, and the qualifier is the object's (C11 6.7.6.3p7, p15)",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(char p[__restrict], int n);",
        outcome: None,
    },
    Row {
        label: "glibc `regexec`'s bound: a qualifier then a variable size naming a parameter",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "typedef struct { long so; long eo; } k_match;\nvoid k_use(k_match p[__restrict n], int n);",
        outcome: None,
    },
    Row {
        label: "a qualifier then a constant size is the same adjustment",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(char p[const 4], int n);",
        outcome: None,
    },
    Row {
        label: "several qualifiers in a bound are all the object's",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(char p[const volatile restrict], int n);",
        outcome: None,
    },
    Row {
        label: "a qualifier in the bound keeps the element's constness",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(const char p[__restrict], int n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "and the faithful const spelling of that element agrees",
        declared: "p: ptr<const, u8>, n: i32",
        c: "void k_use(const char p[__restrict], int n);",
        outcome: None,
    },
    Row {
        label: "`[static 4]` still refuses",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(char p[static 4], int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "`[*]` still refuses",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(char p[*], int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "a size of more than one token still refuses",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(char p[__restrict n + 1], int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "a qualifier after the size is not C, and refuses",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(char p[4 const], int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "an array of pointers with a qualified bound is still two levels",
        declared: "p: ptr<mut, ptr<mut, u8>>, n: i32",
        c: "void k_use(char *p[__restrict], int n);",
        outcome: None,
    },
    Row {
        label: "an array of const pointers is `char *const *`: an inner qualifier, refused as the pointer spelling is",
        declared: "p: ptr<mut, ptr<mut, u8>>, n: i32",
        c: "void k_use(char *const p[], int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "and so with a qualified bound",
        declared: "p: ptr<mut, ptr<mut, u8>>, n: i32",
        c: "void k_use(char *__restrict p[__restrict], int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    // -- GNU builtin words (card W44-LIBC-RESIDUALS) ----------------------
    Row {
        label: "`__builtin_va_list` behind glibc's two typedefs is `va_list` (`vsnprintf`)",
        declared: "p: ptr<mut, u8>, n: va_list",
        c: "typedef __builtin_va_list __gnuc_va_list;\ntypedef __gnuc_va_list va_list;\n\
            void k_use(void *p, __gnuc_va_list n);",
        outcome: None,
    },
    Row {
        label: "so is the builtin written directly",
        declared: "p: ptr<mut, u8>, n: va_list",
        c: "void k_use(void *p, __builtin_va_list n);",
        outcome: None,
    },
    Row {
        label: "an unnamed `va_list` parameter reads the same way",
        declared: "p: ptr<mut, u8>, n: va_list",
        c: "typedef __builtin_va_list va_list;\nvoid k_use(void *p, va_list);",
        outcome: None,
    },
    Row {
        label: "a `va_list` is not the x86-64 object pointer it decays to: no target is assumed",
        declared: "p: ptr<mut, u8>, n: ptr<mut, u8>",
        c: "typedef __builtin_va_list __gnuc_va_list;\nvoid k_use(void *p, __gnuc_va_list n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "nor an integer",
        declared: "p: ptr<mut, u8>, n: usize",
        c: "typedef __builtin_va_list __gnuc_va_list;\nvoid k_use(void *p, __gnuc_va_list n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "and `va_list` is refused for an object pointer",
        declared: "p: ptr<mut, u8>, n: va_list",
        c: "void k_use(void *p, void *n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    Row {
        label: "a pointer to a `va_list` still refuses",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "typedef __builtin_va_list va_list;\nvoid k_use(va_list *p, int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "an array of `va_list` is a pointer to one, and still refuses",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "typedef __builtin_va_list va_list;\nvoid k_use(va_list p[], int n);",
        outcome: Some(AbiRefusal::UnreadableDeclaration),
    },
    Row {
        label: "a unit that typedefs the builtin name is read through its own typedef",
        declared: "p: ptr<mut, u8>, n: va_list",
        c: "typedef int __builtin_va_list;\nvoid k_use(void *p, __builtin_va_list n);",
        outcome: Some(AbiRefusal::ParameterTypeDisagreement),
    },
    // -- arity -------------------------------------------------------------
    Row {
        label: "a C declaration with more parameters disagrees",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(void *p, int n, int extra);",
        outcome: Some(AbiRefusal::ArityDisagreement),
    },
    Row {
        label: "a C declaration with fewer parameters disagrees",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(void *p);",
        outcome: Some(AbiRefusal::ArityDisagreement),
    },
    // -- two declarations of one name --------------------------------------
    Row {
        label: "two agreeing declarations of the same name are one declaration",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(void *p, int n);\nvoid k_use(void *p, int n);",
        outcome: None,
    },
    Row {
        label: "two declarations that project differently are ambiguous",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use(void *p, int n);\nvoid k_use(void *p, unsigned long n);",
        outcome: Some(AbiRefusal::AmbiguousDeclaration),
    },
    Row {
        label: "an unspecified list composes with a specified one (C11 6.2.7p3)",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use();\nvoid k_use(void *p, int n);",
        outcome: None,
    },
    Row {
        label: "an unspecified list alone cannot check a declared parameter vector",
        declared: "p: ptr<mut, u8>, n: i32",
        c: "void k_use();",
        outcome: Some(AbiRefusal::UnspecifiedParameters),
    },
];

/// Every row of the table, decided through the production analyzer.
#[test]
fn the_admission_and_refusal_table_decides_row_by_row() {
    for row in ROWS {
        let contracts = load(&format!(
            "extern fn k_use({}) -> unit\n\
             contract {{\n    \
             requires resource_contract_v1;\n    \
             ensures normal resource_borrowed(p);\n    \
             ensures normal resource_borrowed(n);\n    \
             ensures normal resource_return_none();\n\
             }};\n",
            row.declared
        ));
        let source = format!("{}\nvoid w(void) {{ }}", row.c);
        let measured = declared_abi_census(&source, &contracts);
        let expected = match row.outcome {
            None => AbiOutcome::Agreed,
            Some(refusal) => AbiOutcome::Withdrawn(refusal),
        };
        assert_eq!(
            measured
                .iter()
                .map(|row| row.outcome)
                .collect::<Vec<_>>()
                .as_slice(),
            [expected],
            "{}: {}",
            row.label,
            row.c
        );
    }
}

/// Card W43-LIBC-RESULT-ALIAS: C11 6.7.6.3p15 speaks of *parameters*, and the
/// reader's widening is applied to parameters only. A result declarator that
/// ends in a qualifier after its `*` stays unreadable, exactly as it was.
#[test]
fn a_result_ending_in_a_qualifier_is_not_widened() {
    let contracts = load(
        "\
extern fn k_make() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
};
",
    );
    for (c, expected) in [
        (
            "char *const k_make(void);",
            AbiOutcome::Withdrawn(AbiRefusal::UnreadableDeclaration),
        ),
        ("char *k_make(void);", AbiOutcome::Agreed),
    ] {
        let source = format!("{c}\nvoid w(void) {{ }}");
        assert_eq!(
            declared_abi_census(&source, &contracts)
                .into_iter()
                .map(|row| row.outcome)
                .collect::<Vec<_>>(),
            [expected],
            "{c}"
        );
    }
}

/// Card W44-LIBC-RESIDUALS: a `va_list` result has no spelling. The loader's
/// result vocabulary leaves `va_list` out, and the reader never reads the
/// builtin at a result, so such a declaration is unreadable rather than
/// compared against some other spelling.
#[test]
fn a_va_list_result_has_no_spelling() {
    let contracts = load(
        "\
extern fn k_swap(x: u64) -> u64
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(x);
    ensures normal resource_return_none();
};
",
    );
    for (c, expected) in [
        (
            "typedef __builtin_va_list __gnuc_va_list;\n__gnuc_va_list k_swap(unsigned long long x);",
            AbiOutcome::Withdrawn(AbiRefusal::UnreadableDeclaration),
        ),
        (
            "unsigned long long k_swap(unsigned long long x);",
            AbiOutcome::Agreed,
        ),
    ] {
        let source = format!("{c}\nvoid w(void) {{ }}");
        assert_eq!(
            declared_abi_census(&source, &contracts)
                .iter()
                .map(|row| row.outcome)
                .collect::<Vec<_>>(),
            [expected],
            "{c}"
        );
    }
    let result = load_resource_contract_sources(&BTreeMap::from([(
        "k_api.qcore".to_owned(),
        "module contract;\n\nextern fn k_list(x: u64) -> va_list\ncontract {\n    \
         requires resource_contract_v1;\n    ensures normal resource_borrowed(x);\n    \
         ensures normal resource_return_none();\n};\n"
            .to_owned(),
    )]));
    assert!(result.is_err(), "a `va_list` result must not load");
}

/// A zero-parameter contract against an unspecified C list is admitted, on a
/// stated ground rather than by exception: it claims nothing about parameters,
/// and the analyzer's existing arity gate refuses any call that passes one.
/// The measured instance is Fluent Bit's `double flb_time_now();`.
#[test]
fn a_zero_parameter_contract_survives_an_unspecified_c_list() {
    let contracts = load(
        "\
extern fn k_now() -> f64
contract {
    requires resource_contract_v1;
    ensures normal resource_return_none();
};
",
    );
    let source = "double k_now();\nvoid w(void) { k_now(); }";
    assert_eq!(
        declared_abi_census(source, &contracts)
            .into_iter()
            .map(|row| row.outcome)
            .collect::<Vec<_>>(),
        vec![AbiOutcome::Agreed]
    );
    // The gate that makes it safe: a call that passes an actual is refused by
    // the existing arity rule, not admitted by this declaration.
    let calling = "double k_now();\nvoid w(void) { k_now(1); }";
    assert_eq!(
        unsupported(function(&analyse(calling, &contracts), "w")),
        vec![
            "arguments beyond the declared parameters of `k_now` are outside the resource \
             projection"
                .to_owned()
        ]
    );
}

/// The variadic rows, in both directions.
#[test]
fn the_variadic_tail_agrees_only_where_c_declares_one() {
    let with_tail = load(
        "\
extern fn k_log(format: ptr<const, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(format);
    ensures normal resource_variadic_data_tail();
    ensures normal resource_return_none();
};
",
    );
    let without_tail = load(
        "\
extern fn k_log(format: ptr<const, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(format);
    ensures normal resource_return_none();
};
",
    );
    let variadic = "void k_log(const char *format, ...);\nvoid w(void) { }";
    let plain = "void k_log(const char *format);\nvoid w(void) { }";
    let outcome = |source: &str, contracts: &ResourceContracts| {
        declared_abi_census(source, contracts)
            .into_iter()
            .map(|row| row.outcome)
            .collect::<Vec<_>>()
    };
    assert_eq!(outcome(variadic, &with_tail), vec![AbiOutcome::Agreed]);
    // A declared tail for a function C does not declare variadic would admit
    // actuals C cannot accept at all.
    assert_eq!(
        outcome(plain, &with_tail),
        vec![AbiOutcome::Withdrawn(AbiRefusal::VariadicDisagreement)]
    );
    // The other direction only ever *narrows* what the arity gate admits, so
    // it is not a disagreement.
    assert_eq!(outcome(variadic, &without_tail), vec![AbiOutcome::Agreed]);
    assert_eq!(outcome(plain, &without_tail), vec![AbiOutcome::Agreed]);
}

// ---------------------------------------------------------------------------
// 3. What the comparison rests on, and what it refuses to guess
// ---------------------------------------------------------------------------

/// An environment with no retained declared texts declares no spellings, so
/// nothing is compared. That is the honest reading — an inferred W4 summary or
/// a directly built table never had a declared type — and it is pinned so the
/// direction cannot drift silently.
#[test]
fn an_environment_without_declared_sources_compares_nothing() {
    let mut contracts = ResourceContracts::default();
    contracts.insert_contract(
        "k_use",
        ResourceFunctionContract::new(
            vec![
                ResourceParameterMode::Borrowed,
                ResourceParameterMode::Borrowed,
            ],
            ResourceReturnMode::None,
        ),
    );
    let source = "void k_use(int n, int m);\nvoid w(void) { }";
    assert_eq!(
        declared_abi_census(source, &contracts)
            .into_iter()
            .map(|row| row.outcome)
            .collect::<Vec<_>>(),
        vec![AbiOutcome::Undeclared]
    );
}

/// A function **defined** in the unit whose declared contract its own
/// definition contradicts is withdrawn too: its callers stop trusting the
/// interface, and the body is no longer checked against it.
///
/// The second half is deliberately silent — a body checked contract-free
/// produces no diagnostic of its own — which is the refusing direction (no
/// entry custody is assumed) but is recorded as `LOADER-ABI-SILENT-DEFINITION`
/// rather than implied. The caller-side refusal below is what makes the
/// withdrawal visible in a report.
#[test]
fn a_defined_function_is_withdrawn_from_its_own_callers() {
    let contracts = load(
        "\
extern fn k_dup(s: ptr<const, u8>, n: i32) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(s);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_owned(nullable);
};
",
    );
    // The definition in this unit takes `(char *, unsigned long)`, so neither
    // parameter position agrees with the declaration.
    let source = "void *k_dup(char *s, unsigned long n) { return s; }\n\
                  void w(void) { k_dup(0, 1); }";
    assert_eq!(
        declared_abi_census(source, &contracts)
            .into_iter()
            .map(|row| row.outcome)
            .collect::<Vec<_>>(),
        vec![AbiOutcome::Withdrawn(AbiRefusal::ParameterTypeDisagreement)]
    );
    let summary = analyse(source, &contracts);
    assert_eq!(
        unsupported(function(&summary, "w")),
        vec![withdrawal("k_dup", AbiRefusal::ParameterTypeDisagreement)]
    );
    // The definition itself is analysed, contract-free, and carries no
    // withdrawal diagnostic of its own.
    assert!(
        unsupported(function(&summary, "k_dup")).is_empty(),
        "the definition is checked contract-free, silently: {:?}",
        unsupported(function(&summary, "k_dup"))
    );
}

/// A retained source set the loader cannot re-read poisons the whole
/// comparison: **every** contracted callee is withdrawn, rather than none of
/// them. An empty map would apply every declaration, which is the fail-open
/// direction.
#[test]
fn unreadable_declared_sources_withdraw_every_contract() {
    let mut contracts = load(
        "\
extern fn k_use(p: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};
",
    );
    contracts.set_source_assumptions(BTreeMap::from([(
        "k_api.qcore".to_owned(),
        "module contract; this is not a contract module".to_owned(),
    )]));
    let source = "void k_use(void *p, int n);\nvoid w(void) { k_use(0, 0); }";
    assert_eq!(
        declared_abi_census(source, &contracts)
            .into_iter()
            .map(|row| row.outcome)
            .collect::<Vec<_>>(),
        vec![AbiOutcome::Withdrawn(AbiRefusal::UnreadableDeclaredSources)]
    );
    assert_eq!(
        unsupported(function(&analyse(source, &contracts), "w")),
        vec![withdrawal("k_use", AbiRefusal::UnreadableDeclaredSources)]
    );
}

/// A withdrawal never moves a verdict towards `Safe`: it can only ever turn a
/// modelled call into an unmodelled one.
///
/// **What it can do, measured and recorded rather than implied
/// (`LOADER-ABI-WITHDRAWAL-UNSAFE`):** withdrawing a callee whose declared
/// `Released` mode was discharging an acquisition leaves that acquisition
/// unaccounted, and the analyzer then reports a **provisional** `Unsafe` — the
/// existing behaviour for *any* uncontracted release, and exactly why hard
/// invariant 2 forbids presenting a provisional `Unsafe` as an upstream defect.
/// No withdrawal occurs anywhere in the pinned corpus, so this direction is not
/// reachable there; it is pinned here so a future one is not a surprise.
#[test]
fn a_withdrawal_never_moves_a_verdict_towards_safe() {
    let contracts = load(
        "\
extern fn k_open() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
};

extern fn k_close(h: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(h, nonnull);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};
",
    );
    let faithful = "void *k_open(void);\nvoid k_close(void *h, int n);\n\
                    void w(void) { void *h; h = k_open(); if (!h) { return; } k_close(h, 0); }";
    let unfaithful = faithful.replace("void k_close(void *h, int n);", "void k_close(void *h);");
    let before = function(&analyse(faithful, &contracts), "w").status;
    let withdrawn = function(&analyse(&unfaithful, &contracts), "w").clone();
    assert_eq!(before, AnalysisStatus::Safe, "the faithful control decides");
    assert_ne!(
        withdrawn.status,
        AnalysisStatus::Safe,
        "the withdrawal must never certify"
    );
    // The measured direction was `Unsafe`: the release became invisible and
    // the leak was alleged provisionally. Card W44-LEAK-PIN-AUDIT: the
    // withdrawn call site is a refused statement that hands `h` to `k_close`,
    // so the refused hand-on rule withdraws that allegation; the verdict is
    // `Unknown`, never `Safe`.
    assert_eq!(
        withdrawn.status,
        AnalysisStatus::Unknown,
        "a leak claimed through a refused hand-on is withdrawn, never certified"
    );
    assert!(
        unsupported(&withdrawn).contains(&withdrawal("k_close", AbiRefusal::ArityDisagreement)),
        "the withdrawal is reported, so the allegation is never silent: {:?}",
        unsupported(&withdrawn)
    );
}

/// The keyed refusal vocabulary is closed and round-trips through the token a
/// withdrawn call site carries, and no refusal is typed as a statement about
/// the environment.
#[test]
fn the_refusal_vocabulary_is_closed_and_round_trips() {
    for refusal in AbiRefusal::ALL {
        assert_eq!(AbiRefusal::from_token(refusal.token()), Some(refusal));
        assert!(!refusal.as_str().is_empty());
        assert!(!refusal.reason().is_empty());
        assert_ne!(
            refusal.category().subject(),
            ReasonSubject::Environment,
            "{}: an ABI disagreement is never an environment failure",
            refusal.as_str()
        );
    }
    // A disagreement is evidence bound to another declaration, not a construct
    // the analysed source got wrong.
    assert_eq!(
        AbiRefusal::ArityDisagreement.category(),
        ReasonCategory::StaleEvidence
    );
    assert_eq!(
        AbiRefusal::UnreadableDeclaration.category(),
        ReasonCategory::UnsupportedSyntax
    );
    assert_eq!(
        AbiRefusal::UnreadableDeclaredSources.category(),
        ReasonCategory::MissingPremise
    );
    assert_eq!(
        AbiRefusal::ALL.len(),
        AbiRefusal::ALL
            .iter()
            .map(|refusal| refusal.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        "every key is distinct"
    );
}

/// The same decision on the production entry point, and at certificate
/// issuance: a unit whose declaration was withdrawn cannot be certified, and
/// the faithful control is.
#[test]
fn the_production_entry_point_and_the_certificate_agree_with_the_census() {
    if !clang_available() {
        eprintln!("skipping Clang-dependent LOADER-ABI test");
        return;
    }
    let contracts = load(UNIT_RESULT);
    let withdrawn = format!("void *k_alloc(unsigned long size);\n{DISCARDING_BODY}");
    match analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &withdrawn,
            ..Default::default()
        },
        "w",
        &contracts,
    )
    .expect("Clang accepts the fixture")
    {
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            let summary = partial.expect("a partial summary is published");
            assert_eq!(
                unsupported(function(&summary, "w")),
                vec![withdrawal("k_alloc", AbiRefusal::ReturnTypeDisagreement)]
            );
        }
        other => panic!("a withdrawn declaration must not certify: {other:?}"),
    }
    // The control: the same body with the callee's own faithful contract, which
    // does certify, so the row above is not passing because nothing certifies.
    let faithful = load(
        "\
extern fn k_alloc(size: usize) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(size);
    ensures normal resource_return_none();
};
",
    );
    let agreeing = format!("void k_alloc(__SIZE_TYPE__ size);\n{DISCARDING_BODY}");
    assert!(
        matches!(
            analyze_unrestricted_c11(
                &C11TranslationUnit {
                    source: &agreeing,
                    ..Default::default()
                },
                "w",
                &faithful,
            )
            .expect("Clang accepts the control"),
            UnrestrictedLegacyResult::Safe(_)
        ),
        "the agreeing control must still certify"
    );
}

#[test]
fn msvc_cdecl_pointer_result_keeps_the_acquisition() {
    for declaration in [
        "void * __cdecl k_alloc(unsigned __int64 size);",
        "void __cdecl *k_alloc(unsigned __int64 size);",
    ] {
        let source = format!("{declaration}\n{DISCARDING_BODY}");
        let summary = analyse(&source, &load(UNIT_RESULT));
        assert_eq!(function(&summary, "w").status, AnalysisStatus::Unknown);
        assert_eq!(
            unsupported(function(&summary, "w")),
            vec![withdrawal("k_alloc", AbiRefusal::ReturnTypeDisagreement)]
        );
    }
}
