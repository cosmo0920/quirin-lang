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

//! Source shape imports do not prove ambient list/field stability or custody.
use qcore_c_gateway::collector_identity::*;
use qcore_c_gateway::legacy_c::C11TranslationUnit;
const C: &str = include_str!("fixtures/collector_identity.c");
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

#[test]
fn a_shadowed_tag_or_function_pointer_cannot_borrow_another_definitions_authority() {
    let shadow = C.replace(
        "struct collector *coll; struct worker *thi;",
        "struct collector *coll; struct worker *thi; int (*threaded)(struct instance *)=0;",
    );
    assert!(import_conditional_loop(&unit(&shadow), &loops()).is_err());
    let local_tag=C.replace("struct collector *coll; struct worker *thi;", "struct collector { struct instance *instance; struct loop *evl; }; struct collector *coll; struct worker *thi;");
    assert!(import_conditional_loop(&unit(&local_tag), &loops()).is_err());
}

#[test]
fn increments_compound_writes_and_binding_address_escapes_refuse_entry_correspondence() {
    for statement in [
        "ins++;",
        "ins+=1;",
        "opaque(&ins);",
        "opaque(&config);",
        "opaque(&thi);",
    ] {
        let changed = C.replace(
            "coll->instance=ins;",
            &format!("{statement} coll->instance=ins;"),
        );
        assert!(
            import_conditional_loop(&unit(&changed), &loops()).is_err(),
            "{statement}"
        );
    }
}
