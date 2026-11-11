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

//! Explicit host declarations for fixtures read by the lexical ABI checker.
//! Unlike Clang's AST, that reader does not expand `__SIZE_TYPE__`.

#[cfg(all(windows, target_pointer_width = "64"))]
macro_rules! size_t_declaration {
    () => {
        "typedef unsigned long long size_t;\n"
    };
}

#[cfg(all(not(windows), target_pointer_width = "64"))]
macro_rules! size_t_declaration {
    () => {
        "typedef unsigned long size_t;\n"
    };
}

#[cfg(target_pointer_width = "32")]
macro_rules! size_t_declaration {
    () => {
        "typedef unsigned int size_t;\n"
    };
}

pub(crate) use size_t_declaration;
