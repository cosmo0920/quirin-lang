//! Card CAST-FORMALIZATION (rule 12h): the conversion table and its
//! classifier, bound to `QCore.CastSemantics` by shared vectors.

use super::*;

#[test]
fn every_classify_vector_holds() {
    for (kind, from, to, class) in CLASSIFY_VECTORS {
        assert_eq!(
            classify(kind, from, to).name(),
            class,
            "{kind} {from} -> {to}"
        );
    }
}

#[test]
fn the_class_table_has_one_row_per_class_and_unique_names() {
    let mut names = CONVERSION_TABLE
        .iter()
        .map(|row| row.class.name())
        .collect::<Vec<_>>();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), CONVERSION_TABLE.len());
}

#[test]
fn the_class_table_refines_rule_11b_kind_table() {
    assert!(agrees_with_cast_table());
}

#[test]
fn identity_and_access_rows_are_the_formal_model() {
    use ConversionClass as C;
    // View and round-trip rows preserve the identity.
    for class in [C::ByteView, C::TypedView, C::PunnedView, C::Qualifier] {
        assert!(class.preserves_identity(), "{class:?}");
    }
    // Pointer to integer loses it; integer to pointer and null have none.
    assert_eq!(conversion(C::PointerToInteger).identity, Identity::Lost);
    assert_eq!(conversion(C::NullTest).identity, Identity::Lost);
    assert_eq!(conversion(C::IntegerToPointer).identity, Identity::Absent);
    assert_eq!(conversion(C::NullPointer).identity, Identity::Absent);
    // A typed view needs the effective-type premise; a punned one is refused.
    assert_eq!(conversion(C::TypedView).access, Access::EffectiveType);
    assert!(conversion(C::TypedView).obligations.contains(&P_EFFECTIVE_TYPE));
    assert_eq!(conversion(C::PunnedView).access, Access::Punned);
    assert_eq!(conversion(C::ByteView).access, Access::Character);
    // Function and object pointers never convert into each other.
    assert_eq!(conversion(C::FunctionObject).identity, Identity::Refused);
    // `(void) e` has no value.
    assert_eq!(conversion(C::Discard).identity, Identity::NoValue);
    // Scalars are data.
    assert_eq!(conversion(C::Scalar).identity, Identity::Data);
}

#[test]
fn type_classes_read_clang_spellings() {
    assert_eq!(
        type_class("void (*)(void *)").0,
        TypeClass::Pointer(Pointee::Function)
    );
    assert_eq!(
        type_class("int (*)[4]").0,
        TypeClass::Pointer(Pointee::Object("int[4]".into()))
    );
    // A pointer to a function pointer points to an object.
    assert!(matches!(
        type_class("void (**)(int)").0,
        TypeClass::Pointer(Pointee::Object(_))
    ));
    assert_eq!(
        type_class("const char *const").0,
        TypeClass::Pointer(Pointee::Character("char".into()))
    );
    assert!(type_class("volatile int *").1.volatile);
    assert_eq!(type_class("unsigned long").0, TypeClass::Scalar);
    assert_eq!(type_class("enum mode").0, TypeClass::Scalar);
    assert_eq!(type_class("struct S").0, TypeClass::Other);
    assert_eq!(type_class("void").0, TypeClass::Void);
}

#[test]
fn callee_arity_reads_the_last_parameter_list() {
    assert_eq!(callee_arity("int (*)(const char *, ...)"), Some((1, true)));
    assert_eq!(callee_arity("int (*)(FILE *, const char *, ...)"), Some((2, true)));
    assert_eq!(callee_arity("void (*)(void)"), Some((0, false)));
    assert_eq!(callee_arity("int (*)(int (*)(void), int)"), Some((2, false)));
}
