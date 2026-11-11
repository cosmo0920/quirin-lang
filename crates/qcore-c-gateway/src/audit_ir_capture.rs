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

//! Thread-scoped observational CFG capture. Nothing here decides a verdict.
//! A failed capture is latched and returned to the audit owner after its work;
//! partial objects have no completed-unit index. Uncaptured targets stay named.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::audit_ir_store::{FORMAT, FragmentContext, FragmentStore, ObjectId};
use crate::legacy_c::{
    LegacyCError, LegacyCfg, ResourceContracts, ResourceFunctionContract, Token,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildReference {
    pub function: String,
    pub context: FragmentContext,
    pub object: ObjectId,
    pub is_cfg: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetCapture {
    pub function: String,
    pub cfgs: Vec<ObjectId>,
    pub refusals: Vec<ObjectId>,
    pub not_captured: bool,
}

/// All selected targets, including refusals and targets with no CFG attempt.
/// This index never supplies safety coverage or authenticates its own context.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureIndex {
    pub format: String,
    pub requires_fresh_lowering: bool,
    pub unit: ObjectId,
    pub targets: Vec<TargetCapture>,
    pub builds: Vec<BuildReference>,
}

#[derive(Debug)]
pub struct CaptureReceipt {
    pub unit: ObjectId,
    pub index: ObjectId,
}

struct Capture {
    store: FragmentStore,
    unit: ObjectId,
    builds: HashMap<ObjectId, BuildReference>,
    error: Option<String>,
}

thread_local! {
    static ACTIVE: RefCell<Option<Capture>> = const { RefCell::new(None) };
}

/// Run unchanged analysis while recording its CFG constructions. The caller
/// supplies independently loaded unit context and the complete selected target
/// list. No ambient environment is consulted by the gateway capture itself.
pub fn with_capture<T, C: Serialize>(
    store: FragmentStore,
    unit_context: &C,
    selected_targets: &[String],
    work: impl FnOnce() -> Result<T, String>,
) -> Result<(T, CaptureReceipt), String> {
    if ACTIVE.with(|cell| cell.borrow().is_some()) {
        return Err("nested IR capture is not supported".into());
    }
    if selected_targets.iter().collect::<HashSet<_>>().len() != selected_targets.len() {
        return Err("duplicate selected target in IR capture".into());
    }
    #[derive(Serialize)]
    struct Unit<'a, C> {
        format: &'static str,
        requires_fresh_lowering: bool,
        context: &'a C,
        selected_targets: &'a [String],
    }
    let unit = store.write_object(&Unit {
        format: FORMAT,
        requires_fresh_lowering: true,
        context: unit_context,
        selected_targets,
    })?;
    ACTIVE.with(|cell| {
        *cell.borrow_mut() = Some(Capture {
            store,
            unit,
            builds: HashMap::new(),
            error: None,
        })
    });
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            ACTIVE.with(|cell| {
                cell.borrow_mut().take();
            });
        }
    }
    let _reset = Reset;
    let result = work();
    let capture = ACTIVE.with(|cell| cell.borrow_mut().take().expect("active capture"));
    let value = result?;
    if let Some(error) = capture.error {
        return Err(format!("audit IR capture failed: {error}"));
    }
    let mut builds = capture.builds.into_values().collect::<Vec<_>>();
    builds.sort_by(|a, b| a.object.cmp(&b.object));
    let mut by_function: HashMap<&str, Vec<&BuildReference>> = HashMap::new();
    for build in &builds {
        by_function.entry(&build.function).or_default().push(build);
    }
    let targets = selected_targets
        .iter()
        .map(|function| {
            let records = by_function.get(function.as_str());
            let cfgs = records
                .into_iter()
                .flatten()
                .filter(|r| r.is_cfg)
                .map(|r| r.object.clone())
                .collect::<Vec<_>>();
            let refusals = records
                .into_iter()
                .flatten()
                .filter(|r| !r.is_cfg)
                .map(|r| r.object.clone())
                .collect::<Vec<_>>();
            TargetCapture {
                function: function.clone(),
                not_captured: cfgs.is_empty() && refusals.is_empty(),
                cfgs,
                refusals,
            }
        })
        .collect();
    let index = CaptureIndex {
        format: FORMAT.into(),
        requires_fresh_lowering: true,
        unit: capture.unit.clone(),
        targets,
        builds,
    };
    for build in &index.builds {
        capture.store.checked_bytes(&build.object)?;
        capture
            .store
            .check_context(&build.context, &build.function)?;
    }
    let id = capture.store.publish_unit(&capture.unit, &index)?;
    Ok((
        value,
        CaptureReceipt {
            unit: capture.unit,
            index: id,
        },
    ))
}

/// Record the actual input to a construction, before its source rewrites.
/// Rewritten recursion contexts therefore cannot alias the ordinary body.
pub(crate) fn record_build(
    function: &str,
    tokens: &[Token],
    contracts: &ResourceContracts,
    admit_derived_returns: bool,
    own: Option<&ResourceFunctionContract>,
    result: &Result<LegacyCfg, LegacyCError>,
) {
    ACTIVE.with(|cell| {
        let mut active = cell.borrow_mut();
        let Some(capture) = active.as_mut() else {
            return;
        };
        if capture.error.is_some() {
            return;
        }
        if let Err(error) = capture.record(
            function,
            tokens,
            contracts,
            admit_derived_returns,
            own,
            result,
        ) {
            capture.error = Some(error);
        }
    });
}

impl Capture {
    fn record(
        &mut self,
        function: &str,
        tokens: &[Token],
        contracts: &ResourceContracts,
        admit_derived_returns: bool,
        own: Option<&ResourceFunctionContract>,
        result: &Result<LegacyCfg, LegacyCError>,
    ) -> Result<(), String> {
        #[derive(Serialize)]
        struct Construction<'a> {
            format: &'static str,
            function: &'a str,
            tokens: ObjectId,
            contracts: ObjectId,
            admit_derived_returns: bool,
            own: Option<&'a ResourceFunctionContract>,
        }
        let construction = self.store.write_object(&Construction {
            format: FORMAT,
            function,
            tokens: self.store.write_object(&tokens)?,
            contracts: self.store.write_object(contracts)?,
            admit_derived_returns,
            own,
        })?;
        let context = FragmentContext {
            unit: self.unit.clone(),
            construction,
        };
        let object = match result {
            Ok(cfg) => self.store.write_cfg(&context, cfg)?,
            Err(error) => {
                #[derive(Serialize)]
                struct Refusal<'a> {
                    format: &'static str,
                    context: &'a FragmentContext,
                    function: &'a str,
                    refusal: String,
                }
                self.store.write_object(&Refusal {
                    format: FORMAT,
                    context: &context,
                    function,
                    refusal: error.to_string(),
                })?
            }
        };
        self.builds
            .entry(object.clone())
            .or_insert_with(|| BuildReference {
                function: function.into(),
                context,
                object,
                is_cfg: result.is_ok(),
            });
        Ok(())
    }
}
