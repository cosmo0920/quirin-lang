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

//! Card **AG2-UNRESOLVED-ALIAS** (gateway rule 62): the parser's adapter for
//! [`crate::custody_free_copy`]. Asked once per function, after the CFG is
//! built, and only where some `Alias` node copies a source the traversal
//! exemption does not already cover (a literal, a formal, or a name the
//! declaration reader declares neither scalar nor pointer), so a function
//! with no such copy asks Clang for nothing more.

use std::collections::BTreeSet;

use super::{LegacyCfg, LegacyInstruction, Parser};
use crate::custody_free_copy::{CustodyFreeCopies, read};

impl Parser<'_> {
    pub(super) fn custody_free_copies(
        &self,
        cfg: &LegacyCfg,
        scalars: &BTreeSet<String>,
        pointers: &BTreeSet<String>,
    ) -> CustodyFreeCopies {
        let asked = cfg.nodes.iter().any(|node| match &node.instruction {
            LegacyInstruction::Alias { source, .. } => {
                !(scalars.contains(source) || pointers.contains(source))
            }
            _ => false,
        });
        if !asked {
            return CustodyFreeCopies::default();
        }
        let Some(index) = self.typed_positions_index() else {
            return CustodyFreeCopies::default();
        };
        let function = std::sync::Arc::clone(&index.function);
        read(
            &function,
            &|name| self.position_pointer_root(name),
            &|name| self.position_never_bound(name),
        )
    }
}
