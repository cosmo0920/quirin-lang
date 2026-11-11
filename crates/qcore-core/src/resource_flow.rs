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

use std::collections::BTreeMap;
use std::fmt;

use crate::Footprint;

/// A nonnegative, multidimensional resource quantity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Quantity(BTreeMap<String, u64>);

impl Quantity {
    pub fn zero() -> Self {
        Self::default()
    }

    pub fn single(axis: impl Into<String>, amount: u64) -> Self {
        let mut axes = BTreeMap::new();
        if amount != 0 {
            axes.insert(axis.into(), amount);
        }
        Self(axes)
    }

    pub fn from_axes(axes: impl IntoIterator<Item = (String, u64)>) -> Result<Self, FlowError> {
        let mut result = Self::zero();
        for (axis, amount) in axes {
            let sum = result
                .get(&axis)
                .checked_add(amount)
                .ok_or(FlowError::Overflow)?;
            result.set(axis, sum);
        }
        Ok(result)
    }

    pub fn get(&self, axis: &str) -> u64 {
        self.0.get(axis).copied().unwrap_or(0)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, u64)> {
        self.0.iter().map(|(axis, amount)| (axis.as_str(), *amount))
    }

    pub fn componentwise_le(&self, other: &Self) -> bool {
        self.0
            .iter()
            .all(|(axis, amount)| *amount <= other.get(axis))
    }

    fn set(&mut self, axis: String, amount: u64) {
        if amount == 0 {
            self.0.remove(&axis);
        } else {
            self.0.insert(axis, amount);
        }
    }

    fn checked_add(&self, other: &Self) -> Result<Self, FlowError> {
        let mut result = self.clone();
        for (axis, amount) in &other.0 {
            let sum = result
                .get(axis)
                .checked_add(*amount)
                .ok_or(FlowError::Overflow)?;
            result.set(axis.clone(), sum);
        }
        Ok(result)
    }

    fn checked_scale(&self, factor: u64) -> Result<Self, FlowError> {
        let mut result = Self::zero();
        for (axis, amount) in &self.0 {
            let scaled = amount.checked_mul(factor).ok_or(FlowError::Overflow)?;
            result.set(axis.clone(), scaled);
        }
        Ok(result)
    }

    fn join(&self, other: &Self) -> Self {
        let mut result = self.clone();
        for (axis, amount) in &other.0 {
            result.set(axis.clone(), result.get(axis).max(*amount));
        }
        result
    }
}

impl From<&Footprint> for Quantity {
    fn from(footprint: &Footprint) -> Self {
        Self(
            footprint
                .iter()
                .map(|(axis, amount)| (axis.to_owned(), amount))
                .collect(),
        )
    }
}

/// The checker's internal signed change in live resources.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NetChange(BTreeMap<String, i128>);

impl NetChange {
    pub fn zero() -> Self {
        Self::default()
    }

    pub fn get(&self, axis: &str) -> i128 {
        self.0.get(axis).copied().unwrap_or(0)
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, i128)> {
        self.0.iter().map(|(axis, amount)| (axis.as_str(), *amount))
    }

    fn from_quantity(quantity: &Quantity, negative: bool) -> Self {
        Self(
            quantity
                .0
                .iter()
                .map(|(axis, amount)| {
                    let value = i128::from(*amount);
                    (axis.clone(), if negative { -value } else { value })
                })
                .collect(),
        )
    }

    fn checked_add(&self, other: &Self) -> Result<Self, FlowError> {
        let mut result = self.clone();
        for (axis, amount) in &other.0 {
            let sum = result
                .get(axis)
                .checked_add(*amount)
                .ok_or(FlowError::Overflow)?;
            if sum == 0 {
                result.0.remove(axis);
            } else {
                result.0.insert(axis.clone(), sum);
            }
        }
        Ok(result)
    }

    fn shift_peak(&self, peak: &Quantity) -> Result<Quantity, FlowError> {
        let mut axes = self.0.keys().cloned().collect::<Vec<_>>();
        axes.extend(
            peak.0
                .keys()
                .filter(|axis| !self.0.contains_key(*axis))
                .cloned(),
        );
        let mut result = Quantity::zero();
        for axis in axes {
            let shifted = self
                .get(&axis)
                .checked_add(i128::from(peak.get(&axis)))
                .ok_or(FlowError::Overflow)?;
            if shifted > 0 {
                result.set(
                    axis,
                    u64::try_from(shifted).map_err(|_| FlowError::Overflow)?,
                );
            }
        }
        Ok(result)
    }

    /// Entry quantity needed for `required` after applying this signed change:
    /// `max(0, required - self)` componentwise.
    fn shift_requirement(&self, required: &Quantity) -> Result<Quantity, FlowError> {
        let mut result = Quantity::zero();
        for (axis, amount) in &required.0 {
            let shifted = i128::from(*amount)
                .checked_sub(self.get(axis))
                .ok_or(FlowError::Overflow)?;
            if shifted > 0 {
                result.set(
                    axis.clone(),
                    u64::try_from(shifted).map_err(|_| FlowError::Overflow)?,
                );
            }
        }
        Ok(result)
    }
}

/// Static resource behavior relative to a path's entry state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Flow {
    net: NetChange,
    /// Minimum live quantity required at entry for the flow not to underflow.
    required: Quantity,
    peak: Quantity,
    total: Quantity,
}

impl Flow {
    pub fn zero() -> Self {
        Self::default()
    }

    pub fn acquire(footprint: Quantity, acquisition_cost: Quantity) -> Self {
        Self {
            net: NetChange::from_quantity(&footprint, false),
            required: Quantity::zero(),
            peak: footprint,
            total: acquisition_cost,
        }
    }

    pub fn release(footprint: Quantity, release_cost: Quantity) -> Self {
        Self {
            net: NetChange::from_quantity(&footprint, true),
            required: footprint,
            peak: Quantity::zero(),
            total: release_cost,
        }
    }

    pub fn net(&self) -> &NetChange {
        &self.net
    }

    pub fn peak(&self) -> &Quantity {
        &self.peak
    }

    pub fn required_entry(&self) -> &Quantity {
        &self.required
    }

    pub fn total(&self) -> &Quantity {
        &self.total
    }

    /// Whether this flow is safely over-approximated by `summary`.
    ///
    /// Net ownership change is exact, while peak and cumulative quantities
    /// may be widened.  Cycle 11 uses this as the recursive-summary
    /// post-fixed-point order.
    pub fn is_covered_by(&self, summary: &Self) -> bool {
        self.net == summary.net
            && self.required.componentwise_le(&summary.required)
            && self.peak.componentwise_le(&summary.peak)
            && self.total.componentwise_le(&summary.total)
    }

    /// Sequential composition: `(d1+d2, max(p1, d1+p2), t1+t2)`.
    pub fn then(&self, next: &Self) -> Result<Self, FlowError> {
        Ok(Self {
            net: self.net.checked_add(&next.net)?,
            required: self
                .required
                .join(&self.net.shift_requirement(&next.required)?),
            peak: self.peak.join(&self.net.shift_peak(&next.peak)?),
            total: self.total.checked_add(&next.total)?,
        })
    }

    /// Joins mutually exclusive paths. Their net changes must agree because an
    /// ownership merge may not invent an optional live resource.
    pub fn join_branches(&self, other: &Self) -> Result<Self, FlowError> {
        if self.net != other.net {
            return Err(FlowError::BranchNetMismatch);
        }
        Ok(Self {
            net: self.net.clone(),
            required: self.required.join(&other.required),
            peak: self.peak.join(&other.peak),
            total: self.total.join(&other.total),
        })
    }

    /// Summary for at most `bound` iterations of a ledger-preserving body.
    pub fn bounded_zero_net_loop(&self, bound: u64) -> Result<Self, FlowError> {
        if !self.net.is_zero() {
            return Err(FlowError::NonZeroLoopNet);
        }
        Ok(Self {
            net: NetChange::zero(),
            required: self.required.clone(),
            peak: self.peak.clone(),
            total: self.total.checked_scale(bound)?,
        })
    }

    /// Exact summary for exactly `iterations` executions of an arbitrary body.
    ///
    /// Unlike `bounded_zero_net_loop`, this accepts positive and negative net
    /// changes.  `required_entry` records the invariant needed to make every
    /// release in every iteration defined.  Exponentiation by squaring keeps a
    /// very large checked bound from making the checker itself linear in it.
    pub fn repeat_exact(&self, iterations: u64) -> Result<Self, FlowError> {
        let mut remaining = iterations;
        let mut result = Self::zero();
        let mut power = self.clone();
        while remaining != 0 {
            if remaining & 1 == 1 {
                result = result.then(&power)?;
            }
            remaining >>= 1;
            if remaining != 0 {
                power = power.then(&power)?;
            }
        }
        Ok(result)
    }

    /// Historical spelling for exact repetition; this is not an at-most bound.
    /// Use `bounded_zero_net_loop` for supported maximum-count summaries.
    pub fn bounded_loop(&self, iterations: u64) -> Result<Self, FlowError> {
        self.repeat_exact(iterations)
    }
}

/// A checked, reusable function resource summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionSummary {
    flow: Flow,
}

impl FunctionSummary {
    pub fn new(flow: Flow) -> Self {
        Self { flow }
    }

    pub fn instantiate(&self) -> Flow {
        self.flow.clone()
    }
}

/// Declared componentwise resource bounds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceBounds {
    peak: Quantity,
    total: Quantity,
}

impl ResourceBounds {
    pub fn new(peak: Quantity, total: Quantity) -> Self {
        Self { peak, total }
    }

    pub fn accepts(&self, flow: &Flow) -> bool {
        flow.peak.componentwise_le(&self.peak) && flow.total.componentwise_le(&self.total)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlowError {
    Overflow,
    BranchNetMismatch,
    NonZeroLoopNet,
    RuntimeUnderflow(String),
}

impl fmt::Display for FlowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => f.write_str("resource-flow arithmetic overflow"),
            Self::BranchNetMismatch => {
                f.write_str("conditional branches have different net resource changes")
            }
            Self::NonZeroLoopNet => f.write_str("bounded loop body must have zero net change"),
            Self::RuntimeUnderflow(axis) => {
                write!(f, "runtime resource trace underflows axis `{axis}`")
            }
        }
    }
}

impl std::error::Error for FlowError {}

/// Interprets a sequence of primitive or opaque summarized operations and
/// records the resulting conservative peak and total trace.
pub fn interpret_flow_trace(operations: &[Flow]) -> Result<Flow, FlowError> {
    let mut live = Quantity::zero();
    let mut peak = Quantity::zero();
    let mut total = Quantity::zero();
    for primitive in operations {
        let live_at_entry = NetChange::from_quantity(&live, false);
        peak = peak.join(&live_at_entry.shift_peak(&primitive.peak)?);
        for (axis, delta) in &primitive.net.0 {
            let current = i128::from(live.get(axis));
            let next = current.checked_add(*delta).ok_or(FlowError::Overflow)?;
            if next < 0 {
                return Err(FlowError::RuntimeUnderflow(axis.clone()));
            }
            live.set(
                axis.clone(),
                u64::try_from(next).map_err(|_| FlowError::Overflow)?,
            );
        }
        total = total.checked_add(&primitive.total)?;
    }
    Ok(Flow {
        net: NetChange::from_quantity(&live, false),
        required: Quantity::zero(),
        peak,
        total,
    })
}

#[cfg(test)]
#[path = "../tests/unit/resource_flow.rs"]
mod tests;
