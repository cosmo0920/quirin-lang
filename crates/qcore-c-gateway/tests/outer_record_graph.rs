//! Card OUTER-CONTRACT (rule 31): the record graph's reach, read from
//! Clang's AST, over-approximates what code holding a binder can reach.

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::outer_record_graph::{Reach, RecordGraph, read_unit};

fn graph(source: &str) -> RecordGraph {
    read_unit(
        source,
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
    )
    .expect("the unit reads")
}

const TYPED: &str = r"
struct link { struct link *next, *prev; };
struct instance;
struct plugin { const char *name; int (*cb_init)(struct instance *); };
struct instance { struct plugin *p; struct link _head; struct { int a; struct counter *c; } stats; };
struct counter { unsigned long n; };
struct config { struct link inputs; };
struct unrelated { int x; };
int load(char *path, struct config *config) { return path != 0 && config != 0; }
";

#[test]
fn typed_pointers_embedded_links_and_containers_are_followed() {
    let graph = graph(TYPED);
    let Reach::Records(tags) = graph.reach_of_parameter("load", 1) else {
        panic!("a typed graph reaches a set");
    };
    // config -> link (embedded) -> instance (container of a link) -> plugin,
    // counter (through the anonymous member); a function pointer adds nothing.
    for tag in ["config", "link", "instance", "plugin", "counter"] {
        assert!(tags.contains(tag), "{tag} in {tags:?}");
    }
    assert!(!tags.contains("unrelated"), "{tags:?}");
    assert_eq!(
        graph.reach_of_parameter("load", 0),
        Reach::Records(Default::default())
    );
}

#[test]
fn a_void_pointer_or_an_undefined_record_reaches_every_record() {
    let with_void = TYPED.replace("struct link inputs;", "struct link inputs; void *context;");
    assert!(matches!(
        graph(&with_void).reach_of_parameter("load", 1),
        Reach::All { .. }
    ));
    let opaque = TYPED.replace("struct counter { unsigned long n; };", "struct counter;");
    let Reach::All { witness } = graph(&opaque).reach_of_parameter("load", 1) else {
        panic!("an undefined record reaches every record");
    };
    assert!(witness.contains("counter"), "{witness}");
    assert!(matches!(
        graph(TYPED).reach_of_parameter("absent", 0),
        Reach::All { .. }
    ));
}
