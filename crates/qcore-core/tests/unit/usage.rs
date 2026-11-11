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

fn variable(name: &str) -> UsageExpr {
    UsageExpr::Variable(name.into())
}

fn declarations(entries: &[(&str, Usage)]) -> BTreeMap<String, Usage> {
    entries
        .iter()
        .map(|(name, usage)| ((*name).into(), *usage))
        .collect()
}

fn violations(error: UsageCheckError) -> Vec<UsageViolation> {
    match error {
        UsageCheckError::Violations(violations) => violations,
        UsageCheckError::Demand(error) => panic!("unexpected demand error: {error}"),
    }
}

#[test]
fn interval_operations_follow_the_usage_semiring() {
    assert_eq!(Usage::ONE.checked_add(Usage::ONE), Ok(Usage::finite(2, 2)));
    assert_eq!(Usage::ZERO.checked_multiply(Usage::MANY), Ok(Usage::ZERO));
    assert_eq!(
        Usage::AFFINE.checked_multiply(Usage::finite(2, 2)),
        Ok(Usage::finite(0, 2))
    );
    assert_eq!(
        Usage::finite(u64::MAX, u64::MAX).checked_add(Usage::ONE),
        Err(UsageArithmeticError::Overflow)
    );
}

#[test]
fn exact_one_rejects_loss_and_duplication() {
    let declared = declarations(&[("x", Usage::ONE)]);
    assert!(matches!(
        &violations(check_usage(&declared, &UsageExpr::Unit).unwrap_err())[..],
        [UsageViolation::ExactOneNotConsumed { .. }]
    ));
    let twice = UsageExpr::Sequence(vec![variable("x"), variable("x")]);
    assert!(matches!(
        &violations(check_usage(&declared, &twice).unwrap_err())[..],
        [UsageViolation::ExactOneConsumedMoreThanOnce { .. }]
    ));
    assert!(check_usage(&declared, &variable("x")).is_ok());
}

#[test]
fn affine_allows_loss_but_rejects_duplication() {
    let declared = declarations(&[("x", Usage::AFFINE)]);
    assert!(check_usage(&declared, &UsageExpr::Unit).is_ok());
    assert!(check_usage(&declared, &variable("x")).is_ok());
    let twice = UsageExpr::Sequence(vec![variable("x"), variable("x")]);
    assert!(matches!(
        &violations(check_usage(&declared, &twice).unwrap_err())[..],
        [UsageViolation::DeclaredUsageExceeded { .. }]
    ));
}

#[test]
fn many_allows_weakening_and_contraction() {
    let declared = declarations(&[("x", Usage::MANY)]);
    assert!(check_usage(&declared, &UsageExpr::Unit).is_ok());
    let repeated = UsageExpr::Sequence(vec![variable("x"), variable("x")]);
    assert!(check_usage(&declared, &repeated).is_ok());
}

#[test]
fn relevant_forbids_weakening_but_allows_contraction() {
    let declared = declarations(&[("x", Usage::RELEVANT)]);
    assert!(matches!(
        &violations(check_usage(&declared, &UsageExpr::Unit).unwrap_err())[..],
        [UsageViolation::DeclaredUsageExceeded { .. }]
    ));
    let repeated = UsageExpr::Sequence(vec![variable("x"), variable("x")]);
    assert!(check_usage(&declared, &repeated).is_ok());
}

#[test]
fn erased_variable_cannot_occur_at_runtime() {
    let declared = declarations(&[("proof", Usage::ZERO)]);
    assert!(check_usage(&declared, &UsageExpr::Unit).is_ok());
    assert!(matches!(
        &violations(check_usage(&declared, &variable("proof")).unwrap_err())[..],
        [UsageViolation::ErasedVariableAtRuntime { .. }]
    ));
}

#[test]
fn branch_join_is_the_least_enclosing_interval() {
    let expression = UsageExpr::Branch {
        condition: Box::new(variable("condition")),
        then_expression: Box::new(variable("x")),
        else_expression: Box::new(UsageExpr::Unit),
    };
    let demands = expression.synthesize_demands().unwrap();
    assert_eq!(demands.get("condition"), Usage::ONE);
    assert_eq!(demands.get("x"), Usage::AFFINE);

    let both = UsageExpr::Branch {
        condition: Box::new(UsageExpr::Unit),
        then_expression: Box::new(variable("x")),
        else_expression: Box::new(variable("x")),
    };
    assert_eq!(both.synthesize_demands().unwrap().get("x"), Usage::ONE);
}

#[test]
fn application_substitutes_parameter_usage_pointwise() {
    let argument = UsageExpr::Sequence(vec![variable("x"), variable("x")]);
    let expression = UsageExpr::Apply {
        function: Box::new(variable("f")),
        parameter_usages: vec![Usage::AFFINE],
        arguments: vec![argument],
    };
    let demands = expression.synthesize_demands().unwrap();
    assert_eq!(demands.get("f"), Usage::ONE);
    assert_eq!(demands.get("x"), Usage::finite(0, 2));
}

#[test]
fn application_checks_arity() {
    let expression = UsageExpr::Apply {
        function: Box::new(UsageExpr::Unit),
        parameter_usages: vec![Usage::ONE],
        arguments: Vec::new(),
    };
    assert_eq!(
        expression.synthesize_demands(),
        Err(DemandError::ApplicationArity {
            expected: 1,
            actual: 0
        })
    );
}

fn lambda(parameter: &str, grade: Usage, body: UsageExpr) -> UsageExpr {
    UsageExpr::Lambda {
        parameter: parameter.into(),
        parameter_usage: grade,
        body: Box::new(body),
    }
}

#[test]
fn replicated_affine_function_cannot_duplicate_its_parameter() {
    // The gist's lambda f. lambda x. f (f x), with unrestricted A -> A.
    let apply_f = |argument| UsageExpr::Apply {
        function: Box::new(variable("f")),
        parameter_usages: vec![Usage::MANY],
        arguments: vec![argument],
    };
    let twice = lambda(
        "f",
        Usage::AFFINE,
        lambda("x", Usage::MANY, apply_f(apply_f(variable("x")))),
    );
    for quantity in [Usage::ZERO, Usage::ONE, Usage::MANY] {
        assert!(matches!(twice.synthesize_demands_at(quantity),
            Err(DemandError::ParameterUsage { name, actual, .. })
                if name == "f" && actual == Usage::RELEVANT));
    }
    // Passing it to a replicating or erased parameter must still check its body.
    for grade in [Usage::ZERO, Usage::MANY] {
        let application = UsageExpr::Apply {
            function: Box::new(UsageExpr::Unit),
            parameter_usages: vec![grade],
            arguments: vec![twice.clone()],
        };
        assert!(application.synthesize_demands().is_err());
    }
}

#[test]
fn lambda_replication_scales_captures_after_checking_the_binder() {
    let closure = lambda(
        "x",
        Usage::ONE,
        UsageExpr::Sequence(vec![variable("x"), variable("captured")]),
    );
    let replicated = UsageExpr::Apply {
        function: Box::new(UsageExpr::Unit),
        parameter_usages: vec![Usage::MANY],
        arguments: vec![closure.clone()],
    };
    assert_eq!(closure.synthesize_demands().unwrap().get("x"), Usage::ZERO);
    assert_eq!(
        replicated.synthesize_demands().unwrap().get("captured"),
        Usage::MANY
    );
    assert!(check_usage(&declarations(&[("captured", Usage::AFFINE)]), &replicated).is_err());
    assert!(check_usage(&declarations(&[("captured", Usage::MANY)]), &replicated).is_ok());
    assert!(
        lambda("x", Usage::ONE, variable("x"))
            .synthesize_demands_at(Usage::MANY)
            .is_ok()
    );
}

#[test]
fn lambda_binders_respect_shadowing_erasure_and_branch_obligations() {
    let shadowed = lambda("x", Usage::ZERO, lambda("x", Usage::ONE, variable("x")));
    assert!(
        shadowed
            .synthesize_demands()
            .unwrap()
            .iter()
            .next()
            .is_none()
    );
    assert!(
        lambda("x", Usage::ZERO, variable("x"))
            .synthesize_demands()
            .is_err()
    );
    let branch = UsageExpr::Branch {
        condition: Box::new(UsageExpr::Unit),
        then_expression: Box::new(variable("x")),
        else_expression: Box::new(UsageExpr::Unit),
    };
    assert!(
        lambda("x", Usage::ONE, branch.clone())
            .synthesize_demands()
            .is_err()
    );
    assert!(
        lambda("x", Usage::AFFINE, branch)
            .synthesize_demands()
            .is_ok()
    );
}
