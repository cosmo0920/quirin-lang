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

use crate::parse_program;

use super::*;

#[test]
fn validates_named_and_count_dimensions() {
    let program = parse_program(
        "\
dimension Bytes : Idx + Ordered;
quant axis memory : live<Bytes>;
quant axis files : live<Count<Fd>>;
",
    )
    .unwrap();
    let environment = StaticEnvironment::check(&program).unwrap();
    assert!(environment.dimensions.contains("Bytes"));
    assert_eq!(environment.axes.len(), 2);
}

#[test]
fn rejects_unknown_named_dimension() {
    let program = parse_program("quant axis memory : live<Bytes>;").unwrap();
    assert_eq!(
        StaticEnvironment::check(&program),
        Err(CheckError::UnknownDimension("Bytes".into()))
    );
}

#[test]
fn allows_forward_dimension_references() {
    let program = parse_program(
        "\
quant axis memory : live<Bytes>;
dimension Bytes : Idx + Ordered;
",
    )
    .unwrap();
    assert!(StaticEnvironment::check(&program).is_ok());
}

/// D01: `check_located` reports the declaration index that produced the
/// failure, so a caller can attribute the diagnostic to a real span instead of
/// the degenerate `0..0` span. Also pins `DuplicateDimension`/`DuplicateAxis`
/// directly, which previously had no case of their own at this level.
#[test]
fn locates_the_declaration_that_failed_validation() {
    for (source, expected) in [
        (
            "\
dimension Bytes : Idx + Ordered;
quant axis good : credit<Bytes>;
quant axis bad : credit<Missing>;
",
            (CheckError::UnknownDimension("Missing".into()), 2),
        ),
        (
            "\
dimension Bytes : Idx + Ordered;
dimension Bytes : Idx + Ordered;
",
            (CheckError::DuplicateDimension("Bytes".into()), 1),
        ),
        (
            "\
dimension Bytes : Idx + Ordered;
quant axis x : credit<Bytes>;
quant axis x : credit<Bytes>;
",
            (CheckError::DuplicateAxis("x".into()), 2),
        ),
    ] {
        let program = parse_program(source).unwrap();
        assert_eq!(
            StaticEnvironment::check_located(&program),
            Err(expected.clone()),
            "{source}"
        );
        // The located form must accept and reject exactly what `check` does.
        assert_eq!(StaticEnvironment::check(&program), Err(expected.0));
    }
    let program = parse_program("dimension Bytes : Idx + Ordered;").unwrap();
    assert_eq!(
        StaticEnvironment::check_located(&program).map(|environment| environment.dimensions.len()),
        Ok(1)
    );
}
