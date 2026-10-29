//! Source shape imports do not prove ambient list/field stability or custody.
use qcore_c_gateway::collector_identity::*;
use qcore_c_gateway::legacy_c::C11TranslationUnit;
const C: &str = r#"
typedef __SIZE_TYPE__ size_t;
#define offsetof(TYPE,MEMBER) ((size_t)&((TYPE *)0)->MEMBER)
#define container_of(ptr,type,member) ({ const typeof(((type *)0)->member) *__mptr=(ptr); (type *)((char *)__mptr - offsetof(type,member)); })
#define mk_list_entry(ptr,type,member) container_of(ptr,type,member)
#define mk_list_foreach(curr,head) for(curr=(head)->next;curr!=(head);curr=curr->next)
struct mk_list { struct mk_list *prev,*next; };
struct loop { int fd; };
struct worker { struct loop *evl; };
struct config { struct loop *evl; };
struct instance { int is_threaded; struct worker *thi; struct mk_list collectors; };
struct collector { struct loop *evl; struct instance *instance; struct mk_list _head; int id; };
void *alloc(size_t); void opaque(void *);
static inline int threaded(struct instance *ins) { return ins->is_threaded; }
static struct collector *get_collector(int id,struct instance *in) {
    struct mk_list *head;
    struct collector *coll;
    mk_list_foreach(head,&in->collectors) {
        coll=mk_list_entry(head,struct collector,_head);
        if(coll->id==id) { return coll; }
    }
    return ((void *)0);
}
static struct collector *create(int unused,struct instance *ins,struct config *config) {
    struct collector *coll; struct worker *thi;
    coll=alloc(sizeof(struct collector));
    if(!coll) { return ((void *)0); }
    coll->instance=ins;
    if(threaded(ins)) { thi=ins->thi; coll->evl=thi->evl; }
    else { coll->evl=config->evl; }
    opaque(coll);
    return coll;
}
"#;
fn selector() -> SelectorQuery {
    SelectorQuery {
        function: "get_collector".into(),
        record: "collector".into(),
        owner_record: "instance".into(),
        list_record: "mk_list".into(),
        list_member: "collectors".into(),
        link_member: "_head".into(),
        key_member: "id".into(),
    }
}
fn loops() -> LoopQuery {
    LoopQuery {
        function: "create".into(),
        predicate: "threaded".into(),
        record: "collector".into(),
        owner_record: "instance".into(),
        worker_record: "worker".into(),
        config_record: "config".into(),
        loop_record: "loop".into(),
        instance_member: "instance".into(),
        threaded_member: "is_threaded".into(),
        worker_member: "thi".into(),
        loop_member: "evl".into(),
    }
}
fn unit(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        dialect: qcore_c_gateway::legacy_c::CCompilerDialect::Gnu17,
        source,
        ..Default::default()
    }
}
#[test]
fn exact_production_expansion_and_conditional_snapshot_replay() {
    let selected = import_selection(&unit(C), &selector()).unwrap();
    assert_eq!(selected.parameter_roles(), (0, 1));
    replay_selection(&unit(C), &selected).unwrap();
    let conditional = import_conditional_loop(&unit(C), &loops()).unwrap();
    assert_eq!(conditional.parameter_roles(), (1, 2));
    assert_eq!(conditional.record_local(), "coll");
    replay_conditional_loop(&unit(C), &conditional).unwrap();
    // Imports do not certify opaque(coll), constructor lifetime or the false
    // arm's config == ins->config. Their public fields never assert those facts.
    assert!(serde_json::to_value(&conditional).unwrap()["query"]["function"] == "create");
}
#[test]
fn a_matching_id_or_type_cannot_replace_the_exact_list_and_return_shape() {
    for changed in [
        C.replace("coll->id==id", "coll->id!=id"),
        C.replace("return coll;", "return ((void *)0);"),
        C.replace("curr=curr->next", "curr=curr->prev"),
        C.replace("if(coll->id==id)", "opaque(coll); if(coll->id==id)"),
        C.replace(
            "static struct collector *get_collector",
            "struct collector *get_collector",
        ),
        C.replace(
            "coll=mk_list_entry(head,struct collector,_head);",
            "coll=(struct collector *)head;",
        ),
        C.replace(
            "(char *)__mptr - offsetof(type,member)",
            "(char *)__mptr + offsetof(type,member)",
        ),
        C.replace("int id;", "volatile int id;"),
    ] {
        assert!(
            import_selection(&unit(&changed), &selector()).is_err(),
            "unchecked selector admitted: {changed}"
        );
    }
    let mut wrong = selector();
    wrong.list_member = "other_collectors".into();
    assert!(import_selection(&unit(C), &wrong).is_err());
}
#[test]
fn the_condition_must_name_the_stored_instance_and_exact_worker_and_loop() {
    for changed in [
        C.replace("coll->instance=ins;", "coll->instance=((void *)0);"),
        C.replace("if(threaded(ins))", "if(!threaded(ins))"),
        C.replace(
            "thi=ins->thi; coll->evl=thi->evl;",
            "thi=ins->thi; coll->evl=config->evl;",
        ),
        C.replace("coll->evl=config->evl;", "coll->evl=thi->evl;"),
        C.replace(
            "return ins->is_threaded;",
            "ins->is_threaded=0; return ins->is_threaded;",
        ),
        C.replace("static inline int threaded", "int threaded"),
        C.replace("int is_threaded;", "volatile int is_threaded;"),
        C.replace("coll->instance=ins;", "coll->instance=ins; opaque(coll);"),
        C.replace("return coll;\n}", "ins=((void *)0); return coll;\n}"),
    ] {
        assert!(
            import_conditional_loop(&unit(&changed), &loops()).is_err(),
            "unchecked conditional store admitted: {changed}"
        );
    }
}
#[test]
fn replay_binds_the_source_configuration_and_both_function_bodies() {
    let selection = import_selection(&unit(C), &selector()).unwrap();
    let conditional = import_conditional_loop(&unit(C), &loops()).unwrap();
    let changed = format!("{C}\nvoid unrelated(void) {{}}\n");
    assert!(replay_selection(&unit(&changed), &selection).is_err());
    assert!(replay_conditional_loop(&unit(&changed), &conditional).is_err());
    let mut configured = unit(C);
    configured.definitions.push("DIFFERENT=1".into());
    assert!(replay_selection(&configured, &selection).is_err());
    assert!(replay_conditional_loop(&configured, &conditional).is_err());
}

#[test]
fn a_closed_subobject_helper_frame_preserves_the_constructor_relation_only_at_the_join() {
    let helpers = "struct event { int fd; void *data; }; static inline void init(struct event *e,int fd,void *data) { e->fd=fd; e->data=data; } static inline void zero(struct event *e) { init(e,-1,((void *)0)); } ";
    let c = C
        .replace(
            "struct collector {",
            &format!("{helpers} struct collector {{ struct event event;"),
        )
        .replace(
            "coll->instance=ins;",
            "coll->instance=ins; zero(&coll->event);",
        );
    import_conditional_loop(&unit(&c), &loops()).unwrap();
    for changed in [
        c.replace("e->fd=fd;", "opaque(e); e->fd=fd;"),
        c.replace("static inline void zero", "void zero"),
        c.replace("zero(&coll->event);", "opaque(&coll->event);"),
        c.replace(
            "init(e,-1,((void *)0));",
            "init(e,-1,((void *)0)); zero(e);",
        ),
        c.replace("e->fd=fd;", "e->fd=fd; *((int *)e)=0;"),
    ] {
        assert!(import_conditional_loop(&unit(&changed), &loops()).is_err());
    }
}

#[test]
fn the_builtin_offset_form_uses_the_exact_record_and_member() {
    let c = C.replace(
        "((size_t)&((TYPE *)0)->MEMBER)",
        "__builtin_offsetof(TYPE,MEMBER)",
    );
    import_selection(&unit(&c), &selector()).unwrap();
    let bad = c.replace("- offsetof(type,member)", "- offsetof(type,id)");
    assert!(import_selection(&unit(&bad), &selector()).is_err());
}
