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

//! `quirin-audit-libevent`: the `quirin-audit libevent` subcommand plugin.
//! See the library documentation.

use std::process::ExitCode;

fn main() -> ExitCode {
    quirin_audit_plugin::run(quirin_audit_plugin_libevent::answer)
}
