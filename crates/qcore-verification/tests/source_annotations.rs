use qcore_frontend::frontend::SemanticPass;
use qcore_verification::driver::{CompileOptions, SourceFile, compile_files_with_registry};
use qcore_verifier_host::VerifierRegistry;

fn compile(
    source: &str,
) -> Result<
    qcore_verification::driver::CompiledModule,
    Vec<qcore_verification::driver::FileDiagnostic>,
> {
    compile_files_with_registry(
        &[SourceFile::new("annotation.qcore", source)],
        &CompileOptions::default(),
        &VerifierRegistry::default(),
    )
}

fn refusal(source: &str) -> qcore_verification::driver::FileDiagnostic {
    compile(source)
        .expect_err("the annotated source must be refused")
        .remove(0)
}

#[test]
fn a_matching_i32_annotation_is_checked_and_erases_from_emission() {
    let annotated = compile("fn main() -> i32 { let n: i32 = 3; return n; }").unwrap();
    let inferred = compile("fn main() -> i32 { let n = 3; return n; }").unwrap();
    assert_eq!(
        annotated.reference_observation,
        inferred.reference_observation
    );
    assert_eq!(annotated.c_source, inferred.c_source);
}

#[test]
fn scalar_mismatch_and_unsupported_annotation_refuse_at_ordinary_types() {
    for source in [
        "fn main() -> i32 { let n: unit = 3; return n; }",
        "fn main() -> i32 { let n: Widget = 3; return n; }",
        "fn main() -> i32 { let n Widget = 3; return n; }",
    ] {
        let diagnostic = refusal(source);
        assert_eq!(diagnostic.code, "Q020", "{diagnostic:?}");
        assert_eq!(diagnostic.pass, SemanticPass::OrdinaryTypes);
    }
}

#[test]
fn acquisition_annotations_cannot_claim_an_unpublished_nominal_type() {
    for annotation in ["i32", "Widget", "own<File>"] {
        let source = format!(
            "error Failure {{ Failed; }} fn main() -> i32 {{ let r: {annotation} = qcore_acquire(1, true, Failure); drop r; return 0; }}"
        );
        let diagnostic = refusal(&source);
        assert_eq!(diagnostic.code, "Q020", "{diagnostic:?}");
        assert_eq!(diagnostic.pass, SemanticPass::OrdinaryTypes);
    }
    compile("error Failure { Failed; } fn main() -> i32 { let r = qcore_acquire(1, true, Failure); drop r; return 0; }").unwrap();
}

#[test]
fn region_acquisitions_cannot_bypass_the_annotation_gate() {
    let source = |head: &str| {
        format!(
            "error Failure {{ Failed; }} fn main() -> i32 {{ error region R {{ let r{head} = qcore_acquire(1, true, Failure); qcore_check(false, Failure); }} catch (Failure) quant open R as pack {{ discharge pack; }} return 0; }}"
        )
    };
    compile(&source("")).unwrap();
    for head in [": i32", ": Widget", ": own<File>"] {
        let diagnostic = refusal(&source(head));
        assert_eq!(diagnostic.code, "Q020", "{diagnostic:?}");
        assert_eq!(diagnostic.pass, SemanticPass::OrdinaryTypes);
    }
}

#[test]
fn usage_annotations_keep_their_own_diagnostic() {
    let diagnostic = refusal(
        "error Failure { Failed; } fn main() -> i32 { let r: own<File> use zero = qcore_acquire(1, true, Failure); drop r; return 0; }",
    );
    assert_eq!(diagnostic.code, "Q100");
    assert_eq!(diagnostic.pass, SemanticPass::Usage);
}
