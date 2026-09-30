//! Unit tests of card CG15-STORAGE-DURATION-AND-VOLATILE (gateway rule 11h):
//! the decision table, the qualifier reader, the trace, and the mechanical
//! binding to `proof/QCore/StorageDuration.lean`.

use super::*;

#[test]
fn top_level_volatile_reads_the_object_qualifier() {
    assert!(top_level_volatile("volatile int"));
    assert!(top_level_volatile("const volatile unsigned int"));
    assert!(top_level_volatile("void *volatile"));
    assert!(top_level_volatile("char *const volatile"));
    assert!(top_level_volatile("volatile int[4]"));
    assert!(!top_level_volatile("volatile int *"));
    assert!(!top_level_volatile("int"));
    assert!(!top_level_volatile("struct volatility *"));
    assert!(!top_level_volatile("_Atomic(int)"));
}

#[test]
fn storage_of_reads_clangs_storage_class() {
    let decl = |text: &str| serde_json::from_str::<Value>(text).unwrap();
    let automatic = decl(r#"{"kind":"VarDecl","name":"a"}"#);
    let register = decl(r#"{"kind":"VarDecl","name":"a","storageClass":"register"}"#);
    let local_static = decl(r#"{"kind":"VarDecl","name":"a","storageClass":"static"}"#);
    let local_extern = decl(r#"{"kind":"VarDecl","name":"a","storageClass":"extern"}"#);
    let tls = decl(r#"{"kind":"VarDecl","name":"a","storageClass":"static","tls":"static"}"#);
    assert_eq!(storage_of(&automatic, true), Some(Storage::Automatic));
    assert_eq!(storage_of(&register, true), Some(Storage::Automatic));
    assert_eq!(storage_of(&automatic, false), Some(Storage::Static));
    assert_eq!(storage_of(&local_static, true), Some(Storage::Static));
    assert_eq!(storage_of(&local_extern, true), Some(Storage::Static));
    assert_eq!(storage_of(&tls, true), Some(Storage::ThreadLocal));
    assert_eq!(storage_of(&decl(r#"{"kind":"ParmVarDecl"}"#), true), None);
    assert!(!Storage::Automatic.outlives_frame());
    assert!(Storage::Static.outlives_frame());
    assert!(Storage::ThreadLocal.outlives_frame());
}

#[test]
fn an_unlisted_form_is_refused() {
    assert_eq!(disposition("volatile gadget"), Disposition::Refused);
    assert_eq!(disposition("static declaration"), Disposition::NoEvent);
    assert_eq!(disposition("atomic access"), Disposition::Owned("CG19"));
    for (form, name, row) in FORM_TABLE {
        assert_eq!(disposition(name), row);
        assert_eq!(decide(form), row);
    }
}

#[test]
fn root_name_strips_the_place() {
    assert_eq!(root_name("p"), "p");
    assert_eq!(root_name("p->f"), "p");
    assert_eq!(root_name("*out"), "out");
    assert_eq!(root_name("s.f"), "s");
    assert_eq!(root_name("slots[0]"), "slots");
}
