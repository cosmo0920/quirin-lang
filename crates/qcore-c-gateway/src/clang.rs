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

//! Clang is needed only by the existing-C analysis gateway, not the QCore frontend.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, PoisonError};

const INSTALL_HELP: &str = "Existing-C analysis requires Clang (MSVC cl.exe cannot provide the required AST). Install LLVM or Visual Studio's optional 'C++ Clang tools for Windows', then set QCORE_CLANG to the full compiler executable path if discovery fails. QCORE_CLANG is a path, not a command with arguments. QCore check/emit do not require Clang.";

pub fn command() -> Result<Command, String> {
    let path = resolved_once(
        &RESOLVED,
        ResolutionKey {
            explicit: std::env::var_os("QCORE_CLANG"),
            search_path: std::env::var_os("PATH"),
        },
        |explicit| resolve(explicit, discovery_candidates, probe),
    )?;
    let mut command = Command::new(path);
    // Visual Studio may supply clang-cl.exe. Use the GNU-style driver arguments
    // expected by our preprocessing and JSON AST transports in either case.
    command.arg("--driver-mode=gcc");
    Ok(command)
}

/// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the environment [`command`]
/// resolves the executable under (`QCORE_CLANG`, `PATH`), the part of a
/// launch's identity that is not its arguments or input
/// ([`crate::clang_reuse::UnitKey`]).
pub(crate) fn resolution_environment() -> (Option<OsString>, Option<OsString>) {
    (std::env::var_os("QCORE_CLANG"), std::env::var_os("PATH"))
}

/// Trusted description of the Clang executable that actually performs every
/// gateway preprocessing, syntax and typed-AST run.
///
/// This is provenance, not an attestation: the executable and its environment
/// are trusted and must stay stable while a captured identity is in use. It
/// describes the compiler that ran, never the driver a compilation database
/// declared; compiler equivalence is not established anywhere in this crate.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ClangIdentity {
    program: String,
    version: String,
    target: String,
}

/// Parsed release number of the Clang executable selected by [`command`].
///
/// This is deliberately separate from the compiler identity's complete banner:
/// the banner remains provenance, while this value may only select documented
/// frontend spellings and AST-schema compatibility.  Target ABI decisions never
/// depend on the release number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct ClangVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ClangVersion {
    fn parse(banner: &str) -> Result<Self, String> {
        let marker = "clang version ";
        let start = banner
            .find(marker)
            .ok_or("Clang version banner has no `clang version` field")?
            + marker.len();
        let token = banner[start..]
            .split_whitespace()
            .next()
            .ok_or("Clang version banner has no release number")?;
        let mut parts = token.split('.');
        let parse = |part: Option<&str>| {
            part.unwrap_or("0")
                .chars()
                .take_while(|ch| ch.is_ascii_digit())
                .collect::<String>()
                .parse::<u32>()
                .map_err(|_| format!("unreadable Clang release number `{token}`"))
        };
        Ok(Self {
            major: parse(parts.next())?,
            minor: parse(parts.next())?,
            patch: parse(parts.next())?,
        })
    }

    /// Clang 17 and earlier spell the finalized C23 mode `c2x`.
    pub fn c23_argument(self) -> &'static str {
        if self.major >= 18 {
            "-std=c23"
        } else {
            "-std=c2x"
        }
    }

    /// The C2y driver spelling used by the gateway is available from Clang 19.
    pub fn supports_c2y(self) -> bool {
        self.major >= 19
    }

    /// Clang 17's `c2x` mode predates C23's `constexpr` implementation.
    pub fn supports_c23_constexpr(self) -> bool {
        self.major >= 19
    }

    /// CG19's checked atomic AST schema is the Clang 18-and-later schema.
    /// Older Clang emits `AtomicExpr` for the affected builtins instead.
    pub fn supports_cg19_atomic_ast(self) -> bool {
        self.major >= 18
    }
}

impl ClangIdentity {
    /// Construct a provenance record from values captured by an external audit
    /// coordinator. This records a claim; it does not attest the executable or
    /// prove equivalence to a compilation-database driver.
    pub fn recorded(
        program: impl Into<String>,
        version: impl Into<String>,
        target: impl Into<String>,
    ) -> Result<Self, String> {
        let identity = Self {
            program: program.into(),
            version: version.into(),
            target: target.into(),
        };
        if identity.program.is_empty() || identity.version.is_empty() || identity.target.is_empty()
        {
            return Err("recorded Clang identity has an empty program, version or target".into());
        }
        Ok(identity)
    }

    /// Ask the discovered Clang for its own version and default target.
    pub fn capture() -> Result<Self, String> {
        let mut version_command = command()?;
        let program = version_command
            .get_program()
            .to_str()
            .ok_or("non-UTF-8 Clang path")?
            .to_owned();
        let version = version_command
            .arg("--version")
            .output()
            .map_err(|error| error.to_string())?;
        let target = command()?
            .arg("-dumpmachine")
            .output()
            .map_err(|error| error.to_string())?;
        if !version.status.success() || !target.status.success() {
            return Err("Clang could not report its version and target identity".into());
        }
        let version = String::from_utf8(version.stdout)
            .map_err(|error| error.to_string())?
            .trim()
            .to_owned();
        let target = String::from_utf8(target.stdout)
            .map_err(|error| error.to_string())?
            .trim()
            .to_owned();
        if !version.contains("clang version") || target.is_empty() {
            return Err("Clang reported incomplete version or target identity".into());
        }
        Self::recorded(program, version, target)
    }

    /// The resolved executable name or path, exactly as the gateway invokes it.
    pub fn program(&self) -> &str {
        &self.program
    }
    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn target(&self) -> &str {
        &self.target
    }
}

/// The environment a resolution was made under: the explicit override and the
/// search path the default `clang` is looked up on.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolutionKey {
    explicit: Option<OsString>,
    search_path: Option<OsString>,
}

type ResolutionCache = Mutex<Option<(ResolutionKey, PathBuf)>>;
type VersionCache = Mutex<Option<(ResolutionKey, ClangVersion)>>;

/// The last successful resolution in this process.
static RESOLVED: ResolutionCache = Mutex::new(None);
static VERSION: VersionCache = Mutex::new(None);

/// Detect and parse the selected Clang release once per resolution environment.
pub fn detected_version() -> Result<ClangVersion, String> {
    let key = ResolutionKey {
        explicit: std::env::var_os("QCORE_CLANG"),
        search_path: std::env::var_os("PATH"),
    };
    if let Some((cached, version)) = VERSION
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        && *cached == key
    {
        return Ok(*version);
    }
    let output = command()?
        .arg("--version")
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("Clang could not report its release number".into());
    }
    let banner = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    let version = ClangVersion::parse(&banner)?;
    *VERSION.lock().unwrap_or_else(PoisonError::into_inner) = Some((key, version));
    Ok(version)
}

/// Resolve and probe the executable once per [`ResolutionKey`] per process.
///
/// Every gateway run used to re-run discovery, which launches `clang --version`
/// before the real Clang run; that probe was over half of all Clang launches in
/// the heaviest test binaries (card W44-TEST-SUITE-WALL-TIME). Reusing a
/// successful resolution changes no verdict: the executable is already trusted
/// to stay stable while in use (see [`ClangIdentity`]), a later run still fails
/// closed if it cannot be launched or its output does not parse, a changed
/// `QCORE_CLANG` or `PATH` is resolved afresh, and a failed resolution is never
/// cached, so an unavailable Clang is re-examined on every call.
fn resolved_once(
    cache: &ResolutionCache,
    key: ResolutionKey,
    resolve: impl FnOnce(Option<OsString>) -> Result<PathBuf, String>,
) -> Result<PathBuf, String> {
    if let Some((cached, path)) = cache
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        && *cached == key
    {
        return Ok(path.clone());
    }
    let path = resolve(key.explicit.clone())?;
    *cache.lock().unwrap_or_else(PoisonError::into_inner) = Some((key, path.clone()));
    Ok(path)
}

fn resolve(
    explicit: Option<OsString>,
    discover: impl FnOnce() -> Vec<PathBuf>,
    mut probe: impl FnMut(&Path) -> Result<(), String>,
) -> Result<PathBuf, String> {
    if let Some(value) = explicit {
        if value.is_empty() {
            return Err(format!("QCORE_CLANG is empty. {INSTALL_HELP}"));
        }
        let path = PathBuf::from(value);
        // An explicit setting is authoritative: never hide a typo by falling back.
        return probe(&path).map(|()| path.clone()).map_err(|error| {
            format!(
                "QCORE_CLANG `{}` is unavailable: {error}. {INSTALL_HELP}",
                path.display()
            )
        });
    }
    let mut failures = Vec::new();
    // Avoid running vswhere when Clang is already available on PATH.
    let path = PathBuf::from("clang");
    match probe(&path) {
        Ok(()) => return Ok(path),
        Err(error) => failures.push(format!("{}: {error}", path.display())),
    }
    for path in discover() {
        match probe(&path) {
            Ok(()) => return Ok(path),
            Err(error) => failures.push(format!("{}: {error}", path.display())),
        }
    }
    Err(format!(
        "Clang unavailable. {INSTALL_HELP} Tried: {}",
        failures.join("; ")
    ))
}

fn probe(path: &Path) -> Result<(), String> {
    let output = Command::new(path)
        .args(["--driver-mode=gcc", "--version"])
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() && String::from_utf8_lossy(&output.stdout).contains("clang version")
    {
        Ok(())
    } else {
        Err("executable did not report a successful Clang version".into())
    }
}

#[cfg(not(windows))]
fn discovery_candidates() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(windows)]
fn discovery_candidates() -> Vec<PathBuf> {
    let program_files = ["ProgramFiles", "ProgramFiles(x86)"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    windows_candidates(
        std::env::var_os("VSINSTALLDIR").map(PathBuf::from),
        &program_files,
        |locator| {
            let output = Command::new(locator)
                .args([
                    "-all",
                    "-products",
                    "*",
                    "-prerelease",
                    "-sort",
                    "-utf8",
                    "-property",
                    "installationPath",
                ])
                .output()
                .ok()?;
            output
                .status
                .success()
                .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
        },
    )
}

#[cfg(any(windows, test))]
fn windows_candidates(
    vs_install_dir: Option<PathBuf>,
    program_files: &[PathBuf],
    mut query: impl FnMut(&Path) -> Option<String>,
) -> Vec<PathBuf> {
    let mut roots = vs_install_dir.into_iter().collect::<Vec<_>>();
    let locators = program_files
        .iter()
        .map(|root| root.join("Microsoft Visual Studio/Installer/vswhere.exe"))
        .chain([PathBuf::from("vswhere.exe")]);
    for locator in locators {
        if let Some(output) = query(&locator) {
            let installations = output
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(PathBuf::from)
                .collect::<Vec<_>>();
            if !installations.is_empty() {
                roots.extend(installations);
                break;
            }
        }
    }
    // A missing/stale locator must not hide a standard installation. VS 2026
    // uses the major-version directory `18`; VS 2022 uses the year `2022`.
    // Enumerate editions so Community, Professional, Enterprise and Build Tools
    // are all supported, without requiring a developer-shell environment.
    for version in ["18", "2022"] {
        for root in program_files {
            let directory = root.join("Microsoft Visual Studio").join(version);
            if let Ok(entries) = std::fs::read_dir(directory) {
                let mut editions = entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| path.is_dir())
                    .collect::<Vec<_>>();
                editions.sort();
                roots.extend(editions);
            }
        }
    }
    let mut paths = Vec::new();
    for root in roots {
        for path in visual_studio_candidates(&root) {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }
    paths
}

#[cfg(any(windows, test))]
fn visual_studio_candidates(root: &Path) -> Vec<PathBuf> {
    [
        // Prefer a native ARM64 host tool when present on Windows on Arm.
        #[cfg(target_arch = "aarch64")]
        "VC/Tools/Llvm/ARM64/bin",
        "VC/Tools/Llvm/x64/bin",
        "VC/Tools/Llvm/bin",
        "Common7/IDE/CommonExtensions/Microsoft/Llvm/bin",
    ]
    .into_iter()
    .flat_map(|directory| ["clang.exe", "clang-cl.exe"].map(|name| root.join(directory).join(name)))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(explicit: Option<&str>, search_path: &str) -> ResolutionKey {
        ResolutionKey {
            explicit: explicit.map(OsString::from),
            search_path: Some(search_path.into()),
        }
    }

    #[test]
    fn a_successful_resolution_is_reused_only_under_the_same_environment() {
        let cache = ResolutionCache::new(None);
        let mut calls = 0;
        let mut resolve = |explicit: Option<OsString>| {
            calls += 1;
            Ok(PathBuf::from(explicit.unwrap_or_else(|| "clang".into())))
        };
        let first = resolved_once(&cache, key(Some("/a/clang"), "/bin"), &mut resolve).unwrap();
        let again = resolved_once(&cache, key(Some("/a/clang"), "/bin"), &mut resolve).unwrap();
        assert_eq!((first.clone(), again), (PathBuf::from("/a/clang"), first));
        let other = resolved_once(&cache, key(Some("/b/clang"), "/bin"), &mut resolve).unwrap();
        assert_eq!(other, PathBuf::from("/b/clang"));
        let searched = resolved_once(&cache, key(None, "/bin"), &mut resolve).unwrap();
        let moved = resolved_once(&cache, key(None, "/opt/bin"), &mut resolve).unwrap();
        assert_eq!(
            (searched, moved),
            (PathBuf::from("clang"), PathBuf::from("clang"))
        );
        assert_eq!(
            calls, 4,
            "one probe per distinct environment, none for the repeat"
        );
    }

    #[test]
    fn a_failed_resolution_is_never_cached_or_masked() {
        let cache = ResolutionCache::new(None);
        resolved_once(&cache, key(Some("/good"), "/bin"), |_| Ok("/good".into())).unwrap();
        // A different override that fails reports its own failure, never the
        // earlier success.
        let error = resolved_once(&cache, key(Some("missing"), "/bin"), |_| {
            Err("QCORE_CLANG `missing` is unavailable".into())
        })
        .unwrap_err();
        assert!(error.contains("missing"));
        // A failure is re-examined on the next call rather than remembered.
        let mut calls = 0;
        for _ in 0..2 {
            assert!(
                resolved_once(&cache, key(None, "/empty"), |_| {
                    calls += 1;
                    Err("Clang unavailable".into())
                })
                .is_err()
            );
        }
        assert_eq!(calls, 2);
    }

    #[test]
    fn explicit_path_with_spaces_is_authoritative() {
        let expected = PathBuf::from("custom LLVM/bin/clang.exe");
        let result = resolve(
            Some(expected.clone().into_os_string()),
            || panic!("must not discover"),
            |path| {
                assert_eq!(path, expected);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(result, expected);
        let error = resolve(
            Some("missing".into()),
            || panic!("must not fall back"),
            |_| Err("not found".into()),
        )
        .unwrap_err();
        assert!(error.contains("QCORE_CLANG `missing`"));
        assert!(error.contains("C++ Clang tools for Windows"));
    }

    #[test]
    fn empty_override_is_an_error() {
        assert!(
            resolve(Some(OsString::new()), || panic!(), |_| panic!())
                .unwrap_err()
                .contains("QCORE_CLANG is empty")
        );
    }

    #[test]
    fn path_precedes_visual_studio_and_discovery_checks_all_candidates() {
        assert_eq!(
            resolve(None, || panic!("PATH succeeded"), |_| Ok(())).unwrap(),
            Path::new("clang")
        );
        let candidates = visual_studio_candidates(Path::new("Visual Studio/BuildTools"));
        let expected = candidates.last().unwrap().clone();
        let mut attempted = Vec::new();
        let result = resolve(
            None,
            || candidates.clone(),
            |path| {
                attempted.push(path.to_path_buf());
                if path == expected {
                    Ok(())
                } else {
                    Err("missing".into())
                }
            },
        )
        .unwrap();
        assert_eq!(result, expected);
        assert_eq!(
            attempted,
            [vec![PathBuf::from("clang")], candidates].concat()
        );
    }

    #[test]
    fn absent_clang_reports_installation_and_configuration_help() {
        let error = resolve(None, Vec::new, |_| Err("not found".into())).unwrap_err();
        assert!(error.contains("QCORE_CLANG"));
        assert!(error.contains("MSVC cl.exe cannot provide the required AST"));
    }

    #[test]
    fn vswhere_supports_2026_and_2022_custom_roots_and_environment_precedence() {
        let root_2026 = PathBuf::from("custom VS/18/Community");
        let root_2022 = PathBuf::from("custom VS/2022/BuildTools");
        let candidates = windows_candidates(Some(root_2022.clone()), &[], |_| {
            Some(format!(
                "\r\n {}\r\n{}\r\n",
                root_2026.display(),
                root_2022.display()
            ))
        });
        assert_eq!(
            candidates,
            [
                visual_studio_candidates(&root_2022),
                visual_studio_candidates(&root_2026)
            ]
            .concat()
        );
        // The active/older installation may not have the optional Clang component.
        let expected = root_2026.join("VC/Tools/Llvm/x64/bin/clang.exe");
        assert_eq!(
            resolve(
                None,
                || candidates,
                |path| {
                    if path == expected {
                        Ok(())
                    } else {
                        Err("missing".into())
                    }
                }
            )
            .unwrap(),
            expected
        );
    }

    #[test]
    fn standard_2026_and_2022_installations_work_without_vswhere() {
        let fixture =
            std::env::temp_dir().join(format!("qcore-vs-discovery-{}", std::process::id()));
        let program_files = fixture.join("Program Files");
        let root_2026 = program_files.join("Microsoft Visual Studio/18/Community");
        let root_2022 = program_files.join("Microsoft Visual Studio/2022/BuildTools");
        std::fs::create_dir_all(&root_2026).unwrap();
        std::fs::create_dir_all(&root_2022).unwrap();
        for response in [None, Some("\r\n  \r\n".to_owned())] {
            let candidates = windows_candidates(None, std::slice::from_ref(&program_files), |_| {
                response.clone()
            });
            for root in [&root_2026, &root_2022] {
                let expected = root.join("VC/Tools/Llvm/x64/bin/clang.exe");
                assert_eq!(
                    resolve(
                        None,
                        || candidates.clone(),
                        |path| {
                            if path == expected {
                                Ok(())
                            } else {
                                Err("missing".into())
                            }
                        }
                    )
                    .unwrap(),
                    expected
                );
            }
            assert_eq!(
                candidates,
                [
                    visual_studio_candidates(&root_2026),
                    visual_studio_candidates(&root_2022)
                ]
                .concat()
            );
        }
        std::fs::remove_dir_all(fixture).unwrap();
    }

    #[test]
    fn empty_locator_output_does_not_hide_later_installations() {
        let root = PathBuf::from("custom VS/18/Enterprise");
        let mut queries = 0;
        let candidates =
            windows_candidates(None, &[PathBuf::from("missing Program Files")], |_| {
                queries += 1;
                Some(if queries == 1 {
                    String::new()
                } else {
                    root.display().to_string()
                })
            });
        assert_eq!(queries, 2);
        assert_eq!(candidates, visual_studio_candidates(&root));
    }

    #[test]
    fn release_numbers_are_parsed_without_confusing_vendor_text() {
        assert_eq!(
            ClangVersion::parse("clang version 17.0.3\nTarget: aarch64-pc-windows-msvc\n").unwrap(),
            ClangVersion {
                major: 17,
                minor: 0,
                patch: 3
            }
        );
        assert_eq!(
            ClangVersion::parse("Apple clang version 21.1.0 (clang-2110.0.0.1)").unwrap(),
            ClangVersion {
                major: 21,
                minor: 1,
                patch: 0
            }
        );
        assert!(ClangVersion::parse("clang development snapshot").is_err());
    }

    #[test]
    fn version_effects_are_limited_to_frontend_spellings_and_atomic_schema() {
        let old = ClangVersion {
            major: 17,
            minor: 0,
            patch: 6,
        };
        let current = ClangVersion {
            major: 21,
            minor: 1,
            patch: 0,
        };
        assert_eq!(old.c23_argument(), "-std=c2x");
        assert_eq!(current.c23_argument(), "-std=c23");
        assert!(!old.supports_c2y());
        assert!(current.supports_c2y());
        assert!(!old.supports_c23_constexpr());
        assert!(current.supports_c23_constexpr());
        assert!(!old.supports_cg19_atomic_ast());
        assert!(current.supports_cg19_atomic_ast());
    }
}
