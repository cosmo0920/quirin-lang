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

//! Card W48-LIBCURL-CONTRACT-PLUGIN: the plugin's bindings, its checker's
//! obligation shapes, the fail-closed pin, the subcommand run, and the
//! host's classification and checker-binding route as a library.
//!
//! The tests that need the pinned libcurl release itself read it from
//! `QUIRIN_LIBCURL_SOURCE` (the unpacked `curl-8.18.0` directory `quirin-audit
//! libcurl prepare` writes) and are ignored by default: the release is not
//! in the repository. Run them with `--ignored`; without the variable they
//! fail rather than pass vacuously.

// The plugin and checker binaries run as processes with Unix permissions.
#![cfg(unix)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_audit::subcommand_plugins::{classify_claim, join};
use qcore_audit::user_plugins::{PluginCheckers, run as run_bindings};
use qcore_contract::load_path_manifest::load_contract_load_path_manifest;
use qcore_schemas::verdict::{ReasonedVerdict, Verdict};
use quirin_audit_plugin::testing::invoke;
use quirin_audit_plugin::{Outcome, ProposedVerdict, Request};
use quirin_audit_plugin_libcurl::checker::{
    self, Answer, BINDINGS, Kind, Obligation, Tree, binding, is_typecheck_stub,
};
use quirin_audit_plugin_libcurl::{
    MODULE, MODULE_TEXT, claim, contracted_functions, pin, user_load_path, write_user_crate,
};
use serde_json::Value;

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "quirin-libcurl-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

/// The librdkafka census's libcurl rows (card LIBRDKAFKA-HEADER-INLINES).
fn census() -> (BTreeSet<String>, BTreeSet<String>) {
    let text = fs::read_to_string(
        repository().join("docs/reports/librdkafka-header-inlines/census/census.json"),
    )
    .unwrap();
    let census: Value = serde_json::from_str(&text).unwrap();
    let mut real = BTreeSet::new();
    let mut stubs = BTreeSet::new();
    for row in census["rows"].as_array().unwrap() {
        let callee = row["callee"].as_str().unwrap();
        if callee.starts_with("Wcurl_") {
            stubs.insert(callee.to_owned());
        } else if callee.starts_with("curl_") {
            real.insert(callee.to_owned());
        }
    }
    (real, stubs)
}

/// The function names a contract module declares, read lexically.
fn declared(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| line.trim().strip_prefix("extern fn "))
        .map(|rest| rest.split('(').next().unwrap().trim().to_owned())
        .collect()
}

// ## The bindings

#[test]
fn the_ranking_is_curl_global_init_then_the_census_twelve() {
    let (real, stubs) = census();
    assert_eq!(real.len(), 12, "{real:?}");
    assert_eq!(stubs.len(), 44);
    let bound: BTreeSet<String> = BINDINGS.iter().map(|b| b.function.to_owned()).collect();
    assert_eq!(bound, real);
    let ranks: Vec<u32> = BINDINGS.iter().map(|b| b.rank).collect();
    assert_eq!(ranks, (1..=12).collect::<Vec<_>>());
    assert_eq!(BINDINGS[0].function, "curl_global_init");
}

#[test]
fn no_typecheck_stub_is_bound() {
    let (_, stubs) = census();
    for stub in &stubs {
        assert!(is_typecheck_stub(stub), "{stub}");
        assert!(binding(stub).is_none(), "{stub}");
        assert!(!declared(MODULE_TEXT).contains(stub), "{stub}");
    }
    for b in BINDINGS {
        assert!(!is_typecheck_stub(b.function));
    }
}

#[test]
fn every_obligation_reads_a_pinned_file() {
    for b in BINDINGS {
        let obligations = match b.kind {
            Kind::Contracted { obligations, .. } => obligations,
            Kind::Uncontracted { witnesses, .. } => witnesses,
        };
        assert!(!obligations.is_empty(), "{}", b.function);
        for obligation in obligations {
            assert!(
                pin::pinned_digest(obligation.file()).is_some(),
                "{}: {}",
                b.function,
                obligation.describe()
            );
        }
    }
}

#[test]
fn the_module_declares_exactly_the_contracted_functions_with_their_clauses() {
    let declared = declared(MODULE_TEXT);
    let contracted: BTreeSet<String> = contracted_functions()
        .iter()
        .map(|f| (*f).to_owned())
        .collect();
    assert_eq!(declared, contracted);
    for b in BINDINGS {
        let Kind::Contracted { clauses, .. } = b.kind else {
            continue;
        };
        let start = MODULE_TEXT
            .find(&format!("extern fn {}(", b.function))
            .unwrap();
        let end = start + MODULE_TEXT[start..].find("};").unwrap();
        let contract = &MODULE_TEXT[start..end];
        assert!(contract.contains("requires resource_revision(flexible);"));
        let written: Vec<&str> = contract
            .lines()
            .filter_map(|line| line.trim().strip_prefix("ensures normal "))
            .map(|clause| clause.trim_end_matches(';'))
            .collect();
        assert_eq!(written, clauses.to_vec(), "{}", b.function);
    }
}

#[test]
fn the_user_load_path_binds_every_contracted_function_once() {
    let load_path = user_load_path(Path::new("/shipped/contract-load-path.json"));
    let bound: Vec<&str> = load_path["checkers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|checker| {
            assert_eq!(checker["module"], MODULE);
            let functions = checker["functions"].as_array().unwrap();
            assert_eq!(functions.len(), 1);
            functions[0].as_str().unwrap()
        })
        .collect();
    assert_eq!(bound, contracted_functions());
    assert_eq!(load_path["crates"][0]["resource_view"][0], MODULE);
}

// ## The checker's obligation shapes

fn holds(tree: &Tree, obligation: Obligation) -> bool {
    checker::check(tree, &obligation).holds
}

#[test]
fn each_obligation_shape_holds_and_fails_as_written() {
    let tree = Tree::from_texts(&[
        (
            "lib/slist.c",
            "#include <x.h>\n#define keep(x) x\nint f(char *p, int n);\n\
             int f(char *p, int n)\n{\n  if(!p) return -1;\n  g(p);\n  return n;\n}\n\
             int h(void) { return 0; }\nint h2(void) {\n#ifdef X\n return 1;\n#endif\n return 0; }\n",
        ),
        ("include/curl/curl.h", "extern \"C\" {\nint f(char *p, int n);\n}\n"),
    ])
    .unwrap();
    let file = "lib/slist.c";
    let defined = |header| Obligation::Defined {
        file,
        function: "f",
        header,
    };
    assert!(holds(&tree, defined("int f ( char * p , int n )")));
    assert!(!holds(&tree, defined("int f ( const char * p , int n )")));
    // A directive in a body is refused: its tokens are not the compiled ones.
    assert!(!holds(
        &tree,
        Obligation::Defined {
            file,
            function: "h2",
            header: "int h2 ( void )"
        }
    ));
    let declared = |file, declaration| Obligation::Declared {
        file,
        function: "f",
        declaration,
    };
    assert!(holds(&tree, declared(file, "int f ( char * p , int n ) ;")));
    // Inside a C++ linkage specification, a declaration is still top-level.
    assert!(holds(
        &tree,
        declared("include/curl/curl.h", "int f ( char * p , int n ) ;")
    ));
    assert!(!holds(&tree, declared(file, "int f ( char * p ) ;")));
    let contains = |tokens| Obligation::Contains {
        file,
        function: "f",
        tokens,
    };
    assert!(holds(&tree, contains("if ( ! p ) return - 1 ;")));
    assert!(!holds(&tree, contains("return p ;")));
    let only_in = |patterns| Obligation::OnlyIn {
        file,
        function: "f",
        identifier: "p",
        patterns,
    };
    assert!(holds(&tree, only_in(&["if ( ! p )", "g ( p ) ;"])));
    assert!(!holds(&tree, only_in(&["if ( ! p )"])));
    let returns = |expressions| Obligation::Returns {
        file,
        function: "f",
        expressions,
    };
    assert!(holds(&tree, returns(&["- 1", "n"])));
    assert!(!holds(&tree, returns(&["n"])));
    assert!(!holds(
        &tree,
        Obligation::Returns {
            file,
            function: "missing",
            expressions: &["0"],
        }
    ));
    let file_has = |tokens| Obligation::FileHas { file, tokens };
    assert!(holds(&tree, file_has("# define keep ( x ) x")));
    assert!(!holds(&tree, file_has("# define keep ( y ) y")));
    // A file the checker does not read decides nothing.
    assert!(!holds(
        &tree,
        Obligation::FileHas {
            file: "lib/url.c",
            tokens: "x",
        }
    ));
}

/// `curl_url`'s obligations, written to hold, over a tree not checked
/// against the pin: every obligation holds and nothing is established.
#[test]
fn an_unverified_tree_establishes_nothing() {
    let tree = Tree::from_texts(&[
        (
            "lib/urlapi.c",
            "CURLU *curl_url(void)\n{\n  return curlx_calloc(1, sizeof(struct Curl_URL));\n}\n",
        ),
        (
            "include/curl/urlapi.h",
            "CURL_EXTERN CURLU *curl_url(void);\n",
        ),
        (
            "lib/curl_setup.h",
            "#define curlx_calloc(nbelem, size) Curl_ccalloc(nbelem, size)\n",
        ),
        (
            "lib/easy.c",
            "curl_calloc_callback Curl_ccalloc = (curl_calloc_callback)calloc;\n",
        ),
    ])
    .unwrap();
    assert!(!tree.verified());
    let outcome = checker::answer(&tree, binding("curl_url").unwrap());
    assert!(outcome.checked.iter().all(|checked| checked.holds));
    assert_eq!(outcome.answer, Answer::Unknown);
    assert!(outcome.reasons[0].contains("not checked against the pin"));
}

#[test]
fn an_uncontracted_binding_is_unknown_with_its_reason() {
    let tree = Tree::from_texts(&[]).unwrap();
    for function in [
        "curl_easy_setopt",
        "curl_easy_getinfo",
        "curl_easy_perform",
        "curl_easy_cleanup",
        "curl_url_set",
        "curl_url_get",
    ] {
        let b = binding(function).unwrap();
        assert!(matches!(b.kind, Kind::Uncontracted { .. }), "{function}");
        let outcome = checker::answer(&tree, b);
        assert_eq!(outcome.answer, Answer::Unknown);
        assert!(!outcome.contracted);
        assert!(!outcome.reasons[0].is_empty());
    }
}

// ## The pin, fail-closed

fn bogus_tree(root: &Path) {
    for (file, _) in pin::FILES {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"/* not libcurl */\n").unwrap();
    }
}

#[test]
fn a_tree_that_is_not_the_pin_is_refused_file_by_file() {
    let root = scratch("bogus");
    let Err(missing) = Tree::open(&root) else {
        panic!("an empty tree opened")
    };
    assert!(missing.contains("nothing (missing)"), "{missing}");
    bogus_tree(&root);
    assert_eq!(pin::verify_tree(&root).len(), pin::FILES.len());
    let Err(refused) = Tree::open(&root) else {
        panic!("a bogus tree opened")
    };
    for (file, digest) in pin::FILES {
        assert!(refused.contains(file) && refused.contains(digest), "{file}");
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn the_pin_is_well_formed() {
    let hex = |text: &str| text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit());
    assert!(hex(pin::TARBALL_SHA256));
    assert_eq!(pin::TAG_COMMIT.len(), 40);
    let files: Vec<&str> = pin::FILES.iter().map(|(file, _)| *file).collect();
    let unique: BTreeSet<&str> = files.iter().copied().collect();
    assert_eq!(unique.len(), files.len());
    assert!(pin::FILES.iter().all(|(_, digest)| hex(digest)));
}

// ## The subcommand

fn plugin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_quirin-audit-libcurl"))
}

fn request(output: &Path, arguments: &[&str]) -> Request {
    Request::new(
        "libcurl",
        arguments.iter().map(|a| (*a).to_owned()).collect(),
        &repository(),
        output,
    )
}

fn failed(output: &Path, arguments: &[&str]) -> String {
    let invocation = invoke(&plugin(), &request(output, arguments)).unwrap();
    assert_eq!(invocation.status.code(), Some(1), "{}", invocation.stderr);
    let response = invocation.response.unwrap();
    assert!(response.claims.is_empty());
    assert!(response.artifacts.is_empty());
    match response.outcome {
        Outcome::Failed(reason) => reason,
        Outcome::Completed => panic!("completed: {arguments:?}"),
    }
}

#[test]
fn prepare_refuses_a_tarball_that_is_not_the_pin() {
    let output = scratch("prepare");
    let tarball = output.join("curl-8.18.0.tar.gz");
    fs::write(&tarball, b"not the release").unwrap();
    let reason = failed(
        &output,
        &["prepare", "--tarball", tarball.to_str().unwrap()],
    );
    assert!(reason.contains(pin::TARBALL_SHA256), "{reason}");
    assert!(!output.join("libcurl-source").exists());
    assert!(failed(&output, &["prepare"]).contains("--tarball"));
    assert!(failed(&output, &["prepare", "--prepared", "x"]).contains("unknown argument"));
    assert!(failed(&output, &["unpack"]).contains("usage"));
    let _ = fs::remove_dir_all(output);
}

#[test]
fn check_and_install_refuse_a_tree_that_is_not_the_pin() {
    let output = scratch("check");
    let tree = output.join("tree");
    bogus_tree(&tree);
    for step in ["check", "install"] {
        let reason = failed(&output, &[step, "--prepared", tree.to_str().unwrap()]);
        assert!(reason.contains("is not the pinned libcurl"), "{reason}");
    }
    assert!(
        !output
            .join(quirin_audit_plugin_libcurl::CHECK_RECORD)
            .exists()
    );
    assert!(!output.join("user").exists());
    let _ = fs::remove_dir_all(output);
}

// ## The host's classification (C11.2)

fn outcome(function: &'static str, answer: Answer) -> checker::Outcome {
    checker::Outcome {
        function,
        rank: binding(function).unwrap().rank,
        answer,
        contracted: answer == Answer::Safe,
        reasons: vec![format!("{function}: reason")],
        checked: Vec::new(),
    }
}

#[test]
fn the_host_publishes_an_established_binding_pending_and_an_unestablished_one_unknown() {
    let digest = "0".repeat(64);
    let established = claim(&outcome("curl_global_init", Answer::Safe), &digest);
    assert_eq!(established.verdict, ProposedVerdict::Safe);
    assert_eq!(established.artifact, "libcurl:curl_global_init");
    assert_eq!(established.certificates.len(), 1);
    let pending = classify_claim(&established).verdict("libcurl");
    assert_eq!(pending.verdict(), Verdict::Pending);
    assert_eq!(pending.reasons()[0].code, "plugin-claim-uncertified");
    assert!(pending.reasons()[0].detail.contains("QP016"));
    let not = claim(&outcome("curl_easy_setopt", Answer::Unknown), &digest);
    let unknown = classify_claim(&not).verdict("libcurl");
    assert_eq!(unknown.verdict(), Verdict::Unknown);
    // The Unknown keeps the plugin's own reason.
    assert!(
        unknown.reasons()[0]
            .detail
            .contains("curl_easy_setopt: reason")
    );
    // Joined: a host Unsafe row stays Unsafe, a host Safe row becomes
    // Pending, never Safe, and the run's verdict is the join.
    let host = BTreeMap::from([
        (
            "libcurl:curl_global_init".to_owned(),
            ReasonedVerdict::violation("test", "a host violation"),
        ),
        ("libcurl:curl_url".to_owned(), ReasonedVerdict::safe()),
    ]);
    let url =
        classify_claim(&claim(&outcome("curl_url", Answer::Safe), &digest)).verdict("libcurl");
    let (rows, aggregate) = join(
        &host,
        &ReasonedVerdict::safe(),
        &[
            ("libcurl:curl_global_init".to_owned(), pending.clone()),
            ("libcurl:curl_url".to_owned(), url),
            ("libcurl:curl_easy_setopt".to_owned(), unknown),
        ],
    );
    assert_eq!(rows["libcurl:curl_global_init"].verdict(), Verdict::Unsafe);
    assert_eq!(rows["libcurl:curl_url"].verdict(), Verdict::Pending);
    assert_eq!(rows["libcurl:curl_easy_setopt"].verdict(), Verdict::Unknown);
    assert_eq!(aggregate.unwrap().verdict(), Verdict::Unsafe);
}

// ## The checker package (C11.1)

fn checker_executable() -> Vec<u8> {
    fs::read(env!("CARGO_BIN_EXE_quirin-libcurl-checker")).unwrap()
}

fn shipped() -> PathBuf {
    repository().join("providers/contract-load-path.json")
}

/// Every binding of an installed user load path, as the host runs it.
fn bindings(load_path: &Path) -> Value {
    let manifest = load_contract_load_path_manifest(load_path).unwrap();
    run_bindings(manifest.extension.as_ref(), PluginCheckers::default())
        .unwrap()
        .section
}

#[test]
fn a_checker_package_whose_files_are_not_the_pin_answers_unknown_for_every_binding() {
    let user = scratch("package");
    let files: Vec<(&str, Vec<u8>)> = pin::FILES
        .iter()
        .map(|(file, _)| (*file, b"/* not libcurl */\n".to_vec()))
        .collect();
    let load_path = write_user_crate(&user, &checker_executable(), &files, &shipped()).unwrap();
    let section = bindings(&load_path);
    let rows = section["bindings"].as_array().unwrap();
    assert_eq!(rows.len(), contracted_functions().len());
    for row in rows {
        assert_eq!(row["verdict"], "Unknown", "{row}");
        assert_eq!(row["claim"], "undecided", "{row}");
        let detail = row["reasons"][0]["detail"].as_str().unwrap();
        assert!(detail.contains("pin-mismatch"), "{detail}");
    }
    assert_eq!(section["counts"]["Unknown"], rows.len());
    assert_eq!(section["counts"]["Safe"], 0);
    // An existing crate is never overwritten.
    assert!(write_user_crate(&user, &checker_executable(), &files, &shipped()).is_err());
    let _ = fs::remove_dir_all(user);
}

#[test]
fn a_checker_package_whose_executable_changed_is_a_failure() {
    let user = scratch("tampered");
    let load_path = write_user_crate(&user, &checker_executable(), &[], &shipped()).unwrap();
    let executable = user
        .join(quirin_audit_plugin_libcurl::USER_CRATE)
        .join(quirin_audit_plugin_libcurl::PACKAGE)
        .join(quirin_audit_plugin_libcurl::CHECKER_EXECUTABLE);
    let mut bytes = fs::read(&executable).unwrap();
    bytes.push(0);
    fs::write(&executable, bytes).unwrap();
    let section = bindings(&load_path);
    for row in section["bindings"].as_array().unwrap() {
        assert_eq!(row["verdict"], "Unknown", "{row}");
        assert_eq!(row["claim"], "failure", "{row}");
        assert_eq!(row["reasons"][0]["code"], "provider-failure");
    }
    let _ = fs::remove_dir_all(user);
}

// ## The pinned release (ignored: needs QUIRIN_LIBCURL_SOURCE)

fn release() -> PathBuf {
    PathBuf::from(std::env::var_os("QUIRIN_LIBCURL_SOURCE").expect(
        "QUIRIN_LIBCURL_SOURCE names the unpacked curl-8.18.0 directory `quirin-audit libcurl \
         prepare` writes",
    ))
}

fn release_files() -> Vec<(&'static str, Vec<u8>)> {
    let root = release();
    pin::FILES
        .iter()
        .map(|(file, _)| (*file, fs::read(root.join(file)).unwrap()))
        .collect()
}

const ESTABLISHED: [&str; 6] = [
    "curl_global_init",
    "curl_slist_append",
    "curl_slist_free_all",
    "curl_easy_escape",
    "curl_url",
    "curl_url_cleanup",
];

#[test]
#[ignore = "needs the pinned libcurl release: QUIRIN_LIBCURL_SOURCE"]
fn the_pinned_release_establishes_six_bindings_and_leaves_six_unknown() {
    let tree = Tree::open(&release()).unwrap();
    assert!(tree.verified());
    for outcome in checker::answer_all(&tree) {
        let expected = if ESTABLISHED.contains(&outcome.function) {
            Answer::Safe
        } else {
            Answer::Unknown
        };
        assert_eq!(
            outcome.answer, expected,
            "{}: {:?}",
            outcome.function, outcome.reasons
        );
        // Every obligation of an established binding, and every witness of
        // an unestablished one, holds.
        assert!(
            outcome.checked.iter().all(|c| c.holds),
            "{}",
            outcome.function
        );
    }
    let stubs = checker::stub_evidence(&tree);
    let (_, census) = census();
    let defined: BTreeSet<String> = stubs["stubs_defined"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap().to_owned())
        .collect();
    assert!(census.is_subset(&defined));
    assert_eq!(stubs["definition"]["holds"], true);
    assert_eq!(stubs["guard"]["holds"], true);
}

#[test]
#[ignore = "needs the pinned libcurl release: QUIRIN_LIBCURL_SOURCE"]
fn the_checker_package_publishes_each_established_binding_pending() {
    let user = scratch("release-package");
    let load_path =
        write_user_crate(&user, &checker_executable(), &release_files(), &shipped()).unwrap();
    let section = bindings(&load_path);
    for row in section["bindings"].as_array().unwrap() {
        assert_eq!(row["verdict"], "Pending", "{row}");
        assert_eq!(row["claim"], "uncertified-safe");
        assert_eq!(row["code"], "QP008");
        assert_eq!(row["reasons"][0]["code"], "plugin-claim-uncertified");
    }
    assert_eq!(section["counts"]["Pending"], ESTABLISHED.len());
    assert_eq!(section["counts"]["Safe"], 0);
    let _ = fs::remove_dir_all(user);
}

/// W48-PLUGIN-VERDICT-PRESERVATION: on the checker-binding route the
/// published binding of an uncertified `Safe` claim carries the firewall's
/// `QP008` reason, and the checker's own reason (which obligations
/// established the contract) is kept in the binding's `answer` record
/// (contract spec C11.3), so it reaches the published report.
#[test]
#[ignore = "needs QUIRIN_LIBCURL_SOURCE"]
fn the_checkers_reason_for_an_established_binding_reaches_the_published_row() {
    let user = scratch("gap");
    let load_path =
        write_user_crate(&user, &checker_executable(), &release_files(), &shipped()).unwrap();
    let section = bindings(&load_path);
    for row in section["bindings"].as_array().unwrap() {
        assert!(
            row["reasons"].to_string().contains("QP008"),
            "{}",
            row["reasons"]
        );
        let answer = row["answer"]["reasons"].to_string();
        assert!(answer.contains("obligation(s)"), "{answer}");
    }
    let _ = fs::remove_dir_all(user);
}
