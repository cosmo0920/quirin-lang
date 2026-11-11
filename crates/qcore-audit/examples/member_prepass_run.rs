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

//! Card MEMBER-PREPASS-OTHER-ROUTES: a development and census aid (not
//! evidence by itself). Runs the member-facts pre-pass alone
//! (`member_prepass::run`, with the profile's scope file re-checked) over a
//! compile database, under the contracts of the profile's own manifest, and
//! writes its evidence section to OUT.json; no unit is analysed.
//!
//! usage: member_prepass_run DATABASE SOURCE_ROOT PROFILE_DIRECTORY OUT.json JOBS

fn main() -> Result<(), String> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example member_prepass_run");
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let [database, source_root, profile, out, jobs] = arguments.as_slice() else {
        return Err(
            "usage: member_prepass_run DATABASE SOURCE_ROOT PROFILE_DIRECTORY OUT.json JOBS".into(),
        );
    };
    let jobs: usize = jobs.parse().map_err(|error| format!("JOBS: {error}"))?;
    let text = std::fs::read_to_string(database).map_err(|error| error.to_string())?;
    let entries = qcore_audit::compile_db::compile_entries(&text)?;
    let profile =
        qcore_contract::profile_loader::load_profile_directory(std::path::Path::new(profile))
            .map_err(|error| error.to_string())?;
    let manifest = profile
        .resource_contract_manifest_path()
        .ok_or("the profile names no resource contract manifest")?;
    let contracts = qcore_contract::contract_loader::load_resource_contract_manifest(&manifest)?;
    let scope = qcore_audit::member_prepass::load_scope(profile.directory())?;
    let started = std::time::Instant::now();
    let prepass = qcore_audit::member_prepass::run(
        &entries,
        &contracts,
        &scope,
        Some(std::path::Path::new(source_root)),
        jobs,
    )
    .ok_or("no declaration invokes a member")?;
    let seconds = started.elapsed().as_secs_f64();
    let mut section = prepass.section;
    section["seconds"] = serde_json::json!(seconds);
    std::fs::write(
        out,
        serde_json::to_vec_pretty(&section).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    println!(
        "{} unit(s), {} read, {} unread, scope {} held / {} dropped, {seconds:.1} s",
        section["units"],
        section["read"],
        prepass.facts.unread.len(),
        section["scope"]["held"],
        section["scope"]["dropped"]
            .as_object()
            .map_or(0, serde_json::Map::len)
    );
    Ok(())
}
