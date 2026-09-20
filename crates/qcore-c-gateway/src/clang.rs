//! Clang is needed only by the existing-C analysis gateway, not the QCore frontend.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

const INSTALL_HELP: &str = "Existing-C analysis requires Clang (MSVC cl.exe cannot provide the required AST). Install LLVM or Visual Studio's optional 'C++ Clang tools for Windows', then set QCORE_CLANG to the full compiler executable path if discovery fails. QCORE_CLANG is a path, not a command with arguments. QCore check/emit do not require Clang.";

pub fn command() -> Result<Command, String> {
    let path = resolve(std::env::var_os("QCORE_CLANG"), discovery_candidates, probe)?;
    let mut command = Command::new(path);
    // Visual Studio may supply clang-cl.exe. Use the GNU-style driver arguments
    // expected by our preprocessing and JSON AST transports in either case.
    command.arg("--driver-mode=gcc");
    Ok(command)
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
}
