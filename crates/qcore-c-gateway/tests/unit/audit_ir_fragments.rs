use super::*;
use crate::audit_ir_capture::{CaptureIndex, with_capture};
use crate::legacy_c::{ResourceContracts, build_function_cfg, tokenize};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "qcore-ir-fragment-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn store(&self) -> FragmentStore {
        FragmentStore::new(&self.0, MAX_OBJECT_BYTES).unwrap()
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fresh() -> LegacyCfg {
    build_function_cfg(
        &tokenize("void subject(void) { return; }"),
        "subject",
        &ResourceContracts::default(),
    )
    .unwrap()
}
fn context(store: &FragmentStore) -> FragmentContext {
    FragmentContext {
        unit: store
            .write_object(&serde_json::json!({"format": FORMAT, "requires_fresh_lowering":true}))
            .unwrap(),
        construction: store
            .write_object(&ConstructionContext {
                format: FORMAT.into(),
                function: "subject".into(),
                tokens: store.write_object(&Vec::<String>::new()).unwrap(),
                contracts: store.write_object(&ResourceContracts::default()).unwrap(),
                admit_derived_returns: false,
                own: None,
            })
            .unwrap(),
    }
}

fn path(store: &FragmentStore, id: &ObjectId) -> PathBuf {
    store
        .root()
        .join("objects")
        .join(format!("{}.qc", id.as_str()))
}

#[test]
fn typed_round_trip_requires_independent_context_and_fresh_body() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let context = context(&store);
    let cfg = fresh();
    let id = store.write_cfg(&context, &cfg).unwrap();
    assert_eq!(store.read_cfg(&id, &context, &cfg).unwrap(), cfg);
    let changed = FragmentContext {
        unit: store
            .write_object(&"other build/source/contracts/profile")
            .unwrap(),
        ..context.clone()
    };
    assert!(
        store
            .read_cfg(&id, &changed, &cfg)
            .unwrap_err()
            .contains("context mismatch")
    );
    let mut other_body = cfg.clone();
    other_body.nodes[0].line += 1;
    assert!(
        store
            .read_cfg(&id, &context, &other_body)
            .unwrap_err()
            .contains("fresh source")
    );
    let unsupported = store
        .write_object(&CfgObject {
            format: "future-format".into(),
            requires_fresh_lowering: true,
            context: context.clone(),
            cfg: cfg.clone(),
        })
        .unwrap();
    assert!(store.read_cfg(&unsupported, &context, &cfg).is_err());
    let forbidden = store
        .write_object(&CfgObject {
            format: FORMAT.into(),
            requires_fresh_lowering: false,
            context: context.clone(),
            cfg: cfg.clone(),
        })
        .unwrap();
    assert!(store.read_cfg(&forbidden, &context, &cfg).is_err());
}

#[test]
fn corrupt_partial_oversized_and_lossy_objects_are_rejected() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let cfg = fresh();
    let context = context(&store);
    let id = store.write_cfg(&context, &cfg).unwrap();
    let original = fs::read(path(&store, &id)).unwrap();
    fs::write(path(&store, &id), &original[..original.len() / 2]).unwrap();
    assert!(store.read_cfg(&id, &context, &cfg).is_err());
    assert!(
        store.write_cfg(&context, &cfg).is_err(),
        "never overwrite corrupt existing objects"
    );
    assert_eq!(
        fs::read(path(&store, &id)).unwrap(),
        original[..original.len() / 2]
    );
    let limited = FragmentStore::new(&scratch.0, 32).unwrap();
    assert!(limited.write_object(&"x".repeat(64)).is_err());
    assert!(limited.read_object::<CfgObject>(&id).is_err());
    assert!(ObjectId::try_from("../source.c".to_owned()).is_err());
    assert!(FragmentStore::new(&scratch.0, 0).is_err());
    assert!(FragmentStore::new(&scratch.0, MAX_OBJECT_BYTES + 1).is_err());
    // Unknown fields in a nested CFG must not be ignored on typed readback.
    let mut forged = serde_json::to_value(CfgObject {
        format: FORMAT.into(),
        requires_fresh_lowering: true,
        context: context.clone(),
        cfg: cfg.clone(),
    })
    .unwrap();
    forged["cfg"]["lost_field"] = true.into();
    let forged_id = store.write_object(&forged).unwrap();
    assert!(store.read_cfg(&forged_id, &context, &cfg).is_err());
}

#[test]
fn recursive_instructions_are_bounded_before_serialization() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let context = context(&store);
    let mut cfg = fresh();
    let mut instruction = LegacyInstruction::NoOp;
    for _ in 1..MAX_INSTRUCTION_DEPTH {
        instruction = LegacyInstruction::ScalarMemberStore {
            typed_alternative: Some(Box::new(instruction)),
            base: "p".into(),
            alias_read: None,
        };
    }
    cfg.nodes[0].instruction = instruction;
    let id = store.write_cfg(&context, &cfg).unwrap();
    assert_eq!(store.read_cfg(&id, &context, &cfg).unwrap(), cfg);
    cfg.nodes[0].instruction = LegacyInstruction::ScalarMemberStore {
        typed_alternative: Some(Box::new(cfg.nodes[0].instruction.clone())),
        base: "p".into(),
        alias_read: None,
    };
    assert!(
        store
            .write_cfg(&context, &cfg)
            .unwrap_err()
            .contains("nesting")
    );
}

#[test]
fn missing_construction_dependencies_refuse_even_with_a_fresh_equal_cfg() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let context = context(&store);
    let cfg = fresh();
    let id = store.write_cfg(&context, &cfg).unwrap();
    let inputs: ConstructionContext = store.read_object(&context.construction).unwrap();
    fs::remove_file(path(&store, &inputs.tokens)).unwrap();
    assert!(store.read_cfg(&id, &context, &cfg).is_err());
}

#[test]
fn concurrent_publication_is_atomic_and_content_deduplicated() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let context = context(&store);
    let cfg = fresh();
    let ids = std::thread::scope(|scope| {
        let handles = (0..8)
            .map(|_| scope.spawn(|| store.write_cfg(&context, &cfg).unwrap()))
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert!(ids.iter().all(|id| id == &ids[0]));
    assert_eq!(store.read_cfg(&ids[0], &context, &cfg).unwrap(), cfg);
    // This walk is confined to the temporary directory this test owns.
    assert!(
        fs::read_dir(store.root().join("objects"))
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("tmp-"))
    );
}

#[test]
fn capture_preserves_analysis_and_records_missing_targets_and_refusals() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let source =
        "void external(void); void subject(void) { return; } void undecided(void) { external(); }";
    let contracts = ResourceContracts::default();
    let baseline =
        crate::legacy_c::analyze_legacy_c_translation_unit(source, "subject", &contracts).unwrap();
    let targets = ["subject", "undecided", "missing", "not_attempted"].map(str::to_owned);
    let (captured, receipt) = with_capture(store.clone(), &source, &targets, || {
        assert!(build_function_cfg(&tokenize(source), "missing", &contracts).is_err());
        crate::legacy_c::analyze_legacy_c_translation_unit(source, "subject", &contracts)
            .map_err(|e| e.to_string())
    })
    .unwrap();
    assert_eq!(captured, baseline);
    let index: CaptureIndex = store.read_object(&receipt.index).unwrap();
    assert_eq!(index.unit, receipt.unit);
    assert!(index.requires_fresh_lowering);
    assert_eq!(
        index
            .targets
            .iter()
            .map(|t| &t.function)
            .collect::<Vec<_>>(),
        targets.iter().collect::<Vec<_>>()
    );
    let missing = index
        .targets
        .iter()
        .find(|t| t.function == "missing")
        .unwrap();
    assert!(missing.cfgs.is_empty() && !missing.refusals.is_empty() && !missing.not_captured);
    assert!(
        index
            .targets
            .iter()
            .find(|t| t.function == "not_attempted")
            .unwrap()
            .not_captured
    );
    assert!(
        !index
            .targets
            .iter()
            .find(|t| t.function == "undecided")
            .unwrap()
            .cfgs
            .is_empty()
    );
}

#[test]
fn capture_failure_and_panic_never_publish_completion_or_leak_scope() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let failed = with_capture(store.clone(), &"unit", &["subject".into()], || {
        fs::remove_dir_all(store.root().join("objects")).unwrap();
        fs::write(store.root().join("objects"), b"blocked").unwrap();
        assert!(
            build_function_cfg(
                &tokenize("void subject(void) {}"),
                "subject",
                &ResourceContracts::default()
            )
            .is_ok()
        );
        Ok(())
    });
    assert!(failed.is_err());
    assert!(!store.root().join("units").exists());
    fs::remove_file(store.root().join("objects")).unwrap();
    let panic = std::panic::catch_unwind(|| {
        let _: Result<((), _), _> = with_capture(store.clone(), &"unit", &[], || {
            panic!("capture interruption")
        });
    });
    assert!(panic.is_err());
    assert!(!store.root().join("units").exists());
    assert!(with_capture(store.clone(), &"unit", &[], || Ok(())).is_ok());
    assert!(
        with_capture(store.clone(), &"outer", &[], || {
            assert!(with_capture(store.clone(), &"inner", &[], || Ok(())).is_err());
            Ok(())
        })
        .is_ok()
    );
}

#[test]
fn pointer_store_static_labels_decode_without_leaking_or_accepting_unknown_labels() {
    use crate::pointer_store::PointerStore;
    for label in [
        "null",
        "string",
        "function",
        "function-pointer-name",
        "self-address",
        "interior-address",
        "formal-pointer",
        "local-pointer",
        "pointer-read",
        "call",
        "arithmetic",
        "other-address",
        "other",
    ] {
        let value = PointerStore {
            root: "p".into(),
            place: "p->f".into(),
            owner: None,
            value: label,
            name: None,
        };
        let encoded = serde_json::to_value(&value).unwrap();
        let decoded: PointerStore = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded, value);
    }
    assert!(serde_json::from_value::<PointerStore>(serde_json::json!({"root":"p","place":"p->f","owner":null,"value":"new-class","name":null})).is_err());
}

#[test]
fn full_cfg_round_trip_retains_lifetime_and_refused_escape_metadata() {
    use crate::legacy_c::retained_by_route::Operands;
    use qcore_schemas::loop_lifetime::{
        ForwardedLifetime, Path as LifetimePath, Registration, Root,
    };
    use std::collections::BTreeMap;
    let scratch = Scratch::new();
    let store = scratch.store();
    let context = context(&store);
    let mut cfg = fresh();
    let lifetime_path = LifetimePath {
        root: Root::Formal {
            scope: "subject".into(),
            index: 0,
        },
        fields: vec!["holder".into()],
    };
    let retention = Operands {
        retentions: vec![],
        forwarded: BTreeMap::from([(
            0,
            ForwardedLifetime {
                host: "subject".into(),
                premise: "registered_lifetime".into(),
                registration: Registration {
                    object: lifetime_path.clone(),
                    holder: lifetime_path,
                    poisoned: true,
                },
            },
        )]),
        resets: vec![],
        slots: vec![],
        references: vec![],
        callbacks: None,
        removals: vec![],
        cells: None,
    };
    cfg.nodes[0].instruction = LegacyInstruction::ContractCall {
        function: "callee".into(),
        destination: None,
        arguments: vec![Some("p".into())],
        contract: Default::default(),
        produced_sites: vec![],
        aggregate_reads: vec![],
        aggregate_consumed: vec![],
        aggregate_addresses: vec![],
        alias_reads: vec![],
        nested: vec![],
        container: None,
        retention: Some(Box::new(retention)),
    };
    cfg.publications.line_reads.insert(
        7,
        serde_json::from_value(serde_json::json!({
            "condition":{"p":"Call"},"statement":{"p":"Escape"}
        }))
        .unwrap(),
    );
    cfg.conditional_names.insert("conditional".into());
    cfg.tie_names.insert("tie".into());
    let id = store.write_cfg(&context, &cfg).unwrap();
    let decoded = store.read_cfg(&id, &context, &cfg).unwrap();
    assert_eq!(decoded, cfg);
    let LegacyInstruction::ContractCall {
        retention: Some(retention),
        ..
    } = &decoded.nodes[0].instruction
    else {
        panic!("retention lost")
    };
    assert!(retention.forwarded[&0].registration.poisoned);
    assert!(decoded.publications.line_reads.contains_key(&7));
}
