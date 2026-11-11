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

use super::*;

#[test]
fn parses_and_pretty_prints_initial_declarations() {
    let source = "\
dimension Bytes : Idx + Ordered;
quant axis memory : live<Bytes>;
quant axis allocation : cost.total<Bytes>;
quant axis descriptors : credit<Count<Fd>>;
";
    let program = parse_program(source).unwrap();
    assert_eq!(program.declarations.len(), 4);
    assert_eq!(parse_program(&program.to_string()).unwrap(), program);
}

#[test]
fn rejects_variable_multiplication_as_a_declaration_start() {
    let error = parse_program("n * m").unwrap_err();
    assert!(error.message.contains("expected `dimension`"));
}
