use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use qcore_compiler::{
    CompileOptions, SourceContractClauseKind, SourceContractExitSelector, SourceDeclarationKind,
    SourceFile, compile_files, format_diagnostic, parse_source_program,
};
use qcore_contract::load_path_manifest::LoadPathManifest;
use serde_json::{Value, json};

#[path = "support/contract_corpus.rs"]
mod contract_corpus;

// Card CONTRACT-LOCKS (user decision 1 of 2026-10-24): the per-declaration
// clause pins, the inheritance edges, the module count and the premise
// manifest are no longer literal tables in this file. Each load-path crate's
// generated `contract-inventory.lock` (written by `python3
// providers/contracts.py sync`, checked by `check`) holds them, and these
// tests read it. Hard invariant 5 is unchanged: a contract change still needs
// an intentional, reviewed lock update in the same commit, now a diff of data.
// `every_contract_inventory_lock_equals_its_tree` recomputes every lock from
// the tree with this file's own reading (digest, elaborated clause counts,
// inheritance as written, premises, binding pins, views), so a contract file
// changed without its lock, or a lock changed without its file, fails here.
// The literal tables, with each row's provenance comment, are in this file at
// base 6c4855745 and in the cards' reports.
const INVENTORY_LOCK: &str = "contract-inventory.lock";
const INVENTORY_SCHEMA: u64 = 1;
const BINDING_MARKER: &str = "// qcore-premise-binding-manifest-v1 ";

/// Every load-path crate's inventory lock, parsed: crate to document, and
/// logical module to its crate and file record. A logical path in two locks,
/// a missing lock or one naming another crate fails.
struct InventoryLocks {
    crates: BTreeMap<String, Value>,
    files: BTreeMap<String, (String, Value)>,
}

fn inventory_lock_path(load_path: &LoadPathManifest, krate: &str) -> PathBuf {
    load_path.contract_roots[krate]
        .parent()
        .expect("a contract root has a crate directory")
        .join(INVENTORY_LOCK)
}

fn inventory_locks() -> InventoryLocks {
    let load_path = contract_corpus::load_path().expect("the corpus index names a load path");
    let mut crates = BTreeMap::new();
    let mut files = BTreeMap::new();
    for krate in load_path.contract_roots.keys() {
        let path = inventory_lock_path(&load_path, krate);
        let text = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "{}: {error}; run `python3 providers/contracts.py sync`",
                path.display()
            )
        });
        let lock: Value = serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(
            lock["schema_version"],
            INVENTORY_SCHEMA,
            "{}",
            path.display()
        );
        assert_eq!(lock["crate"], krate.as_str(), "{}", path.display());
        for (logical, record) in lock["files"].as_object().expect("a lock lists files") {
            assert!(
                files
                    .insert(logical.clone(), (krate.clone(), record.clone()))
                    .is_none(),
                "{logical} is locked by two crates"
            );
        }
        crates.insert(krate.clone(), lock);
    }
    InventoryLocks { crates, files }
}

fn lock_array<'a>(record: &'a Value, key: &str) -> &'a Vec<Value> {
    record[key]
        .as_array()
        .unwrap_or_else(|| panic!("lock field `{key}` is not a list"))
}

fn lock_str(value: &Value, key: &str) -> String {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("lock field `{key}` is not a string"))
        .to_owned()
}

/// `(child, parent as written)` for every `contract inherit` site of a
/// written module, in source order.
fn written_inherits(logical: &str, written: &str) -> Vec<(String, String)> {
    let syntax = qcore_frontend::contract_inclusion::scan_contract_inclusion(written)
        .unwrap_or_else(|error| panic!("{logical}: {error}"));
    let mut edges = Vec::new();
    for site in &syntax.inherits {
        // The child is the `extern fn` whose signature the `contract`
        // token closes.
        let head = &written[..site.contract_span.start];
        let child = head
            .rfind("extern fn ")
            .map(|start| {
                head[start + "extern fn ".len()..]
                    .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .next()
                    .unwrap()
                    .to_owned()
            })
            .unwrap_or_else(|| panic!("{logical}: an inheriting site names no declaration"));
        let parent = match &site.namespace {
            Some(namespace) => format!("{namespace}::{}", site.parent),
            None => site.parent.clone(),
        };
        edges.push((child, parent));
    }
    edges
}

/// One module's lock record, recomputed from the tree by this test's own
/// reading.
fn tree_record(logical: &str, path: &Path) -> Value {
    let bytes = fs::read(path).expect("contract source is readable");
    let written = String::from_utf8(bytes.clone()).expect("contract source is UTF-8");
    let elaborated = contract_corpus::text(logical, path);
    let parsed =
        parse_source_program(&elaborated).unwrap_or_else(|error| panic!("{logical}: {error}"));
    let declarations = parsed
        .declarations
        .iter()
        .filter(|d| d.value.kind == SourceDeclarationKind::ExternFunction)
        .map(|d| {
            let contract = d
                .value
                .function_contract
                .as_ref()
                .unwrap_or_else(|| panic!("{logical}: {} has no contract", d.value.name));
            json!({"clauses": contract.clauses.len(), "name": d.value.name})
        })
        .collect::<Vec<_>>();
    let clause_count = declarations
        .iter()
        .map(|d| d["clauses"].as_u64().unwrap())
        .sum::<u64>();
    let inherits = written_inherits(logical, &written)
        .into_iter()
        .map(|(child, parent)| json!({"child": child, "parent": parent}))
        .collect::<Vec<_>>();
    let premises = qcore_contract::contract_loader::module_premises(&elaborated)
        .unwrap_or_else(|error| panic!("{logical}: {error}"))
        .into_iter()
        .map(|(function, list)| {
            let list = list
                .iter()
                .map(|p| json!({"name": p.name, "origin": p.origin.as_str()}))
                .collect::<Vec<_>>();
            (function, Value::from(list))
        })
        .collect::<serde_json::Map<_, _>>();
    let bindings = written
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix(BINDING_MARKER))
        .map(|rest| {
            let fields = rest.split_whitespace().collect::<Vec<_>>();
            let [declaration, premise, sha256] = fields[..] else {
                panic!("{logical}: malformed premise binding pin `{rest}`");
            };
            json!({"declaration": declaration, "premise": premise, "sha256": sha256})
        })
        .collect::<Vec<_>>();
    json!({
        "clause_count": clause_count,
        "declaration_count": declarations.len(),
        "declarations": declarations,
        "inherits": inherits,
        "premise_bindings": bindings,
        "premises": premises,
        "sha256": qcore_schemas::digest::sha256_hex(&bytes),
    })
}

/// Card CONTRACT-LOCKS: tree equals lock, per crate. Every tracked contract
/// file under a load-path crate's root is in that crate's lock with this
/// test's own reading of it, and the lock holds nothing else; the views are
/// the load path's. Byte form too, so the committed lock is the one `sync`
/// writes. Regenerate with `python3 providers/contracts.py sync` after
/// reviewing the contract change.
#[test]
fn every_contract_inventory_lock_equals_its_tree() {
    let load_path = contract_corpus::load_path().expect("the corpus index names a load path");
    let locks = inventory_locks();
    let mut trees: BTreeMap<String, serde_json::Map<String, Value>> = load_path
        .contract_roots
        .keys()
        .map(|krate| (krate.clone(), serde_json::Map::new()))
        .collect();
    for (logical, path) in contract_corpus::sources() {
        let krate = contract_corpus::owning_crate(&load_path, &logical, &path)
            .unwrap_or_else(|| panic!("{logical}: corpus module outside every load-path crate"));
        trees
            .get_mut(&krate)
            .unwrap()
            .insert(logical.clone(), tree_record(&logical, &path));
    }
    let view = |views: &BTreeMap<String, Vec<String>>, krate: &str| {
        Value::from(views.get(krate).cloned().unwrap_or_default())
    };
    for (krate, files) in trees {
        let expected = json!({
            "bounds_view": view(&load_path.bounds_views, &krate),
            "contracts": format!("{krate}/contracts"),
            "crate": krate,
            "files": files,
            "outer_view": view(&load_path.outer_views, &krate),
            "regenerate": "python3 providers/contracts.py sync",
            "resource_view": view(&load_path.resource_views, &krate),
            "schema_version": INVENTORY_SCHEMA,
        });
        let lock = &locks.crates[&krate];
        if *lock != expected {
            let old = lock["files"].as_object().cloned().unwrap_or_default();
            let new = expected["files"].as_object().unwrap();
            let mut reasons = Vec::new();
            for name in old.keys().chain(new.keys()).collect::<BTreeSet<_>>() {
                if old.get(name) != new.get(name) {
                    reasons.push(name.clone());
                }
            }
            panic!(
                "{krate}: {INVENTORY_LOCK} differs from the tree (modules {reasons:?}); review \
                 the contract change, then run `python3 providers/contracts.py sync`"
            );
        }
        let path = inventory_lock_path(&load_path, &krate);
        let canonical = format!("{}\n", serde_json::to_string_pretty(&expected).unwrap());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            canonical,
            "{}: not the byte form `providers/contracts.py sync` writes",
            path.display()
        );
    }
}

#[test]
fn every_qcore_contract_source_parses_round_trips_and_compiles() {
    let sources = contract_corpus::sources();
    // Card CONTRACT-LOCKS: the module set is the union of the inventory
    // locks (138 modules at base 6c4855745; the history of that literal, 60 ->
    // 138, is in this file at that revision).
    assert_eq!(
        sources.keys().cloned().collect::<BTreeSet<_>>(),
        inventory_locks().files.into_keys().collect::<BTreeSet<_>>(),
        "the contract corpus differs from the inventory locks; run `python3 \
         providers/contracts.py sync`"
    );

    for (logical, path) in &sources {
        let written = fs::read_to_string(path).expect("contract source is readable");
        assert!(
            !written.contains(qcore_contract::contract_loader::RETIRED_PREMISE_DIRECTIVE),
            "{}: the retired premise directive must not reappear",
            path.display()
        );
        // Card CONTRACT-INHERIT-ADOPTION: a module that inherits is checked as
        // its elaboration, the text every loader parses and retains.
        let source = contract_corpus::text(logical, path);
        // Card HASH-CLAUSE. The `qcore-premise` comment directive is retired in
        // favour of §C04.5's `resource_literal_argument` clause. The resource
        // loader rejects a module carrying it, but only the modules the
        // manifest selects ever reach that loader; this sweeps the whole corpus,
        // so a directive cannot rot unread in an unselected module either.
        assert!(
            !source.contains(qcore_contract::contract_loader::RETIRED_PREMISE_DIRECTIVE),
            "{}: the retired premise directive must not reappear; declare \
             `requires resource_literal_argument(PARAMETER, LITERAL);`",
            path.display()
        );
        let parsed = parse_source_program(&source)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let reparsed = parse_source_program(&parsed.to_string())
            .unwrap_or_else(|error| panic!("{} failed to round trip: {error}", path.display()));
        assert!(parsed.syntax_equivalent(&reparsed), "{}", path.display());

        let module = compile_files(
            &[SourceFile::new(path.display().to_string(), source)],
            &CompileOptions::default(),
        )
        .unwrap_or_else(|diagnostics| {
            panic!(
                "{} failed compiler regression:\n{}",
                path.display(),
                diagnostics
                    .iter()
                    .map(format_diagnostic)
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        });
        assert_eq!(
            module.mode,
            qcore_compiler::frontend::SourceModuleMode::Contract
        );
        assert!(module.entry.is_none(), "{}", path.display());
        assert!(module.c_source.is_none(), "{}", path.display());
    }
}

#[test]
fn contract_surface_is_pinned_as_structured_clauses() {
    let mut actual = Vec::new();
    let mut all_expressions = Vec::new();
    let mut has_normal = false;
    let mut has_error = false;
    let mut has_any = false;

    for (relative, path) in contract_corpus::sources() {
        // Card CONTRACT-INHERIT-ADOPTION: the pins count **elaborated** clauses
        // (what the loader loads and the environment retains), so converting a
        // hand-written declaration to `contract inherit` leaves its pin
        // unchanged; the written form is pinned separately by each lock's
        // `inherits`.
        let source = contract_corpus::text(&relative, &path);
        let parsed = parse_source_program(&source).expect("contract source parses");
        for declaration in &parsed.declarations {
            if declaration.value.kind == SourceDeclarationKind::ExternFunction {
                let contract = declaration
                    .value
                    .function_contract
                    .as_ref()
                    .unwrap_or_else(|| {
                        panic!("{relative}: {} has no contract", declaration.value.name)
                    });
                actual.push((
                    relative.clone(),
                    declaration.value.name.clone(),
                    contract.clauses.len(),
                ));
                for clause in &contract.clauses {
                    assert!(!clause.value.expression_source.is_empty());
                    all_expressions.push(clause.value.expression_source.clone());
                    match &clause.value.kind {
                        SourceContractClauseKind::Requires
                        | SourceContractClauseKind::QuantCheck
                        | SourceContractClauseKind::QuantNullDeref
                        | SourceContractClauseKind::QuantDeref
                        | SourceContractClauseKind::QuantLive
                        | SourceContractClauseKind::QuantOuter
                        | SourceContractClauseKind::QuantConditionBinding
                        | SourceContractClauseKind::QuantConditionBlock { .. }
                        | SourceContractClauseKind::QuantRegistered
                        | SourceContractClauseKind::QuantCell => {}
                        SourceContractClauseKind::QuantCondition(selector) => {
                            qcore_frontend::quant_condition::parse(
                                selector.clone(),
                                &clause.value.expression_source,
                            )
                            .unwrap();
                        }
                        SourceContractClauseKind::QuantProposition => {
                            qcore_frontend::quant_proposition::parse(
                                &clause.value.expression_source,
                            )
                            .unwrap();
                        }
                        SourceContractClauseKind::QuantTheorem => {
                            qcore_frontend::quant_theorem::parse(&clause.value.expression_source)
                                .unwrap();
                        }
                        SourceContractClauseKind::QuantEffect(selector) => {
                            qcore_frontend::quant_effect::parse(&clause.value.expression_source)
                                .unwrap();
                            match selector {
                                SourceContractExitSelector::Normal => has_normal = true,
                                SourceContractExitSelector::Error(_) => has_error = true,
                                SourceContractExitSelector::Any => has_any = true,
                            }
                        }
                        SourceContractClauseKind::Ensures(selector)
                        | SourceContractClauseKind::Quant(selector) => match selector {
                            SourceContractExitSelector::Normal => has_normal = true,
                            SourceContractExitSelector::Error(_) => has_error = true,
                            SourceContractExitSelector::Any => has_any = true,
                        },
                    }
                }
            }
        }
    }

    // Card CONTRACT-LOCKS: `(module, declaration, elaborated clauses)` in
    // logical-path then source order, read from the inventory locks.
    let expected = inventory_locks()
        .files
        .iter()
        .flat_map(|(module, (_, record))| {
            lock_array(record, "declarations").iter().map(|d| {
                (
                    module.clone(),
                    lock_str(d, "name"),
                    usize::try_from(d["clauses"].as_u64().expect("a clause count")).unwrap(),
                )
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert!(has_normal && has_error && has_any);

    for required in [
        // Card O03-U5: the corrected `evutil_closesocket` error arm. The
        // superseded `descriptor_custody_is_preserved` clause on that
        // declaration is refused below.
        "descriptor_custody_is_unresolved",
        "descriptor_number_must_not_be_reused",
        "no_close_retry_is_permitted",
        "close_error_is_read_on_the_platform_channel",
        "data_durability_is_not_discharged",
        "handoff_has_synchronization_evidence",
        "every_cross_context_transition_has_typed_evidence",
        "delete_only_is_not_object_destruction",
        "nested_teardown_order_is_context_stream_chunk_backend",
        "callback_summaries_are_exact",
        "observed_trace_refines_checked_trace",
        "no_event_certificate_is_issued",
    ] {
        assert!(
            all_expressions
                .iter()
                .any(|expression| expression == required),
            "missing pinned contract predicate `{required}`"
        );
    }
}

/// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (USER DECISION 2026-10-15, "Add
/// premise support"): every corpus module's named premises read through the
/// contract format, whatever its vocabulary, and the Quirin model contract
/// `rd_kafka_q_destroy_final` (outside every resource view, in no lock) names
/// P-RK-PLUGIN-CALLBACK-CLOSURE.
#[test]
fn every_module_reads_its_premises_and_the_model_contract_names_the_plugin_premise() {
    let mut premised = std::collections::BTreeMap::new();
    for (logical, path) in contract_corpus::sources() {
        let source = contract_corpus::text(&logical, &path);
        let premises = qcore_contract::contract_loader::module_premises(&source)
            .unwrap_or_else(|error| panic!("{logical}: {error}"));
        for (function, list) in premises {
            premised.insert((logical.clone(), function), list);
        }
    }
    let model = &premised[&(
        "librdkafka/queue_destructor.qv".to_owned(),
        "rd_kafka_q_destroy_final".to_owned(),
    )];
    assert_eq!(
        model.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        ["rk_plugin_callback_closure"]
    );
    for function in ["rd_kafka_crash", "rd_kafka_dump0"] {
        let module = if function == "rd_kafka_crash" {
            "librdkafka/memory.qv"
        } else {
            "librdkafka/kafka_mem.qv"
        };
        assert!(
            premised[&(module.to_owned(), function.to_owned())]
                .iter()
                .any(|p| p.name == "rk_plugin_callback_closure"),
            "{function}"
        );
    }
    // Card CONTRACT-LOCKS: the whole premise manifest equals the inventory
    // locks', function by function, names and origins in order. At base
    // 6c4855745 the lock gives `librdkafka/null_deref.qv` 71 premised
    // declarations: the 67 stable/scope member declarations of the null view
    // and, since card NONNULL-MEMBER-UNIFIED-QUALIFIER ("Yes, all kinds"), its
    // four plain member declarations.
    let read = premised
        .iter()
        .map(|(key, list)| {
            let list = list
                .iter()
                .map(|p| (p.name.clone(), p.origin.as_str().to_owned()))
                .collect::<Vec<_>>();
            (key.clone(), list)
        })
        .collect::<BTreeMap<_, _>>();
    let locked = inventory_locks()
        .files
        .iter()
        .flat_map(|(module, (_, record))| {
            record["premises"]
                .as_object()
                .expect("a lock's premises are a map")
                .iter()
                .map(|(function, list)| {
                    let list = list
                        .as_array()
                        .expect("a premise list")
                        .iter()
                        .map(|p| (lock_str(p, "name"), lock_str(p, "origin")))
                        .collect::<Vec<_>>();
                    ((module.clone(), function.clone()), list)
                })
                .collect::<Vec<_>>()
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(read, locked);
    assert!(
        premised
            .keys()
            .any(|(module, _)| module == "librdkafka/null_deref.qv")
    );
    for function in [
        "rd_kafka_toppar_new0",
        "rd_kafka_topic_new0",
        "rd_kafka_cgrp_new",
        "rd_kafka_transport_new",
    ] {
        assert!(
            premised[&("librdkafka/null_deref.qv".to_owned(), function.to_owned())]
                .iter()
                .any(|p| p.name == "rk_plugin_callback_closure"),
            "{function}"
        );
    }
    // Fluent Bit's member declarations name none (out-of-tree plugins are
    // OUTER-CONTRACT-DESIGN's).
    assert!(
        !premised
            .keys()
            .any(|(module, _)| module == "fluent_bit/null_deref.qv")
    );
}

/// Card O03-U5. `evutil_closesocket` is `close(sock)` on every non-Windows
/// build and `closesocket(sock)` on Windows, with no error translation
/// (`third_party/libevent/evutil.c:613-621` at the pinned revision). Its
/// declared error arm therefore may not assert preserved custody: on the
/// audited build a failed close has already released the descriptor except on
/// `EBADF`, so "custody is preserved" prescribes the retry the platform
/// documents as a hazard. This pins the corrected arm and refuses the old one
/// by name, on that declaration only — `evutil_socket_geterror` keeps its own
/// `descriptor_custody_is_preserved` clause, which is a borrow and is correct.
#[test]
fn the_closesocket_error_arm_states_an_unresolved_endpoint() {
    let path = contract_corpus::sources()
        .remove("monkey/composition.qv")
        .unwrap();
    let source = fs::read_to_string(&path).expect("contract source is readable");
    let parsed = parse_source_program(&source).expect("contract source parses");

    let mut checked = 0;
    for declaration in &parsed.declarations {
        if declaration.value.name != "evutil_closesocket" {
            continue;
        }
        checked += 1;
        let contract = declaration
            .value
            .function_contract
            .as_ref()
            .expect("evutil_closesocket has a contract");
        let error_arm = contract
            .clauses
            .iter()
            .filter(|clause| {
                matches!(
                    &clause.value.kind,
                    SourceContractClauseKind::Ensures(SourceContractExitSelector::Error(_))
                        | SourceContractClauseKind::Quant(SourceContractExitSelector::Error(_))
                )
            })
            .map(|clause| clause.value.expression_source.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            error_arm,
            vec![
                "descriptor_custody_is_unresolved".to_owned(),
                "descriptor_number_must_not_be_reused".to_owned(),
                "no_close_retry_is_permitted".to_owned(),
                "close_error_is_read_on_the_platform_channel".to_owned(),
            ],
        );
        assert!(
            !contract
                .clauses
                .iter()
                .any(|clause| clause.value.expression_source == "descriptor_custody_is_preserved"),
            "the withdrawn returns-on-failure clause is back on evutil_closesocket"
        );
        assert!(
            contract.clauses.iter().any(|clause| {
                clause.value.expression_source == "data_durability_is_not_discharged"
                    && matches!(
                        &clause.value.kind,
                        SourceContractClauseKind::Ensures(SourceContractExitSelector::Any)
                            | SourceContractClauseKind::Quant(SourceContractExitSelector::Any)
                    )
            }),
            "durability must stay separated from descriptor custody on every exit"
        );
    }
    assert_eq!(
        checked, 1,
        "expected exactly one evutil_closesocket declaration"
    );
}

/// Card CONTRACT-INHERIT-ADOPTION: every declaration in the corpus whose
/// contract is written `contract inherit NS::NAME;`, as
/// `(logical module, child, parent as written)`, in logical-path then source
/// order. The clause pins count each child's **elaborated** clauses; this pins
/// the written form, so a conversion to or from inheritance is a deliberate
/// lock edit even though it moves no clause pin. Card CONTRACT-LOCKS: the
/// expected edges are each inventory lock's `inherits` (11 at base 6c4855745:
/// `fluent_bit/resource_memory.qv` and `resource_string.qv` over libc,
/// `librdkafka/memory.qv` and `monkey/memory.qv` over libc).
#[test]
fn inherited_contracts_are_pinned_in_their_written_form() {
    let mut actual = Vec::new();
    for (logical, path) in contract_corpus::sources() {
        let written = fs::read_to_string(&path).expect("contract source is readable");
        let syntax = qcore_frontend::contract_inclusion::scan_contract_inclusion(&written)
            .unwrap_or_else(|error| panic!("{logical}: {error}"));
        if syntax.is_empty() {
            continue;
        }
        // The module must elaborate (and so load) at all.
        let elaborated = contract_corpus::text(&logical, &path);
        parse_source_program(&elaborated).unwrap();
        for (child, parent) in written_inherits(&logical, &written) {
            actual.push((logical.clone(), child, parent));
        }
    }
    let expected = inventory_locks()
        .files
        .iter()
        .flat_map(|(module, (_, record))| {
            lock_array(record, "inherits").iter().map(|edge| {
                (
                    module.clone(),
                    lock_str(edge, "child"),
                    lock_str(edge, "parent"),
                )
            })
        })
        .collect::<Vec<_>>();
    assert!(!expected.is_empty(), "the locks record no inheritance edge");
    assert_eq!(actual, expected);
}
