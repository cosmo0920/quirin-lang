//! ABI-TYPEDEF-ADDRESS-OF-MEMBER — the addressed-member reader, widened
//! (`docs/reports/wave20-abi-address-of-member/README.md`).
//!
//! Card O04-B09B10 admits `& BASE - > FIELD` as a whole actual at a declared
//! `Borrowed` position, on a member declared with **no** `*`: its storage is
//! disjoint from every member this projection links, so the callee can reach no
//! field link at all. The premise it checks that on —
//! `TypedAccessModel::addressable_field` — required the member's specifier list
//! to classify `struct TAG` or arithmetic, so a `union`-typed member and every
//! typedef the reader would not resolve kept the refusal.
//!
//! Card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF measured the consequence on
//! production `flb_input_chunk_create` and named it: `pthread_mutex_t` is
//! `typedef union { … } pthread_mutex_t;` and `msgpack_packer` is
//! `typedef struct msgpack_packer { … } msgpack_packer;` — aggregates their own
//! typedef *defines*, which the reader abandoned at the opening brace. This card
//! reads that shape, and widens the specifier premise from the
//! `record`/`scalar` classification to "an object type that is not a pointer
//! type", which is all the disjointness argument consumes.
//!
//! **Held-out generic family.** An `a_*` API deliberately unrelated to ChunkIO,
//! to Fluent Bit, to msgpack, to pthread and to the `b_*`/`g_*`/`h_*`/`k_*`/
//! `m_*`/`r_*` families of the earlier cards. `Safe` here is `Safe` for a
//! synthetic declared custody projection over trusted test assumptions; nothing
//! in this file promotes any raw unit.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, LegacySafetyCertificate, ResourceContracts, UnrestrictedLegacyResult,
    analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = include_str!("fixtures/contracts/address_of_member_generic.qv");

/// The existing rejection every refused shape must keep.
const INDIRECT: &str = "indirect memory access is outside the resource projection";

/// Declarations shared by every fixture.
///
/// `a_lock_t` is the untagged-`union` shape of `pthread_mutex_t`; `a_packer`
/// the tagged-`struct` shape of `msgpack_packer`. `a_ctx_ptr` and `a_write_fn`
/// are the two spellings that declare a **pointer** with no declarator star,
/// and both must keep the refusal a plain `struct a_ctx *` member has.
const DECLARATIONS: &str = "\
struct a_ctx; struct a_handle; \
typedef union { int tag; long align; } a_lock_t; \
typedef struct a_packer { void *data; int flags; } a_packer; \
typedef a_lock_t a_lock_alias; \
typedef struct a_ctx *a_ctx_ptr; \
typedef int (*a_write_fn)(void *, int); \
typedef int a_bank_t[2]; \
union a_view { int tag; long value; }; \
struct a_holder { \
    int flags; \
    a_lock_t lock; \
    a_lock_alias aliased_lock; \
    a_packer packer; \
    union a_view view; \
    struct a_ctx *cio; \
    a_ctx_ptr aliased_cio; \
    a_write_fn hook; \
    a_bank_t bank; \
    volatile a_lock_t guarded; \
    a_lock_t table[2]; \
}; \
struct a_handle *a_open(struct a_ctx *cio, int flags); \
void a_close(struct a_handle *h); \
int a_borrow(void *p); \
void a_adopt(void *p); \
void a_release(void *p); \
int a_untracked(void *p);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "address_of_member_generic.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the generic contract fixture loads")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent ABI-TYPEDEF-ADDRESS-OF-MEMBER test: {reason}");
            false
        }
    }
}

fn analyze_with(declarations: &str, body: &str, entry: &str) -> UnrestrictedLegacyResult {
    let source = format!("{declarations} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"))
}

fn analyze(body: &str, entry: &str) -> UnrestrictedLegacyResult {
    analyze_with(DECLARATIONS, body, entry)
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("no projection at all: {other:?}"),
    }
}

fn function<'a>(summary: &'a LegacyProgramSummary, name: &str) -> &'a LegacyFunctionSummary {
    summary
        .functions
        .get(name)
        .unwrap_or_else(|| panic!("missing function `{name}`"))
}

fn certified(body: &str, entry: &str) -> LegacySafetyCertificate {
    match analyze(body, entry) {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate).expect("certificate rechecks");
            certificate
        }
        other => panic!("expected a validated custody projection: {other:?}"),
    }
}

/// A body whose only obstacle is the member address under test: the aggregate
/// is a formal this function never binds, and the acquisition is released on
/// every path.
fn borrowing_body(argument: &str) -> String {
    format!(
        "struct a_handle *w(struct a_holder *hold, int n) {{ \
         struct a_handle *chunk; \
         a_borrow({argument}); \
         chunk = a_open(0, n); if (!chunk) {{ return 0; }} \
         a_close(chunk); return 0; }}"
    )
}

/// Assert that a shape keeps its pre-card answer: `Unknown`, carrying the
/// existing indirect-access diagnostic, and never `Safe`.
fn refused(label: &str, body: &str) {
    refused_as(label, body, INDIRECT);
}

/// [`refused`], carrying `description`.
fn refused_as(label: &str, body: &str, description: &str) {
    let result = analyze(body, "w");
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "{label}: a refused shape must never certify"
    );
    let w = function(program(&result), "w");
    assert_eq!(w.status, AnalysisStatus::Unknown, "{label}");
    assert!(
        w.diagnostics.iter().any(|diagnostic| matches!(
            &diagnostic.kind,
            LegacyDiagnosticKind::Unsupported { description: found } if found == description
        )),
        "{label}: the existing diagnostic must be retained, got {:?}",
        w.diagnostics
    );
}

fn admitted(label: &str, argument: &str) {
    let certificate = certified(&borrowing_body(argument), "w");
    let w = function(&certificate.summary, "w");
    assert_eq!(w.status, AnalysisStatus::Safe, "{label}");
    assert!(
        w.diagnostics.is_empty(),
        "{label}: the admitted statement leaves no diagnostic: {:?}",
        w.diagnostics
    );
}

// --------------------------------------------------------------------------
// Positive — the shapes this card adds, and the control that already worked
// --------------------------------------------------------------------------

/// The measured production shape: `pthread_mutex_t metrics_chunk_lock;` is a
/// member declared over a typedef that **defines** an untagged `union`.
#[test]
fn a_member_whose_typedef_defines_an_untagged_union_is_addressable() {
    if !clang_available() {
        return;
    }
    admitted("an untagged-union typedef member", "&hold->lock");
}

/// The other measured production shape: `msgpack_packer mp_pck;`, whose typedef
/// defines a **tagged** `struct`.
#[test]
fn a_member_whose_typedef_defines_a_tagged_struct_is_addressable() {
    if !clang_available() {
        return;
    }
    admitted("a tagged-struct typedef member", "&hold->packer");
}

/// The reading survives an intervening plain typedef, exactly as every other
/// target does.
#[test]
fn an_aggregate_definition_reached_through_an_alias_is_addressable() {
    if !clang_available() {
        return;
    }
    admitted("an aliased aggregate typedef member", "&hold->aliased_lock");
}

/// A `union`-typed member of a `struct`. Its storage is disjoint from its
/// siblings exactly as any other member's is; the reader had refused it only
/// because the *classifier* answers opaque for a `union` specifier.
#[test]
fn a_union_typed_member_is_addressable() {
    if !clang_available() {
        return;
    }
    admitted("a union-typed member", "&hold->view");
}

/// The control that already worked before this card, unchanged: a scalar
/// member's address.
#[test]
fn a_scalar_member_keeps_the_admission_it_already_had() {
    if !clang_available() {
        return;
    }
    admitted("a scalar member", "&hold->flags");
}

// --------------------------------------------------------------------------
// Adversarial — every one of these must keep the pre-card answer
// --------------------------------------------------------------------------

/// The laundering shape the whole rule exists to refuse, in its three
/// spellings: a declarator star, a pointer typedef and a function-pointer
/// typedef. The address of a pointer member is a handle with which the callee
/// could overwrite the very link this projection maintains, and a typedef does
/// not make it any less one.
#[test]
fn no_spelling_of_a_pointer_member_is_ever_addressable() {
    if !clang_available() {
        return;
    }
    for member in ["cio", "aliased_cio", "hook"] {
        refused(
            "a pointer member's address is never admitted",
            &borrowing_body(&format!("&hold->{member}")),
        );
    }
}

/// The rest of the member premises, one violated per row.
#[test]
fn every_other_refused_member_shape_keeps_its_unknown_and_its_diagnostic() {
    if !clang_available() {
        return;
    }
    // Card CG13 (rule 10w) moves `&hold->lock.tag` out of this table: the
    // continued selection's only refusal was the union step, which rule 10w
    // reads; rule 10n's typed address fallback hands the address of a scalar
    // member of the custody-free union to the borrowed position (no member of
    // a union is ever linked, O04-B09B10's premise). BEFORE: refused.
    admitted("a continued selection through a union", "&hold->lock.tag");
    let cases: [(&str, &str); 5] = [
        ("an array member declared through a typedef", "&hold->bank"),
        ("an array member declared directly", "&hold->table"),
        ("a subscripted member", "&hold->table[0]"),
        ("a `volatile` member", "&hold->guarded"),
        ("a parenthesized address", "&(hold->lock)"),
    ];
    for (label, argument) in cases {
        refused(label, &borrowing_body(argument));
    }
}

/// The position premise, quantified over the two custody-bearing sinks in
/// scope. Releasing or consuming a sub-object of a live aggregate is not an
/// effect this projection could account for, and the widened reader does not
/// touch that gate.
#[test]
fn a_custody_bearing_position_never_admits_a_member_address() {
    if !clang_available() {
        return;
    }
    refused(
        "a custody-bearing position never admits a member address",
        "void w(struct a_holder *hold) { a_adopt(&hold->lock); }",
    );
    // Card MEMBER-EFFECT-SUMMARY (rule 38i) re-pins this deliberately: at a release position the address of a
    // formal's member names its premise; still `Unknown`, never `Safe`.
    refused_as(
        "a custody-bearing position never admits a member address",
        "void w(struct a_holder *hold) { a_release(&hold->lock); }",
        "release of `&hold->lock`, an address inside the formal `hold` \
         (member-address-release-through-formal)",
    );
}

/// A callee with no contract in force never admits an actual, whatever the
/// member is.
#[test]
fn an_uncontracted_callee_never_admits_a_member_address() {
    if !clang_available() {
        return;
    }
    // Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (rule 12k) re-pins this
    // deliberately: the member address is an actual W44 reads at a `Borrowed`
    // position, so the uncontracted callee is what decides and the refusal
    // names it. The shape is still never admitted: `Unknown`, never `Safe`.
    let result = analyze(
        "void w(struct a_holder *hold) { a_untracked(&hold->lock); }",
        "w",
    );
    assert!(!matches!(result, UnrestrictedLegacyResult::Safe(_)));
    let w = function(program(&result), "w");
    assert_eq!(w.status, AnalysisStatus::Unknown);
    assert!(
        w.diagnostics.iter().any(|diagnostic| matches!(
            &diagnostic.kind,
            LegacyDiagnosticKind::Unsupported { description }
                if description == "call to uncontracted function `a_untracked`"
        )),
        "{:?}",
        w.diagnostics
    );
}

/// The premise card C02 §2 had to add after a read through a released pointer
/// turned an honest `Unknown` into `Safe` (`AGENTS.md` constraint 5): a base
/// this body may bind to a tracked identity is never admitted by **this**
/// card's syntactic premise.
///
/// Card W44-INDIRECT-CALL-ARGUMENT-ACCESS moves this pin deliberately. Its call
/// argument rule admits `&hold->lock` at a `Borrowed` position under the
/// flow-sensitive dereference obligation of W43-INDIRECT-ACCESS-GENERAL
/// instead: `hold` is unbound on every reaching path, and the only statement
/// that could have released it, the uncontracted call, is itself refused. So
/// the load-bearing half holds — the body never certifies and stays `Unknown`
/// on the uncontracted call's own diagnostic — while the member address's
/// `indirect memory access` leaf is withdrawn. A base released by a *modeled*
/// statement is refused and accused: `call_argument_access.rs`
/// `a_member_address_after_release_is_accused`.
#[test]
fn a_base_this_body_may_bind_is_never_admitted() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void w(struct a_holder *hold) { a_untracked(hold); a_borrow(&hold->lock); }",
        "w",
    );
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "a base an uncontracted callee received must never certify"
    );
    let w = function(program(&result), "w");
    assert_eq!(w.status, AnalysisStatus::Unknown);
    let descriptions = w
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => description.clone(),
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        descriptions,
        vec!["call to uncontracted function `a_untracked`".to_owned()]
    );
}

/// A name typedef'd twice with disagreeing readings retires, in the lexical
/// reader, to the absent reading, so this card's lexical route keeps its
/// refusal. Re-pinned deliberately by W47-ACCESS-PATH-POSITION-RESIDUALS
/// (gateway rule 10n): the typed address fallback, asked only after that
/// refusal, reads Clang's one type for the member, `struct a_packer`, an
/// object that is not a pointer, array or function pointer, which is exactly
/// this card's disjointness premise; so the member's address now certifies.
/// Every other refusal of this file (pointer, array, element, `volatile`,
/// continued selection, parenthesized place, custody position, uncontracted
/// callee, bound base) passes unchanged. The declaration set is the shared
/// one with a second, plain typedef of `a_packer` appended.
#[test]
fn a_name_typedefd_twice_is_read_on_clangs_one_type() {
    if !clang_available() {
        return;
    }
    let declarations = format!("{DECLARATIONS} typedef struct a_packer a_packer;");
    let body = borrowing_body("&hold->packer");
    let result = analyze_with(&declarations, &body, "w");
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate).expect("certificate rechecks");
        }
        other => panic!("the typed address fallback reads Clang's one type: {other:?}"),
    }
}

// --------------------------------------------------------------------------
// Containment — the widening reaches exactly one premise
// --------------------------------------------------------------------------

/// The new reading is invisible to the *classifier*, so nothing that reads a
/// declared type through it moved. A whole aggregate member is still not a
/// value this projection can bind, and an assignment reading one keeps its
/// refusal even though its address is now admissible.
#[test]
fn an_aggregate_member_is_still_not_a_readable_value() {
    if !clang_available() {
        return;
    }
    refused(
        "reading a whole aggregate member is not a scalar no-op",
        "void w(struct a_holder *hold, struct a_holder *sink) { sink->packer = hold->packer; }",
    );
}

/// And a member whose typedef defines an aggregate is not a *store*
/// destination for the aggregate-store rule, which links single-pointer and
/// `void *` members only.
///
/// Card CG14-AGGREGATE-VALUES-AND-INITIALIZERS flip (gateway rule 11f): the
/// whole-union copy `hold->lock = hold->aliased_lock` is of a custody-free
/// union (`int`, `long`), so it is the source's typed read and the
/// destination's typed store and certifies. BEFORE: `Unknown` with the
/// indirect-access diagnostic. A pointer-bearing member copy
/// (`sink->packer = hold->packer`, above) keeps its refusal.
#[test]
fn an_aggregate_typedef_member_is_not_a_store_destination() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void w(struct a_holder *hold) { hold->lock = hold->aliased_lock; }",
        "w",
    );
    assert!(
        matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "a custody-free union copy certifies (rule 11f): {result:?}"
    );
}

// --------------------------------------------------------------------------
// Independent rederivation
// --------------------------------------------------------------------------

/// The admission rests on the *declaration* of the member, and the independent
/// recheck re-derives it from the certificate's own preprocessed source.
/// Substituting only the declaration, leaving every statement byte-identical,
/// must be refused.
#[test]
fn a_certificate_resting_on_the_admission_is_rederived_from_the_declarations() {
    if !clang_available() {
        return;
    }
    let certificate = certified(&borrowing_body("&hold->lock"), "w");
    let substitutions: [(&str, &str, &str); 3] = [
        (
            "the member's typedef becomes a pointer typedef",
            "typedef union { int tag; long align; } a_lock_t;",
            "typedef union { int tag; long align; } a_lock_body; typedef a_lock_body *a_lock_t;",
        ),
        (
            "the member becomes an array",
            "a_lock_t lock;",
            "a_lock_t lock[2];",
        ),
        (
            "the member becomes volatile",
            "a_lock_t lock;",
            "volatile a_lock_t lock;",
        ),
    ];
    for (label, from, to) in substitutions {
        let mut mutated = certificate.clone();
        assert!(
            mutated.preprocessed_source.contains(from),
            "{label}: anchor not found"
        );
        mutated.preprocessed_source = mutated.preprocessed_source.replacen(from, to, 1);
        let error = check_legacy_safety_certificate(&mutated)
            .expect_err(&format!("{label}: the substitution must be refused"));
        assert!(
            error.contains("do not match the preprocessed source")
                || error.contains("does not match the preprocessed source")
                || error.contains("is not independently Safe")
                || error.contains("retains an unknown instruction"),
            "{label}: unexpected refusal `{error}`"
        );
    }
    check_legacy_safety_certificate(&certificate).expect("the baseline still rechecks");
}

/// And on the *contract mode* of the position the actual occupies: a
/// certificate whose carried contract no longer declares that position borrowed
/// is refused, so the admission cannot survive its own premise.
#[test]
fn a_certificate_resting_on_the_borrowed_position_is_refused_when_the_mode_changes() {
    if !clang_available() {
        return;
    }
    let certificate = certified(&borrowing_body("&hold->lock"), "w");
    let mutated_source = CONTRACTS.replacen(
        "    ensures normal resource_borrowed(p);",
        "    ensures normal resource_consumed(p, nonnull);",
        1,
    );
    assert_ne!(mutated_source, CONTRACTS, "the mode anchor was found");
    let mut mutated = certificate.clone();
    mutated.contracts = load_resource_contract_sources(&BTreeMap::from([(
        "address_of_member_generic.qcore".to_owned(),
        mutated_source,
    )]))
    .expect("the mutated contract fixture loads");
    check_legacy_safety_certificate(&mutated)
        .expect_err("a non-borrowed position must withdraw the admission");
    check_legacy_safety_certificate(&certificate).expect("the baseline still rechecks");
}
