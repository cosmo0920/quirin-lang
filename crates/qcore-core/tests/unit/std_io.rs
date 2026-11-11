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
fn logger_levels_have_the_complete_ordered_vocabulary() {
    let levels = [
        Level::Trace,
        Level::Debug,
        Level::Info,
        Level::Warn,
        Level::Error,
        Level::Fatal,
    ];
    let keywords = levels.map(Level::keyword);
    let tags = levels.map(Level::tag);
    assert_eq!(
        keywords,
        ["trace", "debug", "info", "warn", "error", "fatal"]
    );
    assert_eq!(tags, ["TRACE", "DEBUG", "INFO", "WARN", "ERROR", "FATAL"]);
    for (index, keyword) in keywords.into_iter().enumerate() {
        assert_eq!(Level::parse(keyword), Some(levels[index]));
    }
    assert!(levels.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn streams_compose_pointwise_and_have_an_identity() {
    let left =
        Streams::singleton(Stream::Out, "one").compose(Streams::singleton(Stream::Err, "warning"));
    let right =
        Streams::singleton(Stream::Out, "two").compose(Streams::singleton(Stream::Err, "failure"));
    let combined = left.clone().compose(right.clone());

    assert_eq!(combined.project(Stream::Out), ["one", "two"]);
    assert_eq!(combined.project(Stream::Err), ["warning", "failure"]);
    assert_eq!(Streams::empty().compose(left.clone()), left);
    assert_eq!(right.clone().compose(Streams::empty()), right);
}

#[test]
fn streams_composition_is_associative() {
    let a = Streams::singleton(Stream::Out, "a");
    let b = Streams::singleton(Stream::Err, "b");
    let c = Streams::singleton(Stream::Out, "c");
    assert_eq!(
        a.clone().compose(b.clone()).compose(c.clone()),
        a.compose(b.compose(c))
    );
}

/// **The two line kinds share one framing** (card `STRING-SLICE-IO`).
///
/// A rendered scalar line is the text line of its own digits: same stream, same
/// `[LEVEL] ` prefix, same single newline. Stated as an equation so the two
/// cannot drift — a change to either function that did not change the other
/// fails here.
#[test]
fn a_rendered_scalar_line_is_a_text_line_of_its_digits() {
    for rendering in [Rendering::Dec, Rendering::Hex] {
        for value in [0, 1, -7, i32::MIN, i32::MAX] {
            let printed = StdIoEvent::Print {
                rendering,
                operand: StdIoOperand::Literal(value),
            };
            assert_eq!(
                rendered_line(&printed, value),
                Some(rendered_text_line(None, &rendering.render(value)))
            );
            for level in [Level::Trace, Level::Info, Level::Fatal] {
                let logged = StdIoEvent::Log {
                    level,
                    rendering,
                    operand: StdIoOperand::Literal(value),
                };
                assert_eq!(
                    rendered_line(&logged, value),
                    Some(rendered_text_line(Some(level), &rendering.render(value)))
                );
            }
        }
    }
}

/// The text framing writes its body verbatim: no escaping, no terminator, no
/// newline of its own, and an embedded NUL survives as a byte.
#[test]
fn a_text_line_carries_its_body_byte_for_byte() {
    assert_eq!(
        rendered_text_line(None, "HTTP/1.1 200 OK"),
        ObservedLine {
            stream: Stream::Out,
            text: "HTTP/1.1 200 OK".to_owned(),
        }
    );
    assert_eq!(rendered_text_line(None, "").text, "");
    let with_nul = rendered_text_line(Some(Level::Warn), "ab\0cd");
    assert_eq!(with_nul.stream, Stream::Err);
    assert_eq!(with_nul.text.as_bytes(), b"[WARN] ab\x00cd");
    assert!(!with_nul.text.contains('\n'));
}
