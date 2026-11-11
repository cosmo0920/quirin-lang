-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

import QCore.PackageIdentity

-- Card P01-F3: the package-identity state machine from load to dispatch. Model
-- of `crates/qcore-verifier-host/src/executable_identity.rs` and of the
-- construction discipline that makes `ProcessProvider` reachable only through
-- `verify_executable_package`. Threat model, refusal matrix and the *measured*
-- residual window are in `docs/reports/remaining-p01-f3/README.md`.

-- What acceptance decides, in both directions.
#print axioms QCore.PackageIdentity.revalidate_ok_iff
#print axioms QCore.PackageIdentity.ok_loads_the_rechecked_content
#print axioms QCore.PackageIdentity.ok_loads_content_of_the_verified_digest
#print axioms QCore.PackageIdentity.ok_loads_the_verified_bytes

-- The refusals: rewritten in place, replaced, byte-identically replaced, removed.
#print axioms QCore.PackageIdentity.refusals_are_not_acceptance
#print axioms QCore.PackageIdentity.an_in_place_rewrite_is_refused
#print axioms QCore.PackageIdentity.a_replacement_is_refused
#print axioms QCore.PackageIdentity.a_byte_identical_replacement_is_refused
#print axioms QCore.PackageIdentity.a_removed_executable_is_refused
#print axioms QCore.PackageIdentity.no_substitution_is_admitted
#print axioms QCore.PackageIdentity.both_refusals_are_reachable

-- The load-to-dispatch machine: no path from unverified to dispatched.
#print axioms QCore.PackageIdentity.no_dispatch_from_unverified
#print axioms QCore.PackageIdentity.dispatch_predecessor_is_verified
#print axioms QCore.PackageIdentity.unverified_is_stable
#print axioms QCore.PackageIdentity.never_dispatched_without_an_accepted_verification
#print axioms QCore.PackageIdentity.a_dispatch_requires_an_accepted_verification
#print axioms QCore.PackageIdentity.a_refused_verification_does_not_verify
#print axioms QCore.PackageIdentity.a_refusal_never_dispatches
#print axioms QCore.PackageIdentity.dispatch_carries_its_verification
#print axioms QCore.PackageIdentity.dispatch_iff_ok
#print axioms QCore.PackageIdentity.refusal_and_dispatch_are_exclusive

-- Non-vacuity, the frame property, and the residual this gate does not close.
#print axioms QCore.PackageIdentity.an_unchanged_package_dispatches
#print axioms QCore.PackageIdentity.revalidate_depends_only_on_the_two_observations
#print axioms QCore.PackageIdentity.a_post_check_substitution_is_not_refused
