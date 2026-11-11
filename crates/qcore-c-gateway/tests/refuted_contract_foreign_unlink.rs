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

//! W45-REFUTED-CONTRACT-WITHDRAWAL composed with W45-CONTAINER-ROOT-REACH's
//! summary gate (`docs/reports/w45-refuted-contract-withdrawal/README.md`).
//!
//! A body that takes custody of an element of a container its caller supplied
//! (a foreign unlink) publishes no W4 summary. So the custody-duty arm of the
//! refutation rule has nothing to compare: such a body neither establishes nor
//! refutes a custody declaration, and the declaration stays *unchecked* and
//! applied, exactly as for any body that does not validate. The prelude is
//! W45-CONTAINER-ROOT-REACH's, so the fixture reaches the gateway in the
//! preprocessed form a production unit does.

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, CCompilerDialect, LegacyDiagnosticKind,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    refuted_contract,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = r#"
#include <stddef.h>
#define container_of(ptr, type, member) ({                      \
      const typeof( ((type *)0)->member ) *__mptr = (ptr);      \
      (type *)( (char *)__mptr - offsetof(type,member) );})
struct mk_list { struct mk_list *prev, *next; };
#define mk_list_foreach_safe(curr, n, head) \
    for (curr = (head)->next, n = curr->next; curr != (head); curr = n, n = curr->next)
#define mk_list_entry( ptr, type, member ) container_of( ptr, type, member )
void mk_list_add(struct mk_list *entry, struct mk_list *head);
void mk_list_del(struct mk_list *entry);
void *q_alloc(void); void q_free(void *); void q_use(void *);
struct item { int size; void *payload; struct mk_list _head; };
struct queue { struct mk_list items; struct mk_list done; int total; };
void drain(struct queue *q);
"#;

/// `drain` is declared with a custody duty its body does not keep in any
/// sense W4 could state (it releases what the queue held), next to the list
/// clauses of `monkey/intrusive_list.qv`.
const CONTRACTS: &str = "module contract;
extern fn mk_list_add(entry: ptr<mut, u8>, head: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_borrowed(head); ensures normal resource_published(entry, head); ensures normal resource_return_none(); };
extern fn mk_list_del(entry: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_unpublished(entry); ensures normal resource_return_none(); };
extern fn drain(q: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(q); ensures normal resource_return_none(); };
";

const UNIT: &str = "void drain(struct queue *q) { struct mk_list *it, *tmp; struct item *e;
     mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
     mk_list_del(&e->_head); q_free(e); } }
     void caller(struct queue *q) { drain(q); }";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(
        &[
            (
                "q_base.qcore".to_owned(),
                include_str!("fixtures/contracts/q_base.qv").to_owned(),
            ),
            ("list.qcore".to_owned(), CONTRACTS.to_owned()),
        ]
        .into_iter()
        .collect(),
    )
    .unwrap()
}

fn summary(entry: &str) -> LegacyProgramSummary {
    let source = format!("{PRELUDE}\n{UNIT}");
    match analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            dialect: CCompilerDialect::Gnu17,
            ..Default::default()
        },
        entry,
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected: {error:?}"))
    {
        UnrestrictedLegacyResult::Safe(certificate) => certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.unwrap(),
    }
}

#[test]
fn a_foreign_unlink_body_neither_establishes_nor_refutes_a_custody_declaration() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let summary = summary("drain");
    let drain = &summary.functions["drain"];
    // The body is Safe on its own, and the summary gate publishes nothing, so
    // no `SummaryConflict` exists to compare a duty with.
    assert_eq!(
        drain.status,
        AnalysisStatus::Safe,
        "{:?}",
        drain.diagnostics
    );
    assert!(!drain.diagnostics.iter().any(|diagnostic| matches!(
        diagnostic.kind,
        LegacyDiagnosticKind::SummaryConflict { .. }
    )));
    assert!(drain.refutations.is_empty(), "{:?}", drain.refutations);
    // Unchecked, not refuted: the caller still applies the declaration.
    let caller = &summary.functions["caller"];
    assert!(
        !caller.diagnostics.iter().any(|diagnostic| matches!(
            &diagnostic.kind,
            LegacyDiagnosticKind::Unsupported { description }
                if refuted_contract::held_call(description).is_some()
        )),
        "{:?}",
        caller.diagnostics
    );
}
