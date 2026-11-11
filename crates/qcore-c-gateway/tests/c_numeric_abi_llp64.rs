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

//! Card CG20-NUMERIC-DOMAIN (gateway rule 11q): the lexical ABI reader's LP64
//! premise for plain `long` is removed. Under an analysing Clang whose target
//! is LLP64 (`x86_64-pc-windows-msvc`), a declaration involving plain `long`
//! is unreadable instead of being read as 64 bits; `long long` keeps its
//! reading. Its own test binary: the wrapper Clang it installs through
//! `QCORE_CLANG` is process-wide.

#![cfg(unix)]

use std::collections::BTreeMap;

use qcore_c_gateway::c_numeric;
use qcore_c_gateway::legacy_c::{AbiOutcome, AbiRefusal, declared_abi_census};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::ResourceContracts;

// The wrapper is written by a separate process
// (`tests/support/process_fixture.rs`). This binary has one test, so no
// sibling spawns while it is written; the shared writer keeps it that way
// if a test is added.
#[path = "../../../tests/support/process_fixture.rs"]
mod process_fixture;

const CONTRACT: &str = "module contract;

extern fn k_len(size: usize) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(size);
    ensures normal resource_return_none();
};
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "k_api.qcore".to_owned(),
        CONTRACT.to_owned(),
    )]))
    .expect("the fixture module loads")
}

fn outcome(declaration: &str) -> AbiOutcome {
    let source = format!("{declaration}\nvoid w(void) {{ k_len(8); }}\n");
    let rows = declared_abi_census(&source, &contracts());
    assert_eq!(rows.len(), 1, "{rows:?}");
    rows[0].outcome
}

#[test]
fn plain_long_is_refused_under_an_llp64_clang() {
    let Ok(real) = qcore_c_gateway::clang_command() else {
        assert!(std::env::var_os("QCORE_CLANG").is_none());
        eprintln!("skipping: no Clang");
        return;
    };
    let real = real.get_program().to_owned();
    // Under the host's own Clang (LP64 on this test's Unix hosts) the reading
    // is the established one.
    let host_lp64 = c_numeric::plain_long_is_lp64();
    if host_lp64 {
        assert_eq!(
            outcome("void k_len(unsigned long size);"),
            AbiOutcome::Agreed
        );
    }
    let directory = std::env::temp_dir().join(format!("cg20-llp64-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let wrapper = directory.join("clang-msvc");
    process_fixture::write_executable_mode(
        &wrapper,
        format!(
            "#!/bin/sh\nexec '{}' --target=x86_64-pc-windows-msvc \"$@\"\n",
            real.to_string_lossy()
        )
        .as_bytes(),
        0o755,
    );
    // SAFETY: this test binary has one test, and nothing else reads the
    // environment concurrently.
    unsafe { std::env::set_var("QCORE_CLANG", &wrapper) };
    let model = c_numeric::default_target_model().expect("the wrapper's model");
    assert_eq!(model.data_model(), c_numeric::DataModel::Llp64);
    assert!(!c_numeric::plain_long_is_lp64());
    // Plain `long`, in any spelling, is unreadable: never the LP64 reading.
    for declaration in [
        "void k_len(unsigned long size);",
        "void k_len(long unsigned int size);",
        "typedef unsigned long my_size; void k_len(my_size size);",
    ] {
        assert_eq!(
            outcome(declaration),
            AbiOutcome::Withdrawn(AbiRefusal::UnreadableDeclaration),
            "{declaration}"
        );
    }
    // `long long` and `__int64` keep their explicit 64-bit reading.
    assert_eq!(
        outcome("void k_len(unsigned long long size);"),
        AbiOutcome::Agreed
    );
    assert_eq!(
        outcome("void k_len(unsigned __int64 size);"),
        AbiOutcome::Agreed
    );
    std::fs::remove_dir_all(&directory).ok();
}
