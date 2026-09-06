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
    let mut roots = Vec::new();
    if let Some(root) = std::env::var_os("VSINSTALLDIR") {
        roots.push(PathBuf::from(root));
    }
    let mut locators = Vec::new();
    for variable in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(root) = std::env::var_os(variable) {
            locators
                .push(PathBuf::from(root).join("Microsoft Visual Studio/Installer/vswhere.exe"));
        }
    }
    locators.push(PathBuf::from("vswhere.exe"));
    for locator in locators {
        if let Ok(output) = Command::new(locator)
            .args([
                "-all",
                "-products",
                "*",
                "-prerelease",
                "-utf8",
                "-property",
                "installationPath",
            ])
            .output()
            && output.status.success()
        {
            roots.extend(
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(PathBuf::from),
            );
            break;
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
}
