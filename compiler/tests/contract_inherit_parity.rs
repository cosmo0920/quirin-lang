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

//! Card CONTRACT-INHERIT parity over the real contract corpus.
//!
//! A module that uses neither inclusion form must elaborate to itself, byte
//! for byte, with no provenance record; the load-path guard must not fire on
//! it; and the structural parser must read it exactly as before. Every corpus
//! module is such a module at this card (no real contract adopts inheritance),
//! so the resource manifest and every other consumer load an identical
//! result. The count is not pinned here: the corpus pins live in
//! `qcore_contract_corpus.rs`, and a later card may adopt the feature.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use qcore_contract::contract_inclusion::{
    ContractLoadPath, PublishedContractCrate, elaborate_contract_module,
};
use qcore_frontend::contract_inclusion::{
    contains_contract_inclusion_syntax, scan_contract_inclusion,
};
use qcore_frontend::frontend::parse_source_program;

#[path = "support/contract_corpus.rs"]
mod contract_corpus;

#[test]
fn every_corpus_module_without_inclusion_elaborates_to_itself() {
    let sources = contract_corpus::sources();
    assert!(!sources.is_empty());
    let mut modules = BTreeMap::new();
    for (logical, path) in &sources {
        let text = fs::read_to_string(path).unwrap();
        if contains_contract_inclusion_syntax(&text) {
            // Adopted later; resolved against a real load path, not here.
            continue;
        }
        assert!(
            scan_contract_inclusion(&text).unwrap().is_empty(),
            "{logical}"
        );
        modules.insert(logical.clone(), text);
    }
    let corpus = PublishedContractCrate {
        name: "quirin-provider-corpus".into(),
        namespace: "corpus".into(),
        version: "0.0.0".into(),
        dependencies: BTreeSet::new(),
        published: BTreeSet::new(),
        modules: modules.clone(),
    };
    let load_path = ContractLoadPath::new([corpus]).unwrap();
    let mut checked = 0;
    for (logical, text) in &modules {
        let parsed_before = parse_source_program(text).is_ok();
        let elaborated = elaborate_contract_module(&load_path, "quirin-provider-corpus", logical);
        if parsed_before {
            let elaborated = elaborated.unwrap_or_else(|e| panic!("{logical}: {e}"));
            assert_eq!(&elaborated.text, text, "{logical}");
            assert!(elaborated.evidence.is_none(), "{logical}");
            checked += 1;
        } else {
            // A module the parser refuses stays refused.
            assert!(elaborated.is_err(), "{logical}");
        }
    }
    assert_eq!(checked, modules.len(), "every corpus module parses");
}
