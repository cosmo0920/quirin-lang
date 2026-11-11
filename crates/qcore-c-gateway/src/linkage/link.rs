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

//! The build's half of the pin (card `W48-QRLIB-LINKAGE`, user decision 5):
//! immediately before linking, every file a generated manifest pins is hashed
//! again, and the linker is handed exactly the artifact the manifest names —
//! never a library search (`-l`), never another path.
//!
//! For an `ld` script the script is re-read and must name exactly the pinned
//! inputs, in order and with the same `AS_NEEDED` marks, each hashing to its
//! pinned digest. A dynamic link's runtime library is not hashed: the OS owns
//! it, and the manifest states its identity as a premise.

use std::fmt;
use std::path::PathBuf;

use qcore_core::native_library::NativeLibraryManifest;

use super::ld_script;

/// Why a link is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkRefusal {
    /// The manifest is a hand-written v1 one, with no linkage section.
    NotGenerated { library: String },
    /// A pinned file cannot be read.
    Unreadable { reference: String, reason: String },
    /// A pinned file's bytes no longer hash to the pinned digest.
    DigestMismatch {
        reference: String,
        pinned: String,
        measured: String,
    },
    /// The artifact's script no longer names exactly the pinned inputs.
    ScriptDiffers { reference: String, reason: String },
    /// The build asked to link a file other than the pinned artifact.
    OtherArtifact { pinned: String, requested: String },
}

impl fmt::Display for LinkRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotGenerated { library } => write!(
                f,
                "`{library}`'s manifest is a hand-written v1 manifest; a link is made only against a generated one"
            ),
            Self::Unreadable { reference, reason } => {
                write!(f, "the pinned file `{reference}` cannot be read: {reason}")
            }
            Self::DigestMismatch {
                reference,
                pinned,
                measured,
            } => write!(
                f,
                "the pinned file `{reference}` hashes to {measured}, not the pinned {pinned}; the build does not link a different artifact"
            ),
            Self::ScriptDiffers { reference, reason } => {
                write!(f, "the pinned script `{reference}` {reason}")
            }
            Self::OtherArtifact { pinned, requested } => write!(
                f,
                "the build asked to link `{requested}`, and the manifest pins `{pinned}`; only the pinned artifact is linked"
            ),
        }
    }
}

impl std::error::Error for LinkRefusal {}

/// A verified link: the one path to hand the linker, and every digest checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkPlan {
    pub library: String,
    /// The artifact's resolved path: the only library input of the link.
    pub input: PathBuf,
    /// `(reference, digest)` of every file verified, the artifact first.
    pub verified: Vec<(String, String)>,
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", qcore_schemas::digest::sha256_hex(bytes))
}

/// Verify a generated manifest's pins against the files on disk.
///
/// `resolve` maps a recorded reference to the path it names (a relative
/// artifact against the manifest's own directory; a script input is absolute).
///
/// # Errors
///
/// A [`LinkRefusal`].
pub fn plan(
    manifest: &NativeLibraryManifest,
    resolve: &dyn Fn(&str) -> PathBuf,
) -> Result<LinkPlan, LinkRefusal> {
    let library = manifest.identity.library.clone();
    let Some(linkage) = &manifest.linkage else {
        return Err(LinkRefusal::NotGenerated { library });
    };
    let read = |reference: &str| -> Result<Vec<u8>, LinkRefusal> {
        std::fs::read(resolve(reference)).map_err(|error| LinkRefusal::Unreadable {
            reference: reference.to_owned(),
            reason: error.to_string(),
        })
    };
    let check = |reference: &str, bytes: &[u8], pinned: &str| -> Result<(), LinkRefusal> {
        let measured = digest(bytes);
        if measured == pinned {
            Ok(())
        } else {
            Err(LinkRefusal::DigestMismatch {
                reference: reference.to_owned(),
                pinned: pinned.to_owned(),
                measured,
            })
        }
    };
    let artifact = &manifest.identity.artifact;
    let bytes = read(artifact)?;
    check(artifact, &bytes, &manifest.identity.hash)?;
    let mut verified = vec![(artifact.clone(), manifest.identity.hash.clone())];
    let is_script = ld_script::is_ld_script(&bytes);
    if is_script || !linkage.inputs.is_empty() {
        let named = if is_script {
            ld_script::read(&bytes).map_err(|error| LinkRefusal::ScriptDiffers {
                reference: artifact.clone(),
                reason: format!("no longer reads: {error}"),
            })?
        } else {
            Vec::new()
        };
        let pinned: Vec<(&str, bool)> = linkage
            .inputs
            .iter()
            .map(|input| (input.reference.as_str(), input.as_needed))
            .collect();
        let found: Vec<(&str, bool)> = named
            .iter()
            .map(|input| (input.path.as_str(), input.as_needed))
            .collect();
        if pinned != found {
            return Err(LinkRefusal::ScriptDiffers {
                reference: artifact.clone(),
                reason: "no longer names exactly the pinned inputs".into(),
            });
        }
        for input in &linkage.inputs {
            let bytes = read(&input.reference)?;
            check(&input.reference, &bytes, &input.digest)?;
            verified.push((input.reference.clone(), input.digest.clone()));
        }
    }
    Ok(LinkPlan {
        library,
        input: resolve(artifact),
        verified,
    })
}

impl LinkPlan {
    /// Refuse any library input but the pinned artifact.
    ///
    /// # Errors
    ///
    /// [`LinkRefusal::OtherArtifact`].
    pub fn admit(&self, requested: &std::path::Path) -> Result<(), LinkRefusal> {
        if requested == self.input {
            Ok(())
        } else {
            Err(LinkRefusal::OtherArtifact {
                pinned: self.input.display().to_string(),
                requested: requested.display().to_string(),
            })
        }
    }
}
