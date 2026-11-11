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

//! Rule 36's return route (route map §4 step 2, the head of
//! `return_statement`; registry arm `exact-formal-return` at position
//! `return-route`): the first `ReturnRoute` package.
//!
//! Only the derived-result CFG may lower a bare `return P;` to the host's
//! exact-formal instruction, and only for a `(line, P)` the source
//! conjunction ([`crate::source::exact_formal_returns`]) confirmed. Refusal
//! leaves the return to the host's existing lowering, which is the lowering
//! before rule 36. The ordinary full-body, all-reachable-exit, untouched-base
//! and fresh reported-premise validation gates still decide the summary.

use qcore_gateway_rule_api::traits::{ReturnQuery, ReturnRoute, RulePackage};
use qcore_gateway_rule_api::{Answer, Host};

/// The host's side of the return route.
pub trait ExactFormalHost: Host {
    /// Whether the host is lowering the derived-result CFG.
    fn derived_result_cfg(&self) -> bool;
    /// The formal position the source conjunction confirmed for a bare
    /// `return NAME;` at `line`, or `None`.
    fn confirmed_formal_return(&self, name: &str, line: usize) -> Option<usize>;
    /// The host's lowering of an admitted `return NAME;` as an exact,
    /// non-owning alias of formal `parameter`.
    fn lower_exact_formal_return(
        &mut self,
        query: &ReturnQuery<'_, Self>,
        parameter: usize,
    ) -> Self::Statement;
}

/// The route's decision: the confirmed formal position of a one-token return
/// expression in the derived-result CFG. `confirmed` is asked only there.
pub fn decide(
    derived_result_cfg: bool,
    expression: &[&str],
    confirmed: impl FnOnce(&str) -> Option<usize>,
) -> Option<usize> {
    match expression {
        [name] if derived_result_cfg => confirmed(name),
        _ => None,
    }
}

/// The package: registry rule `36`, return-route arm `exact-formal-return`,
/// route `route::decide`.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExactFormalReturnRoute;

impl RulePackage for ExactFormalReturnRoute {
    fn id(&self) -> &'static str {
        crate::ID
    }
}

impl<H: ExactFormalHost + ?Sized> ReturnRoute<H> for ExactFormalReturnRoute {
    fn ask(&self, host: &mut H, query: &ReturnQuery<'_, H>) -> Answer<H::Statement> {
        let parameter = match query.expression {
            [token] => decide(host.derived_result_cfg(), &[H::token_text(token)], |name| {
                host.confirmed_formal_return(name, query.line)
            }),
            _ => None,
        };
        match parameter {
            Some(parameter) => Answer::admit(host.lower_exact_formal_return(query, parameter)),
            None => Answer::Refuse,
        }
    }
}

#[cfg(test)]
mod tests {
    use qcore_gateway_rule_api::traits::{ReturnQuery, ReturnRoute, RulePackage};
    use qcore_gateway_rule_api::{Answer, Host, consult};

    use super::{ExactFormalHost, ExactFormalReturnRoute, decide};

    #[test]
    fn only_a_confirmed_one_token_return_of_the_derived_cfg_is_decided() {
        let confirmed = |name: &str| (name == "p").then_some(2);
        assert_eq!(decide(true, &["p"], confirmed), Some(2));
        assert_eq!(decide(true, &["q"], confirmed), None);
        assert_eq!(decide(false, &["p"], |_| panic!("not asked")), None);
        assert_eq!(
            decide(true, &["p", "+", "1"], |_| panic!("not asked")),
            None
        );
        assert_eq!(decide(true, &[], |_| panic!("not asked")), None);
    }

    /// A host whose statements are strings.
    struct Toy {
        derived: bool,
        asked: usize,
    }

    impl Host for Toy {
        type Token = &'static str;
        type Decision = ();
        type Statement = String;
        type Condition = ();
        type Node = ();

        fn token_text(token: &Self::Token) -> &str {
            token
        }
        fn token_line(_: &Self::Token) -> usize {
            4
        }
        fn refusal(_: &Self::Decision) -> Option<&str> {
            None
        }
    }

    impl ExactFormalHost for Toy {
        fn derived_result_cfg(&self) -> bool {
            self.derived
        }
        fn confirmed_formal_return(&self, name: &str, line: usize) -> Option<usize> {
            (name == "p" && line == 4).then_some(1)
        }
        fn lower_exact_formal_return(
            &mut self,
            query: &ReturnQuery<'_, Self>,
            parameter: usize,
        ) -> String {
            self.asked += 1;
            format!("exact {} formal {parameter}", query.expression[0])
        }
    }

    fn ask(host: &mut Toy, expression: &[&'static str], on: bool) -> Option<String> {
        let refused = String::from("refused");
        let query = ReturnQuery::<Toy> {
            expression,
            line: 4,
            origin: 0,
            decided: &refused,
        };
        consult(
            &[ExactFormalReturnRoute],
            |package| on || package.id() != crate::ID,
            |_, package| ReturnRoute::ask(package, host, &query),
        )
        .map(|decided| {
            assert!(decided.admission.obligations.is_empty());
            decided.admission.lowering
        })
    }

    #[test]
    fn the_route_admits_a_confirmed_return_and_off_is_not_asked() {
        let mut host = Toy {
            derived: true,
            asked: 0,
        };
        assert_eq!(
            ask(&mut host, &["p"], true).as_deref(),
            Some("exact p formal 1")
        );
        assert_eq!(ask(&mut host, &["q"], true), None);
        assert_eq!(ask(&mut host, &["p"], false), None);
        assert_eq!(host.asked, 1);
        let mut outside = Toy {
            derived: false,
            asked: 0,
        };
        assert_eq!(ask(&mut outside, &["p"], true), None);
        assert_eq!(ExactFormalReturnRoute.id(), "36");
        let answer: Answer<String> = Answer::Refuse;
        assert!(answer.into_admission().is_none());
    }
}
