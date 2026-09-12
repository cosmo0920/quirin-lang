//! The command lines and premise output published by `examples/`, executed
//! (card `EXAMPLES-SUITE`, wave 21).
//!
//! `examples/unsafe-linkage/README.md` prints two premise listings and tells a
//! reader they are what `quirin check` says. This file runs the driver over the
//! published example and the published manifest and compares its stdout with
//! the blocks read out of that page, so the page cannot drift.
//!
//! It also pins the CLI boundary the other pages state: the driver selects no
//! extern contract environment, so the contracted examples are refused here at
//! `Q023` rather than admitted on some default. That is a fail-closed boundary
//! with a named follow-on (`UNSAFE-LINKAGE-CLI`), and stating it in a README is
//! only worth doing if something checks that it is still true.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The driver binary under test: `quirin` (card QUIRIN-RENAME; the crate
/// stays `qcorec`). The pre-merge two-name fallback is gone because `env!`
/// resolves even in the arm the match never takes, so it fails to compile
/// exactly when the fallback name does not exist.
const DRIVER: &str = env!("CARGO_BIN_EXE_quirin");

const UNSAFE_README: &str = include_str!("../../../examples/unsafe-linkage/README.md");

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// The fenced block immediately following `<!-- MARKER -->` in a README.
fn block(readme: &str, marker: &str) -> String {
    let readme = readme.replace("\r\n", "\n");
    let tail = readme
        .split_once(&format!("<!-- {marker} -->\n"))
        .unwrap_or_else(|| panic!("the README no longer carries the marker `{marker}`"))
        .1;
    let body = tail
        .split_once("```")
        .unwrap_or_else(|| panic!("`{marker}` is not followed by a fenced block"))
        .1;
    let body = body
        .split_once('\n')
        .unwrap_or_else(|| panic!("`{marker}`'s fence is unterminated"))
        .1;
    body.split_once("```")
        .unwrap_or_else(|| panic!("`{marker}`'s fenced block is unterminated"))
        .0
        .to_owned()
}

fn check(arguments: &[PathBuf]) -> std::process::Output {
    let mut command = Command::new(DRIVER);
    command.arg("check");
    for argument in arguments {
        command.arg(argument);
    }
    command.output().unwrap()
}

/// The bare waiver listing the page publishes.
#[test]
fn the_page_publishes_the_premises_the_driver_prints() {
    let output = check(&[examples().join("unsafe-linkage/unchecked_calls.qcore")]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        block(UNSAFE_README, "cli: bare"),
        "the page's bare premise listing is not what the driver printed"
    );
    // Non-vacuous: five premises, three of them named symbols.
    let documented = block(UNSAFE_README, "cli: bare");
    assert!(documented.contains("under 5 declared assumption(s):"));
    assert_eq!(documented.matches("unchecked linkage:").count(), 3);
}

/// The same waiver, sharpened by the published manifest.
#[test]
fn selecting_the_published_manifest_names_the_build_the_waiver_came_from() {
    let mut command = Command::new(DRIVER);
    let output = command
        .arg("check")
        .arg("--native-library")
        .arg(examples().join("unsafe-linkage/unchecked_demo.qcorelib"))
        .arg(examples().join("unsafe-linkage/unchecked_calls.qcore"))
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        block(UNSAFE_README, "cli: manifest"),
        "the page's manifest premise listing is not what the driver printed"
    );

    // The count is unchanged: a manifest sharpens premises, it discharges none.
    let documented = block(UNSAFE_README, "cli: manifest");
    assert!(documented.contains("under 5 declared assumption(s):"));
    // And it says what it is: a declaration, not a verification.
    assert_eq!(
        documented
            .matches("the artifact identity is declared, not verified here")
            .count(),
        3
    );
    assert!(documented.contains("libqcore_unchecked_demo.a"));
}

/// The examples whose pages say they check through the CLI, checking through
/// the CLI.
#[test]
fn every_example_the_pages_say_checks_through_the_driver_does() {
    for example in [
        "stdio/renderings.qcore",
        "stdio/severities.qcore",
        "stdio/barrier.qcore",
        "pipes/rendered_lines.qcore",
        "branching/conditional_release.qcore",
        "branching/condition_tier.qcore",
        "branching/early_return.qcore",
        "unsafe-linkage/unchecked_calls.qcore",
    ] {
        let output = check(&[examples().join(example)]);
        assert!(
            output.status.success(),
            "{example}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .starts_with("checked 1 function(s); entry `main`"),
            "{example}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

/// …and the boundary the other pages state, still true.
#[test]
fn the_contracted_examples_are_refused_by_the_driver_rather_than_admitted() {
    for example in [
        "stdio/pipeline.qcore",
        "pipes/scalar_stages.qcore",
        "extern/buffer_round_trip.qcore",
        "extern/scalar_results.qcore",
        "extern/borrowed_render.qcore",
    ] {
        let output = check(&[examples().join(example)]);
        assert!(
            !output.status.success(),
            "{example} must be refused, not admitted on a default environment"
        );
        let rendered = String::from_utf8_lossy(&output.stderr);
        assert!(
            rendered.contains("Q023") && rendered.contains("there is no default environment"),
            "{example}: {rendered}"
        );
    }
}

/// Every `quirin` command line any family page publishes names a file that
/// exists, and every published `quirin check` really checks (card
/// `EXAMPLES-SUITE-WIRE`, wave 26).
///
/// The pages added after wave 21 print driver command lines of their own —
/// `examples/sockets/README.md` and `examples/http/README.md` each open their
/// "Building and running" section with one — and until now nothing ran them. A
/// page that tells a reader to type something is worth exactly as much as the
/// last time somebody typed it.
///
/// The pages are read off disk rather than `include_str!`'d one by one, for the
/// same reason the suite's directory scan exists: a list cannot notice a page
/// nobody added it to.
#[test]
fn every_published_driver_command_line_names_a_real_file_and_works() {
    let root = examples().parent().unwrap().to_owned();
    let mut checked = 0;
    let mut emitted = 0;
    let mut pages: Vec<PathBuf> = std::fs::read_dir(examples())
        .expect("examples/ is readable")
        .map(|entry| entry.expect("a readable entry").path())
        .filter(|path| path.is_dir())
        .map(|path| path.join("README.md"))
        .collect();
    pages.sort();
    pages.push(examples().join("README.md"));

    for page in pages {
        let text = std::fs::read_to_string(&page)
            .unwrap_or_else(|_| panic!("{} is readable", page.display()));
        for line in text.lines().map(str::trim) {
            let Some(rest) = line.strip_prefix("quirin ") else {
                continue;
            };
            let mut words = rest.split_whitespace();
            let (Some(verb), Some(argument)) = (words.next(), words.next()) else {
                continue;
            };
            if !argument.ends_with(".qcore") {
                continue;
            }
            let source = root.join(argument);
            assert!(
                source.exists(),
                "{} publishes `{line}`, whose input does not exist",
                page.display()
            );
            match verb {
                "check" => {
                    let output = check(&[source]);
                    assert!(
                        output.status.success(),
                        "{} publishes `{line}`, which fails: {}",
                        page.display(),
                        String::from_utf8_lossy(&output.stderr)
                    );
                    checked += 1;
                }
                "emit" => emitted += 1,
                _ => {}
            }
        }
    }
    // Non-vacuous, and the counts are the measurement rather than a guard: six
    // pages publish a `quirin check` line and six publish a `quirin emit` line —
    // stdio, pipes, branching, unsafe-linkage, sockets and http. All six checks
    // pass; none of the six was executed by anything before this test. A page
    // that adds or drops a command line moves these numbers, which is the moment
    // to look at it.
    assert_eq!(checked, 6, "published `quirin check` command lines");
    assert_eq!(emitted, 6, "published `quirin emit` command lines");
    // The one command line this scan deliberately does not run is
    // `unsafe-linkage`'s `--native-library` form, whose second word is a flag:
    // the two tests above it execute that one and compare its whole stdout.
    assert!(
        std::fs::read_to_string(examples().join("unsafe-linkage/README.md"))
            .unwrap()
            .contains("quirin check --native-library")
    );
}

/// The emit half of the published command lines, for one example, end to end.
#[test]
fn the_published_emit_command_produces_a_translation_unit() {
    let directory = std::env::temp_dir().join(format!("qcore-examples-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir(&directory).unwrap();
    let generated = directory.join("renderings.c");
    let output = Command::new(DRIVER)
        .arg("emit")
        .arg(examples().join("stdio/renderings.qcore"))
        .arg("-o")
        .arg(&generated)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let c = std::fs::read_to_string(&generated).unwrap();
    assert!(c.contains("QCORE_IO_WRITE("), "{c}");
    std::fs::remove_dir_all(&directory).unwrap();
}
