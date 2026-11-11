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

//! Card CG29-EXECUTION-COVERAGE-AND-RECLAMATION: race freedom over every
//! candidate execution, bound to `proof/QCore/ExecutionCoverage.lean`. The
//! fragments and the `coverageVectors` table are **parsed from the Lean
//! file**, run through `check_program_executions`, and their verdicts and
//! candidate counts compared with the table the Lean theorem
//! `coverage_vectors_hold` proves.

use super::*;
use crate::weak_memory::{MemoryEventKind, check_c11_execution};

const LEAN: &str = include_str!("../../../../proof/QCore/ExecutionCoverage.lean");

fn tokens(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {}
            '[' | ']' | '(' | ')' | '{' | '}' | ',' => result.push(c.to_string()),
            ':' if chars.peek() == Some(&'=') => {
                chars.next();
                result.push(":=".to_owned());
            }
            '"' => {
                let mut word = String::from('"');
                for next in chars.by_ref() {
                    word.push(next);
                    if next == '"' {
                        break;
                    }
                }
                result.push(word);
            }
            _ => {
                let mut word = c.to_string();
                while let Some(&next) = chars.peek() {
                    if next.is_whitespace() || "[](){},".contains(next) {
                        break;
                    }
                    word.push(next);
                    chars.next();
                }
                result.push(word);
            }
        }
    }
    result
}

struct Parser {
    tokens: Vec<String>,
    at: usize,
}

impl Parser {
    fn new(text: &str) -> Self {
        Self {
            tokens: tokens(text),
            at: 0,
        }
    }

    fn peek(&self) -> &str {
        &self.tokens[self.at]
    }

    fn next(&mut self) -> String {
        self.at += 1;
        self.tokens[self.at - 1].clone()
    }

    fn expect(&mut self, token: &str) {
        let found = self.next();
        assert_eq!(found, token, "at token {}", self.at);
    }

    fn number(&mut self) -> i64 {
        self.next().parse().expect("a number")
    }

    fn list<T>(&mut self, mut item: impl FnMut(&mut Self) -> T) -> Vec<T> {
        self.expect("[");
        let mut items = Vec::new();
        while self.peek() != "]" {
            items.push(item(self));
            if self.peek() == "," {
                self.next();
            }
        }
        self.expect("]");
        items
    }

    fn pair(&mut self) -> (i64, i64) {
        self.expect("(");
        let left = self.number();
        self.expect(",");
        let right = self.number();
        self.expect(")");
        (left, right)
    }

    fn order(&mut self) -> MemoryOrder {
        match self.next().as_str() {
            ".relaxed" => MemoryOrder::Relaxed,
            ".acquire" => MemoryOrder::Acquire,
            ".release" => MemoryOrder::Release,
            ".acqRel" => MemoryOrder::AcquireRelease,
            ".seqCst" => MemoryOrder::SequentiallyConsistent,
            other => panic!("unmapped Lean order {other}"),
        }
    }

    fn flag(&mut self) -> bool {
        match self.next().as_str() {
            "true" => true,
            "false" => false,
            other => panic!("unmapped Lean Bool {other}"),
        }
    }

    fn location(&mut self) -> MemoryLocation {
        u32::try_from(self.number()).expect("a location")
    }

    fn register(&mut self) -> Register {
        u32::try_from(self.number()).expect("a register")
    }

    fn instruction(&mut self) -> Instruction {
        let parenthesised = self.peek() == "(";
        if parenthesised {
            self.next();
        }
        let instruction = match self.next().as_str() {
            ".load" => Instruction::Load {
                location: self.location(),
                order: self.order(),
                register: self.register(),
            },
            ".store" => Instruction::Store {
                location: self.location(),
                value: self.number(),
                order: self.order(),
            },
            ".xchg" => Instruction::Exchange {
                location: self.location(),
                value: self.number(),
                order: self.order(),
                register: self.register(),
            },
            ".cas" => Instruction::CompareExchange {
                location: self.location(),
                expected: self.number(),
                desired: self.number(),
                success: self.order(),
                failure: self.order(),
                strength: if self.flag() {
                    CompareExchangeStrength::Weak
                } else {
                    CompareExchangeStrength::Strong
                },
                register: self.register(),
            },
            ".fence" => Instruction::Fence {
                order: self.order(),
                scope: if self.flag() {
                    FenceScope::Signal
                } else {
                    FenceScope::Thread
                },
            },
            ".naRead" => Instruction::NonAtomicRead {
                location: self.location(),
            },
            ".naWrite" => Instruction::NonAtomicWrite {
                location: self.location(),
            },
            ".loop" => Instruction::Loop,
            other => panic!("unmapped Lean instruction {other}"),
        };
        if parenthesised {
            self.expect(")");
        }
        instruction
    }

    fn guarded(&mut self) -> GuardedInstruction {
        self.expect("g");
        let guard = self
            .list(Self::pair)
            .into_iter()
            .map(|(register, value)| (u32::try_from(register).unwrap(), value))
            .collect();
        GuardedInstruction {
            guard,
            instruction: self.instruction(),
        }
    }

    fn fragment(&mut self) -> ProgramFragment {
        self.expect("{");
        self.expect("init");
        self.expect(":=");
        let atomic_locations = self
            .list(Self::pair)
            .into_iter()
            .map(|(location, value)| (u32::try_from(location).unwrap(), value))
            .collect();
        self.expect("threads");
        self.expect(":=");
        let threads = self.list(|parser| parser.list(Self::guarded));
        self.expect("}");
        ProgramFragment {
            atomic_locations,
            threads,
        }
    }

    fn bounds(&mut self) -> CoverageBounds {
        self.expect("{");
        let field = |parser: &mut Self, name: &str| {
            parser.expect(name);
            parser.expect(":=");
            let value = usize::try_from(parser.number()).unwrap();
            if parser.peek() == "," {
                parser.next();
            }
            value
        };
        let bounds = CoverageBounds {
            threads: field(self, "threads"),
            events: field(self, "events"),
            values: field(self, "values"),
            candidates: field(self, "candidates"),
        };
        self.expect("}");
        bounds
    }
}

fn lean() -> String {
    LEAN.replace("\r\n", "\n")
}

/// The body of `def NAME : TYPE :=`.
fn definition(name: &str, type_name: &str) -> String {
    let lean = lean();
    let head = format!("def {name} : {type_name} :=");
    let start = lean.find(&head).unwrap_or_else(|| panic!("{head}")) + head.len();
    let rest = &lean[start..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    rest[..end].to_owned()
}

fn fragment(name: &str) -> ProgramFragment {
    Parser::new(&definition(name, "Frag")).fragment()
}

struct Vector {
    name: String,
    fragment: ProgramFragment,
    bounds: CoverageBounds,
    verdict: String,
    candidates: usize,
}

fn vectors() -> Vec<Vector> {
    let wide = Parser::new(&definition("wide", "Bounds")).bounds();
    let table = definition(
        "coverageVectors",
        "List (String × Frag × Bounds × Verdict × Nat)",
    );
    let mut parser = Parser::new(&table);
    parser.list(|parser| {
        parser.expect("(");
        let name = parser.next().trim_matches('"').to_owned();
        parser.expect(",");
        let fragment = fragment(&parser.next());
        parser.expect(",");
        let bounds = if parser.peek() == "{" {
            parser.bounds()
        } else {
            assert_eq!(parser.next(), "wide");
            wide
        };
        parser.expect(",");
        let mut verdict = parser.next();
        if verdict == ".unknown" {
            verdict = format!(".unknown {}", parser.next());
        }
        parser.expect(",");
        let candidates = usize::try_from(parser.number()).unwrap();
        parser.expect(")");
        Vector {
            name,
            fragment,
            bounds,
            verdict,
            candidates,
        }
    })
}

/// The Lean `Verdict` a Rust verdict corresponds to.
fn lean_verdict(verdict: &ProgramVerdict) -> String {
    match verdict {
        ProgramVerdict::RaceFree(_) => ".raceFree".to_owned(),
        ProgramVerdict::Race { .. } => ".race".to_owned(),
        ProgramVerdict::Unknown(reason) => format!(
            ".unknown {}",
            match reason {
                CoverageUnknown::Loop => ".loop",
                CoverageUnknown::Malformed => ".malformed",
                CoverageUnknown::ThreadBound { .. } => ".threadBound",
                CoverageUnknown::EventBound { .. } => ".eventBound",
                CoverageUnknown::ValueBound { .. } => ".valueBound",
                CoverageUnknown::CandidateBound { .. } => ".candidateBound",
                CoverageUnknown::NoConsistentExecution => ".noConsistentExecution",
            }
        ),
    }
}

fn wide() -> CoverageBounds {
    CoverageBounds {
        threads: 4,
        events: 16,
        values: 4,
        candidates: 4096,
    }
}

/// Every shared vector: the Rust verdict equals the Lean one, and where the
/// candidates are enumerated their number equals `(candidates F).length`.
#[test]
fn lean_coverage_vectors_agree() {
    let vectors = vectors();
    assert_eq!(vectors.len(), 12);
    for vector in &vectors {
        let verdict = check_program_executions(&vector.fragment, vector.bounds);
        assert_eq!(lean_verdict(&verdict), vector.verdict, "{}", vector.name);
        match &verdict {
            ProgramVerdict::RaceFree(evidence) | ProgramVerdict::Race { evidence, .. } => {
                assert_eq!(evidence.candidates, vector.candidates, "{}", vector.name);
            }
            ProgramVerdict::Unknown(_) => {}
        }
    }
    // The decided vectors' counts are checked under wide bounds too, so the
    // bound vectors' counts are bound as well.
    for vector in &vectors {
        if let ProgramVerdict::RaceFree(evidence) | ProgramVerdict::Race { evidence, .. } =
            check_program_executions(&vector.fragment, wide())
        {
            assert_eq!(evidence.candidates, vector.candidates, "{}", vector.name);
        }
    }
}

/// The brief's MP control: two threads, a release store publishing and an
/// acquire load guarding the payload read, race-free over **every**
/// candidate execution (not one graph).
#[test]
fn mp_release_acquire_is_race_free_over_all_executions() {
    let verdict = check_program_executions(&fragment("mpReleaseAcquire"), wide());
    let ProgramVerdict::RaceFree(evidence) = verdict else {
        panic!("{verdict:?}");
    };
    assert_eq!(evidence.threads, 2);
    assert_eq!(evidence.candidates, 4);
    assert!(evidence.consistent >= 2, "{evidence:?}");
    assert_eq!(evidence.value_domains, vec![(1, vec![0, 1])]);
    assert!(evidence.assumption().contains("every candidate execution"));
    // The relaxed version races in the execution whose load reads the store.
    let ProgramVerdict::Race { witness, .. } =
        check_program_executions(&fragment("mpRelaxed"), wide())
    else {
        panic!("mpRelaxed");
    };
    assert!(
        witness
            .events
            .iter()
            .any(|event| matches!(event.kind, MemoryEventKind::AtomicLoad { value: 1, .. }))
    );
}

/// The brief's SB control: with relaxed accesses the handshake races in an
/// execution where both loads read the initial values. The witness is a
/// consistent execution: the checker reports nothing but the race.
#[test]
fn sb_relaxed_reports_the_race_in_some_execution() {
    let ProgramVerdict::Race {
        witness,
        first,
        second,
        ..
    } = check_program_executions(&fragment("sbRelaxed"), wide())
    else {
        panic!("sbRelaxed");
    };
    let loads = witness
        .events
        .iter()
        .filter_map(|event| match event.kind {
            MemoryEventKind::AtomicLoad {
                location, value, ..
            } => Some((location, value)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        loads,
        vec![(1, 1), (2, 0)],
        "both loads read the initial values"
    );
    let errors = check_c11_execution(&witness).expect_err("the witness races");
    assert_eq!(
        errors,
        vec![WeakMemoryError::DataRace { first, second }],
        "the witness is otherwise consistent"
    );
    // Release/acquire does not forbid store buffering either.
    assert_eq!(
        check_program_executions(
            &announce_validate_handshake(MemoryOrder::AcquireRelease, false),
            wide()
        )
        .status(),
        "Race"
    );
    // seq_cst fences, or seq_cst accesses, do.
    assert_eq!(
        check_program_executions(&fragment("sbFences"), wide()).status(),
        "RaceFree"
    );
    let seq_cst = check_program_executions(
        &announce_validate_handshake(MemoryOrder::SequentiallyConsistent, false),
        wide(),
    );
    let ProgramVerdict::RaceFree(evidence) = seq_cst else {
        panic!("{seq_cst:?}");
    };
    assert_eq!(evidence.candidates, 384);
    assert_eq!(
        announce_validate_handshake(MemoryOrder::Relaxed, true),
        fragment("sbFences")
    );
    assert_eq!(
        announce_validate_handshake(MemoryOrder::Relaxed, false),
        fragment("sbRelaxed")
    );
}

/// The brief's bound control: each exceeded bound, a loop and a malformed
/// fragment give `Unknown` with the bound named, never a verdict.
#[test]
fn exceeded_bounds_are_unknown() {
    let mp = fragment("mpReleaseAcquire");
    let with = |change: fn(&mut CoverageBounds)| {
        let mut bounds = wide();
        change(&mut bounds);
        check_program_executions(&mp, bounds)
    };
    assert_eq!(
        with(|bounds| bounds.threads = 1),
        ProgramVerdict::Unknown(CoverageUnknown::ThreadBound {
            threads: 2,
            bound: 1
        })
    );
    assert_eq!(
        with(|bounds| bounds.events = 6),
        ProgramVerdict::Unknown(CoverageUnknown::EventBound {
            events: 7,
            bound: 6
        })
    );
    assert_eq!(
        with(|bounds| bounds.values = 1),
        ProgramVerdict::Unknown(CoverageUnknown::ValueBound {
            location: 1,
            values: 2,
            bound: 1
        })
    );
    assert_eq!(
        with(|bounds| bounds.candidates = 3),
        ProgramVerdict::Unknown(CoverageUnknown::CandidateBound { bound: 3 })
    );
    assert_eq!(
        check_program_executions(&fragment("withLoop"), wide()),
        ProgramVerdict::Unknown(CoverageUnknown::Loop)
    );
    assert_eq!(
        check_program_executions(&fragment("mixedLocation"), wide()),
        ProgramVerdict::Unknown(CoverageUnknown::Malformed)
    );
    let mut undeclared = mp.clone();
    undeclared.atomic_locations.clear();
    assert_eq!(
        check_program_executions(&undeclared, wide()),
        ProgramVerdict::Unknown(CoverageUnknown::Malformed)
    );
    let mut duplicate = mp;
    duplicate.atomic_locations.push((1, 0));
    assert_eq!(
        check_program_executions(&duplicate, wide()),
        ProgramVerdict::Unknown(CoverageUnknown::Malformed)
    );
}

/// The CG00 W07 control (`clang_gap_core.rs`): the same relaxed flag program
/// has an accepted graph. At the program level it races.
#[test]
fn w07_the_program_level_verdict_is_not_the_one_graph() {
    let relaxed = fragment("mpRelaxed");
    let quiet = C11Execution {
        events: vec![
            MemoryEvent {
                id: 0,
                thread: 0,
                kind: MemoryEventKind::AtomicInit {
                    location: 1,
                    value: 0,
                },
            },
            MemoryEvent {
                id: 1,
                thread: 1,
                kind: MemoryEventKind::NonAtomicWrite { location: 9 },
            },
            MemoryEvent {
                id: 2,
                thread: 1,
                kind: MemoryEventKind::AtomicStore {
                    location: 1,
                    value: 1,
                    order: MemoryOrder::Relaxed,
                },
            },
            MemoryEvent {
                id: 3,
                thread: 2,
                kind: MemoryEventKind::AtomicLoad {
                    location: 1,
                    value: 0,
                    order: MemoryOrder::Relaxed,
                },
            },
        ],
        reads_from: vec![RelationEdge::new(0, 3)],
        modification_order: vec![RelationEdge::new(0, 2)],
        ..C11Execution::default()
    };
    check_c11_execution(&quiet).expect("one graph of the program is accepted");
    assert_eq!(check_program_executions(&relaxed, wide()).status(), "Race");
}

/// A weak compare-exchange's spurious failure is enumerated: the strong and
/// weak publications differ in their path counts.
#[test]
fn weak_compare_exchange_failures_are_enumerated() {
    let weak = fragment("casPublication");
    let mut strong = weak.clone();
    if let Instruction::CompareExchange { strength, .. } = &mut strong.threads[0][1].instruction {
        *strength = CompareExchangeStrength::Strong;
    }
    let count = |fragment: &ProgramFragment| match check_program_executions(fragment, wide()) {
        ProgramVerdict::RaceFree(evidence) => evidence.candidates,
        other => panic!("{other:?}"),
    };
    assert_eq!(count(&weak), 5);
    assert_eq!(count(&strong), 4);
}
