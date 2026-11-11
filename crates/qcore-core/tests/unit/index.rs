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

fn bytes() -> DimensionRef {
    DimensionRef::Named("Bytes".into())
}

#[test]
fn same_dimension_addition_is_constructible() {
    let n = IndexTerm::variable("n", bytes());
    let term = n.checked_add(IndexTerm::literal(8, bytes())).unwrap();
    assert_eq!(term.dimension(), &bytes());
    assert!(matches!(term.expression(), IndexExpression::Add(_, _)));
}

#[test]
fn cross_dimension_addition_is_rejected() {
    let memory = IndexTerm::variable("memory", bytes());
    let files = IndexTerm::variable("files", DimensionRef::Count("Fd".into()));
    let error = memory.checked_add(files).unwrap_err();
    assert_eq!(error.operation, "addition");
    assert_eq!(error.left, bytes());
    assert_eq!(error.right, DimensionRef::Count("Fd".into()));
}

#[test]
fn scaling_is_constant_by_construction() {
    let term = IndexTerm::variable("n", bytes()).scale(16);
    assert!(matches!(term.expression(), IndexExpression::Scale(16, _)));
}
