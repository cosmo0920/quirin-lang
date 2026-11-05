//! Card W45-CALLEE-SUMMARY-CASCADE: the member-release rules, end to end
//! through the unit analysis.

use super::super::{
    AnalysisStatus, LegacyInstruction, LegacyProgramSummary, analyze_legacy_c_translation_unit,
    build_function_cfg, tokenize_with_conditionals,
};
use super::guarded_callees;
use qcore_schemas::contract::ResourceContracts;

fn contracts() -> ResourceContracts {
    let sources = [
        (
            "q_base.qcore".to_owned(),
            include_str!("../fixtures/contracts/q_base.qv").to_owned(),
        ),
        (
            "q_nullable.qcore".to_owned(),
            include_str!("../fixtures/contracts/q_nullable.qv").to_owned(),
        ),
        ("q_peek.qcore".to_owned(), Q_PEEK.to_owned()),
    ]
    .into_iter()
    .collect();
    crate::contract_loader::load_resource_contract_sources(&sources).unwrap()
}

/// A borrowing reader, local to these tests.
const Q_PEEK: &str = "module contract;\n\
     extern fn q_peek(p: ptr<mut, u8>) -> unit\n\
     contract {\n\
         requires resource_contract_v1;\n\
         ensures normal resource_borrowed(p);\n\
         ensures normal resource_return_none();\n\
     };\n";

const RECORD: &str = "struct inner { int n; };\n\
     struct node { void *buf; struct node *next; struct inner in; int n; };\n";

fn analyze(body: &str) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit(&format!("{RECORD}{body}"), "subject", &contracts()).unwrap()
}

fn has(summary: &LegacyProgramSummary, function: &str, needle: &str) -> bool {
    summary.functions[function]
        .diagnostics
        .iter()
        .any(|diagnostic| format!("{:?}", diagnostic.kind).contains(needle))
}

fn status(summary: &LegacyProgramSummary, function: &str) -> AnalysisStatus {
    summary.functions[function].status
}

const DESTROY: &str = "static void node_destroy(struct node *n) {\n\
     if (n->buf) q_nullable_free(n->buf);\n\
     q_nullable_free(n);\n}\n";

#[test]
fn a_destructor_body_validates_and_its_caller_certifies() {
    let summary = analyze(&format!(
        "{DESTROY}void subject(void) {{ struct node *n = q_alloc(); node_destroy(n); }}\n"
    ));
    assert_eq!(
        status(&summary, "node_destroy"),
        AnalysisStatus::Safe,
        "{:?}",
        summary.functions["node_destroy"].diagnostics
    );
    assert_eq!(
        status(&summary, "subject"),
        AnalysisStatus::Safe,
        "{:?}",
        summary.functions["subject"].diagnostics
    );
}

#[test]
fn a_body_that_keeps_its_formal_gets_no_member_release() {
    // P4: a body that releases through a formal without releasing the formal
    // gets no rule 1. Card MEMBER-EFFECT-SUMMARY (rule 38j) flips the body
    // deliberately: it is admitted and states a member effect, which no
    // ordinary reader sees; the caller's tracked local is no formal parent, so
    // its call is refused by name and it stays non-`Safe`.
    let summary = analyze(
        "static void node_clear(struct node *n) { q_nullable_free(n->buf); }\n\
         void subject(void) { struct node *n = q_alloc(); node_clear(n); }\n",
    );
    assert_eq!(status(&summary, "node_clear"), AnalysisStatus::Safe);
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
    assert!(has(&summary, "subject", "member-effect-call-refused"));
}

#[test]
fn the_released_formal_is_still_released_exactly_once() {
    // The member release touches no tracked identity: releasing the formal
    // twice is still not Safe.
    let summary = analyze(
        "void subject(struct node *n) { q_nullable_free(n->buf); \
         q_nullable_free(n); q_nullable_free(n); }\n",
    );
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
}

/// The integration review's counterexamples, and their neighbours: none may be
/// `Safe`.
#[test]
fn no_second_use_of_a_released_member_is_safe() {
    let cases: &[(&str, &str)] = &[
        (
            "a second release on the same path",
            "void subject(struct node *n) { q_nullable_free(n->buf); q_nullable_free(n->buf); \
             q_nullable_free(n); }\n",
        ),
        (
            "a release in a branch arm, then again after it",
            "void subject(struct node *n) { if (n->next) { q_nullable_free(n->buf); } \
             q_nullable_free(n->buf); q_nullable_free(n); }\n",
        ),
        (
            "a release in both arms, then again after them",
            "void subject(struct node *n) { if (n->next) q_nullable_free(n->buf); \
             else q_nullable_free(n->buf); q_nullable_free(n->buf); q_nullable_free(n); }\n",
        ),
        (
            "a release in a for loop",
            "void subject(struct node *n, int k) { int i; for (i = 0; i < k; i++) \
             q_nullable_free(n->buf); q_nullable_free(n); }\n",
        ),
        (
            "a release in a while loop",
            "void subject(struct node *n) { while (n->next) { q_nullable_free(n->buf); } \
             q_nullable_free(n); }\n",
        ),
        (
            "a release in a do loop that is not do-while(0)",
            "void subject(struct node *n, int k) { do { q_nullable_free(n->buf); } while (k); \
             q_nullable_free(n); }\n",
        ),
        (
            "a null test after the release",
            "void subject(struct node *n) { q_nullable_free(n->buf); if (n->buf) n->n = 1; \
             q_nullable_free(n); }\n",
        ),
        (
            "a read after the release",
            "void subject(struct node *n) { q_nullable_free(n->buf); q_peek(n->buf); \
             q_nullable_free(n); }\n",
        ),
        (
            "a copy of the base",
            "void subject(struct node *n) { struct node *m = n; q_nullable_free(n->buf); \
             q_nullable_free(m->buf); q_nullable_free(n); }\n",
        ),
        (
            "a re-read of the member into a local",
            "void subject(struct node *n) { void *p = n->buf; q_nullable_free(n->buf); \
             q_nullable_free(p); q_nullable_free(n); }\n",
        ),
        (
            "the member copied into another member",
            "void subject(struct node *n) { n->next = n->buf; q_nullable_free(n->buf); \
             q_nullable_free(n->next); q_nullable_free(n); }\n",
        ),
        (
            "the base passed to another call",
            "void subject(struct node *n) { q_nullable_free(n->buf); q_peek(n); \
             q_nullable_free(n); }\n",
        ),
        (
            "the base tested in a condition that is not a null test",
            "void subject(struct node *n) { if (n == n->next) return; q_nullable_free(n->buf); \
             q_peek(n->buf); q_nullable_free(n); }\n",
        ),
        (
            "a goto in the body",
            "void subject(struct node *n) { again: q_nullable_free(n->buf); \
             if (n->next) goto again; q_nullable_free(n); }\n",
        ),
        (
            "a return between the member and the base release",
            "void subject(struct node *n) { q_nullable_free(n->buf); if (n->next) return; \
             q_nullable_free(n); }\n",
        ),
        (
            "the base released only conditionally",
            "void subject(struct node *n) { q_nullable_free(n->buf); if (n->next) \
             q_nullable_free(n); }\n",
        ),
    ];
    for (reason, body) in cases {
        let summary = analyze(body);
        assert_ne!(
            status(&summary, "subject"),
            AnalysisStatus::Safe,
            "{reason}: {:?}",
            summary.functions["subject"].diagnostics
        );
    }
}

/// The summary route: a callee that releases the member, then the caller.
#[test]
fn a_callee_release_is_never_followed_safely_by_the_caller() {
    let partial = "static void d(struct node *n) { q_nullable_free(n->buf); }\n";
    let cases: &[(&str, String)] = &[
        (
            "the review's case: a callee that keeps the base",
            format!(
                "{partial}void subject(struct node *n) {{ d(n); q_nullable_free(n->buf); \
                 q_nullable_free(n); }}\n"
            ),
        ),
        (
            "a full destructor, then the caller releases the member",
            format!(
                "{DESTROY}void subject(struct node *n) {{ node_destroy(n); \
                 q_nullable_free(n->buf); }}\n"
            ),
        ),
        (
            "a full destructor, then the caller null-tests the member",
            format!(
                "{DESTROY}void subject(struct node *n) {{ node_destroy(n); if (n->buf) return; }}\n"
            ),
        ),
        (
            "a full destructor, then the caller reads the member",
            format!(
                "{DESTROY}void subject(struct node *n) {{ node_destroy(n); q_peek(n->buf); }}\n"
            ),
        ),
        (
            "a full destructor, then the caller passes the base",
            format!(
                "{DESTROY}void subject(struct node *n) {{ node_destroy(n); node_destroy(n); }}\n"
            ),
        ),
        (
            "the caller copied the member first",
            format!(
                "{DESTROY}void subject(struct node *n) {{ void *p = n->buf; node_destroy(n); \
                 q_nullable_free(p); }}\n"
            ),
        ),
        (
            "an acquired base, destroyed, then its member released",
            format!(
                "{DESTROY}void subject(void) {{ struct node *n = q_alloc(); node_destroy(n); \
                 q_nullable_free(n->buf); }}\n"
            ),
        ),
    ];
    for (reason, source) in cases {
        let summary = analyze(source);
        assert_ne!(
            status(&summary, "subject"),
            AnalysisStatus::Safe,
            "{reason}: {:?}",
            summary.functions["subject"].diagnostics
        );
    }
}

/// The destructor shapes that must keep working: distinct members, then the
/// base, on every path.
#[test]
fn destructor_shapes_still_certify() {
    let cases: &[(&str, &str)] = &[
        (
            "two members and the base",
            "static void d(struct node *n) { if (n->buf) q_nullable_free(n->buf); \
             q_nullable_free(n->next); q_nullable_free(n); }\n",
        ),
        (
            "the RD_IF_FREE shape (do-while(0) is not a loop)",
            "static void d(struct node *n) { do { if ((n->buf)) q_nullable_free(n->buf); } \
             while (0); q_nullable_free(n); }\n",
        ),
        (
            "a __builtin_expect null test",
            "static void d(struct node *n) { if (__builtin_expect((n->buf != ((void *)0)), 1)) \
             q_nullable_free(n->buf); q_nullable_free(n); }\n",
        ),
        (
            "a null test of the base first (cmt_histogram_buckets_destroy)",
            "static void d(struct node *n) { if (!n) { return; } if (n->buf) { \
             q_nullable_free(n->buf); } q_nullable_free(n); }\n",
        ),
        (
            "a scalar member written before the releases",
            "static void d(struct node *n) { n->n = 0; q_nullable_free(n->buf); \
             q_nullable_free(n); }\n",
        ),
    ];
    for (reason, body) in cases {
        let summary = analyze(&format!(
            "{body}void subject(void) {{ struct node *n = q_alloc(); d(n); }}\n"
        ));
        assert_eq!(
            status(&summary, "d"),
            AnalysisStatus::Safe,
            "{reason}: {:?}",
            summary.functions["d"].diagnostics
        );
        assert_eq!(
            status(&summary, "subject"),
            AnalysisStatus::Safe,
            "{reason}: {:?}",
            summary.functions["subject"].diagnostics
        );
    }
}

/// `AGGREGATE-NULL-TEST-AFTER-RELEASE`: the O04-B08 aggregate null test now
/// carries its base's dereference obligation.
#[test]
fn an_aggregate_null_test_after_release_is_not_safe() {
    let base = |condition: &str| {
        format!(
            "void subject(void) {{ struct node *n = q_alloc(); if (n == 0) return; \
             q_free(n); if ({condition}) return; }}\n"
        )
    };
    for condition in ["n->buf", "n->buf == 0", "!n->buf", "n->next != ((void *)0)"] {
        let summary = analyze(&base(condition));
        assert_ne!(
            status(&summary, "subject"),
            AnalysisStatus::Safe,
            "{condition}: {:?}",
            summary.functions["subject"].diagnostics
        );
    }
    // Before the release the same test is admitted.
    let summary = analyze(
        "void subject(void) { struct node *n = q_alloc(); if (n == 0) return; \
         if (n->buf) { q_free(n); return; } q_free(n); }\n",
    );
    assert_eq!(
        status(&summary, "subject"),
        AnalysisStatus::Safe,
        "{:?}",
        summary.functions["subject"].diagnostics
    );
}

#[test]
fn a_place_obliges_every_pointer_it_reads_through() {
    assert_eq!(super::dereferenced_prefixes("b->item"), vec!["b"]);
    assert_eq!(super::dereferenced_prefixes("a->b->c"), vec!["a", "a->b"]);
    assert!(super::dereferenced_prefixes("s.f").is_empty());
    assert_eq!(super::dereferenced_prefixes("s.f->g"), vec!["s.f"]);
}

#[test]
fn a_member_read_through_a_released_base_is_refused() {
    let summary = analyze(
        "void subject(void) { struct node *n = q_alloc(); q_nullable_free(n); \
         q_nullable_free(n->buf); }\n",
    );
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
}

#[test]
fn a_non_null_release_keeps_the_refusal() {
    let summary = analyze("void subject(struct node *n) { q_free(n->buf); q_nullable_free(n); }\n");
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
    // Card MEMBER-EFFECT-SUMMARY (rule 38i) re-pins the text: the refusal
    // names its premise.
    assert!(has(&summary, "subject", "member-release-through-formal"));
}

#[test]
fn a_member_address_is_never_released() {
    let summary =
        analyze("void subject(struct node *n) { q_nullable_free(&n->in); q_nullable_free(n); }\n");
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
}

#[test]
fn a_consuming_position_keeps_the_refusal() {
    let summary = analyze("void subject(struct node *n) { q_transfer(n->buf); }\n");
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
    assert!(has(&summary, "subject", "indirect memory access"));
}

#[test]
fn a_released_member_of_a_linked_owner_is_refused() {
    // The body links a tracked child at `n->buf`, so the read may name it.
    let summary = analyze(
        "void subject(void) { struct node *n = q_alloc(); n->buf = q_alloc(); \
         q_nullable_free(n->buf); q_nullable_free(n); }\n",
    );
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
}

#[test]
fn the_guard_refuses_a_caller_whose_actual_owns_sub_custody() {
    // The destructor releases whatever `n->buf` holds; a caller that linked a
    // tracked child there may not read the call as releasing `n` alone.
    let summary = analyze(&format!(
        "{DESTROY}void subject(void) {{ struct node *n = q_alloc(); n->buf = q_alloc(); \
         node_destroy(n); }}\n"
    ));
    assert_eq!(status(&summary, "node_destroy"), AnalysisStatus::Safe);
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
    assert!(
        has(&summary, "subject", "UnknownAlias"),
        "{:?}",
        summary.functions["subject"].diagnostics
    );
}

#[test]
fn the_guard_precedes_only_undeclared_unit_callees_that_may_release_through_a_formal() {
    let source = format!(
        "{RECORD}{DESTROY}void q_free(void *p) {{ }}\n\
         static void plain_destroy(struct node *n) {{ q_nullable_free(n); }}\n\
         void subject(void) {{ struct node *n = q_alloc(); \
         node_destroy(n); struct node *m = q_alloc(); plain_destroy(m); \
         void *p = q_alloc(); q_free(p); }}\n"
    );
    // `plain_destroy` reads no member, so its summary hides nothing.
    // `q_free` is defined here too, but declared: its declaration governs.
    let (tokens, _) = tokenize_with_conditionals(&source);
    // The environment the caller is analysed in once the destructor's summary
    // is installed.
    let mut environment = contracts();
    let released = environment.contract("q_nullable_free").unwrap().clone();
    environment.insert_contract("node_destroy", released.clone());
    environment.insert_contract("plain_destroy", released);
    let cfg = build_function_cfg(&tokens, "subject", &environment).unwrap();
    assert_eq!(
        guarded_callees(&cfg).into_iter().collect::<Vec<_>>(),
        vec!["node_destroy".to_owned()]
    );
}

#[test]
fn a_member_null_test_is_a_typed_read_of_its_base() {
    let summary = analyze(
        "void subject(struct node *n) { if (n->buf == NULL) return; \
         if (!(n->next)) return; while (n->buf != ((void *)0)) { return; } }\n",
    );
    assert_eq!(
        status(&summary, "subject"),
        AnalysisStatus::Safe,
        "{:?}",
        summary.functions["subject"].diagnostics
    );
}

#[test]
fn a_member_null_test_reads_only_single_pointer_members() {
    // W47 re-pinned the chain/comparison fallbacks below. W48-BOTTOM-UP-
    // BATCH4 deliberately re-pins the remaining compound shape: both old
    // readers already supplied checked reads, so its short-circuit plan now
    // stands without requiring a newer typed-path leaf. Each dereferenced
    // base remains obliged. The member-null reader itself still reads one
    // member, and the cast refusal below is unchanged.
    let summary = analyze("void subject(struct node *n) { if (n->buf && n->n) return; }\n");
    assert_eq!(status(&summary, "subject"), AnalysisStatus::Safe);
    assert!(summary.functions["subject"].diagnostics.is_empty());
    assert!(
        summary.functions["subject"]
            .cfg
            .nodes
            .iter()
            .filter(|node| {
                matches!(
                    node.instruction,
                    LegacyInstruction::IndirectScalarAccess { .. }
                )
            })
            .count()
            >= 2,
        "both source reads must retain their base obligations"
    );
    for condition in ["n->next->buf", "n->buf == n->next"] {
        let summary = analyze(&format!(
            "void subject(struct node *n) {{ if ({condition}) return; }}\n"
        ));
        assert!(
            !has(&summary, "subject", "branch condition"),
            "{condition}: {:?}",
            summary.functions["subject"].diagnostics
        );
    }
    let summary = analyze("void subject(struct node *n) { if ((char *) n->buf) return; }\n");
    assert!(
        has(&summary, "subject", "branch condition"),
        "{:?}",
        summary.functions["subject"].diagnostics
    );
}

#[test]
fn a_member_null_test_over_a_released_base_is_refused() {
    // Over a formal: a body-acquired base's member test is the pre-existing
    // aggregate null condition (card O04-B08), not this rule.
    let summary =
        analyze("void subject(struct node *n) { q_nullable_free(n); if (n->buf) return; }\n");
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
    assert!(has(&summary, "subject", "UnknownAlias"));
}

#[test]
fn the_releasing_set_is_closed_under_calls() {
    let source = format!(
        "{RECORD}{DESTROY}static void outer(struct node *n) {{ node_destroy(n); }}\n\
         static void plain(struct node *n) {{ q_nullable_free(n); }}\n\
         static void element(void **v) {{ q_nullable_free(v[0]); }}\n"
    );
    let (tokens, _) = tokenize_with_conditionals(&source);
    let set = super::member_releasing_functions(&tokens);
    assert_eq!(
        set.iter().cloned().collect::<Vec<_>>(),
        vec![
            "element".to_owned(),
            "node_destroy".to_owned(),
            "outer".to_owned()
        ]
    );
}

// Card W45-GATEWAY-MEMO-KEY made the per-thread memo serve an entry only on key
// equality; card W45-MEMO-MEMORY removed the memo. No state is carried from one
// token stream to the next, so alternating streams each get their own answer.

fn releasing_source() -> String {
    format!(
        "{RECORD}{DESTROY}static void outer(struct node *n) {{ node_destroy(n); }}\n\
         static void plain(struct node *n) {{ q_nullable_free(n); }}\n"
    )
}

#[test]
fn interleaved_streams_always_get_their_own_answer() {
    let first = tokenize_with_conditionals(&releasing_source()).0;
    let second = tokenize_with_conditionals(&format!(
        "{RECORD}static void element(void **v) {{ q_nullable_free(v[0]); }}\n"
    ))
    .0;
    let expected = [
        super::member_releasing_functions(&first),
        super::member_releasing_functions(&second),
    ];
    assert!(expected[0].contains("node_destroy") && !expected[0].contains("plain"));
    assert_eq!(expected[1].iter().collect::<Vec<_>>(), ["element"]);
    for round in 0..4 {
        for (tokens, want) in [(&first, &expected[0]), (&second, &expected[1])] {
            assert_eq!(
                super::member_releasing_functions(tokens),
                *want,
                "round {round}"
            );
            assert_eq!(
                super::member_releasing_functions(tokens),
                *want,
                "round {round}"
            );
        }
    }
}
